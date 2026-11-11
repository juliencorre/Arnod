//! Domaine « arborescence » : dossiers et requêtes d'un projet (A1 à A8).
//!
//! Chaque modification est diffusée aux autres fenêtres (F11) ; un renommage,
//! un déplacement ou une suppression met aussi à jour les onglets ouverts.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::contract;
use crate::error::CommandError;
use crate::layout::windows::Windows;
use crate::local;
use crate::results::ResultBook;
use crate::state::AppState;
use crate::tree::{self, TreeNode};
use crate::windows::{self, ProjectScope};

/// Arborescence d'un projet et dossiers repliés.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeView {
    pub nodes: Vec<TreeNode>,
    /// Référentiel de tests : l'arborescence sans les requêtes rangées sous un
    /// contrat (docs/plan-requetes-dans-contrats.md, R1, R5).
    pub tests: Vec<TreeNode>,
    pub collapsed: Vec<String>,
}

#[tauri::command]
pub fn tree_load(state: State<'_, AppState>, project: Uuid) -> Result<TreeView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let nodes = tree::read(&ctx.root)?;
    let tests = tree::tests_view(&nodes, &contract::file::paths(&ctx.root));
    Ok(TreeView {
        nodes,
        tests,
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

/// Prévient les fenêtres d'une modification réussie de l'arborescence.
fn tree_changed<T>(
    window: &WebviewWindow,
    project: Uuid,
    result: Result<T, CommandError>,
) -> Result<T, CommandError> {
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Tree,
            None,
        );
    }
    result
}

/// Les onglets ouverts sur `from` suivent vers `to` dans toutes les fenêtres.
fn follow_path(window: &WebviewWindow, project: Uuid, from: &str, to: &str) {
    let _ = windows::apply(window.app_handle(), |w: &mut Windows| {
        w.rename_path(project, from, to);
        Ok(())
    });
    // Le dernier résultat suit aussi la requête renommée.
    if let Ok(mut book) = window.app_handle().state::<Mutex<ResultBook>>().lock() {
        book.rename(project, from, to);
    }
}

#[tauri::command]
pub fn tree_create_folder(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    parent: String,
    name: String,
) -> Result<String, CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::create_folder(&ctx.root, &parent, &name));
    tree_changed(&window, project, result)
}

#[tauri::command]
pub fn tree_create_request(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    folder: String,
    name: Option<String>,
) -> Result<String, CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::create_request(&ctx.root, &folder, name.as_deref()));
    tree_changed(&window, project, result)
}

#[tauri::command]
pub fn tree_rename(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    name: String,
) -> Result<String, CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::rename(&ctx.root, &path, &name));
    if let Ok(renamed) = &result {
        follow_path(&window, project, &path, renamed);
    }
    tree_changed(&window, project, result)
}

#[tauri::command]
pub fn tree_move(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    target_folder: String,
) -> Result<String, CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::move_to(&ctx.root, &path, &target_folder));
    if let Ok(moved) = &result {
        follow_path(&window, project, &path, moved);
    }
    tree_changed(&window, project, result)
}

#[tauri::command]
pub fn tree_duplicate(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<String, CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::duplicate(&ctx.root, &path));
    tree_changed(&window, project, result)
}

#[tauri::command]
pub fn tree_delete(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    permanent: bool,
) -> Result<(), CommandError> {
    let result = state
        .project_ctx(project)
        .and_then(|ctx| tree::delete(&ctx.root, &path, permanent));
    if result.is_ok() {
        let _ = windows::apply(window.app_handle(), |w: &mut Windows| {
            w.close_path(project, &path);
            Ok(())
        });
    }
    tree_changed(&window, project, result)
}
