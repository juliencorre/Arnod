//! Cœur natif d'Arnod.
//!
//! L'interface (WebView) ne contient aucune logique métier : elle appelle
//! les commandes typées définies dans [`commands`]. Les commandes faillibles
//! renvoient une [`error::CommandError`]. Les règles des projets et workspaces
//! vivent dans [`catalog`], leur persistance dans [`storage`] et [`manifest`],
//! l'état partagé dans [`state`].

pub mod bruno;
pub mod catalog;
pub mod commands;
pub mod contract;
pub mod environment;
pub mod error;
pub mod http;
pub mod import;
pub mod layout;
pub mod local;
pub mod manifest;
mod menu;
pub mod opencollection;
pub mod request;
pub mod results;
pub mod secrets;
pub mod state;
pub mod statuses;
pub mod steps;
pub mod storage;
pub mod tree;
pub mod trial;
#[cfg(test)]
mod ts_contract;
pub mod windows;
pub mod workbench;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::results::ResultBook;
use crate::secrets::SystemSecretStore;
use crate::state::{AppState, DEFAULT_LOCATION_DIR};
use crate::windows::WindowsState;
use crate::workbench::Workbench;

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
            app.manage(AppState::load(data_dir.clone(), default_location));
            let bench = Arc::new(Workbench::new(Arc::new(SystemSecretStore)));
            // Avancement des runs et statuts des requêtes diffusés aux fenêtres (R3, R4).
            bench.set_notifier(Arc::new(windows::EventNotifier::new(app.handle().clone())));
            app.manage(bench);
            app.manage(WindowsState::load(data_dir));
            app.manage(Mutex::new(ResultBook::default()));
            // Chaque modification du catalogue est diffusée à toutes les fenêtres (F11).
            let handle = app.handle().clone();
            app.state::<AppState>().on_change(Box::new(move |snapshot| {
                windows::catalog_changed(&handle, snapshot)
            }));
            windows::restore(app.handle());
            Ok(())
        })
        .on_window_event(windows::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::hello::hello,
            commands::catalog::catalog_load,
            commands::catalog::undo,
            commands::projects::project_suggest_folder,
            commands::projects::project_pick_folder,
            commands::projects::project_create,
            commands::projects::project_add_existing,
            commands::projects::project_forget,
            commands::projects::project_open,
            commands::workspaces::workspace_open,
            commands::workspaces::workspace_create,
            commands::workspaces::workspace_rename,
            commands::workspaces::workspace_set_color,
            commands::workspaces::workspace_add_projects,
            commands::workspaces::workspace_remove_project,
            commands::workspaces::workspace_delete,
            commands::window::window_set_title,
            commands::window::windows_load,
            commands::window::layout_open,
            commands::window::layout_open_selection,
            commands::window::layout_activate,
            commands::window::layout_close,
            commands::window::layout_move,
            commands::window::layout_split,
            commands::window::layout_resize,
            commands::window::layout_preset,
            commands::window::layout_set_placement,
            commands::window::layout_set_follow,
            commands::window::layout_set_sidebar_width,
            commands::window::window_detach_zone,
            commands::window::window_detach_tab,
            commands::window::window_detach_sidebar,
            commands::window::window_detach_result,
            commands::window::window_reattach,
            commands::window::window_reattach_all,
            commands::contracts::contract_list,
            commands::contracts::contract_read,
            commands::contracts::contract_add,
            commands::contracts::contract_import_file,
            commands::contracts::contract_save,
            commands::contracts::contract_delete,
            commands::contracts::contract_generate,
            commands::contracts::contract_run,
            commands::tree::tree_load,
            commands::tree::tree_set_collapsed,
            commands::tree::tree_create_folder,
            commands::tree::tree_create_request,
            commands::tree::tree_rename,
            commands::tree::tree_move,
            commands::tree::tree_duplicate,
            commands::tree::tree_delete,
            commands::requests::request_read,
            commands::requests::request_save,
            commands::requests::request_send,
            commands::requests::request_cancel,
            commands::requests::request_result,
            commands::requests::request_statuses,
            commands::requests::result_ignore_check,
            commands::requests::request_propose_assertions,
            commands::requests::request_example_from_response,
            commands::environments::environments_load,
            commands::environments::environment_select,
            commands::environments::environment_save,
            commands::environments::environment_delete,
            commands::environments::environments_reorder,
            commands::environments::local_set,
            commands::runs::run_start,
            commands::runs::run_status,
            commands::runs::run_cancel,
            commands::runs::run_retry_failures,
            commands::runs::run_ignore_check,
            commands::runs::run_last,
            commands::runs::project_last_runs,
            commands::steps::steps_catalog,
            commands::steps::steps_detect,
            commands::trial::trial_send,
            // Lot C - Import Bruno.
            commands::import::import_pick,
            commands::import::import_detect,
            commands::import::import_run,
            commands::import::import_report,
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement d'Arnod")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Positions et tailles des fenêtres détachées, pour la restauration (F13).
                app.state::<WindowsState>().flush();
            }
        });
}
