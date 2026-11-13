//! Commandes cURL (`docs/plan-mvp-b-saisie-curl.md`) : lecture d'une commande
//! collée (2.1-a sexies) et écriture d'une requête prête à partir (5.1-b quater).
//!
//! Module pur : ni Tauri, ni disque, ni coffre. La lecture suit la syntaxe d'un
//! shell POSIX (bash, zsh) : guillemets simples et doubles, `$'…'`, `\` de fin
//! de ligne ; aucune variable n'est développée.

pub mod parse;
pub mod render;

#[cfg(test)]
mod tests;

pub use parse::{CurlRequest, Ignored, Secret, parse, take_secrets};
pub use render::{command, render};
