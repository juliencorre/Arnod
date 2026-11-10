//! Domaine « requêtes » : lecture, enregistrement, envoi et vérifications proposées.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tauri::{Emitter, Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::CommandError;
use crate::request::assertions::{self, ResponseData};
use crate::request::model::{Assertion, Example, Request};
use crate::request::{example, file, inherit};
use crate::results::{RequestResult, ResultBook, ResultChange};
use crate::state::AppState;
use crate::tree;
use crate::windows::{self, ProjectScope};
use crate::workbench::{SendOutcome, Workbench};

#[tauri::command]
pub fn request_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<Request, CommandError> {
    let root = state.project_ctx(project)?.root;
    let mut request = file::read(&tree::resolve(&root, &path)?)?;
    // Héritage des dossiers et de la collection, pour l'éditeur (R3, R4).
    request.inherited = inherit::resolve(&root, &path);
    Ok(request)
}

#[tauri::command]
pub fn request_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    request: Request,
) -> Result<(), CommandError> {
    let run = || {
        let root = state.project_ctx(project)?.root;
        file::write(&tree::resolve(&root, &path)?, &request)
    };
    let result = run();
    if result.is_ok() {
        windows::project_changed(
            window.app_handle(),
            window.label(),
            project,
            ProjectScope::Request,
            Some(path.as_str()),
        );
    }
    result
}

/// Envoie la requête telle qu'elle est dans l'éditeur, enregistrée ou non (E1).
/// `path` situe la requête pour l'héritage de ses dossiers (R3, R4). Le début
/// et la fin de l'envoi sont gardés et diffusés à toutes les fenêtres (R5).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn request_send(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    results: State<'_, Mutex<ResultBook>>,
    project: Uuid,
    exec_id: Uuid,
    environment: String,
    path: Option<String>,
    request: Request,
    confirmed: bool,
) -> Result<SendOutcome, CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut request = request;
    request.inherited = path
        .as_deref()
        .map(|path| inherit::resolve(&ctx.root, path))
        .unwrap_or_default();
    bench.precheck(&ctx, &environment, &request, confirmed)?;
    if let Some(path) = path.as_deref() {
        let started = results.lock().map(|mut book| book.start(project, path));
        if let Ok(result) = started {
            share(&window, result);
        }
    }
    let sent = bench
        .send(&ctx, exec_id, &environment, request, confirmed)
        .await;
    if let Some(path) = path.as_deref() {
        let finished = results
            .lock()
            .map(|mut book| book.finish(project, path, &sent, now_ms()));
        if let Ok(result) = finished {
            share(&window, result);
        }
    }
    sent
}

/// Diffuse le résultat d'un envoi à toutes les fenêtres (R5).
fn share(window: &WebviewWindow, result: RequestResult) {
    let _ = window.app_handle().emit(
        windows::events::RESULT,
        ResultChange {
            result,
            origin: window.label().to_owned(),
        },
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Dernier résultat d'une requête, pour une fenêtre qui s'ouvre (D2).
#[tauri::command]
pub fn request_result(
    results: State<'_, Mutex<ResultBook>>,
    project: Uuid,
    path: String,
) -> Option<RequestResult> {
    results
        .lock()
        .ok()
        .and_then(|book| book.get(project, &path).cloned())
}

#[tauri::command]
pub fn request_cancel(bench: State<'_, Arc<Workbench>>, exec_id: Uuid) {
    bench.cancel(exec_id);
}

/// Réponse sur laquelle proposer des vérifications (R10).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeInput {
    pub status: u16,
    pub duration_ms: u64,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[tauri::command]
pub fn request_propose_assertions(response: ProposeInput) -> Vec<Assertion> {
    assertions::propose(&ResponseData {
        status: response.status,
        duration_ms: response.duration_ms,
        headers: response.headers,
        body: response.body,
    })
}

/// Réponse reçue à reprendre comme réponse attendue (R7).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleInput {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[tauri::command]
pub fn request_example_from_response(response: ExampleInput, name: String) -> Example {
    example::from_response(
        &name,
        response.status,
        &response.status_text,
        &response.headers,
        &response.body,
    )
}
