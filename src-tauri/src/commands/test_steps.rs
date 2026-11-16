//! Domaine « tests et étapes » (`docs/plan-mvp-e-tests-etapes.md`) : onglet
//! d'un test, nouveau test, « + Étape », variables lues par les autres requêtes.

use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::CommandError;
use crate::state::AppState;
use crate::tree::{self, CreatedTest, TestView, VariableUser};
use crate::windows::{self, ProjectScope};

fn tree_changed(window: &WebviewWindow, project: Uuid) {
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Tree,
        None,
    );
}

/// Test ouvert dans un onglet (TE2) : ses étapes dans l'ordre.
#[tauri::command]
pub fn test_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<TestView, CommandError> {
    let ctx = state.project_ctx(project)?;
    tree::test_view(&ctx.root, &path)
}

/// « Nouveau test » (NT1) : dossier du test et son étape 1.
#[tauri::command]
pub fn tree_create_test(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    parent: String,
) -> Result<CreatedTest, CommandError> {
    let ctx = state.project_ctx(project)?;
    let created = tree::create_test(&ctx.root, &parent)?;
    tree_changed(&window, project);
    Ok(created)
}

/// « + Étape » (NT2) : étape suivante du test ; renvoie son chemin.
#[tauri::command]
pub fn tree_add_step(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    test: String,
) -> Result<String, CommandError> {
    let ctx = state.project_ctx(project)?;
    let path = tree::add_step(&ctx.root, &test)?;
    tree_changed(&window, project);
    Ok(path)
}

/// Requêtes du projet, hors `path`, qui lisent l'une des variables `names` (EX5).
#[tauri::command]
pub fn request_variable_users(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    names: Vec<String>,
) -> Result<Vec<VariableUser>, CommandError> {
    let ctx = state.project_ctx(project)?;
    tree::variable_users(&ctx.root, &path, &names)
}
