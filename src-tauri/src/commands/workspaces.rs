//! Domaine « workspaces » : ouverture, création, gestion et regroupement des
//! projets. Façades sur les règles de [`crate::catalog::Catalog`].

use tauri::State;
use uuid::Uuid;

use crate::catalog::model::WorkspaceColor;
use crate::catalog::snapshot::{CatalogSnapshot, Undoable, WorkspaceCreated};
use crate::error::CommandError;
use crate::state::AppState;

/// Ouvre un workspace et le mémorise pour le prochain démarrage (W2, R7).
#[tauri::command]
pub fn workspace_open(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<CatalogSnapshot, CommandError> {
    snapshot(state.mutate(|catalog| catalog.open_workspace(workspace_id)))
}

/// Crée un workspace vide (W5).
#[tauri::command]
pub fn workspace_create(
    state: State<'_, AppState>,
    name: String,
) -> Result<WorkspaceCreated, CommandError> {
    let (workspace_id, snapshot) = state.mutate(|catalog| catalog.create_workspace(&name))?;
    Ok(WorkspaceCreated {
        snapshot,
        workspace_id,
    })
}

/// Renomme un workspace (W7).
#[tauri::command]
pub fn workspace_rename(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    name: String,
) -> Result<CatalogSnapshot, CommandError> {
    snapshot(state.mutate(|catalog| catalog.rename_workspace(workspace_id, &name)))
}

/// Change la couleur d'un workspace (W8).
#[tauri::command]
pub fn workspace_set_color(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    color: WorkspaceColor,
) -> Result<CatalogSnapshot, CommandError> {
    snapshot(state.mutate(|catalog| catalog.set_workspace_color(workspace_id, color)))
}

/// Transfère le rôle de workspace par défaut (W9).
#[tauri::command]
pub fn workspace_set_default(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<CatalogSnapshot, CommandError> {
    snapshot(state.mutate(|catalog| catalog.set_default_workspace(workspace_id)))
}

/// Ajoute des projets du référentiel à un workspace (W6, P6).
#[tauri::command]
pub fn workspace_add_projects(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    project_ids: Vec<Uuid>,
) -> Result<CatalogSnapshot, CommandError> {
    snapshot(state.mutate(|catalog| catalog.add_projects(workspace_id, &project_ids)))
}

/// Retire un projet d'un workspace, sans le supprimer ; annulable (P7, R5, R6).
#[tauri::command]
pub fn workspace_remove_project(
    state: State<'_, AppState>,
    workspace_id: Uuid,
    project_id: Uuid,
) -> Result<Undoable, CommandError> {
    state.mutate_undoable(|catalog| catalog.remove_project(workspace_id, project_id))
}

/// Supprime un workspace, sans supprimer ses projets ; annulable (W11, R5, R6).
#[tauri::command]
pub fn workspace_delete(
    state: State<'_, AppState>,
    workspace_id: Uuid,
) -> Result<Undoable, CommandError> {
    state.mutate_undoable(|catalog| catalog.delete_workspace(workspace_id))
}

/// Garde l'instantané d'une modification qui ne renvoie rien d'autre.
fn snapshot(
    result: Result<((), CatalogSnapshot), CommandError>,
) -> Result<CatalogSnapshot, CommandError> {
    result.map(|((), snapshot)| snapshot)
}
