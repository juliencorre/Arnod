//! Domaine « tests en schéma » (`docs/plan-v4.md`, lots A, B et E) : schéma d'un test, étapes
//! ajoutées sur un lien, Avant / Après et propriétés d'un dossier.

use std::sync::Arc;

use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::CommandError;
use crate::flow::edit::{self, FolderProperty, HooksView, Placement, StepAdded};
use crate::flow::{Flow, FolderHooks, HookPhase, StepKind};
use crate::state::AppState;
use crate::tree::{self, TestView};
use crate::windows::{self, ProjectScope};
use crate::workbench::Workbench;

fn tree_changed(window: &WebviewWindow, project: Uuid) {
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Tree,
        None,
    );
}

/// Enregistre le schéma d'un test (R1, R3) et rend l'onglet du test.
#[tauri::command]
pub fn flow_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    flow: Flow,
) -> Result<TestView, CommandError> {
    let ctx = state.project_ctx(project)?;
    edit::save(&ctx.root, &path, &flow)?;
    tree_changed(&window, project);
    tree::test_view(&ctx.root, &path)
}

/// Ajoute une étape de nature `kind` au test `path`, sur un lien ou en fin de liste (4.3-e,
/// 4.3-b bis) ; une requête ou une attente reçoit sa requête neuve.
#[tauri::command]
pub fn flow_add_step(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    kind: StepKind,
    placement: Placement,
    name: Option<String>,
) -> Result<StepAdded, CommandError> {
    let ctx = state.project_ctx(project)?;
    let added = edit::add(&ctx.root, &path, kind, &placement, name.as_deref())?;
    tree_changed(&window, project);
    Ok(added)
}

/// « Supprimer l'étape » ou « Annuler l'insertion » (4.3-b quater) : ses renvois passent à sa
/// suite, sa requête va à la corbeille.
#[tauri::command]
pub fn flow_remove_step(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    step: String,
) -> Result<TestView, CommandError> {
    let ctx = state.project_ctx(project)?;
    edit::remove_step(&ctx.root, &path, &step)?;
    tree_changed(&window, project);
    tree::test_view(&ctx.root, &path)
}

/// Donne à une étape requête insérée la requête d'une opération d'un contrat du projet
/// (4.3-b quater, « Service » puis « Opération »), sans lien au contrat ; `environment` reçoit
/// `baseUrl` si elle y est vide.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn flow_step_operation(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    step: String,
    contract: String,
    operation: String,
    environment: Option<String>,
) -> Result<TestView, CommandError> {
    let ctx = state.project_ctx(project)?;
    edit::from_operation(
        &ctx.root,
        &path,
        &step,
        &contract,
        &operation,
        environment.as_deref(),
    )?;
    tree_changed(&window, project);
    if environment.is_some() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Environments,
            None,
        );
    }
    tree::test_view(&ctx.root, &path)
}

/// Retire une étape d'un Avant / Après (4.7-c) ; sa requête va à la corbeille.
#[tauri::command]
pub fn hooks_remove_step(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    phase: HookPhase,
    step: String,
) -> Result<HooksView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = edit::remove_hook(&ctx.root, &path, phase, &step)?;
    tree_changed(&window, project);
    Ok(view)
}

/// Avant / Après d'un dossier (4.7-c).
#[tauri::command]
pub fn hooks_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<HooksView, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(edit::hooks_view(&ctx.root, &path))
}

/// Enregistre les Avant / Après d'un dossier (4.7-c, R13).
#[tauri::command]
pub fn hooks_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    hooks: FolderHooks,
) -> Result<HooksView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = edit::save_hooks(&ctx.root, &path, &hooks)?;
    tree_changed(&window, project);
    Ok(view)
}

/// « + Étape » d'une phase d'Avant / Après (4.7-c) : une requête reçoit sa requête neuve,
/// rangée dans le dossier et absente de l'arborescence (R14).
#[tauri::command]
pub fn hooks_add_step(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    phase: HookPhase,
    kind: StepKind,
    name: Option<String>,
) -> Result<HooksView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = edit::add_hook(&ctx.root, &path, phase, kind, name.as_deref())?;
    tree_changed(&window, project);
    Ok(view)
}

/// Propriétés d'un dossier, avec leur origine, dans l'environnement `environment` (4.8-a).
#[tauri::command]
pub fn folder_properties(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    path: String,
    environment: String,
) -> Result<Vec<FolderProperty>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let values = bench
        .variables(&ctx, &environment)
        .map(|(variables, _)| {
            // Un secret n'est jamais montré (R9 du premier usage).
            variables
                .values
                .into_iter()
                .filter(|(name, _)| !variables.secrets.contains(name))
                .collect()
        })
        .unwrap_or_default();
    Ok(edit::properties(&ctx.root, &path, &environment, &values))
}

/// Propriété définie dans un dossier (4.8-a) : nom et valeur.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyInput {
    pub name: String,
    pub value: String,
}

/// Enregistre les propriétés définies dans un dossier (`request.variables` de son `folder.yml`).
#[tauri::command]
pub fn folder_set_properties(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    path: String,
    properties: Vec<PropertyInput>,
    environment: String,
) -> Result<Vec<FolderProperty>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let list: Vec<(String, String, String)> = properties
        .into_iter()
        .map(|p| (p.name, p.value, String::new()))
        .collect();
    tree::set_folder_variables(&ctx.root, &path, &list)?;
    tree_changed(&window, project);
    folder_properties(state, bench, project, path, environment)
}
