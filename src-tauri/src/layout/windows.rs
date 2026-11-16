//! Fenêtres de l'application et déplacements entre elles (F8 à F13, Z5 à Z8).
//!
//! La fenêtre principale (`main`) existe toujours. Une fenêtre de contenu porte
//! des zones détachées ; la barre latérale détachée (au plus une) pilote les
//! fenêtres qui suivent la sélection. Les fenêtres du système sont créées et
//! fermées par l'appelant d'après la liste avant et après chaque opération.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Content, Layout, Preset, Tab};
use crate::error::{CommandError, CommandErrorKind};

/// Étiquette de la fenêtre principale (celle de `tauri.conf.json`).
pub const MAIN: &str = "main";
/// Préfixe des fenêtres détachées (capacité `default` : `fenetre-*`).
pub const DETACHED_PREFIX: &str = "fenetre-";
/// Largeur de la barre latérale par défaut, en pixels (F5).
pub const DEFAULT_SIDEBAR_WIDTH: u32 = 260;
/// Bornes de la largeur de la barre latérale.
pub const MIN_SIDEBAR_WIDTH: u32 = 200;
pub const MAX_SIDEBAR_WIDTH: u32 = 600;
/// Version du format `windows.json`.
pub const WINDOWS_VERSION: u32 = 1;

/// Rôle d'une fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    /// Onglets des workspaces, barre latérale (sauf détachée) et zones.
    Main,
    /// Zones détachées, qui suivent la sélection ou restent épinglées.
    Content,
    /// Barre latérale détachée : elle pilote les fenêtres qui la suivent.
    Sidebar,
}

/// Position et taille d'une fenêtre à l'écran, en pixels physiques.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// État d'une fenêtre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub label: String,
    pub kind: WindowKind,
    /// Zones de la fenêtre ; absentes pour la barre latérale détachée.
    pub layout: Option<Layout>,
    /// Fenêtre de contenu : suit la sélection (sinon épinglée) (F10).
    #[serde(default)]
    pub follow: bool,
    /// Fenêtre principale : largeur de la barre latérale (F5).
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    /// Fenêtre principale : barre latérale détachée (F9).
    #[serde(default)]
    pub sidebar_detached: bool,
    /// Dernière position connue (fenêtres détachées), pour la restauration (F13).
    #[serde(default)]
    pub bounds: Option<Bounds>,
    /// Fenêtre principale : barre latérale masquée (Ctrl+\, 1.4-j, R8).
    #[serde(default)]
    pub sidebar_hidden: bool,
}

fn default_sidebar_width() -> u32 {
    DEFAULT_SIDEBAR_WIDTH
}

impl WindowState {
    fn main() -> Self {
        Self {
            label: MAIN.to_owned(),
            kind: WindowKind::Main,
            layout: Some(Layout::single(Vec::new())),
            follow: true,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_detached: false,
            bounds: None,
            sidebar_hidden: false,
        }
    }

    fn detached(kind: WindowKind, layout: Option<Layout>, bounds: Option<Bounds>) -> Self {
        let id = Uuid::new_v4().simple().to_string();
        Self {
            label: format!("{DETACHED_PREFIX}{}", &id[..8]),
            kind,
            layout,
            follow: true,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_detached: false,
            bounds,
            sidebar_hidden: false,
        }
    }

    /// La fenêtre accueille-t-elle la sélection de la barre latérale (Z5) ?
    fn takes_selection(&self) -> bool {
        match self.kind {
            WindowKind::Main => true,
            WindowKind::Content => self.follow,
            WindowKind::Sidebar => false,
        }
    }
}

/// Requête active : celle dont la fenêtre de résultat montre le résultat
/// (`docs/plan-resultat-detachable.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRequest {
    pub project_id: Uuid,
    pub path: String,
}

/// Fichier `windows.json` et instantané envoyé aux fenêtres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsFile {
    pub version: u32,
    pub windows: Vec<WindowState>,
}

/// Toutes les fenêtres, et l'ordre dans lequel elles ont eu le focus.
#[derive(Debug, Clone, PartialEq)]
pub struct Windows {
    windows: Vec<WindowState>,
    /// Étiquettes, de la plus récemment utilisée à la plus ancienne (Z5).
    recent: Vec<String>,
}

fn not_found() -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, "Cette fenêtre n'existe plus.")
}

fn refused(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

impl Default for Windows {
    fn default() -> Self {
        Self {
            windows: vec![WindowState::main()],
            recent: vec![MAIN.to_owned()],
        }
    }
}

impl Windows {
    /// Fenêtres d'un fichier ; les incohérences sont réparées : fenêtre principale
    /// absente ou en double, fenêtre de contenu sans zones, barre latérale
    /// détachée sans sa fenêtre (ou l'inverse).
    pub fn from_file(file: WindowsFile) -> Self {
        let mut main: Option<WindowState> = None;
        let mut others = Vec::new();
        for mut window in file.windows {
            match window.kind {
                WindowKind::Main if main.is_none() && window.label == MAIN => {
                    if window.layout.is_none() {
                        window.layout = Some(Layout::single(Vec::new()));
                    }
                    main = Some(window);
                }
                WindowKind::Content
                    if window.label.starts_with(DETACHED_PREFIX)
                        && window.layout.as_ref().is_some_and(|l| !l.tabs().is_empty()) =>
                {
                    others.push(window);
                }
                WindowKind::Sidebar
                    if window.label.starts_with(DETACHED_PREFIX)
                        && !others
                            .iter()
                            .any(|w: &WindowState| w.kind == WindowKind::Sidebar) =>
                {
                    window.layout = None;
                    others.push(window);
                }
                _ => {}
            }
        }
        let mut main = main.unwrap_or_else(WindowState::main);
        main.sidebar_width = main
            .sidebar_width
            .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
        main.sidebar_detached = others.iter().any(|w| w.kind == WindowKind::Sidebar);
        let mut windows = vec![main];
        windows.extend(others);
        for layout in windows.iter_mut().filter_map(|w| w.layout.as_mut()) {
            layout.repair();
        }
        let recent = windows.iter().map(|w| w.label.clone()).collect();
        Self { windows, recent }
    }

    pub fn to_file(&self) -> WindowsFile {
        WindowsFile {
            version: WINDOWS_VERSION,
            windows: self.windows.clone(),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        self.windows.iter().map(|w| w.label.clone()).collect()
    }

    pub fn get(&self, label: &str) -> Result<&WindowState, CommandError> {
        self.windows
            .iter()
            .find(|w| w.label == label)
            .ok_or_else(not_found)
    }

    fn get_mut(&mut self, label: &str) -> Result<&mut WindowState, CommandError> {
        self.windows
            .iter_mut()
            .find(|w| w.label == label)
            .ok_or_else(not_found)
    }

    fn main_mut(&mut self) -> &mut WindowState {
        let index = self
            .windows
            .iter()
            .position(|w| w.kind == WindowKind::Main)
            .unwrap_or(0);
        &mut self.windows[index]
    }

    /// Zones d'une fenêtre (principale ou de contenu).
    pub fn layout_mut(&mut self, label: &str) -> Result<&mut Layout, CommandError> {
        self.get_mut(label)?
            .layout
            .as_mut()
            .ok_or_else(|| refused("La barre latérale n'a pas de zones."))
    }

    /// Note la fenêtre qui reçoit le focus (Z5).
    pub fn focus(&mut self, label: &str) {
        if self.windows.iter().any(|w| w.label == label) {
            self.recent.retain(|l| l != label);
            self.recent.insert(0, label.to_owned());
        }
    }

    /// Fenêtre qui reçoit la sélection de la barre latérale : la plus récemment
    /// utilisée qui la suit, la principale à défaut (Z5).
    pub fn selection_target(&self) -> String {
        self.recent
            .iter()
            .filter_map(|label| self.windows.iter().find(|w| &w.label == label))
            .find(|w| w.takes_selection())
            .map_or_else(|| MAIN.to_owned(), |w| w.label.clone())
    }

    /// Fenêtres qui suivent la sélection, principale comprise.
    pub fn followers(&self) -> usize {
        self.windows.iter().filter(|w| w.takes_selection()).count()
    }

    /// Ouvre la sélection de la barre latérale (Z5), en aperçu si `preview`
    /// (1.3-a, R1). Renvoie la fenêtre choisie.
    pub fn open_selection(
        &mut self,
        content: Content,
        preview: bool,
    ) -> Result<String, CommandError> {
        let target = self.selection_target();
        self.layout_mut(&target)?.open_tab(content, None, preview)?;
        Ok(target)
    }

    /// Barre latérale de la fenêtre principale masquée ou affichée (1.4-j, R8).
    pub fn set_sidebar_hidden(&mut self, label: &str, hidden: bool) -> Result<(), CommandError> {
        let window = self.get_mut(label)?;
        if window.kind != WindowKind::Main {
            return Err(refused("Seule la fenêtre principale a une barre latérale."));
        }
        window.sidebar_hidden = hidden;
        Ok(())
    }

    /// Ferme plusieurs onglets d'une fenêtre (« Fermer les autres », « Fermer tout »,
    /// 1.3-b, R6). Un onglet inconnu refuse tout ; une fenêtre de contenu vidée se ferme.
    pub fn close_tabs(&mut self, label: &str, tabs: &[Uuid]) -> Result<(), CommandError> {
        let layout = self.layout_mut(label)?;
        if tabs.iter().any(|tab| layout.zone_of(*tab).is_none()) {
            return Err(CommandError::new(
                CommandErrorKind::NotFound,
                "Cet onglet n'existe plus.",
            ));
        }
        for tab in tabs {
            layout.close(*tab)?;
        }
        self.drop_empty();
        Ok(())
    }

    /// Fenêtre de contenu suit la sélection ou reste épinglée (F10).
    pub fn set_follow(&mut self, label: &str, follow: bool) -> Result<(), CommandError> {
        let window = self.get_mut(label)?;
        if window.kind != WindowKind::Content {
            return Err(refused("Seule une fenêtre de contenu peut être épinglée."));
        }
        window.follow = follow;
        Ok(())
    }

    pub fn set_sidebar_width(&mut self, width: u32) {
        self.main_mut().sidebar_width = width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
    }

    pub fn set_bounds(&mut self, label: &str, bounds: Bounds) {
        if let Ok(window) = self.get_mut(label) {
            window.bounds = Some(bounds);
        }
    }

    /// Ferme les fenêtres de contenu restées sans onglet.
    fn drop_empty(&mut self) {
        self.windows.retain(|w| {
            w.kind != WindowKind::Content || w.layout.as_ref().is_some_and(|l| !l.tabs().is_empty())
        });
        let labels = self.labels();
        self.recent.retain(|l| labels.contains(l));
    }

    fn add(&mut self, window: WindowState) -> String {
        let label = window.label.clone();
        self.windows.push(window);
        self.recent.insert(0, label.clone());
        label
    }

    /// Ferme un onglet (F1). Une fenêtre de contenu vidée se ferme.
    pub fn close_tab(&mut self, label: &str, tab: Uuid) -> Result<(), CommandError> {
        self.layout_mut(label)?.close(tab)?;
        self.drop_empty();
        Ok(())
    }

    /// Déplace un onglet vers une zone d'une fenêtre, la même ou une autre (F3).
    pub fn move_tab(
        &mut self,
        from: &str,
        tab: Uuid,
        to: &str,
        zone: Option<Uuid>,
        index: Option<usize>,
    ) -> Result<(), CommandError> {
        if from == to {
            let layout = self.layout_mut(from)?;
            let target = zone.unwrap_or(layout.active_zone);
            return layout.move_tab(tab, target, index);
        }
        let target = {
            let layout = self.layout_mut(to)?;
            let target = zone.unwrap_or(layout.active_zone);
            layout.zone(target)?;
            target
        };
        let mut moved = self.layout_mut(from)?.take(tab)?;
        moved.preview = false;
        self.layout_mut(to)?.insert(target, moved, index)?;
        self.drop_empty();
        Ok(())
    }

    /// La zone devient une fenêtre de contenu (F8). Renvoie son étiquette.
    pub fn detach_zone(
        &mut self,
        label: &str,
        zone: Uuid,
        bounds: Option<Bounds>,
    ) -> Result<String, CommandError> {
        let layout = self.layout_mut(label)?;
        if layout.zone(zone)?.tabs.is_empty() {
            return Err(refused(
                "Cette zone est vide : il n'y a rien à ouvrir dans une fenêtre.",
            ));
        }
        let taken = layout.take_zone(zone)?;
        let detached = WindowState::detached(
            WindowKind::Content,
            Some(Layout::single(taken.tabs)),
            bounds,
        );
        let created = self.add(detached);
        self.drop_empty();
        Ok(created)
    }

    /// L'onglet seul devient une fenêtre de contenu (F8).
    pub fn detach_tab(
        &mut self,
        label: &str,
        tab: Uuid,
        bounds: Option<Bounds>,
    ) -> Result<String, CommandError> {
        let mut taken: Tab = self.layout_mut(label)?.take(tab)?;
        taken.preview = false;
        let detached = WindowState::detached(
            WindowKind::Content,
            Some(Layout::single(vec![taken])),
            bounds,
        );
        let created = self.add(detached);
        self.drop_empty();
        Ok(created)
    }

    /// La barre latérale devient une fenêtre pilote (F9). Déjà détachée : sa
    /// fenêtre est renvoyée.
    pub fn detach_sidebar(&mut self, bounds: Option<Bounds>) -> String {
        if let Some(existing) = self.windows.iter().find(|w| w.kind == WindowKind::Sidebar) {
            return existing.label.clone();
        }
        self.main_mut().sidebar_detached = true;
        self.add(WindowState::detached(WindowKind::Sidebar, None, bounds))
    }

    /// Fenêtre qui porte l'onglet de résultat, s'il existe (R1).
    pub fn result_window(&self) -> Option<&WindowState> {
        self.windows.iter().find(|w| {
            w.layout
                .as_ref()
                .is_some_and(|layout| layout.tabs().iter().any(|t| t.content == Content::Result))
        })
    }

    /// Ouvre le résultat dans sa propre fenêtre (D1). Il n'y en a qu'un (R1) :
    /// s'il existe déjà, sa fenêtre est renvoyée. La fenêtre ne reçoit pas la
    /// sélection de la barre latérale (R2).
    pub fn detach_result(&mut self, bounds: Option<Bounds>) -> String {
        if let Some(existing) = self.result_window() {
            return existing.label.clone();
        }
        let mut window = WindowState::detached(
            WindowKind::Content,
            Some(Layout::single(vec![Tab::new(Content::Result)])),
            bounds,
        );
        window.follow = false;
        self.add(window)
    }

    /// Requête active (D2) : dans la fenêtre utilisée le plus récemment qui en
    /// montre une, l'onglet actif de la zone active, sinon d'une autre zone
    /// (le résultat peut occuper la zone active).
    pub fn active_request(&self) -> Option<ActiveRequest> {
        let by_recency = self
            .recent
            .iter()
            .filter_map(|label| self.windows.iter().find(|w| &w.label == label))
            .chain(
                self.windows
                    .iter()
                    .filter(|w| !self.recent.contains(&w.label)),
            );
        by_recency
            .filter_map(|w| w.layout.as_ref())
            .find_map(|layout| {
                let zones = layout.zones();
                let mut active_first = zones
                    .iter()
                    .filter(|z| z.id == layout.active_zone)
                    .chain(zones.iter().filter(|z| z.id != layout.active_zone));
                active_first.find_map(|zone| {
                    let tab = zone.tabs.iter().find(|t| Some(t.id) == zone.active_tab)?;
                    match &tab.content {
                        Content::Request { project_id, path } => Some(ActiveRequest {
                            project_id: *project_id,
                            path: path.clone(),
                        }),
                        _ => None,
                    }
                })
            })
    }

    /// Rattache une fenêtre détachée (F12, Z6) : ses onglets reviennent dans la
    /// zone active de la fenêtre principale ; la barre latérale reprend sa place.
    pub fn reattach(&mut self, label: &str) -> Result<(), CommandError> {
        let index = self
            .windows
            .iter()
            .position(|w| w.label == label)
            .ok_or_else(not_found)?;
        if self.windows[index].kind == WindowKind::Main {
            return Err(refused("La fenêtre principale n'est pas détachée."));
        }
        let window = self.windows.remove(index);
        self.recent.retain(|l| l != label);
        let main = self.main_mut();
        match window.kind {
            WindowKind::Sidebar => main.sidebar_detached = false,
            WindowKind::Content => {
                // Le résultat revient dans l'onglet de chaque requête (R4).
                let tabs = window
                    .layout
                    .map(|layout| {
                        layout
                            .tabs()
                            .into_iter()
                            .filter(|tab| tab.content != Content::Result)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(layout) = main.layout.as_mut() {
                    layout.absorb(tabs)?;
                }
            }
            WindowKind::Main => {}
        }
        Ok(())
    }

    /// Rattache toutes les fenêtres détachées (F12).
    pub fn reattach_all(&mut self) -> Result<(), CommandError> {
        let detached: Vec<String> = self
            .windows
            .iter()
            .filter(|w| w.kind != WindowKind::Main)
            .map(|w| w.label.clone())
            .collect();
        for label in detached {
            self.reattach(&label)?;
        }
        Ok(())
    }

    /// Découpage du menu Fenêtre › Dispositions (F7).
    pub fn apply_preset(&mut self, label: &str, preset: Preset) -> Result<(), CommandError> {
        self.layout_mut(label)?.apply_preset(preset);
        Ok(())
    }

    /// Garde les seuls onglets des projets connus ; renvoie `true` si un onglet
    /// a été fermé (projet oublié dans une fenêtre, F11).
    pub fn retain_projects(&mut self, known: &[Uuid]) -> bool {
        let forgotten: Vec<Uuid> = self
            .windows
            .iter()
            .filter_map(|w| w.layout.as_ref())
            .flat_map(Layout::tabs)
            .filter_map(|tab| tab.content.project_id())
            .filter(|id| !known.contains(id))
            .collect();
        for project_id in &forgotten {
            self.forget_project(*project_id);
        }
        !forgotten.is_empty()
    }

    /// Fenêtre à zones utilisée le plus récemment (menu Fenêtre).
    pub fn last_with_layout(&self) -> String {
        self.recent
            .iter()
            .filter_map(|label| self.windows.iter().find(|w| &w.label == label))
            .find(|w| w.layout.is_some())
            .map_or_else(|| MAIN.to_owned(), |w| w.label.clone())
    }

    /// Une requête ou un dossier renommé, déplacé : les onglets suivent (F11).
    pub fn rename_path(&mut self, project_id: Uuid, from: &str, to: &str) {
        for window in &mut self.windows {
            if let Some(layout) = window.layout.as_mut() {
                layout.rename_path(project_id, from, to);
            }
        }
    }

    /// Une requête ou un dossier mis à la corbeille : ses onglets se ferment (F11).
    pub fn close_path(&mut self, project_id: Uuid, path: &str) {
        for window in &mut self.windows {
            if let Some(layout) = window.layout.as_mut() {
                layout.close_path(project_id, path);
            }
        }
        self.drop_empty();
    }

    /// Projet oublié : ses onglets se ferment partout.
    pub fn forget_project(&mut self, project_id: Uuid) {
        for window in &mut self.windows {
            if let Some(layout) = window.layout.as_mut() {
                layout.forget_project(project_id);
            }
        }
        self.drop_empty();
    }
}
