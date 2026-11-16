//! Collections Bruno en fichiers `.bru` (docs/plan-mvp-c-import-bruno.md).
//!
//! `parse` lit le langage `.bru`, `read` une collection sur disque (sans
//! jamais l'écrire), `convert` la traduit en fichiers OpenCollection 1.0.0
//! valides et en rapport d'import. L'écriture du projet est dans
//! [`crate::import`].

pub mod convert;
pub mod parse;
pub mod read;

#[cfg(test)]
pub(crate) mod tests;

use serde::{Deserialize, Serialize};

/// Nature d'un élément du rapport : ce qui l'a produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    /// Bloc `tests`.
    Tests,
    /// `script:pre-request`.
    PreRequest,
    /// `script:post-response`.
    PostResponse,
    /// Variable d'environnement (`vars:secret`, `.env`).
    Env,
    /// Authentification.
    Auth,
    /// Corps de requête.
    Body,
    /// Requête entière (GraphQL, fichier illisible…).
    Request,
    /// Vérification (`assert`).
    Assertion,
    /// Extraction (`vars:post-response`).
    Extraction,
    /// Réglage ou bloc sans équivalent.
    Option,
}

/// Suite à donner à un élément du rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// Gardé dans le fichier, pas exécuté (scripts).
    NotRun,
    /// Valeur à renseigner (secret).
    ToFill,
    /// Repris, à vérifier (opérateur non évalué, authentification non envoyée…).
    ToCheck,
    /// Non importé.
    Skipped,
    /// Valeur connue, rangée dans le coffre du système.
    Stored,
}

/// Élément signalé par l'import, avec le fichier d'origine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportItem {
    pub kind: ItemKind,
    pub status: ItemStatus,
    /// Requête, dossier ou variable concerné.
    pub name: String,
    /// Dossier (« Commandes ») ou environnement ; vide à la racine.
    pub place: String,
    /// Fichier d'origine, relatif à la collection (`Commandes/Créer.bru`).
    pub file: String,
    /// Bloc d'origine (`tests`, `auth:oauth2`…), s'il y en a un.
    pub block: Option<String>,
    /// Fichier créé dans le projet, relatif à sa racine, pour l'ouvrir.
    pub request: Option<String>,
    /// Ce qu'Arnod en a fait, en français.
    pub detail: String,
    /// Contenu d'origine (script, valeur), pour « Avant · Bruno ».
    pub code: Option<String>,
}

/// Exemple de conversion sans code : avant (Bruno), après (Arnod).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionExample {
    pub before: String,
    pub after: String,
}

/// Comptes du rapport.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCounts {
    pub folders: usize,
    pub requests: usize,
    pub environments: usize,
    pub assertions: usize,
    pub extractions: usize,
    /// Variables locales créées (`vars:secret`, `.env`).
    pub secrets: usize,
    /// Valeurs dynamiques (`{{$randomUUID}}`…) rencontrées.
    pub dynamic_values: usize,
    /// Scripts et tests gardés sans exécution.
    pub scripts: usize,
}
