//! Données du catalogue : projets du référentiel, workspaces et format disque.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projet du référentiel : un dossier sur disque contenant `arnod.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Identifiant, également écrit dans le manifeste du projet.
    pub id: Uuid,
    /// Nom du projet, qui est aussi le nom de son dossier.
    pub name: String,
    /// Dossier du projet, canonique quand il existe.
    pub path: PathBuf,
}

/// Workspace : regroupement nommé et coloré de projets du référentiel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub color: WorkspaceColor,
    /// Projets regroupés, dans l'ordre où ils ont été ajoutés.
    pub project_ids: Vec<Uuid>,
}

/// Couleur d'un workspace, rendue par les jetons `--color-ws-*` de `src/styles.css`.
///
/// Miroir TypeScript : `WorkspaceColor` dans `src/types/ipc.ts` (lot C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceColor {
    Gray,
    Orange,
    Blue,
    Green,
    Purple,
    Red,
    Yellow,
    Teal,
}

impl WorkspaceColor {
    /// Ordre d'attribution automatique aux nouveaux workspaces (W5). Le gris est
    /// réservé en priorité au workspace par défaut et vient donc en dernier.
    pub const ASSIGNMENT_ORDER: [Self; 8] = [
        Self::Orange,
        Self::Blue,
        Self::Green,
        Self::Purple,
        Self::Red,
        Self::Yellow,
        Self::Teal,
        Self::Gray,
    ];
}

/// Format du fichier `arnod.json` (§ 6.1 du plan).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFile {
    /// Version du format, pour les migrations.
    pub version: u32,
    /// Référentiel, dans l'ordre de création (R11).
    pub projects: Vec<Project>,
    /// Workspaces, dans l'ordre de création.
    pub workspaces: Vec<Workspace>,
    /// Workspace par défaut : porté par un seul champ, il est unique par construction (R3).
    pub default_workspace_id: Uuid,
    /// Dernier workspace ouvert (R7).
    pub last_workspace_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_en_camel_case() {
        assert_eq!(
            serde_json::to_value(WorkspaceColor::Teal).unwrap(),
            serde_json::json!("teal")
        );
        assert_eq!(
            serde_json::from_value::<WorkspaceColor>(serde_json::json!("gray")).unwrap(),
            WorkspaceColor::Gray
        );
    }

    #[test]
    fn ordre_d_attribution_complet_et_sans_doublon() {
        let unique: std::collections::HashSet<_> =
            WorkspaceColor::ASSIGNMENT_ORDER.into_iter().collect();
        assert_eq!(unique.len(), 8);
        assert_eq!(WorkspaceColor::ASSIGNMENT_ORDER[7], WorkspaceColor::Gray);
    }

    #[test]
    fn format_disque_en_camel_case() {
        let id = Uuid::nil();
        let file = CatalogFile {
            version: 1,
            projects: vec![Project {
                id,
                name: "boutique-tests".to_owned(),
                path: PathBuf::from("/home/julien/arnod/boutique-tests"),
            }],
            workspaces: vec![Workspace {
                id,
                name: "Mes projets".to_owned(),
                color: WorkspaceColor::Gray,
                project_ids: vec![id],
            }],
            default_workspace_id: id,
            last_workspace_id: id,
        };
        let nil = "00000000-0000-0000-0000-000000000000";
        assert_eq!(
            serde_json::to_value(&file).unwrap(),
            serde_json::json!({
                "version": 1,
                "projects": [
                    { "id": nil, "name": "boutique-tests", "path": "/home/julien/arnod/boutique-tests" }
                ],
                "workspaces": [
                    { "id": nil, "name": "Mes projets", "color": "gray", "projectIds": [nil] }
                ],
                "defaultWorkspaceId": nil,
                "lastWorkspaceId": nil
            })
        );
    }
}
