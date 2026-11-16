//! Fichiers modifiés hors d'Arnod (lot I, `docs/plan-mvp-i-fichiers-externes.md`).
//!
//! Le **registre** garde l'état connu des fichiers des projets surveillés (empreinte du contenu
//! de chaque fichier suivi, présence de chaque dossier). Arnod le met à jour à chacune de ses
//! écritures ([`settle`], [`record_write`]) : un événement du système sur un fichier dont l'état
//! n'a pas changé est l'écho d'une écriture d'Arnod et n'a aucun effet (R2). Le surveillant
//! ([`service`]) regroupe les événements (300 ms), puis [`apply`] compare l'état du disque à
//! l'état connu et en tire les changements ([`FilesChange`], R3 à R6).
//!
//! Le registre garde aussi l'empreinte de la version de chaque requête qu'Arnod a lue ou écrite
//! en dernier ([`acknowledge`]) : un enregistrement sur un fichier modifié depuis est refusé
//! ([`check_unchanged`], R8), le brouillon passe alors en conflit (R9).

pub mod app;
pub mod conflict;
pub mod service;

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde::Serialize;
use uuid::Uuid;

use crate::contract::CONTRACTS_DIR;
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::{FOLDER_FILE, ROOT_FILE};
use crate::tree::ENVIRONMENTS_DIR;

/// Profondeur au-delà de laquelle un dossier n'est plus parcouru (liens en boucle).
const MAX_DEPTH: usize = 32;

/// État connu d'un élément du projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Fichier suivi, avec l'empreinte de son contenu.
    File(u64),
    Dir,
}

/// Requête renommée ou déplacée hors d'Arnod (R5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    pub from: String,
    pub to: String,
}

/// Changements d'un projet faits hors d'Arnod, diffusés par `arnod://fichiers` (R3 à R6).
/// Les chemins sont ceux des requêtes, relatifs au projet et séparés par `/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesChange {
    pub project_id: Uuid,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub renamed: Vec<Renamed>,
    /// L'arborescence a changé (requête, dossier, `folder.yml`, racine).
    pub tree: bool,
    /// Les environnements ont changé (`environments/`, `opencollection.yml`).
    pub environments: bool,
    /// Les contrats ont changé (`contracts/`).
    pub contracts: bool,
}

impl FilesChange {
    /// Aucun changement à diffuser.
    pub fn is_empty(&self) -> bool {
        !self.tree && !self.environments && !self.contracts
    }
}

/// Registre des états connus, partagé par les écritures et le surveillant.
#[derive(Default)]
struct Registry {
    /// Racines des projets surveillés.
    roots: HashMap<Uuid, PathBuf>,
    /// État connu de chaque élément des projets surveillés, par chemin absolu.
    known: BTreeMap<PathBuf, Entry>,
    /// Empreinte de la dernière version lue ou écrite par Arnod, par fichier (R8).
    acknowledged: HashMap<PathBuf, u64>,
}

fn registry() -> MutexGuard<'static, Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Empreinte d'un contenu.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn file_fingerprint(path: &Path) -> Option<u64> {
    fs::read(path).ok().map(|bytes| fingerprint(&bytes))
}

/// Élément ignoré (R1) : caché (`.git/`, fichiers d'échange des éditeurs), fichier temporaire
/// d'une écriture atomique (`.tmp`) ou de sauvegarde (`~`). `relative` est relatif au projet.
pub fn ignored(relative: &Path) -> bool {
    let hidden = relative.components().any(|component| match component {
        Component::Normal(name) => name.to_string_lossy().starts_with('.'),
        _ => false,
    });
    let name = relative
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    hidden || name.ends_with(".tmp") || name.ends_with('~')
}

/// Fichier suivi : YAML (requêtes, dossiers, environnements, contrats) ou JSON (contrats).
fn tracked_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["yml", "yaml", "json"]
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

/// État actuel de `path` et de ce qu'il contient, éléments ignorés exclus.
fn scan(root: &Path, path: &Path) -> Vec<(PathBuf, Entry)> {
    let mut found = Vec::new();
    scan_into(root, path, 0, &mut found);
    found
}

fn scan_into(root: &Path, path: &Path, depth: usize, found: &mut Vec<(PathBuf, Entry)>) {
    if path != root && path.strip_prefix(root).is_ok_and(ignored) {
        return;
    }
    if path.is_dir() {
        if path != root {
            found.push((path.to_path_buf(), Entry::Dir));
        }
        if depth >= MAX_DEPTH {
            return;
        }
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            scan_into(root, &entry.path(), depth + 1, found);
        }
    } else if tracked_file(path)
        && let Some(hash) = file_fingerprint(path)
    {
        found.push((path.to_path_buf(), Entry::File(hash)));
    }
}

impl Registry {
    fn root_of(&self, path: &Path) -> Option<(Uuid, PathBuf)> {
        self.roots
            .iter()
            .filter(|(_, root)| path.starts_with(root))
            .max_by_key(|(_, root)| root.components().count())
            .map(|(id, root)| (*id, root.clone()))
    }

    /// Éléments connus sous `path`, `path` compris.
    fn known_under(&self, path: &Path) -> Vec<(PathBuf, Entry)> {
        self.known
            .range(path.to_path_buf()..)
            .take_while(|(key, _)| key.starts_with(path))
            .map(|(key, entry)| (key.clone(), *entry))
            .collect()
    }

    /// Remplace l'état connu sous `path` par `current`.
    fn replace_under(&mut self, path: &Path, current: &[(PathBuf, Entry)]) {
        for (key, _) in self.known_under(path) {
            self.known.remove(&key);
        }
        self.known.extend(current.iter().cloned());
    }

    /// Met l'état connu de `path` (et des dossiers qui le contiennent) en accord avec le disque.
    fn settle(&mut self, path: &Path) {
        let Some((_, root)) = self.root_of(path) else {
            return;
        };
        let current = scan(&root, path);
        self.replace_under(path, &current);
        for ancestor in path.ancestors().skip(1) {
            if ancestor == root || !ancestor.starts_with(&root) {
                break;
            }
            if ancestor.is_dir() && !ancestor.strip_prefix(&root).is_ok_and(ignored) {
                self.known.insert(ancestor.to_path_buf(), Entry::Dir);
            }
        }
    }
}

/// Commence à suivre un projet : son état actuel devient l'état connu.
pub fn add_root(project: Uuid, root: &Path) {
    let mut registry = registry();
    if let Some(previous) = registry.roots.insert(project, root.to_path_buf())
        && previous != root
    {
        registry.replace_under(&previous, &[]);
    }
    let current = scan(root, root);
    registry.replace_under(root, &current);
}

/// Cesse de suivre un projet.
pub fn remove_root(project: Uuid) {
    let mut registry = registry();
    if let Some(root) = registry.roots.remove(&project) {
        registry.replace_under(&root, &[]);
    }
}

/// Après une opération d'Arnod sur `path` (création, renommage, suppression) : son nouvel état
/// est connu, et l'événement du système qui suit est ignoré (R2).
pub fn settle(path: &Path) {
    registry().settle(path);
}

/// Après une écriture d'Arnod : le contenu écrit est la version connue de ce fichier (R2, R8).
pub fn record_write(path: &Path, bytes: &[u8]) {
    let mut registry = registry();
    registry
        .acknowledged
        .insert(path.to_path_buf(), fingerprint(bytes));
    registry.settle(path);
}

/// Après une lecture de la requête par l'interface : cette version est celle qu'elle modifie (R8).
pub fn acknowledge(path: &Path) {
    let mut registry = registry();
    match file_fingerprint(path) {
        Some(hash) => registry.acknowledged.insert(path.to_path_buf(), hash),
        None => registry.acknowledged.remove(path),
    };
}

/// Refuse d'écrire sur un fichier modifié depuis la dernière version qu'Arnod en a lue ou écrite
/// (R8). Un fichier inconnu ou absent peut être écrit.
pub fn check_unchanged(path: &Path) -> Result<(), CommandError> {
    let registry = registry();
    let Some(expected) = registry.acknowledged.get(path) else {
        return Ok(());
    };
    match file_fingerprint(path) {
        Some(current) if current != *expected => Err(CommandError::new(
            CommandErrorKind::ChangedOnDisk,
            format!(
                "Le fichier {} a été modifié hors d'Arnod : choisissez ce que vous gardez avant d'enregistrer.",
                path.display()
            ),
        )),
        _ => Ok(()),
    }
}

/// Chemin relatif d'un élément, séparé par `/`.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Différence entre l'état connu et l'état actuel d'un élément.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub path: String,
    pub before: Option<Entry>,
    pub after: Option<Entry>,
}

/// Compare le disque à l'état connu pour les chemins touchés, met l'état connu à jour et renvoie
/// les différences. `None` si le projet n'est pas suivi.
pub fn refresh(project: Uuid, touched: &[PathBuf]) -> Option<Vec<Delta>> {
    let mut registry = registry();
    let root = registry.roots.get(&project)?.clone();
    let mut paths: Vec<PathBuf> = touched
        .iter()
        .filter(|path| path.starts_with(&root) && path.as_path() != root)
        .filter(|path| !path.strip_prefix(&root).is_ok_and(ignored))
        .cloned()
        .collect();
    paths.sort();
    paths.dedup();
    // Un chemin dont un parent est déjà relu est couvert par lui.
    let mut kept: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !kept.iter().any(|parent| path.starts_with(parent)) {
            kept.push(path);
        }
    }
    let mut deltas = Vec::new();
    for path in kept {
        let before: BTreeMap<PathBuf, Entry> = registry.known_under(&path).into_iter().collect();
        let current = scan(&root, &path);
        let after: BTreeMap<PathBuf, Entry> = current.iter().cloned().collect();
        let mut keys: Vec<&PathBuf> = before.keys().chain(after.keys()).collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            let (old, new) = (before.get(key).copied(), after.get(key).copied());
            if old != new {
                deltas.push(Delta {
                    path: relative(&root, key),
                    before: old,
                    after: new,
                });
            }
        }
        registry.replace_under(&path, &current);
    }
    Some(deltas)
}

/// Partie du projet d'un chemin relatif.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Collection,
    Environments,
    Contracts,
    FolderSettings,
    Request,
    Other,
}

fn part_of(path: &str, entry: Option<Entry>) -> Part {
    let first = path.split('/').next().unwrap_or_default();
    let name = path.rsplit('/').next().unwrap_or_default();
    if path == ROOT_FILE {
        Part::Collection
    } else if first == ENVIRONMENTS_DIR {
        Part::Environments
    } else if first == CONTRACTS_DIR {
        Part::Contracts
    } else if name == FOLDER_FILE {
        Part::FolderSettings
    } else if matches!(entry, Some(Entry::File(_)))
        && (name.ends_with(".yml") || name.ends_with(".yaml"))
    {
        Part::Request
    } else if matches!(entry, Some(Entry::Dir)) {
        Part::FolderSettings
    } else {
        Part::Other
    }
}

/// Changements à diffuser d'après les différences (R3 à R5) : portées touchées, requêtes
/// ajoutées, modifiées, supprimées ; un fichier supprimé et un fichier ajouté de même contenu
/// sont un renommage.
pub fn classify(project_id: Uuid, deltas: &[Delta]) -> FilesChange {
    let mut change = FilesChange {
        project_id,
        ..FilesChange::default()
    };
    let mut added: Vec<(String, u64)> = Vec::new();
    let mut removed: Vec<(String, u64)> = Vec::new();
    for delta in deltas {
        let part = part_of(&delta.path, delta.after.or(delta.before));
        match part {
            Part::Collection => {
                change.tree = true;
                change.environments = true;
            }
            Part::Environments => change.environments = true,
            Part::Contracts => {
                change.contracts = true;
                change.tree = true;
            }
            Part::FolderSettings => change.tree = true,
            Part::Other => {}
            Part::Request => {
                change.tree = true;
                match (delta.before, delta.after) {
                    (Some(Entry::File(_)), Some(Entry::File(_))) => {
                        change.modified.push(delta.path.clone());
                    }
                    (Some(Entry::File(hash)), _) => removed.push((delta.path.clone(), hash)),
                    (_, Some(Entry::File(hash))) => added.push((delta.path.clone(), hash)),
                    _ => {}
                }
            }
        }
    }
    for (path, hash) in removed {
        let twins: Vec<usize> = added
            .iter()
            .enumerate()
            .filter(|(_, (_, other))| *other == hash)
            .map(|(index, _)| index)
            .collect();
        if let [index] = twins.as_slice() {
            let (to, _) = added.remove(*index);
            change.renamed.push(Renamed { from: path, to });
        } else {
            change.removed.push(path);
        }
    }
    change.added = added.into_iter().map(|(path, _)| path).collect();
    change
}

/// Changements d'un projet suivi, après des événements du système sur `touched` (R2 à R5).
pub fn apply(project: Uuid, touched: &[PathBuf]) -> Option<FilesChange> {
    let deltas = refresh(project, touched)?;
    let change = classify(project, &deltas);
    (!change.is_empty()).then_some(change)
}

#[cfg(test)]
mod tests;
