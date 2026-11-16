//! Contrats de services (docs/plan-contrats-services.md) : lecture des contrats
//! OpenAPI 3.x et Swagger 2.0, requêtes créées depuis leurs opérations et
//! conformité des réponses.
//!
//! `parse`, `sample`, `generate` et `conformance` sont purs (ni Tauri, ni
//! disque) ; `file` range les contrats dans le dossier `contracts/` du projet.

pub mod conformance;
pub mod deduce;
pub mod file;
pub mod generate;
pub mod parse;
pub mod sample;

#[cfg(test)]
mod tests;

pub use file::CONTRACTS_DIR;
