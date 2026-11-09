//! État partagé de l'application, confié à Tauri par `app.manage`.
//!
//! Relie le catalogue (règles pures), le stockage (`arnod.json`), le dossier
//! des projets et la pile d'annulation. Chaque modification s'applique sur une
//! copie du catalogue, est écrite sur disque, puis remplace l'état : si
//! l'écriture échoue, rien ne change.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use uuid::Uuid;

use crate::catalog::snapshot::{
    CatalogLoad, CatalogSnapshot, ProjectCreated, ProjectStatus, ProjectView, Undoable,
};
use crate::catalog::undo::{UndoAction, UndoStack};
use crate::catalog::{Catalog, names};
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::manifest::{self, Manifest};
use crate::storage::{self, Loaded};
use crate::workbench::ProjectCtx;

/// Dossier, dans le dossier personnel, où les projets sont créés par défaut.
pub const DEFAULT_LOCATION_DIR: &str = "arnod";

/// État partagé entre les commandes.
pub struct AppState {
    /// Dossier de données qui contient `arnod.json`.
    data_dir: PathBuf,
    /// Emplacement proposé pour les nouveaux projets (`~/arnod`).
    default_location: PathBuf,
    inner: Mutex<Inner>,
    /// Prévenu après chaque modification enregistrée : la synchronisation des
    /// fenêtres (F11) diffuse l'instantané. Absent dans les tests.
    listener: OnceLock<ChangeListener>,
}

/// Fonction prévenue à chaque nouvel instantané du catalogue.
pub type ChangeListener = Box<dyn Fn(&CatalogSnapshot) + Send + Sync>;

struct Inner {
    catalog: Catalog,
    undo: UndoStack,
    /// Erreur qui interdit toute lecture et écriture, par exemple un fichier
    /// d'état écrit par une version plus récente d'Arnod.
    blocked: Option<CommandError>,
    /// Avertissement de démarrage, renvoyé une seule fois par `catalog_load`.
    warning: Option<CommandError>,
}

impl AppState {
    /// Charge l'état depuis `data_dir`. N'échoue jamais : une erreur bloquante
    /// est mémorisée et renvoyée par chaque commande.
    pub fn load(data_dir: PathBuf, default_location: PathBuf) -> Self {
        let (catalog, blocked, warning) = match storage::load(&data_dir) {
            Ok(Loaded::Existing(catalog)) => (catalog, None, None),
            Ok(Loaded::FirstLaunch(catalog)) => {
                // Enregistré tout de suite pour que les identifiants restent stables ;
                // en cas d'échec, la prochaine modification signalera l'erreur.
                let _ = storage::save(&data_dir, &catalog);
                (catalog, None, None)
            }
            Ok(Loaded::Recovered {
                catalog,
                backup,
                reason,
            }) => {
                let _ = storage::save(&data_dir, &catalog);
                let warning = CommandError::new(
                    CommandErrorKind::CorruptState,
                    format!(
                        "L'état d'Arnod était illisible ({reason}). Il a été mis de côté sous {} \
                         et Arnod repart d'un état neuf.",
                        backup.display()
                    ),
                );
                (catalog, None, Some(warning))
            }
            Err(error) => (Catalog::first_launch(), Some(error), None),
        };
        Self {
            data_dir,
            default_location,
            inner: Mutex::new(Inner {
                catalog,
                undo: UndoStack::default(),
                blocked,
                warning,
            }),
            listener: OnceLock::new(),
        }
    }

    /// Instantané du démarrage, avec l'avertissement éventuel (une seule fois).
    pub fn load_catalog(&self) -> Result<CatalogLoad, CommandError> {
        let mut inner = self.lock()?;
        Ok(CatalogLoad {
            snapshot: snapshot_of(&inner.catalog),
            warning: inner.warning.take(),
        })
    }

    /// Instantané courant, sans consommer l'avertissement de démarrage.
    pub fn snapshot(&self) -> Result<CatalogSnapshot, CommandError> {
        Ok(snapshot_of(&self.lock()?.catalog))
    }

    /// Applique une modification du catalogue et l'enregistre.
    pub fn mutate<T>(
        &self,
        change: impl FnOnce(&mut Catalog) -> Result<T, CommandError>,
    ) -> Result<(T, CatalogSnapshot), CommandError> {
        let mut inner = self.lock()?;
        let mut next = inner.catalog.clone();
        let value = change(&mut next)?;
        let snapshot = self.commit(&mut inner, next)?;
        Ok((value, snapshot))
    }

    /// Applique une modification annulable et renvoie son jeton d'annulation (R6).
    pub fn mutate_undoable(
        &self,
        change: impl FnOnce(&mut Catalog) -> Result<UndoAction, CommandError>,
    ) -> Result<Undoable, CommandError> {
        let mut inner = self.lock()?;
        let mut next = inner.catalog.clone();
        let action = change(&mut next)?;
        let snapshot = self.commit(&mut inner, next)?;
        let undo_token = inner.undo.push(action);
        Ok(Undoable {
            snapshot,
            undo_token,
        })
    }

    /// Annule l'action associée au jeton.
    pub fn undo(&self, token: Uuid) -> Result<CatalogSnapshot, CommandError> {
        let mut inner = self.lock()?;
        let action = inner.undo.get(token)?.clone();
        let mut next = inner.catalog.clone();
        next.undo(action)?;
        let snapshot = self.commit(&mut inner, next)?;
        inner.undo.remove(token);
        Ok(snapshot)
    }

    /// Dossier proposé pour un projet : `<emplacement>/<nom>`, l'emplacement
    /// valant `~/arnod` par défaut. Aperçu sans validation du nom.
    pub fn suggest_folder(&self, name: &str, location: Option<&str>) -> String {
        let location = location.map_or_else(|| self.default_location.clone(), PathBuf::from);
        let name = name.trim();
        let folder = if name.is_empty() {
            location
        } else {
            location.join(name)
        };
        folder.display().to_string()
    }

    /// Crée un projet : dossier et manifeste sur disque, puis inscription au
    /// référentiel (P1, P2, R1, R4, R9, R10). Le dossier créé est supprimé si
    /// l'inscription échoue.
    pub fn create_project(
        &self,
        raw_name: &str,
        location: Option<&str>,
        workspace_id: Option<Uuid>,
    ) -> Result<ProjectCreated, CommandError> {
        let location = location.map_or_else(|| self.default_location.clone(), PathBuf::from);
        if !location.is_absolute() {
            return Err(CommandError::new(
                CommandErrorKind::Io,
                format!(
                    "L'emplacement du projet doit être un chemin absolu : {}.",
                    location.display()
                ),
            ));
        }
        let mut inner = self.lock()?;

        // Toutes les règles sont vérifiées avant de toucher au disque.
        let name = names::project_name(raw_name)?;
        let folder = location.join(&name);
        inner
            .catalog
            .check_new_project(&name, &canonical(&folder), workspace_id)?;

        let project_id = Uuid::new_v4();
        let created = manifest::create_project_folder(&folder, &Manifest::new(project_id, &name))?;
        // Chaque projet démarre avec un environnement « Local » (N1).
        if let Err(error) = environment::init(created.path()) {
            created.rollback();
            return Err(error);
        }
        let mut next = inner.catalog.clone();
        let registered = next
            .register_project(project_id, &name, canonical(created.path()), workspace_id)
            .and_then(|()| self.commit(&mut inner, next));
        match registered {
            Ok(snapshot) => Ok(ProjectCreated {
                snapshot,
                project_id,
            }),
            Err(error) => {
                created.rollback();
                Err(error)
            }
        }
    }

    /// Inscrit dans « Tous les projets » un dossier projet existant, reconnu par
    /// sa racine `opencollection.yml` (P12, après un clone Git par exemple ; une
    /// collection Bruno 3 aussi, S5), et l'ajoute à `workspace_id` s'il est
    /// donné (R4). Seules écritures possibles : la conversion d'un projet au
    /// format précédent (R6), l'identifiant Arnod d'une collection qui n'en a pas (Q4).
    pub fn add_existing_project(
        &self,
        path: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<ProjectCreated, CommandError> {
        let folder = PathBuf::from(path);
        if !folder.is_absolute() {
            return Err(CommandError::new(
                CommandErrorKind::Io,
                format!(
                    "Le dossier doit être un chemin absolu : {}.",
                    folder.display()
                ),
            ));
        }
        let mut inner = self.lock()?;
        let manifest = manifest::read_manifest(&folder)?;
        if manifest.version > manifest::MANIFEST_VERSION {
            return Err(CommandError::new(
                CommandErrorKind::NewerVersion,
                format!(
                    "Le projet « {} » a été créé par une version plus récente d'Arnod (format {}).",
                    manifest.name, manifest.version
                ),
            ));
        }
        let mut next = inner.catalog.clone();
        next.register_project(
            manifest.id,
            &manifest.name,
            canonical(&folder),
            workspace_id,
        )?;
        let snapshot = self.commit(&mut inner, next)?;
        Ok(ProjectCreated {
            snapshot,
            project_id: manifest.id,
        })
    }

    /// Projet à ouvrir (P5). Refusé si son dossier est introuvable (P10).
    pub fn open_project(&self, project_id: Uuid) -> Result<ProjectView, CommandError> {
        let inner = self.lock()?;
        inner.catalog.project(project_id)?;
        let project = snapshot_of(&inner.catalog)
            .projects
            .into_iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| {
                CommandError::new(
                    CommandErrorKind::NotFound,
                    "Ce projet n'est plus dans « Tous les projets ».",
                )
            })?;
        if project.status == ProjectStatus::Missing {
            return Err(CommandError::new(
                CommandErrorKind::NotFound,
                format!(
                    "Le dossier du projet « {} » est introuvable : {}. Il a peut-être été déplacé ou supprimé.",
                    project.name, project.path
                ),
            ));
        }
        Ok(project)
    }

    /// Dossier de données de l'application (`arnod.json`, données locales des projets).
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Dossier et données locales d'un projet dont le dossier existe encore.
    pub fn project_ctx(&self, project_id: Uuid) -> Result<ProjectCtx, CommandError> {
        let inner = self.lock()?;
        let project = inner.catalog.project(project_id)?;
        if !project.path.is_dir() {
            return Err(CommandError::new(
                CommandErrorKind::NotFound,
                format!(
                    "Le dossier du projet « {} » est introuvable : {}.",
                    project.name,
                    project.path.display()
                ),
            ));
        }
        // Projet au format précédent : converti en OpenCollection, une fois (R6).
        crate::opencollection::migrate::run(&project.path)?;
        Ok(ProjectCtx {
            id: project_id,
            root: project.path.clone(),
            data_dir: self.data_dir.clone(),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Inner>, CommandError> {
        let inner = self.inner.lock().map_err(|_| {
            CommandError::new(
                CommandErrorKind::Io,
                "L'état d'Arnod est indisponible : redémarrez l'application.",
            )
        })?;
        match &inner.blocked {
            Some(blocked) => Err(blocked.clone()),
            None => Ok(inner),
        }
    }

    /// Enregistre `next` puis le substitue à l'état courant.
    fn commit(&self, inner: &mut Inner, next: Catalog) -> Result<CatalogSnapshot, CommandError> {
        storage::save(&self.data_dir, &next)?;
        inner.catalog = next;
        let snapshot = snapshot_of(&inner.catalog);
        if let Some(listener) = self.listener.get() {
            listener(&snapshot);
        }
        Ok(snapshot)
    }

    /// Fonction prévenue après chaque modification du catalogue (une seule).
    pub fn on_change(&self, listener: ChangeListener) {
        let _ = self.listener.set(listener);
    }
}

fn snapshot_of(catalog: &Catalog) -> CatalogSnapshot {
    catalog.snapshot(Path::is_dir)
}

/// Chemin canonique s'il existe (sans préfixe `\\?\` sous Windows), sinon tel quel.
fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests;
