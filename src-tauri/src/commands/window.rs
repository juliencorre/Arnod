//! Domaine « fenêtre » : titre, zones et fenêtres détachées (F1 à F13).
//!
//! L'interface sait ce qui est affiché : elle demande le titre à chaque
//! changement de vue. Les zones et fenêtres vivent dans le cœur
//! ([`crate::layout`]) : chaque commande renvoie l'instantané de toutes les
//! fenêtres, aussi diffusé aux autres (`arnod://fenetres`). Appelées depuis
//! Rust, ces opérations ne demandent aucune permission WebView.

use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::layout::windows::{ActiveRequest, Bounds, Windows, WindowsFile};
use crate::layout::{Content, Placement, Preset, Side};
use crate::windows::{self, WindowsState};

/// Nom de l'application, seul titre quand rien n'est ouvert.
pub const APP_TITLE: &str = "Arnod";

/// Titre « Arnod — <sujet> » (ex. « Arnod — Boutique »), ou « Arnod » sans sujet.
pub fn window_title(subject: Option<&str>) -> String {
    match subject.map(str::trim).filter(|subject| !subject.is_empty()) {
        Some(subject) => format!("{APP_TITLE} — {subject}"),
        None => APP_TITLE.to_owned(),
    }
}

/// Met à jour le titre de la fenêtre avec le workspace ou le projet affiché.
#[tauri::command]
pub fn window_set_title(
    window: WebviewWindow,
    subject: Option<String>,
) -> Result<(), CommandError> {
    window
        .set_title(&window_title(subject.as_deref()))
        .map_err(|error| {
            CommandError::new(
                CommandErrorKind::Io,
                format!("Impossible de changer le titre de la fenêtre : {error}"),
            )
        })
}

/// Fenêtre qui appelle et état de toutes les fenêtres.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowContext {
    pub label: String,
    pub windows: WindowsFile,
    /// Requête active, que suit la fenêtre de résultat (D2).
    pub active_request: Option<ActiveRequest>,
}

/// Rôle et zones de la fenêtre appelante, au démarrage de son interface.
#[tauri::command]
pub fn windows_load(
    window: WebviewWindow,
    state: State<'_, WindowsState>,
) -> Result<WindowContext, CommandError> {
    Ok(WindowContext {
        label: window.label().to_owned(),
        windows: state.snapshot()?,
        active_request: state.read(Windows::active_request)?,
    })
}

/// Ouvre un contenu dans la fenêtre appelante (F1, Z3).
#[tauri::command]
pub fn layout_open(
    app: AppHandle,
    window: WebviewWindow,
    content: Content,
    zone: Option<Uuid>,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.layout_mut(&label)?.open(content, zone)).map(|(_, file)| file)
}

/// Ouvre la sélection de la barre latérale dans la fenêtre qui la suit (Z5).
#[tauri::command]
pub fn layout_open_selection(
    app: AppHandle,
    content: Content,
) -> Result<WindowsFile, CommandError> {
    windows::apply(&app, |w| w.open_selection(content)).map(|(_, file)| file)
}

#[tauri::command]
pub fn layout_activate(
    app: AppHandle,
    window: WebviewWindow,
    zone: Uuid,
    tab: Uuid,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.layout_mut(&label)?.activate(zone, tab)).map(|(_, file)| file)
}

/// Ferme un onglet (F1) ; une fenêtre de contenu vidée se ferme.
#[tauri::command]
pub fn layout_close(
    app: AppHandle,
    window: WebviewWindow,
    tab: Uuid,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.close_tab(&label, tab)).map(|(_, file)| file)
}

/// Déplace un onglet vers une zone de cette fenêtre ou d'une autre (F3).
#[tauri::command]
pub fn layout_move(
    app: AppHandle,
    window: WebviewWindow,
    tab: Uuid,
    to_window: String,
    zone: Option<Uuid>,
    index: Option<usize>,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.move_tab(&label, tab, &to_window, zone, index)).map(|(_, file)| file)
}

/// Ouvre un onglet dans une nouvelle zone, à droite ou en dessous (F2, F3, Z1).
#[tauri::command]
pub fn layout_split(
    app: AppHandle,
    window: WebviewWindow,
    zone: Uuid,
    tab: Uuid,
    side: Side,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.layout_mut(&label)?.split(zone, tab, side)).map(|(_, file)| file)
}

/// Règle une séparation (F4, Z4).
#[tauri::command]
pub fn layout_resize(
    app: AppHandle,
    window: WebviewWindow,
    split: Uuid,
    ratio: f64,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.layout_mut(&label)?.resize(split, ratio)).map(|(_, file)| file)
}

/// Découpage du menu Fenêtre › Dispositions (F7).
#[tauri::command]
pub fn layout_preset(
    app: AppHandle,
    window: WebviewWindow,
    preset: Preset,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.apply_preset(&label, preset)).map(|(_, file)| file)
}

/// Place du résultat dans une zone (F6).
#[tauri::command]
pub fn layout_set_placement(
    app: AppHandle,
    window: WebviewWindow,
    zone: Uuid,
    placement: Placement,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| {
        w.layout_mut(&label)?.set_placement(zone, placement)
    })
    .map(|(_, file)| file)
}

/// La fenêtre de contenu suit la sélection, ou reste épinglée (F10).
#[tauri::command]
pub fn layout_set_follow(
    app: AppHandle,
    window: WebviewWindow,
    follow: bool,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.set_follow(&label, follow)).map(|(_, file)| file)
}

/// Largeur de la barre latérale (F5).
#[tauri::command]
pub fn layout_set_sidebar_width(app: AppHandle, width: u32) -> Result<WindowsFile, CommandError> {
    windows::apply(&app, |w| {
        w.set_sidebar_width(width);
        Ok(())
    })
    .map(|(_, file)| file)
}

/// La zone devient une fenêtre, à `bounds` si donné (F8).
#[tauri::command]
pub fn window_detach_zone(
    app: AppHandle,
    window: WebviewWindow,
    zone: Uuid,
    bounds: Option<Bounds>,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.detach_zone(&label, zone, bounds)).map(|(_, file)| file)
}

/// L'onglet seul devient une fenêtre, à `bounds` si donné (F8).
#[tauri::command]
pub fn window_detach_tab(
    app: AppHandle,
    window: WebviewWindow,
    tab: Uuid,
    bounds: Option<Bounds>,
) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.detach_tab(&label, tab, bounds)).map(|(_, file)| file)
}

/// La barre latérale devient une fenêtre pilote (F9).
#[tauri::command]
pub fn window_detach_sidebar(
    app: AppHandle,
    bounds: Option<Bounds>,
) -> Result<WindowsFile, CommandError> {
    windows::apply(&app, |w| Ok(w.detach_sidebar(bounds))).map(|(_, file)| file)
}

/// Ouvre le résultat dans sa propre fenêtre (D1) ; déjà ouvert, sa fenêtre
/// passe au premier plan (R1).
#[tauri::command]
pub fn window_detach_result(
    app: AppHandle,
    bounds: Option<Bounds>,
) -> Result<WindowsFile, CommandError> {
    let (label, file) = windows::apply(&app, |w| Ok(w.detach_result(bounds)))?;
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
    }
    Ok(file)
}

/// Rattache la fenêtre appelante à la fenêtre principale (F12, Z6).
#[tauri::command]
pub fn window_reattach(app: AppHandle, window: WebviewWindow) -> Result<WindowsFile, CommandError> {
    let label = window.label().to_owned();
    windows::apply(&app, |w| w.reattach(&label)).map(|(_, file)| file)
}

/// Rattache toutes les fenêtres détachées (F12).
#[tauri::command]
pub fn window_reattach_all(app: AppHandle) -> Result<WindowsFile, CommandError> {
    windows::apply(&app, Windows::reattach_all).map(|(_, file)| file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titre_avec_ou_sans_sujet() {
        assert_eq!(window_title(Some("Boutique")), "Arnod — Boutique");
        assert_eq!(
            window_title(Some("  boutique-tests ")),
            "Arnod — boutique-tests"
        );
        assert_eq!(window_title(Some("   ")), "Arnod");
        assert_eq!(window_title(None), "Arnod");
    }
}
