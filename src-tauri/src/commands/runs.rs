//! Domaine « runs » : exécution d'un dossier de requêtes (X1 à X8).

use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::error::CommandError;
use crate::state::AppState;
use crate::workbench::{RunView, Workbench};

#[tauri::command]
pub fn run_start(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    folder: String,
    environment: String,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.start_run(&ctx, &folder, &environment, None, confirmed)
}

#[tauri::command]
pub fn run_status(bench: State<'_, Arc<Workbench>>, run: Uuid) -> Result<RunView, CommandError> {
    bench.run_status(run)
}

#[tauri::command]
pub fn run_cancel(bench: State<'_, Arc<Workbench>>, run: Uuid) -> Result<(), CommandError> {
    bench.cancel_run(run)
}

#[tauri::command]
pub fn run_retry_failures(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    run: Uuid,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.retry_failures(&ctx, run, confirmed)
}

#[tauri::command]
pub fn run_last(
    state: State<'_, AppState>,
    project: Uuid,
) -> Result<Option<RunView>, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(Workbench::last_run(&ctx))
}
