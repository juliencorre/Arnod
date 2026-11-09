//! Format OpenCollection 1.0.0, strict (docs/plan-opencollection.md).
//!
//! Un projet Arnod est une collection OpenCollection sur disque :
//! `opencollection.yml` à la racine, un fichier `.yml` par requête, des
//! dossiers réels et `environments/`. Tout fichier écrit est valide pour le
//! schéma officiel (`schema`), et ce qu'Arnod ne gère pas est conservé.

pub mod collection;
pub mod migrate;
pub mod request;
pub mod schema;
pub mod yaml;

#[cfg(test)]
mod tests;

/// Version de la spécification écrite par Arnod.
pub const SPEC_VERSION: &str = "1.0.0";

/// Fichier racine d'une collection (et donc d'un projet).
pub const ROOT_FILE: &str = "opencollection.yml";

/// Réglages d'un dossier.
pub const FOLDER_FILE: &str = "folder.yml";
