//! Fenêtres du système et synchronisation entre elles (F8 à F13, Z5 à Z8).
//!
//! [`WindowsState`] garde les fenêtres ([`Windows`], module pur) et les
//! enregistre dans `windows.json`. [`apply`] applique une opération puis met
//! les fenêtres du système en accord : création des nouvelles, fermeture des
//! disparues, et diffusion de l'instantané à toutes les fenêtres.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl,
    WebviewWindowBuilder, Window, WindowEvent,
};
use uuid::Uuid;

use crate::catalog::snapshot::CatalogSnapshot;
use crate::error::{CommandError, CommandErrorKind};
use crate::layout::windows::{Bounds, MAIN, WindowKind, WindowState, Windows, WindowsFile};

/// Fichier des fenêtres, dans les données locales (Z8).
pub const WINDOWS_FILE: &str = "windows.json";

/// Événements diffusés aux fenêtres (F11).
pub mod events {
    /// Toutes les fenêtres et leurs zones (`WindowsFile`).
    pub const WINDOWS: &str = "arnod://fenetres";
    /// Nouvel instantané du catalogue (`CatalogSnapshot`).
    pub const CATALOG: &str = "arnod://catalogue";
    /// Modification d'un projet (`ProjectChange`).
    pub const PROJECT: &str = "arnod://projet";
    /// Demande de fermeture d'une fenêtre détachée : elle enregistre puis se rattache (Z6).
    pub const CLOSE: &str = "arnod://fermer";
}

/// Partie d'un projet modifiée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectScope {
    Tree,
    Request,
    Environments,
    /// Contrats du projet (ajout, source, suppression).
    Contracts,
}

/// Modification d'un projet, diffusée aux autres fenêtres (F11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChange {
    pub project_id: Uuid,
    pub scope: ProjectScope,
    /// Requête concernée (portée `request`).
    pub path: Option<String>,
    /// Fenêtre à l'origine du changement, qui l'a déjà appliqué.
    pub origin: String,
}

/// Fenêtres de l'application et leur fichier.
pub struct WindowsState {
    data_dir: PathBuf,
    inner: Mutex<Windows>,
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(WINDOWS_FILE)
}

fn busy() -> CommandError {
    CommandError::new(
        CommandErrorKind::Io,
        "L'état des fenêtres est indisponible : redémarrez l'application.",
    )
}

impl WindowsState {
    /// Charge `windows.json` ; absent ou illisible : une seule fenêtre, une zone vide.
    pub fn load(data_dir: PathBuf) -> Self {
        let windows = fs::read_to_string(file(&data_dir))
            .ok()
            .and_then(|text| serde_json::from_str::<WindowsFile>(&text).ok())
            .map(Windows::from_file)
            .unwrap_or_default();
        Self {
            data_dir,
            inner: Mutex::new(windows),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Windows>, CommandError> {
        self.inner.lock().map_err(|_| busy())
    }

    pub fn snapshot(&self) -> Result<WindowsFile, CommandError> {
        Ok(self.lock()?.to_file())
    }

    pub fn read<T>(&self, read: impl FnOnce(&Windows) -> T) -> Result<T, CommandError> {
        Ok(read(&*self.lock()?))
    }

    /// Applique une opération ; en cas d'erreur, rien ne change. Renvoie la
    /// valeur, les étiquettes avant et l'instantané après.
    pub fn update<T>(
        &self,
        change: impl FnOnce(&mut Windows) -> Result<T, CommandError>,
    ) -> Result<(T, Vec<String>, WindowsFile), CommandError> {
        let mut windows = self.lock()?;
        let mut next = windows.clone();
        let value = change(&mut next)?;
        let before = windows.labels();
        *windows = next;
        let file = windows.to_file();
        // Une écriture manquée n'empêche pas de travailler : la suivante rattrape.
        let _ = self.save(&file);
        Ok((value, before, file))
    }

    /// Modification sans conséquence visible (focus, position) : ni écriture, ni diffusion.
    pub fn quietly(&self, change: impl FnOnce(&mut Windows)) {
        if let Ok(mut windows) = self.lock() {
            change(&mut windows);
        }
    }

    /// Écrit `windows.json` de façon atomique (fichier temporaire puis renommage).
    pub fn save(&self, windows: &WindowsFile) -> io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        let target = file(&self.data_dir);
        let tmp = target.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(windows)?)?;
        fs::rename(&tmp, &target)
    }

    /// Enregistre l'état courant (positions comprises), à la sortie.
    pub fn flush(&self) {
        if let Ok(file) = self.snapshot() {
            let _ = self.save(&file);
        }
    }
}

/// Applique une opération sur les fenêtres puis met le système en accord :
/// fenêtres créées ou fermées, instantané diffusé à toutes (F11).
pub fn apply<R: Runtime, T>(
    app: &AppHandle<R>,
    change: impl FnOnce(&mut Windows) -> Result<T, CommandError>,
) -> Result<(T, WindowsFile), CommandError> {
    let state = app.state::<WindowsState>();
    let (value, before, file) = state.update(change)?;
    for window in file.windows.iter().filter(|w| !before.contains(&w.label)) {
        spawn(app, window.clone());
    }
    for label in before
        .iter()
        .filter(|label| !file.windows.iter().any(|w| &w.label == *label))
    {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.destroy();
        }
    }
    let _ = app.emit(events::WINDOWS, &file);
    Ok((value, file))
}

/// Crée la fenêtre du système d'une fenêtre détachée, hors du fil principal
/// (une création depuis une commande synchrone bloquerait sous Windows).
pub fn spawn<R: Runtime>(app: &AppHandle<R>, window: WindowState) {
    if window.kind == WindowKind::Main {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let (title, width, height, min_width) = match window.kind {
            WindowKind::Sidebar => ("Arnod — Barre latérale", 320.0, 760.0, 240.0),
            _ => ("Arnod", 980.0, 720.0, 480.0),
        };
        let mut builder =
            WebviewWindowBuilder::new(&app, &window.label, WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(width, height)
                .min_inner_size(min_width, 360.0)
                // Glisser-déposer HTML (onglets, projets), comme la fenêtre principale.
                .disable_drag_drop_handler();
        if let Some(bounds) = window.bounds {
            builder = builder
                .position(f64::from(bounds.x), f64::from(bounds.y))
                .inner_size(f64::from(bounds.width), f64::from(bounds.height));
        }
        // Échec de création (écran indisponible…) : la fenêtre se rattache.
        if builder.build().is_err() {
            let label = window.label.clone();
            let _ = apply(&app, |windows| windows.reattach(&label));
        }
    });
}

/// Recrée au démarrage les fenêtres détachées enregistrées (F13).
pub fn restore<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<WindowsState>();
    if let Ok(file) = state.snapshot() {
        for window in file.windows {
            spawn(app, window);
        }
    }
}

/// Position et taille logiques d'une fenêtre, pour la restauration (F13).
fn bounds_of<R: Runtime>(window: &Window<R>) -> Option<Bounds> {
    let scale = window.scale_factor().ok()?;
    let position: LogicalPosition<f64> = window.outer_position().ok()?.to_logical(scale);
    let size: LogicalSize<f64> = window.inner_size().ok()?.to_logical(scale);
    // Conversions bornées : des coordonnées d'écran tiennent dans un i32.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(Bounds {
        x: position.x.round() as i32,
        y: position.y.round() as i32,
        width: size.width.round().max(0.0) as u32,
        height: size.height.round().max(0.0) as u32,
    })
}

/// Événements des fenêtres : focus (Z5), position (F13), fermetures (Z6, Z7).
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let app = window.app_handle();
    let state = app.state::<WindowsState>();
    let label = window.label().to_owned();
    match event {
        WindowEvent::Focused(true) => state.quietly(|windows| windows.focus(&label)),
        WindowEvent::Moved(_) | WindowEvent::Resized(_) if label != MAIN => {
            if let Some(bounds) = bounds_of(window) {
                state.quietly(|windows| windows.set_bounds(&label, bounds));
            }
        }
        WindowEvent::CloseRequested { api, .. } if label != MAIN => {
            // La fenêtre enregistre ses requêtes modifiées puis se rattache (Z6).
            if state
                .read(|windows| windows.get(&label).is_ok())
                .unwrap_or(false)
            {
                api.prevent_close();
                let _ = app.emit_to(label.as_str(), events::CLOSE, ());
            }
        }
        WindowEvent::Destroyed => {
            if label == MAIN {
                // Fermer la fenêtre principale quitte Arnod (Z7).
                state.flush();
                app.exit(0);
            } else if state
                .read(|windows| windows.get(&label).is_ok())
                .unwrap_or(false)
            {
                // Fenêtre détruite sans se rattacher (plantage) : ses onglets reviennent.
                let _ = apply(app, |windows| windows.reattach(&label));
            }
        }
        _ => {}
    }
}

/// Diffuse un nouvel instantané du catalogue et ferme les onglets des projets
/// oubliés (F11).
pub fn catalog_changed<R: Runtime>(app: &AppHandle<R>, snapshot: &CatalogSnapshot) {
    let _ = app.emit(events::CATALOG, snapshot);
    let known: Vec<Uuid> = snapshot.projects.iter().map(|p| p.id).collect();
    let forgotten = app
        .state::<WindowsState>()
        .read(|windows| {
            windows.to_file().windows.iter().any(|w| {
                w.layout.as_ref().is_some_and(|layout| {
                    layout
                        .tabs()
                        .iter()
                        .any(|tab| !known.contains(&tab.content.project_id()))
                })
            })
        })
        .unwrap_or(false);
    if forgotten {
        let _ = apply(app, |windows| Ok(windows.retain_projects(&known)));
    }
}

/// Diffuse la modification d'un projet aux fenêtres (F11).
pub fn project_changed<R: Runtime>(
    app: &AppHandle<R>,
    origin: &str,
    project_id: Uuid,
    scope: ProjectScope,
    path: Option<&str>,
) {
    let _ = app.emit(
        events::PROJECT,
        ProjectChange {
            project_id,
            scope,
            path: path.map(str::to_owned),
            origin: origin.to_owned(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Content;

    #[test]
    fn enregistre_et_recharge_windows_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowsState::load(dir.path().to_path_buf());
        let (label, before, file) = state
            .update(|windows| {
                windows.layout_mut(MAIN)?.open(
                    Content::Run {
                        project_id: Uuid::from_u128(1),
                    },
                    None,
                )?;
                Ok(windows.detach_sidebar(None))
            })
            .unwrap();

        assert_eq!(before, [MAIN]);
        assert!(file.windows.iter().any(|w| w.label == label));
        let reloaded = WindowsState::load(dir.path().to_path_buf());
        assert_eq!(reloaded.snapshot().unwrap(), file);
    }

    #[test]
    fn une_operation_refusee_ne_change_rien() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowsState::load(dir.path().to_path_buf());
        let before = state.snapshot().unwrap();

        let error = state.update(|windows| windows.reattach(MAIN)).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        assert_eq!(state.snapshot().unwrap(), before);
        assert!(!file(dir.path()).exists());
    }

    #[test]
    fn fichier_illisible_donne_une_seule_fenetre() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(file(dir.path()), "{").unwrap();

        let state = WindowsState::load(dir.path().to_path_buf());

        assert_eq!(state.read(Windows::labels).unwrap(), [MAIN]);
    }

    #[test]
    fn modification_de_projet_en_camel_case() {
        let change = ProjectChange {
            project_id: Uuid::from_u128(1),
            scope: ProjectScope::Request,
            path: Some("a.yml".into()),
            origin: MAIN.into(),
        };
        assert_eq!(
            serde_json::to_value(change).unwrap(),
            serde_json::json!({
                "projectId": Uuid::from_u128(1).to_string(),
                "scope": "request",
                "path": "a.yml",
                "origin": "main"
            })
        );
    }
}
