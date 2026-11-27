//! Historique des envois d'une requête (écran 5.1-c bis, `docs/plan-v1.md`). Les règles et le
//! fichier sont dans [`crate::history`] ; l'envoi garde chaque réponse reçue
//! (`commands::requests::request_send`).

use tauri::State;
use uuid::Uuid;

use crate::error::CommandError;
use crate::history::{self, HistoryDiff, HistoryEntry, HistoryView};
use crate::state::AppState;

/// Envois gardés de la requête `path`, chacun comparé au précédent (H2, H3).
#[tauri::command]
pub fn history_load(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<HistoryView, CommandError> {
    let ctx = state.project_ctx(project)?;
    Ok(history::view(&ctx.data_dir, project, &path))
}

/// Différences de l'envoi `id` avec le précédent (« Voir les différences », H4).
#[tauri::command]
pub fn history_diff(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    id: Uuid,
) -> Result<HistoryDiff, CommandError> {
    let ctx = state.project_ctx(project)?;
    history::diff_of(&ctx.data_dir, project, &path, id)
}

/// « Effacer » l'historique de la requête (H5).
#[tauri::command]
pub fn history_clear(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<HistoryView, CommandError> {
    let ctx = state.project_ctx(project)?;
    history::clear(&ctx.data_dir, project, &path)?;
    Ok(history::view(&ctx.data_dir, project, &path))
}

/// « Annuler » après « Effacer » : les envois effacés reviennent (H5).
#[tauri::command]
pub fn history_restore(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    entries: Vec<HistoryEntry>,
) -> Result<HistoryView, CommandError> {
    let ctx = state.project_ctx(project)?;
    history::restore(&ctx.data_dir, project, &path, entries)?;
    Ok(history::view(&ctx.data_dir, project, &path))
}
