//! Cœur natif d'Arnod.
//!
//! L'interface (WebView) ne contient aucune logique métier : elle appelle
//! les commandes typées définies dans [`commands`]. Les commandes faillibles
//! renvoient une [`error::CommandError`]. Les règles des projets et workspaces
//! vivent dans [`catalog`], leur persistance dans [`storage`] et [`manifest`],
//! l'état partagé dans [`state`].

pub mod catalog;
pub mod commands;
pub mod error;
pub mod manifest;
mod menu;
pub mod state;
pub mod storage;
#[cfg(test)]
mod ts_contract;

use tauri::Manager;

use crate::state::{AppState, DEFAULT_LOCATION_DIR};

/// Point d'entrée de l'application, appelé par `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Sélecteur de dossier, appelé uniquement depuis Rust : aucune permission WebView.
        .plugin(tauri_plugin_dialog::init())
        .menu(menu::build)
        .on_menu_event(menu::handle_event)
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let default_location = app.path().home_dir()?.join(DEFAULT_LOCATION_DIR);
            app.manage(AppState::load(data_dir, default_location));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hello::hello,
            commands::catalog::catalog_load,
            commands::catalog::undo,
            commands::projects::project_suggest_folder,
            commands::projects::project_pick_folder,
            commands::projects::project_create,
            commands::projects::project_add_existing,
            commands::projects::project_open,
            commands::workspaces::workspace_open,
            commands::workspaces::workspace_create,
            commands::workspaces::workspace_rename,
            commands::workspaces::workspace_set_color,
            commands::workspaces::workspace_set_default,
            commands::workspaces::workspace_add_projects,
            commands::workspaces::workspace_remove_project,
            commands::workspaces::workspace_delete,
            commands::window::window_set_title,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'Arnod");
}
