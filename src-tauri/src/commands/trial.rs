//! Domaine « essais » : une étape essayée depuis l'accueil, sans projet
//! (docs/plan-accueil-non-classes.md, A5, N1, N2).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::catalog::snapshot::CatalogSnapshot;
use crate::commands::requests::{now_ms, share};
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::request::resolve::Variables;
use crate::results::ResultBook;
use crate::state::AppState;
use crate::steps::{self, definition::Engine};
use crate::trial::{self, TRIAL_PREFIX};
use crate::windows::{self, ProjectScope};
use crate::workbench::{ProjectCtx, SendOutcome, Workbench};

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
    secrets: Option<BTreeMap<String, String>>,
) -> Result<TrialResult, CommandError> {
    let secrets = secrets.unwrap_or_default();
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
    // Secrets d'une commande cURL collée (docs/plan-mvp-b-saisie-curl.md, R5) :
    // remplacés à l'envoi seulement, masqués dans la réponse, gardés au coffre.
    let masks = Variables {
        values: secrets.clone(),
        secrets: secrets.keys().cloned().collect(),
        environment: environment::LOCAL.to_owned(),
    };
    let sent = steps::http::with_secrets(&request, &secrets);
    let mut response = bench
        .send_detached(exec_id, &sent)
        .await
        .map_err(|mut error| {
            error.message = masks.mask(&error.message);
            error
        })?;
    if !secrets.is_empty() {
        response.body = masks.mask_owned(response.body);
        for (_, value) in &mut response.headers {
            *value = masks.mask(value);
        }
    }

    let (ctx, created) = state.ensure_inbox()?;
    if let Some(snapshot) = &created {
        // « Non classés » créé ou réinscrit : toutes les fenêtres le voient (F11).
        windows::catalog_changed(window.app_handle(), snapshot);
    }
    keep_secrets(&bench, &ctx, &secrets)?;
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

/// Secrets d'un essai au coffre, en variables locales secrètes de l'environnement
/// « Local » de « Non classés » (docs/plan-mvp-b-saisie-curl.md, R5).
fn keep_secrets(
    bench: &Workbench,
    ctx: &ProjectCtx,
    secrets: &BTreeMap<String, String>,
) -> Result<(), CommandError> {
    if secrets.is_empty() {
        return Ok(());
    }
    environment::init(&ctx.root)?;
    let mut local = environment::list(&ctx.root)
        .into_iter()
        .find(|env| env.name == environment::LOCAL)
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorKind::NotFound,
                "L'environnement « Local » de « Non classés » est introuvable.",
            )
        })?;
    let missing: Vec<String> = secrets
        .keys()
        .filter(|name| !local.locals.contains(name))
        .cloned()
        .collect();
    if !missing.is_empty() {
        // Une variable partagée du même nom céderait sa place à la locale.
        local.variables.retain(|v| !missing.contains(&v.name));
        local.locals.extend(missing);
        bench.save_environment(ctx, &local, Some(environment::LOCAL))?;
    }
    for (name, value) in secrets {
        bench.set_local(ctx, environment::LOCAL, name, value, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{MemorySecretStore, SecretStore, key};
    use crate::workbench::ProjectCtx;

    #[test]
    fn r5_secrets_d_un_essai_au_coffre_en_variables_locales() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Non classés");
        std::fs::create_dir_all(&root).unwrap();
        let ctx = ProjectCtx {
            id: Uuid::new_v4(),
            root,
            data_dir: dir.path().join("data"),
        };
        let store = Arc::new(MemorySecretStore::default());
        let bench = Workbench::new(store.clone());
        let secrets = BTreeMap::from([("token".to_owned(), "eyJ.x".to_owned())]);
        keep_secrets(&bench, &ctx, &secrets).unwrap();
        // Deux fois : la variable n'est déclarée qu'une fois, la valeur remplacée.
        let again = BTreeMap::from([("token".to_owned(), "eyJ.y".to_owned())]);
        keep_secrets(&bench, &ctx, &again).unwrap();
        let local = environment::list(&ctx.root).remove(0);
        assert_eq!(local.name, environment::LOCAL);
        assert_eq!(local.locals, ["token"]);
        assert_eq!(
            store
                .get(&key(ctx.id, "Local", "token"))
                .unwrap()
                .as_deref(),
            Some("eyJ.y")
        );
        // Jamais en clair dans le projet.
        let files = std::fs::read_dir(ctx.root.join("environments")).unwrap();
        for file in files.flatten() {
            let text = std::fs::read_to_string(file.path()).unwrap();
            assert!(!text.contains("eyJ"), "{text}");
        }
        let (variables, _) = bench.variables(&ctx, "Local").unwrap();
        assert_eq!(
            variables.values.get("token").map(String::as_str),
            Some("eyJ.y")
        );
    }
}
