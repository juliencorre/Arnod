//! Domaine « requêtes » : lecture, enregistrement, envoi et vérifications proposées.

use std::sync::Arc;

use serde::Deserialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::CommandError;
use crate::request::assertions::{self, ResponseData};
use crate::request::file;
use crate::request::model::{Assertion, Request};
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
    file::read(&tree::resolve(&root, &path)?)
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
#[tauri::command]
pub async fn request_send(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    exec_id: Uuid,
    environment: String,
    request: Request,
    confirmed: bool,
) -> Result<SendOutcome, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench
        .send(&ctx, exec_id, &environment, request, confirmed)
        .await
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
