//! Menu natif de la fenêtre principale.
//!
//! Volontairement minimal : un seul menu « Fichier » avec « Quitter » (F4).

use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// Identifiant de l'entrée « Quitter ».
pub const QUIT_ID: &str = "quit";

/// Construit la barre de menus de l'application.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let quit = MenuItemBuilder::with_id(QUIT_ID, "&Quitter")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let file = SubmenuBuilder::new(app, "&Fichier").item(&quit).build()?;
    MenuBuilder::new(app).item(&file).build()
}

/// Traite les clics sur les entrées du menu.
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id() == QUIT_ID {
        // Fermeture propre : déclenche `RunEvent::ExitRequested` puis `RunEvent::Exit`.
        app.exit(0);
    }
}
