//! Domaine « projets » : création, choix de l'emplacement et ouverture.

use std::path::Path;

use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::catalog::snapshot::{ProjectCreated, ProjectView};
use crate::error::{CommandError, CommandErrorKind};
use crate::state::AppState;

/// Dossier proposé pour un projet : `<emplacement>/<nom>`, `~/arnod` par défaut
/// (P1, P2). Simple aperçu : le nom est validé à la création.
#[tauri::command]
pub fn project_suggest_folder(
    state: State<'_, AppState>,
    name: String,
    location: Option<String>,
) -> String {
    state.suggest_folder(&name, location.as_deref())
}

/// Ouvre le sélecteur de dossier natif pour choisir l'emplacement d'un projet
/// (P3, P12). Renvoie `None` si l'utilisateur annule.
///
/// Le plugin dialog n'est appelé que depuis Rust : aucune permission WebView.
/// Commande asynchrone, car le sélecteur ne doit pas bloquer le fil principal.
#[tauri::command]
pub async fn project_pick_folder(
    window: WebviewWindow,
    location: Option<String>,
) -> Result<Option<String>, CommandError> {
    let mut dialog = window
        .dialog()
        .file()
        .set_title("Choisir l'emplacement du projet")
        .set_parent(&window);
    if let Some(location) = location.filter(|location| Path::new(location).is_dir()) {
        dialog = dialog.set_directory(location);
    }
    let Some(picked) = dialog.blocking_pick_folder() else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le dossier choisi est inutilisable : {error}"),
        )
    })?;
    Ok(Some(path.display().to_string()))
}

/// Crée un projet dans `<emplacement>/<nom>` et l'ajoute au workspace par
/// défaut et à `workspace_id` (P1, P2, R4).
#[tauri::command]
pub fn project_create(
    state: State<'_, AppState>,
    name: String,
    location: Option<String>,
    workspace_id: Uuid,
) -> Result<ProjectCreated, CommandError> {
    state.create_project(&name, location.as_deref(), workspace_id)
}

/// Projet à ouvrir (P5) ; refusé si son dossier est introuvable (P10).
#[tauri::command]
pub fn project_open(
    state: State<'_, AppState>,
    project_id: Uuid,
) -> Result<ProjectView, CommandError> {
    state.open_project(project_id)
}
