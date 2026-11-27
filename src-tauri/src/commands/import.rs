//! Domaine « import » : reconnaître un dossier ou un fichier, l'importer dans
//! un nouveau projet ou dans un projet, relire le rapport d'import
//! (docs/plan-mvp-c-import-bruno.md). La logique est dans [`crate::import`].

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::import::{
    self, ImportDone, ImportOptions, ImportReport, ImportSource, ImportTarget, ScriptChoice,
    ScriptResolved, ScriptUndo,
};
use crate::state::AppState;
use crate::tree::undo::UndoBook;
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

/// Importe `path` dans un nouveau projet ou dans un projet existant (R1 à R10), avec les choix
/// de la fenêtre « Importer » : environnements et secrets (AM41).
#[tauri::command]
pub fn import_run(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    path: String,
    target: ImportTarget,
    options: ImportOptions,
) -> Result<ImportDone, CommandError> {
    let done = import::run(
        &state,
        &PathBuf::from(path),
        &target,
        &options,
        &|ctx, secret| bench.set_local(ctx, &secret.environment, &secret.name, &secret.value, true),
    )?;
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

/// Scripts repris récemment, à remettre (« Annuler »).
static SCRIPTS: Mutex<UndoBook<ScriptUndo>> = Mutex::new(UndoBook::new());

/// Reprend un script Bruno du rapport (2.3-f, AM42) : « Accepter sans ce script » ou
/// « Remplacer par une assertion… » le retirent du fichier ; l'élément est marqué repris.
#[tauri::command]
pub fn import_script(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: Uuid,
    index: usize,
    choice: ScriptChoice,
) -> Result<ScriptResolved, CommandError> {
    let ctx = state.project_ctx(project_id)?;
    let (report, undo) =
        import::resolve_script(state.data_dir(), &ctx.root, project_id, index, choice)?;
    let request = report
        .items
        .get(index)
        .and_then(|item| item.request.clone());
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project_id,
        ProjectScope::Tree,
        request.as_deref(),
    );
    let undo_token = SCRIPTS
        .lock()
        .map(|mut book| book.push(undo))
        .unwrap_or_else(|_| Uuid::new_v4());
    Ok(ScriptResolved { report, undo_token })
}

/// « Annuler » un script repris : il revient dans son fichier, l'élément redevient « non repris ».
#[tauri::command]
pub fn import_script_undo(
    window: WebviewWindow,
    state: State<'_, AppState>,
    undo_token: Uuid,
) -> Result<ImportReport, CommandError> {
    let entry = SCRIPTS
        .lock()
        .ok()
        .and_then(|mut book| book.take(undo_token));
    let Some(undo) = entry else {
        return Err(CommandError::new(
            CommandErrorKind::UndoExpired,
            "Ce script ne peut plus être remis.",
        ));
    };
    let report = import::undo_script(state.data_dir(), &undo)?;
    let request = report
        .items
        .get(undo.index)
        .and_then(|item| item.request.clone());
    windows::project_changed(
        window.app_handle(),
        window.label(),
        undo.project,
        ProjectScope::Tree,
        request.as_deref(),
    );
    Ok(report)
}
