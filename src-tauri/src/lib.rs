//! Cœur natif d'Arnod.
//!
//! L'interface (WebView) ne contient aucune logique métier : elle appelle
//! les commandes typées définies dans [`commands`]. Les commandes faillibles
//! renvoient une [`error::CommandError`].

pub mod commands;
pub mod error;
mod menu;

/// Point d'entrée de l'application, appelé par `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .menu(menu::build)
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![commands::hello::hello])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'Arnod");
}
