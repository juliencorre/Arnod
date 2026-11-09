//! Zones et fenêtres (`docs/plan-fenetres-zones.md`, règles Z1 à Z6).
//!
//! Module pur, sans Tauri ni disque : une fenêtre range ses onglets de contenu
//! dans des zones, disposées en arbre binaire de séparations. [`windows`]
//! regroupe les fenêtres de l'application (principale, de contenu, barre
//! latérale détachée) et les déplacements de l'une à l'autre.

pub mod windows;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};

/// Au plus 4 zones par fenêtre (Z1).
pub const MAX_ZONES: usize = 4;
/// Proportion minimale d'une séparation (Z4).
pub const MIN_RATIO: f64 = 0.1;
/// Proportion maximale d'une séparation (Z4).
pub const MAX_RATIO: f64 = 0.9;

/// Ce qu'affiche un onglet. Un même contenu n'est ouvert qu'une fois par fenêtre (Z3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    /// Une requête du projet, par son chemin relatif.
    #[serde(rename_all = "camelCase")]
    Request { project_id: Uuid, path: String },
    /// Le run (en cours ou dernier) d'un projet.
    #[serde(rename_all = "camelCase")]
    Run { project_id: Uuid },
    /// Un contrat du projet, par son chemin relatif (`contracts/…`).
    #[serde(rename_all = "camelCase")]
    Contract { project_id: Uuid, path: String },
}

impl Content {
    pub fn project_id(&self) -> Uuid {
        match self {
            Self::Request { project_id, .. }
            | Self::Run { project_id }
            | Self::Contract { project_id, .. } => *project_id,
        }
    }
}

/// Onglet de contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: Uuid,
    pub content: Content,
}

impl Tab {
    pub fn new(content: Content) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
        }
    }
}

/// Place du résultat d'une requête dans sa zone (F6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    /// Dessous quand la zone est étroite, à droite sinon.
    #[default]
    Auto,
    Right,
    Below,
}

/// Zone : des onglets, l'un d'eux actif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub id: Uuid,
    pub tabs: Vec<Tab>,
    pub active_tab: Option<Uuid>,
    #[serde(default)]
    pub placement: Placement,
}

impl Zone {
    fn new(tabs: Vec<Tab>) -> Self {
        Self {
            id: Uuid::new_v4(),
            active_tab: tabs.first().map(|tab| tab.id),
            tabs,
            placement: Placement::Auto,
        }
    }

    /// Garde un onglet actif valide après un retrait.
    fn repair_active(&mut self, removed_index: usize) {
        if self
            .active_tab
            .is_some_and(|id| self.tabs.iter().any(|tab| tab.id == id))
        {
            return;
        }
        let index = removed_index.min(self.tabs.len().saturating_sub(1));
        self.active_tab = self.tabs.get(index).map(|tab| tab.id);
    }
}

/// Orientation d'une séparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Axis {
    /// Côte à côte : `first` à gauche, `second` à droite.
    Row,
    /// L'une sous l'autre : `first` en haut, `second` en bas.
    Column,
}

/// Côté où ouvrir une nouvelle zone (F2, F3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Right,
    Bottom,
}

/// Nœud de la disposition : une zone, ou une séparation entre deux nœuds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Node {
    Zone(Zone),
    #[serde(rename_all = "camelCase")]
    Split {
        id: Uuid,
        axis: Axis,
        /// Part de `first`, entre [`MIN_RATIO`] et [`MAX_RATIO`].
        ratio: f64,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    fn split(axis: Axis, first: Node, second: Node) -> Self {
        Self::Split {
            id: Uuid::new_v4(),
            axis,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    fn collect_zones<'a>(&'a self, into: &mut Vec<&'a Zone>) {
        match self {
            Self::Zone(zone) => into.push(zone),
            Self::Split { first, second, .. } => {
                first.collect_zones(into);
                second.collect_zones(into);
            }
        }
    }

    fn zone_mut(&mut self, id: Uuid) -> Option<&mut Zone> {
        match self {
            Self::Zone(zone) => (zone.id == id).then_some(zone),
            Self::Split { first, second, .. } => first.zone_mut(id).or_else(|| second.zone_mut(id)),
        }
    }

    fn split_ratio_mut(&mut self, id: Uuid) -> Option<&mut f64> {
        match self {
            Self::Zone(_) => None,
            Self::Split {
                id: split_id,
                ratio,
                first,
                second,
                ..
            } => {
                if *split_id == id {
                    Some(ratio)
                } else {
                    first
                        .split_ratio_mut(id)
                        .or_else(|| second.split_ratio_mut(id))
                }
            }
        }
    }

    /// Retire la zone `id` : sa voisine prend la place de leur séparation (Z2).
    /// Renvoie `false` si la zone est la racine ou absente.
    fn remove_zone(&mut self, id: Uuid) -> bool {
        let Self::Split { first, second, .. } = self else {
            return false;
        };
        let replacement = if matches!(first.as_ref(), Self::Zone(zone) if zone.id == id) {
            Some(std::mem::replace(second.as_mut(), placeholder()))
        } else if matches!(second.as_ref(), Self::Zone(zone) if zone.id == id) {
            Some(std::mem::replace(first.as_mut(), placeholder()))
        } else {
            None
        };
        match replacement {
            Some(node) => {
                *self = node;
                true
            }
            None => first.remove_zone(id) || second.remove_zone(id),
        }
    }

    /// Remplace la zone `id` par une séparation entre elle et `added`.
    fn wrap_zone(&mut self, id: Uuid, side: Side, added: Zone) -> bool {
        match self {
            Self::Zone(zone) if zone.id == id => {
                let current = Self::Zone(zone.clone());
                let axis = match side {
                    Side::Right => Axis::Row,
                    Side::Bottom => Axis::Column,
                };
                *self = Self::split(axis, current, Self::Zone(added));
                true
            }
            Self::Zone(_) => false,
            Self::Split { first, second, .. } => {
                if first.wrap_zone(id, side, added.clone()) {
                    true
                } else {
                    second.wrap_zone(id, side, added)
                }
            }
        }
    }
}

/// Reste du chemin `path` sous `prefix` (lui-même ou un descendant) : `""`
/// pour lui-même, `"/x.yml"` pour un descendant.
fn within<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    (rest.is_empty() || rest.starts_with('/')).then_some(rest)
}

/// Nœud provisoire, remplacé aussitôt (évite un `Option` dans l'arbre).
fn placeholder() -> Node {
    Node::Zone(Zone::new(Vec::new()))
}

/// Découpages vides du menu Fenêtre › Dispositions (F7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    One,
    TwoColumns,
    TwoRows,
    /// Une zone à gauche, deux l'une sous l'autre à droite.
    Three,
    /// Deux colonnes de deux zones.
    Four,
}

impl Preset {
    pub fn zone_count(self) -> usize {
        match self {
            Self::One => 1,
            Self::TwoColumns | Self::TwoRows => 2,
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

/// Disposition d'une fenêtre : l'arbre des zones et la zone active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub root: Node,
    pub active_zone: Uuid,
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, message)
}

fn refused(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

impl Layout {
    /// Une zone avec ces onglets.
    pub fn single(tabs: Vec<Tab>) -> Self {
        let zone = Zone::new(tabs);
        Self {
            active_zone: zone.id,
            root: Node::Zone(zone),
        }
    }

    /// Zones dans l'ordre de lecture (gauche puis droite, haut puis bas).
    pub fn zones(&self) -> Vec<&Zone> {
        let mut zones = Vec::new();
        self.root.collect_zones(&mut zones);
        zones
    }

    pub fn zone(&self, id: Uuid) -> Result<&Zone, CommandError> {
        self.zones()
            .into_iter()
            .find(|zone| zone.id == id)
            .ok_or_else(|| not_found("Cette zone n'existe plus."))
    }

    fn zone_mut(&mut self, id: Uuid) -> Result<&mut Zone, CommandError> {
        self.root
            .zone_mut(id)
            .ok_or_else(|| not_found("Cette zone n'existe plus."))
    }

    pub fn tabs(&self) -> Vec<&Tab> {
        self.zones()
            .into_iter()
            .flat_map(|zone| &zone.tabs)
            .collect()
    }

    /// Zone qui contient l'onglet `tab`.
    pub fn zone_of(&self, tab: Uuid) -> Option<Uuid> {
        self.zones()
            .into_iter()
            .find(|zone| zone.tabs.iter().any(|t| t.id == tab))
            .map(|zone| zone.id)
    }

    fn find_content(&self, content: &Content) -> Option<(Uuid, Uuid)> {
        self.zones().into_iter().find_map(|zone| {
            zone.tabs
                .iter()
                .find(|tab| tab.content == *content)
                .map(|tab| (zone.id, tab.id))
        })
    }

    /// Ouvre un contenu dans `zone` (ou la zone active) ; déjà ouvert, il est
    /// seulement activé (Z3). Renvoie l'onglet.
    pub fn open(&mut self, content: Content, zone: Option<Uuid>) -> Result<Uuid, CommandError> {
        if let Some((zone_id, tab_id)) = self.find_content(&content) {
            self.activate(zone_id, tab_id)?;
            return Ok(tab_id);
        }
        let target = zone.unwrap_or(self.active_zone);
        let tab = Tab::new(content);
        let id = tab.id;
        self.insert(target, tab, None)?;
        Ok(id)
    }

    /// Active un onglet et sa zone.
    pub fn activate(&mut self, zone: Uuid, tab: Uuid) -> Result<(), CommandError> {
        let target = self.zone_mut(zone)?;
        if !target.tabs.iter().any(|t| t.id == tab) {
            return Err(not_found("Cet onglet n'existe plus."));
        }
        target.active_tab = Some(tab);
        self.active_zone = zone;
        Ok(())
    }

    /// Range un onglet dans une zone, à `index` ou à la fin, et l'active. Un
    /// contenu déjà ouvert ailleurs dans la fenêtre est seulement activé (Z3).
    pub fn insert(
        &mut self,
        zone: Uuid,
        tab: Tab,
        index: Option<usize>,
    ) -> Result<(), CommandError> {
        self.zone(zone)?;
        if let Some((zone_id, tab_id)) = self.find_content(&tab.content) {
            return self.activate(zone_id, tab_id);
        }
        let target = self.zone_mut(zone)?;
        let at = index.unwrap_or(target.tabs.len()).min(target.tabs.len());
        target.active_tab = Some(tab.id);
        target.tabs.insert(at, tab);
        self.active_zone = zone;
        Ok(())
    }

    /// Retire un onglet ; une zone vidée disparaît, sauf la dernière (Z2).
    pub fn take(&mut self, tab: Uuid) -> Result<Tab, CommandError> {
        let zone_id = self
            .zone_of(tab)
            .ok_or_else(|| not_found("Cet onglet n'existe plus."))?;
        let zone = self.zone_mut(zone_id)?;
        let index = zone
            .tabs
            .iter()
            .position(|t| t.id == tab)
            .ok_or_else(|| not_found("Cet onglet n'existe plus."))?;
        let removed = zone.tabs.remove(index);
        zone.repair_active(index);
        if zone.tabs.is_empty() {
            self.remove_zone(zone_id);
        }
        Ok(removed)
    }

    /// Retire une zone (Z2) ; la dernière reste, vidée. Renvoie ses onglets.
    pub fn take_zone(&mut self, zone: Uuid) -> Result<Zone, CommandError> {
        let taken = self.zone(zone)?.clone();
        if !self.remove_zone(zone) {
            let only = self.zone_mut(zone)?;
            only.tabs.clear();
            only.active_tab = None;
        }
        Ok(taken)
    }

    fn remove_zone(&mut self, zone: Uuid) -> bool {
        let removed = self.root.remove_zone(zone);
        if removed && self.active_zone == zone {
            self.active_zone = self.zones().first().map_or(zone, |z| z.id);
        }
        removed
    }

    /// Ferme un onglet (F1).
    pub fn close(&mut self, tab: Uuid) -> Result<(), CommandError> {
        self.take(tab).map(|_| ())
    }

    /// Déplace un onglet de la fenêtre dans une autre de ses zones (F3).
    pub fn move_tab(
        &mut self,
        tab: Uuid,
        zone: Uuid,
        index: Option<usize>,
    ) -> Result<(), CommandError> {
        self.zone(zone)?;
        let source = self
            .zone_of(tab)
            .ok_or_else(|| not_found("Cet onglet n'existe plus."))?;
        if source == zone {
            let target = self.zone_mut(zone)?;
            let from = target
                .tabs
                .iter()
                .position(|t| t.id == tab)
                .ok_or_else(|| not_found("Cet onglet n'existe plus."))?;
            let moved = target.tabs.remove(from);
            let at = index.unwrap_or(target.tabs.len()).min(target.tabs.len());
            target.tabs.insert(at, moved);
            return self.activate(zone, tab);
        }
        let moved = self.take(tab)?;
        self.insert(zone, moved, index)
    }

    /// Ouvre l'onglet `tab` dans une nouvelle zone, à droite ou en dessous de
    /// `zone` (F2, F3). Au plus [`MAX_ZONES`] zones (Z1).
    pub fn split(&mut self, zone: Uuid, tab: Uuid, side: Side) -> Result<Uuid, CommandError> {
        self.zone(zone)?;
        if self.zones().len() >= MAX_ZONES {
            return Err(refused(format!(
                "Au plus {MAX_ZONES} zones par fenêtre : fermez ou détachez une zone d'abord."
            )));
        }
        let source = self
            .zone_of(tab)
            .ok_or_else(|| not_found("Cet onglet n'existe plus."))?;
        if source == zone && self.zone(zone)?.tabs.len() == 1 {
            return Err(refused(
                "Seul onglet de sa zone : il n'y a rien à diviser. Glissez-le dans une autre zone.",
            ));
        }
        let moved = self.take(tab)?;
        let added = Zone::new(vec![moved]);
        let added_id = added.id;
        if !self.root.wrap_zone(zone, side, added) {
            return Err(not_found("Cette zone n'existe plus."));
        }
        self.active_zone = added_id;
        Ok(added_id)
    }

    /// Règle une séparation (F4) ; la proportion reste entre 10 % et 90 % (Z4).
    pub fn resize(&mut self, split: Uuid, ratio: f64) -> Result<(), CommandError> {
        let value = self
            .root
            .split_ratio_mut(split)
            .ok_or_else(|| not_found("Cette séparation n'existe plus."))?;
        *value = if ratio.is_finite() {
            ratio.clamp(MIN_RATIO, MAX_RATIO)
        } else {
            0.5
        };
        Ok(())
    }

    pub fn set_placement(&mut self, zone: Uuid, placement: Placement) -> Result<(), CommandError> {
        self.zone_mut(zone)?.placement = placement;
        Ok(())
    }

    /// Applique un découpage (F7) : les onglets des zones existantes vont, dans
    /// l'ordre, dans les nouvelles zones ; le surplus rejoint la dernière.
    pub fn apply_preset(&mut self, preset: Preset) {
        let old: Vec<Zone> = self.zones().into_iter().cloned().collect();
        let count = preset.zone_count();
        let mut zones: Vec<Zone> = (0..count).map(|_| Zone::new(Vec::new())).collect();
        for (index, zone) in old.into_iter().enumerate() {
            let target = &mut zones[index.min(count - 1)];
            if target.tabs.is_empty() {
                target.placement = zone.placement;
                target.active_tab = zone.active_tab;
            }
            target.tabs.extend(zone.tabs);
            if target.active_tab.is_none() {
                target.active_tab = target.tabs.first().map(|tab| tab.id);
            }
        }
        self.active_zone = zones[0].id;
        let mut nodes = zones.into_iter().map(Node::Zone);
        let mut next = || nodes.next().unwrap_or_else(placeholder);
        self.root = match preset {
            Preset::One => next(),
            Preset::TwoColumns => Node::split(Axis::Row, next(), next()),
            Preset::TwoRows => Node::split(Axis::Column, next(), next()),
            Preset::Three => {
                let left = next();
                let right = Node::split(Axis::Column, next(), next());
                Node::split(Axis::Row, left, right)
            }
            Preset::Four => {
                // Ordre de lecture des zones : colonne de gauche, puis de droite.
                let left = Node::split(Axis::Column, next(), next());
                let right = Node::split(Axis::Column, next(), next());
                Node::split(Axis::Row, left, right)
            }
        };
    }

    /// Accueille des onglets dans la zone active, sans doublon (Z3, Z6).
    pub fn absorb(&mut self, tabs: Vec<Tab>) -> Result<(), CommandError> {
        let target = self.active_zone;
        for tab in tabs {
            self.insert(target, tab, None)?;
        }
        Ok(())
    }

    /// Renomme le chemin des requêtes `from` (fichier ou dossier) en `to`.
    pub fn rename_path(&mut self, project_id: Uuid, from: &str, to: &str) {
        fn visit(node: &mut Node, project_id: Uuid, from: &str, to: &str) {
            match node {
                Node::Zone(zone) => {
                    for tab in &mut zone.tabs {
                        if let Content::Request {
                            project_id: id,
                            path,
                        } = &mut tab.content
                            && *id == project_id
                            && let Some(rest) = within(path, from)
                        {
                            *path = format!("{to}{rest}");
                        }
                    }
                }
                Node::Split { first, second, .. } => {
                    visit(first, project_id, from, to);
                    visit(second, project_id, from, to);
                }
            }
        }
        visit(&mut self.root, project_id, from, to);
    }

    /// Ferme les onglets des requêtes ou du contrat situés à `path` (fichier ou dossier).
    pub fn close_path(&mut self, project_id: Uuid, path: &str) {
        let ids: Vec<Uuid> = self
            .tabs()
            .into_iter()
            .filter(|tab| match &tab.content {
                Content::Request {
                    project_id: id,
                    path: tab_path,
                }
                | Content::Contract {
                    project_id: id,
                    path: tab_path,
                } => *id == project_id && within(tab_path, path).is_some(),
                Content::Run { .. } => false,
            })
            .map(|tab| tab.id)
            .collect();
        for id in ids {
            let _ = self.take(id);
        }
    }

    /// Retire les onglets d'un projet oublié.
    pub fn forget_project(&mut self, project_id: Uuid) {
        let ids: Vec<Uuid> = self
            .tabs()
            .into_iter()
            .filter(|tab| tab.content.project_id() == project_id)
            .map(|tab| tab.id)
            .collect();
        for id in ids {
            let _ = self.take(id);
        }
    }
}
