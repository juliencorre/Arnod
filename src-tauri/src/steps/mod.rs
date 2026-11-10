//! Catalogue des natures d'étape (`docs/etude-catalogue-etapes.md`,
//! `docs/plan-accueil-non-classes.md`) : définitions déclaratives, leur
//! reconnaissance et les valeurs d'un essai.
//!
//! Module pur : les définitions viennent aujourd'hui du binaire (intégrées),
//! demain d'un cache synchronisé, sans changer ce qui suit.

pub mod definition;
pub mod detect;
pub mod http;
pub mod values;

#[cfg(test)]
mod tests;

use serde::Serialize;

pub use definition::{Definition, Family};
pub use detect::Detection;

/// Date des définitions intégrées à cette version d'Arnod.
pub const BUILTIN_DATE: &str = "2026-11-10";

/// Textes des définitions intégrées, dans l'ordre du catalogue.
const BUILTIN: [(&str, &str); 1] = [("rest.json", include_str!("../../steps/rest.json"))];

/// Origine d'une nature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// Livrée avec Arnod.
    Builtin,
}

/// Nature telle que l'interface l'affiche et en rend le formulaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepNature {
    #[serde(flatten)]
    pub definition: Definition,
    pub origin: Origin,
}

/// Source des définitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSource {
    /// Nom du catalogue (`arnod-steps`).
    pub name: String,
    /// Date des définitions, `AAAA-MM-JJ`.
    pub updated: String,
    /// Synchronisation en place ; `false` : définitions intégrées seulement.
    pub synced: bool,
}

/// Catalogue des natures disponibles sur ce poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepCatalog {
    pub source: CatalogSource,
    pub natures: Vec<StepNature>,
}

/// Définitions intégrées valides (R1) ; une définition invalide n'est pas proposée.
pub fn builtin() -> Vec<Definition> {
    BUILTIN
        .iter()
        .filter_map(|(_, text)| definition::parse(text).ok())
        .collect()
}

/// Catalogue de ce poste.
pub fn catalog() -> StepCatalog {
    StepCatalog {
        source: CatalogSource {
            name: "arnod-steps".to_owned(),
            updated: BUILTIN_DATE.to_owned(),
            synced: false,
        },
        natures: builtin()
            .into_iter()
            .map(|definition| StepNature {
                definition,
                origin: Origin::Builtin,
            })
            .collect(),
    }
}

/// Définition de la nature `id`.
pub fn find(id: &str) -> Option<Definition> {
    builtin().into_iter().find(|d| d.id == id)
}

/// Nature reconnue de `input` (R2).
pub fn detect(input: &str) -> Option<Detection> {
    detect::detect(&builtin(), input)
}
