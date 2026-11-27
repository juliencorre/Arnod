//! Préférences d'interface du poste (`docs/plan-v1.md`) : apparence (1.6-f) et raccourcis
//! (1.6-f bis). Les règles sont dans [`crate::interface`] ; chaque changement est enregistré
//! dans `interface.json`, appliqué aux fenêtres (thème de la fenêtre native, zoom) et diffusé
//! par `arnod://interface`.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow};

use crate::error::CommandError;
use crate::interface::{self, Appearance, Assigned, InterfacePrefs, InterfaceView, Theme};
use crate::state::AppState;
use crate::windows::events;

/// Issue d'un changement de raccourci : la vue, et l'action qui tient déjà la combinaison si
/// rien n'a été changé (R6).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShortcutChange {
    pub view: InterfaceView,
    pub conflict: Option<interface::ShortcutConflict>,
}

fn native_theme(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// Thème de la fenêtre native (barre de titre, contrôles du système) et zoom de la page (R1,
/// R2). Différé sur la boucle d'événements : appliqué pendant la commande, sur le fil
/// principal et pendant le chargement de la page, il bloquait le démarrage (WebKitGTK). Au
/// chargement (`changed` faux), les réglages par défaut ne touchent pas la fenêtre. Un échec
/// (plateforme sans zoom) n'empêche rien : la page applique son thème seule.
fn apply<R: Runtime>(window: &WebviewWindow<R>, appearance: &Appearance, changed: bool) {
    let theme = appearance.theme;
    let zoom = appearance.zoom;
    if !changed && theme == Theme::System && zoom == 100 {
        return;
    }
    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        if changed || theme != Theme::System {
            let _ = target.set_theme(native_theme(theme));
        }
        if changed || zoom != 100 {
            let _ = target.set_zoom(f64::from(zoom) / 100.0);
        }
    });
}

/// Enregistre, applique à toutes les fenêtres et diffuse.
fn commit<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    prefs: &InterfacePrefs,
) -> Result<InterfaceView, CommandError> {
    interface::save(state.data_dir(), prefs)?;
    for window in app.webview_windows().values() {
        apply(window, &prefs.appearance, true);
    }
    let view = InterfaceView::of(prefs);
    let _ = app.emit(events::INTERFACE, &view);
    Ok(view)
}

/// Préférences d'interface du poste ; la fenêtre qui les lit prend son thème et son zoom.
#[tauri::command]
pub fn interface_load<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
) -> InterfaceView {
    let prefs = interface::load(state.data_dir());
    apply(&window, &prefs.appearance, false);
    InterfaceView::of(&prefs)
}

/// Thème, zoom et police des éditeurs (1.6-f).
#[tauri::command]
pub fn interface_save_appearance<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    appearance: Appearance,
) -> Result<InterfaceView, CommandError> {
    interface::validate(&appearance)?;
    let mut prefs = interface::load(state.data_dir());
    prefs.appearance = appearance;
    commit(&app, &state, &prefs)
}

/// Donne une combinaison à une action (`keys` absent : retire son raccourci). Un conflit est
/// renvoyé sans rien changer, sauf `replace` (R6).
#[tauri::command]
pub fn interface_set_shortcut<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    action: String,
    keys: Option<String>,
    replace: bool,
) -> Result<ShortcutChange, CommandError> {
    let mut prefs = interface::load(state.data_dir());
    match interface::assign(&mut prefs.shortcuts, &action, keys.as_deref(), replace)? {
        Assigned::Conflict(conflict) => Ok(ShortcutChange {
            view: InterfaceView::of(&prefs),
            conflict: Some(conflict),
        }),
        Assigned::Done => Ok(ShortcutChange {
            view: commit(&app, &state, &prefs)?,
            conflict: None,
        }),
    }
}

/// « Rétablir les raccourcis par défaut » (R7).
#[tauri::command]
pub fn interface_reset_shortcuts<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<InterfaceView, CommandError> {
    let mut prefs = interface::load(state.data_dir());
    prefs.shortcuts.clear();
    commit(&app, &state, &prefs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_conflit_se_serialise_avec_la_vue() {
        let prefs = InterfacePrefs::default();
        let change = ShortcutChange {
            view: InterfaceView::of(&prefs),
            conflict: Some(interface::ShortcutConflict {
                keys: "Ctrl+J".into(),
                action: "togglePlacement".into(),
                label: "Résultat dessous ou à côté".into(),
            }),
        };
        let json = serde_json::to_value(&change).unwrap();
        assert_eq!(json["conflict"]["action"], "togglePlacement");
        assert_eq!(json["view"]["appearance"]["theme"], "system");
        assert_eq!(json["view"]["zooms"][2], 100);
        assert_eq!(json["view"]["shortcuts"][0]["id"], "send");
        assert_eq!(json["view"]["shortcuts"][0]["group"], "request");
        assert_eq!(json["view"]["shortcuts"][0]["scope"], "editor");
        assert_eq!(json["view"]["shortcuts"][0]["keys"][0], "Ctrl+Enter");
    }

    #[test]
    fn theme_natif_de_chaque_choix() {
        assert_eq!(native_theme(Theme::System), None);
        assert_eq!(native_theme(Theme::Light), Some(tauri::Theme::Light));
        assert_eq!(native_theme(Theme::Dark), Some(tauri::Theme::Dark));
    }
}
