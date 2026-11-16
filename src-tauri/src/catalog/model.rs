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
    /// vient en dernier : c'était la couleur de l'ancien workspace par défaut.
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
    /// « Tous les projets », dans l'ordre de création.
    pub projects: Vec<Project>,
    /// Workspaces, dans l'ordre des onglets : l'ordre de création (R11).
    pub workspaces: Vec<Workspace>,
    /// Dernier onglet ouvert ; `null` pour « Tous les projets » (R7).
    pub last_workspace_id: Option<Uuid>,
    /// Projet « Non classés » (`docs/plan-accueil-non-classes.md`, R4). Champ
    /// facultatif du format 2 : absent, aucun ; une version précédente l'ignore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_project_id: Option<Uuid>,
}

/// Format version 1, avec un workspace par défaut, lu seulement pour être migré (R12).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFileV1 {
    pub projects: Vec<Project>,
    pub workspaces: Vec<Workspace>,
    pub default_workspace_id: Uuid,
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
    fn couleurs_identiques_cote_typescript() {
        let rust: std::collections::BTreeSet<String> = WorkspaceColor::ASSIGNMENT_ORDER
            .into_iter()
            .map(crate::ts_contract::serialized_name)
            .collect();
        assert_eq!(rust, crate::ts_contract::union_members("WorkspaceColor"));
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
            version: 2,
            projects: vec![Project {
                id,
                name: "boutique-tests".to_owned(),
                path: PathBuf::from("/home/julien/arnod/boutique-tests"),
            }],
            workspaces: vec![Workspace {
                id,
                name: "E-commerce".to_owned(),
                color: WorkspaceColor::Orange,
                project_ids: vec![id],
            }],
            last_workspace_id: None,
            inbox_project_id: None,
        };
        let nil = "00000000-0000-0000-0000-000000000000";
        assert_eq!(
            serde_json::to_value(&file).unwrap(),
            serde_json::json!({
                "version": 2,
                "projects": [
                    { "id": nil, "name": "boutique-tests", "path": "/home/julien/arnod/boutique-tests" }
                ],
                "workspaces": [
                    { "id": nil, "name": "E-commerce", "color": "orange", "projectIds": [nil] }
                ],
                "lastWorkspaceId": null
            })
        );
    }
}
