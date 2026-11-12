//! Domaine « runs » : exécution d'un dossier de requêtes (X1 à X8).

use std::sync::Arc;

use tauri::{State, WebviewWindow};
use uuid::Uuid;

use crate::error::CommandError;
use crate::state::AppState;
use crate::windows;
use crate::workbench::{ProjectCtx, ProjectLastRun, RunView, Workbench};

#[tauri::command]
pub fn run_start(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    folder: String,
    environment: String,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let run = bench.start_run(&ctx, &folder, &environment, None, confirmed)?;
    windows::run_started(&window, project, &run);
    Ok(run)
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
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    run: Uuid,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = bench.retry_failures(&ctx, run, confirmed)?;
    windows::run_started(&window, project, &view);
    Ok(view)
}

/// Ignore l'échec d'une vérification dans une ligne d'un run
/// (`docs/plan-fin-premier-usage.md`, R7).
#[tauri::command]
pub fn run_ignore_check(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    run: Uuid,
    path: String,
    check: String,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.ignore_in_run(&ctx, run, &path, &check)
}

/// Dernier run de chaque projet qui en a un, pour « Tous les projets » (O12).
#[tauri::command]
pub fn project_last_runs(state: State<'_, AppState>) -> Result<Vec<ProjectLastRun>, CommandError> {
    let snapshot = state.snapshot()?;
    Ok(snapshot
        .projects
        .iter()
        .filter_map(|project| {
            let ctx = ProjectCtx {
                id: project.id,
                root: project.path.clone().into(),
                data_dir: state.data_dir().to_path_buf(),
            };
            Workbench::last_run(&ctx).map(|run| ProjectLastRun::of(project.id, &run))
        })
        .collect())
}

#[tauri::command]
pub fn run_last(
    state: State<'_, AppState>,
    project: Uuid,
) -> Result<Option<RunView>, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(Workbench::last_run(&ctx))
}
