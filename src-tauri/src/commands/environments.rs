//! Domaine « environnements » : fichiers du projet et valeurs locales (N1 à N6).

use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::environment::{self, Environment};
use crate::error::CommandError;
use crate::state::AppState;
use crate::workbench::{EnvironmentsView, Workbench};

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
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    name: String,
) -> Result<EnvironmentsView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.select_environment(&ctx, &name)?;
    Ok(bench.environments(&ctx))
}

#[tauri::command]
pub fn environment_save(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: Environment,
    previous: Option<String>,
) -> Result<EnvironmentsView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.save_environment(&ctx, &environment, previous.as_deref())?;
    Ok(bench.environments(&ctx))
}

#[tauri::command]
pub fn environment_delete(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    name: String,
) -> Result<EnvironmentsView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.delete_environment(&ctx, &name)?;
    Ok(bench.environments(&ctx))
}

#[tauri::command]
pub fn environments_reorder(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    names: Vec<String>,
) -> Result<EnvironmentsView, CommandError> {
    let ctx = state.project_ctx(project)?;
    environment::reorder(&ctx.root, &names)?;
    Ok(bench.environments(&ctx))
}

/// Renseigne une variable locale ; un secret va dans le coffre du système (R9).
#[tauri::command]
pub fn local_set(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    name: String,
    value: String,
    secret: bool,
) -> Result<EnvironmentsView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.set_local(&ctx, &environment, &name, &value, secret)?;
    Ok(bench.environments(&ctx))
}
