//! Saisie d'une requête (`docs/plan-mvp-b-saisie-curl.md`) : variables connues et
//! inconnues (5.1-b ter), en-têtes ajoutés à l'envoi, listes éditées en texte
//! (5.1-b quater).
//!
//! Module pur : il reçoit les environnements et les données locales déjà lus.

pub mod added;
pub mod text;
pub mod variables;

#[cfg(test)]
mod tests;

use serde::Serialize;

pub use added::{AddedHeader, AddedOrigin, added_headers, fill};
pub use text::{PairsFormat, PairsText, TextError, from_text, to_text};
pub use variables::{
    UnknownVariable, VariableSource, known_names, unknown, used_names, written_names,
};

/// Ce que l'éditeur montre pendant la saisie (`request_input`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestInput {
    /// Environnement actif, nommé dans les messages.
    pub environment: String,
    /// Variables connues : environnement actif (partagées et locales
    /// renseignées), puis dossiers et collection (R1).
    pub known: Vec<String>,
    /// Variables utilisées par la requête et inconnues (R2), valeurs dynamiques
    /// `$…` exclues.
    pub unknown: Vec<UnknownVariable>,
    /// En-têtes ajoutés par Arnod à l'envoi (R3).
    pub added: Vec<AddedHeader>,
}
