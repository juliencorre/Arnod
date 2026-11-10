//! Domaine « essais » : une étape essayée depuis l'accueil, sans projet
//! (docs/plan-accueil-non-classes.md, A5, N1, N2).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::catalog::snapshot::CatalogSnapshot;
use crate::commands::requests::{now_ms, share};
use crate::error::{CommandError, CommandErrorKind};
use crate::results::ResultBook;
use crate::state::AppState;
use crate::steps::{self, definition::Engine};
use crate::trial::{self, TRIAL_PREFIX};
use crate::windows::{self, ProjectScope};
use crate::workbench::{SendOutcome, Workbench};

/// Essai enregistré dans « Non classés » et sa réponse.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialResult {
    /// Instantané du catalogue, « Non classés » compris.
    pub snapshot: CatalogSnapshot,
    pub project_id: Uuid,
    /// Requête « Essai N », relative au projet.
    pub path: String,
    /// Contrat déduit (`contracts/…`).
    pub contract: String,
    pub outcome: SendOutcome,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Raison en minuscules du cœur, en phrase prête à afficher.
fn sentence(reason: &str) -> String {
    let mut chars = reason.chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
    format!("{}{}.", first.unwrap_or_default(), chars.as_str())
}

/// Envoie l'essai, puis seulement l'enregistre dans « Non classés » (R12) :
/// contrat déduit, requête « Essai N », dernier résultat diffusé (R5 du
/// résultat détachable).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn trial_send(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    results: State<'_, Mutex<ResultBook>>,
    exec_id: Uuid,
    nature: String,
    subject: String,
    values: BTreeMap<String, String>,
) -> Result<TrialResult, CommandError> {
    let definition = steps::find(&nature).ok_or_else(|| {
        invalid(format!(
            "La nature d'étape « {nature} » n'est pas disponible."
        ))
    })?;
    let values =
        steps::values::normalize(&definition, &values).map_err(|r| invalid(sentence(&r)))?;
    let request = match &definition.engine {
        Engine::Builtin { id, .. } if id == "http" => {
            steps::http::request(TRIAL_PREFIX, &subject, &values)
                .map_err(|r| invalid(sentence(&r)))?
        }
        _ => {
            return Err(invalid(format!(
                "La nature « {} » ne peut pas encore être essayée ici.",
                definition.name
            )));
        }
    };
    let response = bench.send_detached(exec_id, &request).await?;

    let (ctx, created) = state.ensure_inbox()?;
    if let Some(snapshot) = &created {
        // « Non classés » créé ou réinscrit : toutes les fenêtres le voient (F11).
        windows::catalog_changed(window.app_handle(), snapshot);
    }
    let recorded = trial::record(&ctx.root, request, &response)?;
    let outcome = Workbench::outcome(&ctx.root, &recorded.request, response);
    let finished = results
        .lock()
        .map(|mut book| book.finish(ctx.id, &recorded.path, &Ok(outcome.clone()), now_ms()));
    if let Ok(result) = finished {
        share(&window, result);
    }
    for scope in [ProjectScope::Tree, ProjectScope::Contracts] {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            ctx.id,
            scope,
            Some(recorded.path.as_str()),
        );
    }
    Ok(TrialResult {
        snapshot: state.snapshot()?,
        project_id: ctx.id,
        path: recorded.path,
        contract: recorded.contract,
        outcome,
    })
}
