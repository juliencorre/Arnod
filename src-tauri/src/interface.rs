//! Préférences d'interface du poste (`docs/plan-v1.md`) : apparence (1.6-f) et raccourcis
//! (1.6-f bis).
//!
//! Module pur pour les règles (valeurs proposées, combinaisons valides, conflits, réservées) ;
//! `load` et `save` lisent et écrivent `interface.json` dans le dossier de données, de façon
//! atomique, comme `updates.json`. Rien n'est écrit dans un projet : ces réglages sont propres
//! au poste.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};

/// Thème de l'interface : celui du système, ou forcé (R1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename = "InterfaceTheme"))]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Zooms proposés, en pour cent (R2).
pub const ZOOMS: [u16; 8] = [80, 90, 100, 110, 125, 150, 175, 200];
/// Polices proposées pour les éditeurs, en plus de la police du système (R3). Une police
/// absente du poste laisse la place à la suivante de la pile (`--font-mono`).
pub const FONTS: [&str; 8] = [
    "Cascadia Mono",
    "Consolas",
    "DejaVu Sans Mono",
    "Fira Code",
    "JetBrains Mono",
    "Menlo",
    "Source Code Pro",
    "Ubuntu Mono",
];
/// Tailles proposées pour le texte des éditeurs, en pixels (R3).
pub const SIZES: [u8; 8] = [11, 12, 13, 14, 15, 16, 17, 18];

/// Apparence du poste (1.6-f).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    #[serde(default)]
    pub theme: Theme,
    /// Zoom de l'interface, en pour cent.
    #[serde(default = "default_zoom")]
    pub zoom: u16,
    /// Police des éditeurs ; `None` : la police à chasse fixe du système.
    #[serde(default)]
    pub editor_font: Option<String>,
    /// Taille du texte des éditeurs, en pixels.
    #[serde(default = "default_size")]
    pub editor_font_size: u8,
}

fn default_zoom() -> u16 {
    100
}

fn default_size() -> u8 {
    13
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            zoom: default_zoom(),
            editor_font: None,
            editor_font_size: default_size(),
        }
    }
}

/// Une apparence n'accepte que les valeurs proposées (R2, R3).
pub fn validate(appearance: &Appearance) -> Result<(), CommandError> {
    if !ZOOMS.contains(&appearance.zoom) {
        return Err(invalid(format!(
            "Zoom de {} % non proposé.",
            appearance.zoom
        )));
    }
    if let Some(font) = &appearance.editor_font
        && !FONTS.contains(&font.as_str())
    {
        return Err(invalid(format!("Police « {font} » non proposée.")));
    }
    if !SIZES.contains(&appearance.editor_font_size) {
        return Err(invalid(format!(
            "Taille de {} px non proposée.",
            appearance.editor_font_size
        )));
    }
    Ok(())
}

fn invalid(message: String) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

// ------------------------------------------------------------------------- raccourcis

/// Groupe d'actions de la liste des raccourcis (1.6-f bis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ShortcutGroup {
    /// Requête et réponse.
    Request,
    /// Fenêtre et onglets.
    Window,
    /// Projet et workspace.
    Project,
}

/// Où une combinaison agit : partout, ou seulement quand le focus est dans une partie de la
/// fenêtre. Deux actions de portées locales différentes peuvent partager une combinaison (R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ShortcutScope {
    Global,
    /// Éditeur d'une requête.
    Editor,
    /// Corps de la réponse.
    Response,
    /// Barre latérale.
    Sidebar,
    /// Onglet d'un contrat.
    Contract,
}

/// Action qui a un raccourci.
pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub group: ShortcutGroup,
    pub scope: ShortcutScope,
    /// Combinaisons par défaut (forme canonique, voir [`normalize`]) ; une action fixe en
    /// décrit la famille (« Ctrl+1…9 »).
    pub defaults: &'static [&'static str],
    /// La combinaison se change dans les préférences (R4).
    pub editable: bool,
}

const fn action(
    id: &'static str,
    label: &'static str,
    group: ShortcutGroup,
    scope: ShortcutScope,
    defaults: &'static [&'static str],
) -> Action {
    Action {
        id,
        label,
        group,
        scope,
        defaults,
        editable: true,
    }
}

const fn fixed(
    id: &'static str,
    label: &'static str,
    group: ShortcutGroup,
    defaults: &'static [&'static str],
) -> Action {
    Action {
        id,
        label,
        group,
        scope: ShortcutScope::Global,
        defaults,
        editable: false,
    }
}

use ShortcutGroup as G;
use ShortcutScope as S;

/// Actions et raccourcis par défaut, dans l'ordre de la liste (1.6-f bis). Les mêmes valeurs
/// sont dans `DEFAULT_SHORTCUTS` (`src/lib/shortcuts.ts`), vérifiées par un test.
pub const ACTIONS: [Action; 16] = [
    action("send", "Envoyer", G::Request, S::Editor, &["Ctrl+Enter"]),
    action(
        "copyCurl",
        "Copier en cURL",
        G::Request,
        S::Editor,
        &["Ctrl+Shift+C"],
    ),
    action(
        "filterResponse",
        "Filtrer la réponse",
        G::Request,
        S::Response,
        &["Ctrl+F"],
    ),
    action(
        "newTest",
        "Nouveau test",
        G::Request,
        S::Global,
        &["Ctrl+N"],
    ),
    action(
        "newFolder",
        "Nouveau dossier",
        G::Request,
        S::Global,
        &["Ctrl+Shift+N"],
    ),
    action(
        "filterTree",
        "Filtrer le référentiel",
        G::Request,
        S::Sidebar,
        &["Ctrl+F"],
    ),
    action(
        "saveContract",
        "Enregistrer le contrat",
        G::Request,
        S::Contract,
        &["Ctrl+S"],
    ),
    action(
        "togglePlacement",
        "Résultat dessous ou à côté",
        G::Window,
        S::Global,
        &["Ctrl+J"],
    ),
    action(
        "toggleSidebar",
        "Afficher ou masquer la barre latérale",
        G::Window,
        S::Global,
        &["Ctrl+Backslash"],
    ),
    action("search", "Rechercher", G::Window, S::Global, &["Ctrl+K"]),
    fixed(
        "nextTab",
        "Onglet suivant, précédent",
        G::Window,
        &["Ctrl+Tab", "Ctrl+Shift+Tab"],
    ),
    action(
        "closeTab",
        "Fermer l'onglet",
        G::Window,
        S::Global,
        &["Ctrl+W"],
    ),
    action(
        "environments",
        "Environnements du projet",
        G::Project,
        S::Global,
        &["Ctrl+Shift+E"],
    ),
    action(
        "manageWorkspaces",
        "Gérer les workspaces",
        G::Project,
        S::Global,
        &["Ctrl+Shift+W"],
    ),
    fixed(
        "workspaces",
        "Workspace 1 à 9, Tous les projets",
        G::Project,
        &["Ctrl+1…9", "Ctrl+0"],
    ),
    action(
        "preferences",
        "Préférences",
        G::Project,
        S::Global,
        &["Ctrl+,"],
    ),
];

/// Action connue, par son identifiant.
pub fn find(id: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|action| action.id == id)
}

/// Touches nommées acceptées après les modificateurs (forme canonique).
const NAMED_KEYS: [&str; 14] = [
    "Enter",
    "Tab",
    "Space",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
];
/// Signes acceptés tels quels (touche tapée), plus `Backslash` (touche physique).
const SIGNS: [&str; 10] = [",", ".", ";", "/", "'", "[", "]", "-", "=", "`"];

/// Combinaisons réservées : édition du texte, zoom, workspaces, onglets (R5). Le message dit
/// à quoi elles servent.
fn reserved(combo: &str) -> Option<&'static str> {
    match combo {
        "Ctrl+C" | "Ctrl+V" | "Ctrl+X" | "Ctrl+Z" | "Ctrl+Y" | "Ctrl+A" | "Ctrl+Shift+Z" => {
            Some("l'édition du texte")
        }
        "Ctrl+=" | "Ctrl+-" | "Ctrl+Shift+=" => Some("le zoom de l'interface"),
        "Ctrl+Tab" | "Ctrl+Shift+Tab" => Some("passer d'un onglet à l'autre"),
        _ if combo.len() == 6
            && combo.starts_with("Ctrl+")
            && combo.as_bytes()[5].is_ascii_digit() =>
        {
            Some("les workspaces")
        }
        _ => None,
    }
}

/// Forme canonique d'une combinaison : modificateurs dans l'ordre `Ctrl`, `Alt`, `Shift`, puis
/// une touche (lettre majuscule, chiffre, `F1` à `F12`, touche nommée, signe). Une combinaison
/// sans `Ctrl` ni `Alt` n'est acceptée que sur une touche de fonction (R5).
pub fn normalize(combo: &str) -> Result<String, CommandError> {
    let refuse = || invalid(format!("Combinaison « {combo} » non reconnue."));
    let mut parts: Vec<&str> = combo.split('+').collect();
    // « Ctrl++ » n'existe pas : le signe plus se note « = ».
    let key = parts
        .pop()
        .filter(|key| !key.is_empty())
        .ok_or_else(refuse)?;
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    for part in parts {
        let flag = match part {
            "Ctrl" => &mut ctrl,
            "Alt" => &mut alt,
            "Shift" => &mut shift,
            _ => return Err(refuse()),
        };
        if *flag {
            return Err(refuse());
        }
        *flag = true;
    }
    let upper = key.to_ascii_uppercase();
    let function = upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=12).contains(&n));
    let alphanumeric = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    let key = if alphanumeric || function {
        upper
    } else if NAMED_KEYS.contains(&key) || SIGNS.contains(&key) || key == "Backslash" {
        key.to_owned()
    } else {
        return Err(refuse());
    };
    if !ctrl && !alt && !function {
        return Err(invalid(format!(
            "« {combo} » : ajoutez Ctrl ou Alt, ou choisissez une touche de fonction."
        )));
    }
    let mut out = String::new();
    for (on, name) in [(ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            out.push_str(name);
        }
    }
    out.push_str(&key);
    Ok(out)
}

/// Raccourci d'une action, tel que la liste le montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShortcutView {
    pub id: String,
    pub label: String,
    pub group: ShortcutGroup,
    pub scope: ShortcutScope,
    /// Combinaisons en vigueur (canoniques) ; vide : l'action n'a plus de raccourci.
    pub keys: Vec<String>,
    pub defaults: Vec<String>,
    pub editable: bool,
}

/// Combinaisons changées sur ce poste, par action ; `None` : raccourci retiré (« Remplacer »
/// l'a donné à une autre action).
pub type Overrides = BTreeMap<String, Option<String>>;

/// Combinaisons en vigueur de chaque action, dans l'ordre de la liste.
pub fn effective(overrides: &Overrides) -> Vec<ShortcutView> {
    ACTIONS
        .iter()
        .map(|action| {
            let defaults: Vec<String> = action.defaults.iter().map(|&k| k.to_owned()).collect();
            let keys = match overrides.get(action.id) {
                Some(Some(keys)) if action.editable => vec![keys.clone()],
                Some(None) if action.editable => Vec::new(),
                _ => defaults.clone(),
            };
            ShortcutView {
                id: action.id.to_owned(),
                label: action.label.to_owned(),
                group: action.group,
                scope: action.scope,
                keys,
                defaults,
                editable: action.editable,
            }
        })
        .collect()
}

/// Deux portées se gênent : la même, ou l'une est globale (R6).
fn overlap(a: ShortcutScope, b: ShortcutScope) -> bool {
    a == b || a == ShortcutScope::Global || b == ShortcutScope::Global
}

/// Action qui tient déjà la combinaison, signalée avant d'être remplacée (R6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConflict {
    pub keys: String,
    pub action: String,
    pub label: String,
}

/// Issue d'un changement de raccourci : enregistré, ou en conflit (rien n'est changé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assigned {
    Done,
    Conflict(ShortcutConflict),
}

/// Donne la combinaison `keys` à l'action `id` (`None` : retire son raccourci). Une
/// combinaison tenue par une autre action de portée voisine est un conflit, sauf `replace` :
/// l'autre action perd alors son raccourci (R6). Une combinaison réservée est refusée (R5) ;
/// revenir à la combinaison par défaut efface la modification.
pub fn assign(
    overrides: &mut Overrides,
    id: &str,
    keys: Option<&str>,
    replace: bool,
) -> Result<Assigned, CommandError> {
    let action = find(id).ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::NotFound,
            format!("Action « {id} » inconnue."),
        )
    })?;
    if !action.editable {
        return Err(invalid(format!(
            "Le raccourci de « {} » ne se change pas.",
            action.label
        )));
    }
    let Some(keys) = keys else {
        overrides.insert(id.to_owned(), None);
        return Ok(Assigned::Done);
    };
    let keys = normalize(keys)?;
    if let Some(usage) = reserved(&keys) {
        return Err(invalid(format!("{keys} est réservé à {usage}.")));
    }
    let holders: Vec<ShortcutView> = effective(overrides)
        .into_iter()
        .filter(|other| other.id != id && other.keys.contains(&keys))
        .filter(|other| overlap(other.scope, action.scope))
        .collect();
    if let Some(holder) = holders.first() {
        if !replace || !holder.editable {
            return Ok(Assigned::Conflict(ShortcutConflict {
                keys,
                action: holder.id.clone(),
                label: holder.label.clone(),
            }));
        }
        for holder in &holders {
            overrides.insert(holder.id.clone(), None);
        }
    }
    if action.defaults == [keys.as_str()] {
        overrides.remove(id);
    } else {
        overrides.insert(id.to_owned(), Some(keys));
    }
    Ok(Assigned::Done)
}

// ------------------------------------------------------------------------- fichier

/// Contenu de `interface.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfacePrefs {
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub shortcuts: Overrides,
}

/// Préférences d'interface et valeurs proposées, envoyées à l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InterfaceView {
    pub appearance: Appearance,
    pub zooms: Vec<u16>,
    pub fonts: Vec<String>,
    pub sizes: Vec<u8>,
    pub shortcuts: Vec<ShortcutView>,
}

impl InterfaceView {
    pub fn of(prefs: &InterfacePrefs) -> Self {
        Self {
            appearance: prefs.appearance.clone(),
            zooms: ZOOMS.to_vec(),
            fonts: FONTS.iter().map(|&f| f.to_owned()).collect(),
            sizes: SIZES.to_vec(),
            shortcuts: effective(&prefs.shortcuts),
        }
    }
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("interface.json")
}

/// Préférences du poste ; un fichier absent ou illisible donne les valeurs par défaut, une
/// valeur hors des propositions revient à sa valeur par défaut.
pub fn load(data_dir: &Path) -> InterfacePrefs {
    let mut prefs: InterfacePrefs = fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if validate(&prefs.appearance).is_err() {
        prefs.appearance = Appearance::default();
    }
    prefs.shortcuts.retain(|id, keys| {
        find(id).is_some_and(|action| action.editable)
            && keys
                .as_deref()
                .is_none_or(|k| normalize(k).is_ok_and(|n| n == k))
    });
    prefs
}

/// Écrit les préférences de façon atomique.
pub fn save(data_dir: &Path, prefs: &InterfacePrefs) -> Result<(), CommandError> {
    fs::create_dir_all(data_dir).map_err(|error| {
        CommandError::io(
            "Création du dossier de données impossible",
            data_dir,
            &error,
        )
    })?;
    let path = file(data_dir);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(prefs).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture impossible : {error}"),
        )
    })?;
    fs::write(&tmp, text).map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, &path).map_err(|error| CommandError::io("Écriture impossible", &path, &error))
}

#[cfg(test)]
mod tests;
