//! Instantané du catalogue renvoyé à l'interface.
//!
//! Chaque commande qui modifie le catalogue renvoie un [`CatalogSnapshot`]
//! complet : l'interface remplace son état sans logique de synchronisation.
//! Miroirs TypeScript dans `src/types/ipc.ts` (lot C).

use serde::Serialize;
use uuid::Uuid;

use super::model::WorkspaceColor;
use crate::error::CommandError;

/// Réponse de `catalog_load` : l'instantané et, une seule fois, l'avertissement
/// d'un fichier d'état illisible mis de côté (nature `corruptState`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogLoad {
    pub snapshot: CatalogSnapshot,
    pub warning: Option<CommandError>,
}

/// Réponse d'une action annulable : l'instantané et le jeton à passer à `undo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undoable {
    pub snapshot: CatalogSnapshot,
    pub undo_token: Uuid,
}

/// Réponse de `project_create`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCreated {
    pub snapshot: CatalogSnapshot,
    pub project_id: Uuid,
}

/// Réponse de `workspace_create`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCreated {
    pub snapshot: CatalogSnapshot,
    pub workspace_id: Uuid,
}

/// État complet du catalogue, prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    /// Référentiel, dans l'ordre de création (R11).
    pub projects: Vec<ProjectView>,
    /// Workspace par défaut d'abord, puis les autres par nom (R11).
    pub workspaces: Vec<WorkspaceView>,
    pub default_workspace_id: Uuid,
    pub last_workspace_id: Uuid,
}

/// Projet du référentiel vu par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    /// État du dossier au moment de l'instantané (P10).
    pub status: ProjectStatus,
    /// Workspaces qui contiennent le projet, dans l'ordre R11.
    pub workspace_ids: Vec<Uuid>,
}

/// Présence du dossier d'un projet sur disque (P10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    /// Le dossier existe.
    Ok,
    /// Le dossier a été déplacé ou supprimé hors d'Arnod.
    Missing,
}

/// Workspace vu par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: Uuid,
    pub name: String,
    pub color: WorkspaceColor,
    pub is_default: bool,
    /// Projets du workspace, dans l'ordre où ils ont été ajoutés.
    pub project_ids: Vec<Uuid>,
}
