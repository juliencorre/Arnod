//! Domaine « catalogue » : chargement de l'état au démarrage et annulation.

use tauri::State;
use uuid::Uuid;

use crate::catalog::snapshot::{CatalogLoad, CatalogSnapshot};
use crate::error::CommandError;
use crate::state::AppState;

/// Instantané du catalogue au démarrage, avec l'avertissement éventuel d'un
/// fichier d'état illisible mis de côté (renvoyé une seule fois).
#[tauri::command]
pub fn catalog_load(state: State<'_, AppState>) -> Result<CatalogLoad, CommandError> {
    state.load_catalog()
}

/// Annule le retrait d'un projet ou la suppression d'un workspace (R6).
#[tauri::command]
pub fn undo(state: State<'_, AppState>, undo_token: Uuid) -> Result<CatalogSnapshot, CommandError> {
    state.undo(undo_token)
}
