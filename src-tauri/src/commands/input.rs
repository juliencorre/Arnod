//! Domaine « saisie » : variables connues, en-têtes ajoutés, listes en texte et
//! copie en cURL d'une requête (`docs/plan-mvp-b-saisie-curl.md`).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::curl;
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::input::{self, PairsFormat, PairsText, RequestInput};
use crate::local;
use crate::request::inherit;
use crate::request::model::{Pair, Request};
use crate::state::AppState;
use crate::workbench::Workbench;

/// Commande cURL d'une requête (`request_curl`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlCopy {
    pub command: String,
    /// Environnement dont les variables sont résolues.
    pub environment: String,
}

/// Variables connues et inconnues, en-têtes ajoutés à l'envoi (R1 à R3) : lu sur
/// disque (environnements, `local.json`, dossiers), sans ouvrir le coffre.
#[tauri::command]
pub fn request_input(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    path: Option<String>,
    request: Request,
) -> Result<RequestInput, CommandError> {
    let ctx = state.project_ctx(project)?;
    let all = environment::list(&ctx.root);
    let active = all
        .iter()
        .find(|env| env.name.eq_ignore_ascii_case(&environment))
        .cloned()
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorKind::NotFound,
                format!("L'environnement « {environment} » n'existe pas."),
            )
        })?;
    let mut request = request;
    let (inherited, inherited_vars) = path.as_deref().map_or_else(Default::default, |path| {
        (
            inherit::resolve(&ctx.root, path),
            inherit::variable_names(&ctx.root, path),
        )
    });
    request.inherited = inherited;
    let data = local::load(&ctx.data_dir, ctx.id);
    let locals = data.locals.get(&active.name).cloned().unwrap_or_default();
    let mut known = input::known_names(&active, &locals, &inherited_vars);
    // Lot E : valeurs extraites par les étapes précédentes du test, et variables
    // d'exécution de la session (docs/plan-mvp-e-tests-etapes.md, R5, R6).
    let extracted = path
        .as_deref()
        .map(|path| crate::tree::extracted_before(&ctx.root, path))
        .unwrap_or_default();
    for name in extracted
        .into_iter()
        .chain(bench.runtime_variables(ctx.id).into_keys())
    {
        if !known.contains(&name) {
            known.push(name);
        }
    }
    let unknown = input::unknown(&input::used_names(&request), &known, &all, &active.name);
    // Valeurs non secrètes, pour nommer l'hôte.
    let mut values: BTreeMap<String, String> = active
        .variables
        .iter()
        .map(|v| (v.name.clone(), v.value.clone()))
        .collect();
    for var in locals.iter().filter(|v| !v.secret) {
        if let Some(value) = &var.value {
            values.insert(var.name.clone(), value.clone());
        }
    }
    Ok(RequestInput {
        environment: active.name.clone(),
        known,
        unknown,
        added: input::added_headers(&request, &values),
    })
}

/// La requête de l'éditeur en commande cURL (R8 à R10).
#[tauri::command]
pub fn request_curl(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    path: Option<String>,
    request: Request,
) -> Result<CurlCopy, CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut request = request;
    request.inherited = path
        .as_deref()
        .map(|path| inherit::resolve(&ctx.root, path))
        .unwrap_or_default();
    let (variables, _) = bench.variables(&ctx, &environment)?;
    Ok(CurlCopy {
        command: curl::command(&request, &variables)?,
        environment: variables.environment,
    })
}

/// Une liste de paires en texte, une ligne par paire (R11).
#[tauri::command]
pub fn pairs_to_text(pairs: Vec<Pair>, format: PairsFormat) -> String {
    input::to_text(&pairs, format)
}

/// Les paires d'un texte, et ses lignes illisibles (R12, R13).
#[tauri::command]
pub fn pairs_from_text(text: String, format: PairsFormat, previous: Vec<Pair>) -> PairsText {
    input::from_text(&text, format, &previous)
}

/// Valeurs dynamiques proposées à la frappe de `{{$` (5.1-h), chacune avec un exemple tiré.
#[tauri::command]
pub fn dynamic_values() -> Vec<crate::request::dynamic::DynamicValue> {
    crate::request::dynamic::catalog(&mut crate::request::dynamic::Dice::new())
}
