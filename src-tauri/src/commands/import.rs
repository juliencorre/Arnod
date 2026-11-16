//! Domaine « import » : reconnaître un dossier ou un fichier, l'importer dans
//! un nouveau projet ou dans un projet, relire le rapport d'import
//! (docs/plan-mvp-c-import-bruno.md). La logique est dans [`crate::import`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::import::{self, ImportDone, ImportReport, ImportSource, ImportTarget};
use crate::state::AppState;
use crate::windows::{self, ProjectScope};
use crate::workbench::Workbench;

/// Ouvre le sélecteur natif : un dossier (collection Bruno, projet) ou, avec
/// `file`, un fichier (contrat OpenAPI, `bruno.json`). `None` si l'utilisateur
/// annule. Le plugin dialog n'est appelé que depuis Rust (aucune permission WebView).
#[tauri::command]
pub async fn import_pick(
    window: WebviewWindow,
    file: bool,
) -> Result<Option<String>, CommandError> {
    let dialog = window
        .dialog()
        .file()
        .set_title(if file {
            "Choisir un fichier à importer"
        } else {
            "Choisir un dossier à importer"
        })
        .set_parent(&window);
    let picked = if file {
        dialog
            .add_filter(
                "Contrat OpenAPI, collection Bruno",
                &["yaml", "yml", "json", "bru"],
            )
            .blocking_pick_file()
    } else {
        dialog.blocking_pick_folder()
    };
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le chemin choisi est inutilisable : {error}"),
        )
    })?;
    Ok(Some(path.display().to_string()))
}

/// Reconnaît le format de `path`, sans rien écrire (R2).
#[tauri::command]
pub fn import_detect(path: String) -> Result<ImportSource, CommandError> {
    import::detect(Path::new(&path))
}

/// Importe `path` dans un nouveau projet ou dans un projet existant (R1 à R10).
#[tauri::command]
pub fn import_run(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    path: String,
    target: ImportTarget,
) -> Result<ImportDone, CommandError> {
    let done = import::run(&state, &PathBuf::from(path), &target, &|ctx, secret| {
        bench.set_local(ctx, &secret.environment, &secret.name, &secret.value, true)
    })?;
    if let ImportTarget::Project { project_id } = target {
        for scope in [
            ProjectScope::Tree,
            ProjectScope::Environments,
            ProjectScope::Contracts,
        ] {
            windows::project_changed(window.app_handle(), window.label(), project_id, scope, None);
        }
    }
    Ok(done)
}

/// Dernier rapport d'import d'un projet (2.3-f), s'il y en a un.
#[tauri::command]
pub fn import_report(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> Result<Option<ImportReport>, CommandError> {
    state.project_ctx(project_id)?;
    Ok(import::load_report(state.data_dir(), project_id))
}
