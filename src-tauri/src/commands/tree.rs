//! Domaine « arborescence » : dossiers et requêtes d'un projet (A1 à A8).

use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::error::CommandError;
use crate::local;
use crate::state::AppState;
use crate::tree::{self, TreeNode};

/// Arborescence d'un projet et dossiers repliés.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeView {
    pub nodes: Vec<TreeNode>,
    pub collapsed: Vec<String>,
}

#[tauri::command]
pub fn tree_load(state: State<'_, AppState>, project: Uuid) -> Result<TreeView, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(TreeView {
        nodes: tree::read(&ctx.root)?,
        collapsed: local::load(&ctx.data_dir, ctx.id).collapsed,
    })
}

#[tauri::command]
pub fn tree_set_collapsed(
    state: State<'_, AppState>,
    project: Uuid,
    collapsed: Vec<String>,
) -> Result<(), CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut data = local::load(&ctx.data_dir, ctx.id);
    data.collapsed = collapsed;
    local::save(&ctx.data_dir, ctx.id, &data)
}

#[tauri::command]
pub fn tree_create_folder(
    state: State<'_, AppState>,
    project: Uuid,
    parent: String,
    name: String,
) -> Result<String, CommandError> {
    tree::create_folder(&state.project_ctx(project)?.root, &parent, &name)
}

#[tauri::command]
pub fn tree_create_request(
    state: State<'_, AppState>,
    project: Uuid,
    folder: String,
    name: Option<String>,
) -> Result<String, CommandError> {
    tree::create_request(&state.project_ctx(project)?.root, &folder, name.as_deref())
}

#[tauri::command]
pub fn tree_rename(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    name: String,
) -> Result<String, CommandError> {
    tree::rename(&state.project_ctx(project)?.root, &path, &name)
}

#[tauri::command]
pub fn tree_move(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    target_folder: String,
) -> Result<String, CommandError> {
    tree::move_to(&state.project_ctx(project)?.root, &path, &target_folder)
}

#[tauri::command]
pub fn tree_duplicate(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<String, CommandError> {
    tree::duplicate(&state.project_ctx(project)?.root, &path)
}

#[tauri::command]
pub fn tree_delete(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    permanent: bool,
) -> Result<(), CommandError> {
    tree::delete(&state.project_ctx(project)?.root, &path, permanent)
}
