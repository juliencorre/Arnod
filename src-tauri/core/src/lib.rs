//! Cœur d'Arnod, sans Tauri (`docs/plan-v2.md`, lot C).
//!
//! Modèle et fichiers des projets ([`request`], [`tree`], [`opencollection`], [`manifest`],
//! [`environment`], [`local`], [`contract`]), envoi ([`http`], [`network`], [`secrets`]), établi
//! ([`workbench`], [`statuses`], [`history`], [`results`]) et recherche ([`search`]). L'application
//! (Tauri) et la ligne de commande s'en servent ; rien ici ne connaît une fenêtre ni l'IPC. Le seul
//! lien de l'établi au runtime de l'application passe par [`workbench::Spawner`].

pub mod bruno;
pub mod catalog;
pub mod command;
pub mod contract;
pub mod curl;
pub mod environment;
pub mod error;
pub mod history;
pub mod http;
pub mod import;
pub mod input;
pub mod layout;
pub mod local;
pub mod manifest;
pub mod network;
pub mod opencollection;
pub mod report;
pub mod request;
pub mod response;
pub mod results;
pub mod search;
pub mod secrets;
pub mod startup;
pub mod state;
pub mod statuses;
pub mod steps;
pub mod storage;
#[cfg(feature = "sync")]
pub mod sync;
pub mod tree;
pub mod trial;
#[cfg(any(test, feature = "ts"))]
pub mod ts_contract;
pub mod watch;
pub mod workbench;

#[cfg(test)]
mod tests {
    /// Lot C (`docs/plan-v2.md`) : le cœur ne dépend pas de Tauri, la ligne de commande s'en sert
    /// sans fenêtre ni WebView.
    #[test]
    fn le_coeur_ne_depend_pas_de_tauri() {
        let manifest = include_str!("../Cargo.toml");
        let deps: Vec<&str> = manifest
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("tauri") || line.starts_with("wry"))
            .collect();
        assert!(deps.is_empty(), "dépendances Tauri dans le cœur : {deps:?}");
    }
}
