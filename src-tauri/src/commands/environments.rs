//! Domaine « environnements » : fichiers du projet et valeurs locales (N1 à N6).

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::environment::{self, Environment};
use crate::error::{CommandError, CommandErrorKind};
use crate::state::AppState;
use crate::tree::undo::UndoBook;
use crate::windows::{self, ProjectScope};
use crate::workbench::{EnvironmentRestore, EnvironmentsView, Workbench};

/// Environnements après une suppression, et le jeton de « Annuler » (ER1).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDeleted {
    pub view: EnvironmentsView,
    pub undo_token: Uuid,
}

/// Environnements supprimés récemment, à remettre : projet et ce qui a été retiré (secrets
/// compris, en mémoire seulement).
static DELETED: Mutex<UndoBook<(Uuid, EnvironmentRestore)>> = Mutex::new(UndoBook::new());

#[tauri::command]
pub fn environments_load(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
) -> Result<EnvironmentsView, CommandError> {
    Ok(bench.environments(&state.project_ctx(project)?))
}

#[tauri::command]
pub fn environment_select(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    name: String,
) -> Result<EnvironmentsView, CommandError> {
    let run = || {
        let ctx = state.project_ctx(project)?;
        bench.select_environment(&ctx, &name)?;
        Ok(bench.environments(&ctx))
    };
    let result = run();
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Environments,
            None,
        );
    }
    result
}

#[tauri::command]
pub fn environment_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: Environment,
    previous: Option<String>,
) -> Result<EnvironmentsView, CommandError> {
    let run = || {
        let ctx = state.project_ctx(project)?;
        bench.save_environment(&ctx, &environment, previous.as_deref())?;
        Ok(bench.environments(&ctx))
    };
    let result = run();
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Environments,
            None,
        );
    }
    result
}

#[tauri::command]
pub fn environment_delete(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    name: String,
) -> Result<EnvironmentDeleted, CommandError> {
    let ctx = state.project_ctx(project)?;
    let restore = bench.delete_environment(&ctx, &name)?;
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Environments,
        None,
    );
    let undo_token = DELETED
        .lock()
        .map(|mut book| book.push((project, restore)))
        .unwrap_or_else(|_| Uuid::new_v4());
    Ok(EnvironmentDeleted {
        view: bench.environments(&ctx),
        undo_token,
    })
}

/// « Annuler » la suppression d'un environnement : fichiers, place, valeurs locales et secrets.
#[tauri::command]
pub fn environment_delete_undo(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    undo_token: Uuid,
) -> Result<EnvironmentsView, CommandError> {
    let entry = DELETED
        .lock()
        .ok()
        .and_then(|mut book| book.take(undo_token));
    let Some((project, restore)) = entry else {
        return Err(CommandError::new(
            CommandErrorKind::UndoExpired,
            "Cette suppression ne peut plus être annulée.",
        ));
    };
    let ctx = state.project_ctx(project)?;
    bench.restore_environment(&ctx, &restore)?;
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Environments,
        None,
    );
    Ok(bench.environments(&ctx))
}

#[tauri::command]
pub fn environments_reorder(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    names: Vec<String>,
) -> Result<EnvironmentsView, CommandError> {
    let run = || {
        let ctx = state.project_ctx(project)?;
        environment::reorder(&ctx.root, &names)?;
        Ok(bench.environments(&ctx))
    };
    let result = run();
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Environments,
            None,
        );
    }
    result
}

/// Renseigne une variable locale ; un secret va dans le coffre du système (R9).
#[tauri::command]
pub fn local_set(
    window: WebviewWindow,
    project: Uuid,
    environment: String,
    name: String,
    value: String,
    secret: bool,
) -> Result<EnvironmentsView, CommandError> {
    let state = window.state::<AppState>();
    let bench = window.state::<Arc<Workbench>>();
    let run = || {
        let ctx = state.project_ctx(project)?;
        bench.set_local(&ctx, &environment, &name, &value, secret)?;
        Ok(bench.environments(&ctx))
    };
    let result = run();
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Environments,
            None,
        );
    }
    result
}
