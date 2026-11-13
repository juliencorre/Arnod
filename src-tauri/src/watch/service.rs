//! Surveillant des dossiers des projets ouverts (R1, R7).
//!
//! Un observateur `notify` par projet envoie les chemins touchés à un seul fil, qui les regroupe
//! par projet et ne les traite que 300 ms après le dernier événement ([`DEBOUNCE`]) : un `git
//! pull` ou l'enregistrement d'un éditeur (fichier temporaire, renommage) donne un seul lot.
//! Le lot est comparé à l'état connu ([`super::apply`]) et ses changements passent au [`Sink`].

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use notify::event::{AccessKind, AccessMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use uuid::Uuid;

use super::FilesChange;
use crate::error::{CommandError, CommandErrorKind};

/// Attente après le dernier événement d'un projet avant de traiter le lot.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// Reçoit les changements faits hors d'Arnod (diffusion aux fenêtres, ou enregistreur des tests).
pub trait Sink: Send + Sync {
    fn changed(&self, change: FilesChange);
}

/// Chemins touchés dans un projet.
struct Touched {
    project: Uuid,
    paths: Vec<PathBuf>,
}

struct Watched {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

/// Observateurs des projets ouverts, démarrés et arrêtés avec eux.
pub struct Watchers {
    watched: Mutex<HashMap<Uuid, Watched>>,
    sender: Mutex<Sender<Touched>>,
}

/// Événement qui peut changer un fichier : une simple lecture n'en est pas un.
fn relevant(event: &Event) -> bool {
    match event.kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        EventKind::Access(_) => false,
        _ => true,
    }
}

impl Watchers {
    /// Démarre le fil de regroupement ; `sink` reçoit chaque lot de changements.
    pub fn new(sink: Arc<dyn Sink>) -> Self {
        let (sender, receiver) = mpsc::channel::<Touched>();
        thread::Builder::new()
            .name("arnod-surveillance".to_owned())
            .spawn(move || {
                let mut pending: HashMap<Uuid, (BTreeSet<PathBuf>, Instant)> = HashMap::new();
                loop {
                    let wait = pending
                        .values()
                        .map(|(_, last)| (*last + DEBOUNCE).saturating_duration_since(Instant::now()))
                        .min()
                        .unwrap_or(Duration::from_secs(3600));
                    match receiver.recv_timeout(wait) {
                        Ok(touched) => {
                            let batch = pending
                                .entry(touched.project)
                                .or_insert_with(|| (BTreeSet::new(), Instant::now()));
                            batch.0.extend(touched.paths);
                            batch.1 = Instant::now();
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    let now = Instant::now();
                    let ready: Vec<Uuid> = pending
                        .iter()
                        .filter(|(_, (_, last))| now.duration_since(*last) >= DEBOUNCE)
                        .map(|(project, _)| *project)
                        .collect();
                    for project in ready {
                        if let Some((paths, _)) = pending.remove(&project) {
                            let paths: Vec<PathBuf> = paths.into_iter().collect();
                            if let Some(change) = super::apply(project, &paths) {
                                sink.changed(change);
                            }
                        }
                    }
                }
            })
            .ok();
        Self {
            watched: Mutex::new(HashMap::new()),
            sender: Mutex::new(sender),
        }
    }

    /// Surveille le dossier d'un projet ; sans effet s'il l'est déjà.
    pub fn watch(&self, project: Uuid, root: &Path) -> Result<(), CommandError> {
        let mut watched = self.watched.lock().map_err(|_| unavailable())?;
        if watched.get(&project).is_some_and(|w| w.root == root) {
            return Ok(());
        }
        let sender = self.sender.lock().map_err(|_| unavailable())?.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            if let Ok(event) = result
                && relevant(&event)
            {
                let _ = sender.send(Touched {
                    project,
                    paths: event.paths,
                });
            }
        })
        .map_err(|error| watch_failed(root, &error))?;
        // État connu d'abord : un événement arrivé pendant la lecture se compare à lui.
        super::add_root(project, root);
        if let Err(error) = watcher.watch(root, RecursiveMode::Recursive) {
            super::remove_root(project);
            return Err(watch_failed(root, &error));
        }
        watched.insert(
            project,
            Watched {
                root: root.to_path_buf(),
                _watcher: watcher,
            },
        );
        Ok(())
    }

    /// Arrête la surveillance d'un projet.
    pub fn unwatch(&self, project: Uuid) {
        if let Ok(mut watched) = self.watched.lock()
            && watched.remove(&project).is_some()
        {
            super::remove_root(project);
        }
    }

    /// Arrête la surveillance des projets que `keep` refuse (oubliés, dossier introuvable).
    pub fn retain(&self, keep: impl Fn(Uuid) -> bool) {
        let gone: Vec<Uuid> = match self.watched.lock() {
            Ok(watched) => watched.keys().copied().filter(|id| !keep(*id)).collect(),
            Err(_) => return,
        };
        for project in gone {
            self.unwatch(project);
        }
    }

    /// Projet surveillé.
    pub fn is_watched(&self, project: Uuid) -> bool {
        self.watched
            .lock()
            .is_ok_and(|watched| watched.contains_key(&project))
    }
}

fn unavailable() -> CommandError {
    CommandError::new(
        CommandErrorKind::Io,
        "La surveillance des fichiers est indisponible : redémarrez l'application.",
    )
}

fn watch_failed(root: &Path, error: &notify::Error) -> CommandError {
    CommandError::new(
        CommandErrorKind::Io,
        format!(
            "Le dossier {} ne peut pas être surveillé ({error}) : les modifications faites hors d'Arnod n'apparaîtront qu'à la réouverture.",
            root.display()
        ),
    )
}
