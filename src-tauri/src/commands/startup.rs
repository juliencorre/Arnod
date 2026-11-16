//! Domaine « démarrage » (lot G) : fenêtre montrée prête, erreur bloquante expliquée et ses
//! issues. Chaque issue ne détruit rien : un fichier mis de côté est renommé, jamais supprimé.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, Runtime, State, WebviewWindow};

use crate::error::{CommandError, CommandErrorKind};
use crate::startup::{self, Env, Failure, ProblemNature, StartupProblem, StartupTarget, StateFile};
use crate::state::AppState;
use crate::windows::{self as native, WindowsState};

/// Essais automatiques déjà faits pour l'échec en cours (R4).
#[derive(Default)]
pub struct StartupAttempts(Mutex<(Option<ProblemNature>, u32)>);

impl StartupAttempts {
    /// Essais faits pour `nature` ; un autre échec repart de zéro.
    fn count(&self, nature: ProblemNature) -> u32 {
        let Ok(mut done) = self.0.lock() else {
            return 0;
        };
        if done.0 != Some(nature) {
            *done = (Some(nature), 0);
        }
        done.1
    }

    fn add(&self, nature: ProblemNature) {
        if let Ok(mut done) = self.0.lock()
            && done.0 == Some(nature)
        {
            done.1 += 1;
        }
    }
}

/// Premier échec en cours : l'état d'abord, puis les fenêtres.
fn current<R: Runtime>(app: &AppHandle<R>) -> Option<Failure> {
    app.state::<AppState>()
        .failure()
        .or_else(|| app.state::<WindowsState>().failure())
}

fn env() -> Env {
    Env {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        webview: tauri::webview_version().ok(),
        now: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default(),
    }
}

fn describe_current<R: Runtime>(app: &AppHandle<R>) -> Option<StartupProblem> {
    let failure = current(app)?;
    let attempts = app.state::<StartupAttempts>().count(failure.nature());
    Some(startup::describe(&failure, &env(), attempts))
}

/// Relit ce qui a échoué ; tout réglé, recrée les fenêtres détachées enregistrées (F13).
fn reload_all<R: Runtime>(app: &AppHandle<R>) -> Option<StartupProblem> {
    let was_blocked = current(app).is_some();
    if app.state::<AppState>().reload().is_none() {
        app.state::<WindowsState>().reload();
    }
    let problem = describe_current(app);
    if problem.is_none() && was_blocked {
        native::restore(app);
    }
    problem
}

/// Erreur bloquante du démarrage, décrite ; `null` si Arnod peut démarrer.
#[tauri::command]
pub fn startup_status(app: AppHandle) -> Option<StartupProblem> {
    describe_current(&app)
}

/// Relit le fichier en échec (« Réessayer », ou essai automatique d'un verrou, R4).
#[tauri::command]
pub fn startup_retry(
    app: AppHandle,
    attempts: State<'_, StartupAttempts>,
    automatic: bool,
) -> Option<StartupProblem> {
    if automatic && let Some(failure) = current(&app) {
        attempts.add(failure.nature());
    }
    reload_all(&app)
}

/// « Repartir d'un état vide » : met de côté le fichier en échec, renommé et intact, puis
/// relit (R3). Le nom choisi est notifié au premier `catalog_load`.
#[tauri::command]
pub fn startup_start_empty(app: AppHandle) -> Result<Option<StartupProblem>, CommandError> {
    let failure = current(&app).ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::NotFound,
            "Aucune erreur de démarrage en cours.",
        )
    })?;
    let suffix = failure.aside_suffix().ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Ce fichier ne se met pas de côté : réessayez une fois le verrou levé.",
        )
    })?;
    let aside = startup::set_aside(&failure.path, &suffix)?;
    let name = aside
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let notice = match failure.file {
        StateFile::Catalog => format!(
            "L'ancien état est gardé sous {name}, dans le dossier de données ; rouvrez vos \
             projets avec « Ouvrir un projet »."
        ),
        StateFile::Windows => format!(
            "L'ancienne disposition des fenêtres est gardée sous {name}, dans le dossier de \
             données."
        ),
    };
    let problem = reload_all(&app);
    if problem.is_none() {
        let kind = match failure.cause {
            startup::Cause::Newer { .. } => CommandErrorKind::NewerVersion,
            _ => CommandErrorKind::CorruptState,
        };
        app.state::<AppState>()
            .set_warning(CommandError::new(kind, notice));
        // Un état vide : les onglets des projets qu'il ne connaît plus se ferment (F11).
        if let Ok(snapshot) = app.state::<AppState>().snapshot() {
            native::catalog_changed(&app, &snapshot);
        }
    }
    Ok(problem)
}

/// Ouvre le dossier de données, la page des versions ou l'issue préremplie avec le programme
/// du système. Rien n'est envoyé : l'issue s'ouvre dans le navigateur, à relire (R6).
#[tauri::command]
pub fn startup_open(app: AppHandle, target: StartupTarget) -> Result<(), CommandError> {
    let unavailable = || {
        CommandError::new(
            CommandErrorKind::NotFound,
            "Aucune erreur de démarrage en cours.",
        )
    };
    let failure = current(&app).ok_or_else(unavailable)?;
    let address = match target {
        StartupTarget::Folder => {
            let folder = if failure.path.is_dir() {
                failure.path.clone()
            } else {
                failure
                    .path
                    .parent()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or_default()
            };
            folder.display().to_string()
        }
        StartupTarget::Releases => format!("{}/releases", startup::REPOSITORY),
        StartupTarget::Issue => describe_current(&app).ok_or_else(unavailable)?.issue_url,
    };
    startup::open_external(&address).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Ouverture impossible ({address}) : {error}"),
        )
    })
}

/// « Quitter » : ferme Arnod sans rien écrire.
#[tauri::command]
pub fn startup_quit(app: AppHandle) {
    app.exit(0);
}

/// L'interface est prête (ou montre une erreur) : la fenêtre, créée cachée, apparaît (0.3-a).
#[tauri::command]
pub fn window_ready<R: Runtime>(window: WebviewWindow<R>) -> Result<(), CommandError> {
    window.show().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Affichage de la fenêtre impossible : {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essais_comptes_par_nature() {
        let attempts = StartupAttempts::default();
        assert_eq!(attempts.count(ProblemNature::Locked), 0);
        attempts.add(ProblemNature::Locked);
        attempts.add(ProblemNature::Locked);
        assert_eq!(attempts.count(ProblemNature::Locked), 2);
        // Un autre échec repart de zéro.
        assert_eq!(attempts.count(ProblemNature::Inaccessible), 0);
        attempts.add(ProblemNature::Locked);
        assert_eq!(attempts.count(ProblemNature::Inaccessible), 0);
    }
}
