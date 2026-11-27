//! Domaine « templates » (`docs/plan-v4.md`, lot F, R16 à R19) et sections de la barre latérale
//! (« Plus », R21).

use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::local;
use crate::request::file;
use crate::state::AppState;
use crate::template::{
    self, InstanceView, Literal, TemplateCandidate, TemplateParam, TemplateSummary, TemplateView,
};
use crate::tree;
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

/// Templates du projet (barre latérale, « Utiliser un template »).
#[tauri::command]
pub fn template_list(
    state: State<'_, AppState>,
    project: Uuid,
) -> Result<Vec<TemplateSummary>, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(template::list(&ctx.root))
}

/// Dossiers de rangement de `templates/` (« Ranger dans »).
#[tauri::command]
pub fn template_categories(
    state: State<'_, AppState>,
    project: Uuid,
) -> Result<Vec<String>, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(template::categories(&ctx.root))
}

/// Template ouvert dans un onglet (4.8-c).
#[tauri::command]
pub fn template_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<TemplateView, CommandError> {
    let ctx = state.project_ctx(project)?;
    template::read(&ctx.root, &path)
}

/// Enregistre les paramètres d'un template (4.8-c).
#[tauri::command]
pub fn template_save_params(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    params: Vec<TemplateParam>,
) -> Result<TemplateView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = template::save_parameters(&ctx.root, &path, &params)?;
    tree_changed(&window, project);
    Ok(view)
}

/// Paramètres proposés pour un template créé depuis le dossier `folder` (4.8-b, R18).
#[tauri::command]
pub fn template_candidates(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    folder: String,
    environment: String,
) -> Result<Vec<TemplateCandidate>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let values: BTreeMap<String, String> = bench
        .variables(&ctx, &environment)
        .map(|(variables, _)| {
            variables
                .values
                .into_iter()
                .filter(|(name, _)| !variables.secrets.contains(name))
                .collect()
        })
        .unwrap_or_default();
    template::candidates(&ctx.root, &folder, &values, &environment)
}

/// « Créer un template » depuis un dossier (4.8-b, R18).
#[tauri::command]
pub fn template_create(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    folder: String,
    name: String,
    category: String,
    params: Vec<TemplateParam>,
    replace: bool,
) -> Result<TemplateView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = template::create(&ctx.root, &folder, &name, &category, &params, replace)?;
    tree_changed(&window, project);
    Ok(view)
}

/// « Utiliser dans un dossier… » (4.8-c, 4.8-d) : rend le chemin de l'utilisation.
#[tauri::command]
pub fn template_use(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    template: String,
    folder: String,
    name: String,
) -> Result<String, CommandError> {
    let ctx = state.project_ctx(project)?;
    let path = template::use_in(&ctx.root, &template, &folder, &name)?;
    tree_changed(&window, project);
    Ok(path)
}

/// Utilisation d'un template ouverte dans un onglet (4.8-d).
#[tauri::command]
pub fn template_instance(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<InstanceView, CommandError> {
    let ctx = state.project_ctx(project)?;
    template::instance(&ctx.root, &path)
}

/// Enregistre les valeurs saisies dans une utilisation (4.8-d, R17).
#[tauri::command]
pub fn template_set_values(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    values: BTreeMap<String, String>,
) -> Result<InstanceView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = template::set_values(&ctx.root, &path, &values)?;
    tree_changed(&window, project);
    Ok(view)
}

/// Valeurs écrites d'un endpoint, proposées comme paramètres (3.2-d, R19).
#[tauri::command]
pub fn template_literals(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<Vec<Literal>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let request = file::read(&tree::resolve(&ctx.root, &path)?)?;
    Ok(template::literals(&request))
}

/// « Transférer vers les templates » (3.2-d, R19) : l'endpoint devient un template test.
#[tauri::command]
pub fn template_from_request(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    name: String,
    category: String,
    params: Vec<String>,
) -> Result<TemplateView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let view = template::from_request(&ctx.root, &path, &name, &category, &params)?;
    // La requête quitte le contrat et le référentiel : ses onglets se ferment.
    let _ = windows::apply(
        window.app_handle(),
        |w: &mut crate::layout::windows::Windows| {
            w.close_path(project, &path);
            Ok(())
        },
    );
    windows::paths_changed(
        window.app_handle(),
        window.label(),
        project,
        Vec::new(),
        vec![path.clone()],
    );
    tree_changed(&window, project);
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Contracts,
        None,
    );
    Ok(view)
}

/// Sections de la barre latérale que « Plus » propose (R21) : nom affiché par l'interface.
pub const SECTIONS: [&str; 1] = ["templates"];

/// « Afficher » ou masquer une section depuis « Plus » (1.2-d bis, R21) ; rend les sections
/// affichées.
#[tauri::command]
pub fn sections_show(
    state: State<'_, AppState>,
    project: Uuid,
    section: String,
    shown: bool,
) -> Result<Vec<String>, CommandError> {
    if !SECTIONS.contains(&section.as_str()) {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("La section « {section} » n'existe pas dans cette version."),
        ));
    }
    let ctx = state.project_ctx(project)?;
    let mut data = local::load(&ctx.data_dir, ctx.id);
    data.sections.retain(|s| s != &section);
    if shown {
        data.sections.push(section);
    }
    local::save(&ctx.data_dir, ctx.id, &data)?;
    Ok(data.sections)
}
