//! Schémas des tests et Avant / Après des dossiers dans `extensions.arnod` de la racine
//! (`docs/plan-v4.md`, R1, R4, Q1) :
//!
//! ```yaml
//! extensions:
//!   arnod:
//!     tests:
//!       commandes/v2/parcours-achat: { onFailure: stop, steps: [...] }
//!     folders:
//!       commandes/v2: { beforeAll: [...], afterAll: [...] }
//! ```
//!
//! Lecture tolérante : une entrée illisible est ignorée (le test se joue comme la liste de ses
//! requêtes), jamais réécrite tant que l'utilisateur n'enregistre pas.

use std::collections::HashSet;
use std::path::Path;

use serde_json::{Map, Value};

use super::{Action, Flow, FolderHooks, Step};
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::collection;

/// Clé des schémas des tests.
pub const TESTS: &str = "tests";
/// Clé des Avant / Après des dossiers.
pub const FOLDERS: &str = "folders";

/// `extensions.arnod` d'un projet, lu une fois pour plusieurs tests.
#[derive(Debug, Clone, Default)]
pub struct Extensions {
    arnod: Option<Value>,
}

impl Extensions {
    /// Lit `extensions.arnod` du projet `root` ; une racine illisible n'en a pas.
    pub fn read(root: &Path) -> Self {
        Self {
            arnod: collection::read_arnod(root),
        }
    }

    pub fn from_value(arnod: Option<Value>) -> Self {
        Self { arnod }
    }

    fn entry(&self, key: &str, path: &str) -> Option<&Value> {
        self.arnod.as_ref()?.get(key)?.get(path)
    }

    /// Schéma écrit du test `path`, s'il est lisible.
    pub fn flow(&self, path: &str) -> Option<Flow> {
        serde_json::from_value(self.entry(TESTS, path)?.clone()).ok()
    }

    /// Avant / Après écrits du dossier `path` ; illisibles ou absents : aucun.
    pub fn hooks(&self, path: &str) -> FolderHooks {
        self.entry(FOLDERS, path)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }

    /// Dossiers qui ont des Avant / Après.
    pub fn hooked_folders(&self) -> Vec<String> {
        self.arnod
            .as_ref()
            .and_then(|a| a.get(FOLDERS))
            .and_then(Value::as_object)
            .map(|map| {
                map.keys()
                    .filter(|path| !self.hooks(path).is_empty())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Fichiers de requête propres à une étape (R14) : ceux des Avant / Après et des attentes,
    /// en chemins relatifs au projet. Ils n'apparaissent pas dans l'arborescence.
    pub fn owned_files(&self) -> HashSet<String> {
        let mut owned = HashSet::new();
        let Some(arnod) = &self.arnod else {
            return owned;
        };
        let join = |folder: &str, file: &str| {
            if folder.is_empty() {
                file.to_owned()
            } else {
                format!("{folder}/{file}")
            }
        };
        if let Some(folders) = arnod.get(FOLDERS).and_then(Value::as_object) {
            for folder in folders.keys() {
                for (_, step) in self.hooks(folder).steps() {
                    if let Some(file) = step.file() {
                        owned.insert(join(folder, file));
                    }
                }
            }
        }
        if let Some(tests) = arnod.get(TESTS).and_then(Value::as_object) {
            for test in tests.keys() {
                for step in self.flow(test).map(|f| f.steps).unwrap_or_default() {
                    if let Action::Wait {
                        file: Some(file), ..
                    } = &step.action
                    {
                        owned.insert(join(test, file));
                    }
                }
            }
        }
        owned
    }

    /// Fichiers d'un dossier propres à ses Avant / Après (noms de fichier).
    pub fn hook_files(&self, folder: &str) -> HashSet<String> {
        self.hooks(folder)
            .steps()
            .filter_map(|(_, step)| step.file().map(str::to_owned))
            .collect()
    }
}

/// Une étape écrite sans ce qui vaut la valeur par défaut (nom vide d'une requête).
fn step_value(step: &Step) -> Value {
    let mut value = serde_json::to_value(step).unwrap_or(Value::Null);
    if let Some(map) = value.as_object_mut()
        && map.get("name").and_then(Value::as_str) == Some("")
    {
        map.remove("name");
    }
    value
}

fn steps_value(steps: &[Step]) -> Value {
    Value::Array(steps.iter().map(step_value).collect())
}

fn flow_value(flow: &Flow) -> Value {
    let mut map = Map::new();
    map.insert(
        "onFailure".to_owned(),
        serde_json::to_value(flow.on_failure).unwrap_or(Value::Null),
    );
    map.insert("steps".to_owned(), steps_value(&flow.steps));
    Value::Object(map)
}

fn hooks_value(hooks: &FolderHooks) -> Value {
    let mut map = Map::new();
    for (key, steps) in [
        ("beforeAll", &hooks.before_all),
        ("beforeEach", &hooks.before_each),
        ("afterEach", &hooks.after_each),
        ("afterAll", &hooks.after_all),
    ] {
        if !steps.is_empty() {
            map.insert(key.to_owned(), steps_value(steps));
        }
    }
    Value::Object(map)
}

/// Range `value` sous `key`/`path` d'`extensions.arnod` (ou l'en retire).
fn set_entry(arnod: &mut Map<String, Value>, key: &str, path: &str, value: Option<Value>) {
    let section = arnod
        .entry(key.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if !section.is_object() {
        *section = Value::Object(Map::new());
    }
    if let Some(map) = section.as_object_mut() {
        match value {
            Some(value) => {
                map.insert(path.to_owned(), value);
            }
            None => {
                map.remove(path);
            }
        }
        if map.is_empty() {
            arnod.remove(key);
        }
    }
}

fn not_a_project(root: &Path) -> CommandError {
    CommandError::new(
        CommandErrorKind::NotAProject,
        format!("Le dossier {} n'est plus un projet.", root.display()),
    )
}

/// Écrit le schéma du test `path` ; `None` le retire (test linéaire, R1).
pub fn save_flow(root: &Path, path: &str, flow: Option<&Flow>) -> Result<(), CommandError> {
    let current = Extensions::read(root);
    let wanted = flow.map(flow_value);
    if current.entry(TESTS, path).cloned() == wanted {
        return Ok(());
    }
    if !collection::exists(root) {
        return Err(not_a_project(root));
    }
    collection::update_arnod(root, |arnod| set_entry(arnod, TESTS, path, wanted))
}

/// Écrit les Avant / Après du dossier `path` ; vides, ils sont retirés.
pub fn save_hooks(root: &Path, path: &str, hooks: &FolderHooks) -> Result<(), CommandError> {
    let current = Extensions::read(root);
    let wanted = (!hooks.is_empty()).then(|| hooks_value(hooks));
    if current.entry(FOLDERS, path).cloned() == wanted {
        return Ok(());
    }
    if !collection::exists(root) {
        return Err(not_a_project(root));
    }
    collection::update_arnod(root, |arnod| set_entry(arnod, FOLDERS, path, wanted))
}

/// Nouveau chemin de `path` quand `from` devient `to` (lui-même ou un de ses descendants).
pub fn moved(path: &str, from: &str, to: &str) -> Option<String> {
    if path == from {
        return Some(to.to_owned());
    }
    path.strip_prefix(from)
        .and_then(|rest| rest.strip_prefix('/'))
        .map(|rest| format!("{to}/{rest}"))
}

/// Dossier et nom de fichier d'un chemin relatif.
fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

/// Fait suivre une étape : test appelé, template joué (R4).
fn follow_step(step: &mut Step, from: &str, to: &str) -> bool {
    match &mut step.action {
        Action::Test { test } => match moved(test, from, to) {
            Some(new) => {
                *test = new;
                true
            }
            None => false,
        },
        Action::Template { template, .. } => match moved(template, from, to) {
            Some(new) => {
                *template = new;
                true
            }
            None => false,
        },
        _ => false,
    }
}

/// Renomme un fichier joué par l'étape (même dossier).
fn rename_file(step: &mut Step, old: &str, new: &str) -> bool {
    match &mut step.action {
        Action::Request { file } if file == old => {
            *file = new.to_owned();
            true
        }
        Action::Wait {
            file: Some(file), ..
        } if file == old => {
            *file = new.to_owned();
            true
        }
        _ => false,
    }
}

/// Transforme chaque entrée de `section` : `edit(chemin, valeur)` rend la nouvelle clé et la
/// nouvelle valeur, ou `None` pour la retirer.
fn rewrite(
    arnod: &mut Map<String, Value>,
    key: &str,
    mut edit: impl FnMut(&str, Value) -> Vec<(String, Value)>,
) -> bool {
    let Some(section) = arnod.get_mut(key).and_then(Value::as_object_mut) else {
        return false;
    };
    let before = section.clone();
    let mut after = Map::new();
    for (path, value) in before.clone() {
        for (new_path, new_value) in edit(&path, value) {
            after.insert(new_path, new_value);
        }
    }
    let changed = after != before;
    if changed {
        *section = after;
    }
    changed
}

/// Applique `edit` à chaque étape d'une valeur de schéma ou d'Avant / Après.
fn edit_steps(value: &mut Value, mut edit: impl FnMut(&mut Step) -> bool) -> bool {
    let mut changed = false;
    let lists: Vec<&mut Value> = match value {
        Value::Object(map) => map
            .iter_mut()
            .filter(|(k, _)| {
                matches!(
                    k.as_str(),
                    "steps" | "beforeAll" | "beforeEach" | "afterEach" | "afterAll"
                )
            })
            .map(|(_, v)| v)
            .collect(),
        _ => Vec::new(),
    };
    for list in lists {
        let Some(items) = list.as_array_mut() else {
            continue;
        };
        for item in items {
            let Ok(mut step) = serde_json::from_value::<Step>(item.clone()) else {
                continue;
            };
            if edit(&mut step) {
                *item = step_value(&step);
                changed = true;
            }
        }
    }
    changed
}

/// Un renommage ou un déplacement fait dans Arnod (`from` → `to`, dossier ou requête) fait
/// suivre les schémas, les Avant / Après et les renvois vers un test (R4).
pub fn follow(root: &Path, from: &str, to: &str) -> Result<(), CommandError> {
    if from == to || !collection::exists(root) {
        return Ok(());
    }
    let ext = Extensions::read(root);
    let Some(arnod) = ext.arnod.as_ref().and_then(Value::as_object) else {
        return Ok(());
    };
    let mut arnod = arnod.clone();
    let (old_dir, old_file) = split(from);
    let (new_dir, new_file) = split(to);
    let same_dir = old_dir == new_dir && old_file != new_file;
    let mut changed = false;
    for key in [super::store::TESTS, FOLDERS] {
        changed |= rewrite(&mut arnod, key, |path, mut value| {
            // Une requête renommée dans son dossier : les étapes qui la jouent suivent.
            if same_dir && path == old_dir {
                edit_steps(&mut value, |step| rename_file(step, old_file, new_file));
            }
            edit_steps(&mut value, |step| follow_step(step, from, to));
            let path = moved(path, from, to).unwrap_or_else(|| path.to_owned());
            vec![(path, value)]
        });
    }
    if !changed {
        return Ok(());
    }
    collection::update_arnod(root, |current| *current = arnod)
}

/// Une suppression faite dans Arnod oublie le schéma et les Avant / Après de `path` et de ses
/// descendants (R4).
pub fn forget(root: &Path, path: &str) -> Result<(), CommandError> {
    if !collection::exists(root) {
        return Ok(());
    }
    let ext = Extensions::read(root);
    let Some(arnod) = ext.arnod.as_ref().and_then(Value::as_object) else {
        return Ok(());
    };
    let mut arnod = arnod.clone();
    let mut changed = false;
    for key in [TESTS, FOLDERS] {
        changed |= rewrite(&mut arnod, key, |entry, value| {
            if moved(entry, path, path).is_some() {
                Vec::new()
            } else {
                vec![(entry.to_owned(), value)]
            }
        });
        if arnod
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(Map::is_empty)
        {
            arnod.remove(key);
        }
    }
    if !changed {
        return Ok(());
    }
    collection::update_arnod(root, |current| *current = arnod)
}

/// Une copie faite dans Arnod (`from` → `to`) copie le schéma et les Avant / Après de `from` et
/// de ses descendants (R4).
pub fn copy(root: &Path, from: &str, to: &str) -> Result<(), CommandError> {
    if from == to || !collection::exists(root) {
        return Ok(());
    }
    let ext = Extensions::read(root);
    let Some(arnod) = ext.arnod.as_ref().and_then(Value::as_object) else {
        return Ok(());
    };
    let mut arnod = arnod.clone();
    let mut changed = false;
    for key in [TESTS, FOLDERS] {
        changed |= rewrite(&mut arnod, key, |path, value| {
            let mut out = vec![(path.to_owned(), value.clone())];
            if let Some(new_path) = moved(path, from, to) {
                out.push((new_path, value));
            }
            out
        });
    }
    if !changed {
        return Ok(());
    }
    collection::update_arnod(root, |current| *current = arnod)
}
