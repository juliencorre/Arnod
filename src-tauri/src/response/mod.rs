//! Corps d'une réponse reçue (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis) : format détecté,
//! mise en forme, aperçu HTML sans script, filtre JSONPath et vérification tirée d'un résultat.
//!
//! Module pur (ni Tauri, ni disque) : les commandes de `commands/response.rs` n'en sont que les
//! façades. Les chemins des valeurs sont écrits sous une forme unique (`path::canonical`), la même
//! dans la mise en forme et dans les résultats du filtre : l'interface surligne une valeur en
//! comparant des chemins, sans relire le JSON.

pub mod filter;
pub mod format;
pub mod markup;
pub mod path;
pub mod preview;
pub mod render;

#[cfg(test)]
mod tests;

pub use filter::{FilterMatch, FilterResult, filter};
pub use format::{ResponseFormat, detect, file_name};
pub use preview::{HtmlNode, preview};
pub use render::{RenderedBody, Token, TokenKind, render};
