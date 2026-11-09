//! Menu natif des fenêtres.
//!
//! « Fichier » avec « Quitter » (F4) et « Fenêtre » (F7, F8, F9, F12) : ses
//! accélérateurs (Ctrl+Maj+O, Ctrl+Maj+D) valent dans toutes les fenêtres et
//! agissent sur la dernière fenêtre à zones utilisée.

use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime};

use crate::layout::Preset;
use crate::layout::windows::{MAIN, Windows};
use crate::windows::{self, WindowsState};

/// Identifiant de l'entrée « Quitter ».
pub const QUIT_ID: &str = "quit";
/// « Ouvrir la zone dans une fenêtre » (F8).
pub const DETACH_ZONE_ID: &str = "zone-fenetre";
/// « Détacher la barre latérale » (F9).
pub const DETACH_SIDEBAR_ID: &str = "barre-detacher";
/// « Rattacher toutes les fenêtres » (F12).
pub const REATTACH_ALL_ID: &str = "rattacher-tout";
/// Préfixe des dispositions (F7).
pub const PRESET_PREFIX: &str = "disposition-";

/// Dispositions du menu, avec leur identifiant et leur libellé.
const PRESETS: [(Preset, &str, &str); 5] = [
    (Preset::One, "une", "1 zone"),
    (Preset::TwoColumns, "deux-colonnes", "2 colonnes"),
    (Preset::TwoRows, "deux-lignes", "2 lignes"),
    (Preset::Three, "trois", "3 zones"),
    (Preset::Four, "quatre", "4 zones"),
];

/// Construit la barre de menus de l'application.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let quit = MenuItemBuilder::with_id(QUIT_ID, "&Quitter")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let file = SubmenuBuilder::new(app, "&Fichier").item(&quit).build()?;
    let mut layouts = SubmenuBuilder::new(app, "&Dispositions");
    for (_, id, label) in PRESETS {
        layouts = layouts.text(format!("{PRESET_PREFIX}{id}"), label);
    }
    let detach_zone = MenuItemBuilder::with_id(DETACH_ZONE_ID, "Ouvrir la &zone dans une fenêtre")
        .accelerator("CmdOrCtrl+Shift+O")
        .build(app)?;
    let detach_sidebar = MenuItemBuilder::with_id(DETACH_SIDEBAR_ID, "Détacher la &barre latérale")
        .accelerator("CmdOrCtrl+Shift+D")
        .build(app)?;
    let window = SubmenuBuilder::new(app, "Fe&nêtre")
        .item(&layouts.build()?)
        .separator()
        .item(&detach_zone)
        .item(&detach_sidebar)
        .text(REATTACH_ALL_ID, "&Rattacher toutes les fenêtres")
        .build()?;
    MenuBuilder::new(app).item(&file).item(&window).build()
}

/// Traite les clics sur les entrées du menu.
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id().as_ref();
    if id == QUIT_ID {
        // Fermeture propre : déclenche `RunEvent::ExitRequested` puis `RunEvent::Exit`.
        app.exit(0);
        return;
    }
    let target = app
        .state::<WindowsState>()
        .read(Windows::last_with_layout)
        .unwrap_or_else(|_| MAIN.to_owned());
    // Les erreurs (zone vide à détacher…) n'ont pas de fenêtre où s'afficher :
    // le menu reste sans effet.
    let result = match id {
        DETACH_ZONE_ID => windows::apply(app, |w| {
            let zone = w.layout_mut(&target)?.active_zone;
            w.detach_zone(&target, zone, None).map(|_| ())
        })
        .map(|_| ()),
        DETACH_SIDEBAR_ID => windows::apply(app, |w| {
            w.detach_sidebar(None);
            Ok(())
        })
        .map(|_| ()),
        REATTACH_ALL_ID => windows::apply(app, Windows::reattach_all).map(|_| ()),
        _ => PRESETS
            .iter()
            .find(|(_, name, _)| id.strip_prefix(PRESET_PREFIX) == Some(*name))
            .map_or(Ok(()), |(preset, _, _)| {
                windows::apply(app, |w| w.apply_preset(&target, *preset)).map(|_| ())
            }),
    };
    let _ = result;
}
