//! Domaine « jeux de données » (`docs/plan-v4.md`, lot D, R11, R12) : onglet « Données » d'une
//! requête, envoi par cas.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::dataset::{self, DataSet, DataTable};
use crate::error::{CommandError, CommandErrorKind};
use crate::request::model::Request;
use crate::request::{file, inherit, resolve};
use crate::state::AppState;
use crate::tree;
use crate::windows::{self, ProjectScope};
use crate::workbench::{CasesOutcome, Workbench};

fn request_changed(window: &WebviewWindow, project: Uuid, path: &str) {
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Request,
        Some(path),
    );
}

/// Jeu de données de la requête `path`, s'il y en a un (4.7-a).
#[tauri::command]
pub fn data_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<Option<DataSet>, CommandError> {
    let ctx = state.project_ctx(project)?;
    dataset::of_request(&ctx.root, &path)
}

/// Enregistre le tableau de la requête `path` (4.7-a, R11).
#[tauri::command]
pub fn data_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    table: DataTable,
) -> Result<DataSet, CommandError> {
    let ctx = state.project_ctx(project)?;
    let set = dataset::save(&ctx.root, &path, &table)?;
    request_changed(&window, project, &path);
    Ok(set)
}

/// « Fichier CSV… » (4.7-a bis) : choisi dans le sélecteur natif, rangé dans `données/` en
/// lecture seule. `None` si l'utilisateur annule. Le plugin dialog n'est appelé que depuis Rust.
#[tauri::command]
pub async fn data_import(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<Option<DataSet>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let picked = window
        .dialog()
        .file()
        .set_title("Jeu de données : un fichier CSV")
        .add_filter("Fichier CSV", &["csv", "tsv", "txt"])
        .set_parent(&window)
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let source = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le fichier choisi n'est pas accessible : {error}."),
        )
    })?;
    let set = dataset::import(&ctx.root, &path, &source)?;
    request_changed(&window, project, &path);
    Ok(Some(set))
}

/// Délie le jeu de données de la requête `path`.
#[tauri::command]
pub fn data_remove(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<(), CommandError> {
    let ctx = state.project_ctx(project)?;
    dataset::remove(&ctx.root, &path)?;
    request_changed(&window, project, &path);
    Ok(())
}

/// Lignes d'un texte collé depuis un tableur ou un CSV (4.7-a, JD2).
#[tauri::command]
pub fn data_parse(text: String) -> Vec<Vec<String>> {
    dataset::parse_rows(&text)
}

/// « Ouvrir » le fichier d'un jeu de données dans le programme du système (4.7-a bis).
#[tauri::command]
pub fn data_open(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<(), CommandError> {
    let ctx = state.project_ctx(project)?;
    let Some(set) = dataset::of_request(&ctx.root, &path)? else {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Cette requête n'a pas de jeu de données.",
        ));
    };
    let target = tree::resolve(&ctx.root, &set.link.file)?;
    crate::startup::open_external(&target.to_string_lossy()).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le fichier n'a pas pu être ouvert : {error}."),
        )
    })
}

/// Requête d'un cas, résolue et masquée (4.7-a « Requête du cas »).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CasePreview {
    pub method: String,
    /// Adresse résolue ; une variable inconnue reste écrite `{{nom}}`.
    pub url: String,
    pub values: Vec<(String, String)>,
}

/// Requête du cas `case` (à partir de 1) du jeu de données de `request` (4.7-a).
#[tauri::command]
pub fn data_case(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    path: String,
    request: Request,
    environment: String,
    case: usize,
) -> Result<CasePreview, CommandError> {
    let ctx = state.project_ctx(project)?;
    let Some(link) = request.data.clone() else {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Cette requête n'a pas de jeu de données.",
        ));
    };
    let set = dataset::read(&ctx.root, &link);
    let values: BTreeMap<String, String> = dataset::cases(&set.table)
        .into_iter()
        .nth(case.saturating_sub(1))
        .unwrap_or_default();
    let mut request = request;
    request.inherited = inherit::resolve(&ctx.root, &path);
    let (mut variables, _) = bench.variables(&ctx, &environment)?;
    for variable in request.inherited.variables.iter().rev() {
        if !variable.folder.is_empty() || !variables.values.contains_key(&variable.name) {
            variables
                .values
                .insert(variable.name.clone(), variable.value.clone());
        }
    }
    variables.values.extend(bench.runtime_variables(project));
    variables.values.extend(values.clone());
    let url = match resolve::prepare(&request, &variables) {
        Ok(prepared) => prepared.url,
        Err(_) => resolve::substitute(&request.url, &variables).unwrap_or(request.url.clone()),
    };
    Ok(CasePreview {
        method: request.method.as_str().to_owned(),
        url: variables.mask(&url),
        values: values
            .into_iter()
            .map(|(k, v)| (k, variables.mask(&v)))
            .collect(),
    })
}

/// « Envoyer » une requête à jeu de données (4.7-b, R12) : tous ses cas, ou `cases` seulement.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn request_send_cases(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    exec_id: Uuid,
    environment: String,
    path: String,
    request: Request,
    confirmed: bool,
    cases: Option<Vec<usize>>,
) -> Result<CasesOutcome, CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut request = request;
    request.inherited = inherit::resolve(&ctx.root, &path);
    // Scripts de la requête relus du fichier (R10 du plan V4).
    request.scripts = tree::resolve(&ctx.root, &path)
        .and_then(|p| file::read(&p))
        .map(|r| r.scripts)
        .unwrap_or_default();
    let outcome = bench
        .send_cases(&ctx, exec_id, &environment, request, confirmed, cases)
        .await?;
    if outcome
        .cases
        .iter()
        .any(|c| c.outcome.is_some() || c.error.is_some())
    {
        bench.record_status(&ctx, &path, outcome.passed);
    }
    Ok(outcome)
}
