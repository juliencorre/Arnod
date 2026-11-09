//! Arborescence d'un projet : dossiers réels et fichiers de requête (R1 à R5).
//!
//! Module sans Tauri ni état : il travaille sur le dossier du projet. Les
//! chemins échangés sont relatifs à la racine du projet, séparés par `/`.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::catalog::names::{self, sort_key};
use crate::contract::CONTRACTS_DIR;
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{FOLDER_FILE, ROOT_FILE, yaml};
use crate::request::file;
use crate::request::model::{Method, Request};

/// Dossier réservé aux environnements, à la racine du projet.
pub const ENVIRONMENTS_DIR: &str = "environments";

/// Nœud de l'arborescence affichée dans la barre latérale (A1).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TreeNode {
    Folder {
        name: String,
        path: String,
        children: Vec<TreeNode>,
        /// Nombre de requêtes dans le dossier, sous-dossiers compris.
        request_count: usize,
    },
    Request {
        name: String,
        path: String,
        method: Option<Method>,
        assertion_count: usize,
        /// Fichier illisible : listé sans bloquer le reste (A10).
        invalid: bool,
    },
}

impl TreeNode {
    fn count(&self) -> usize {
        match self {
            Self::Folder { request_count, .. } => *request_count,
            Self::Request { .. } => 1,
        }
    }
}

fn invalid_request(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn is_request_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

/// Lit l'arborescence du projet (R1) : dossiers d'abord, puis requêtes, chacun
/// par nom sans tenir compte de la casse ; éléments cachés, racine de la
/// collection et réglages de dossier (`folder.yml`) ignorés (R7 du format).
pub fn read(root: &Path) -> Result<Vec<TreeNode>, CommandError> {
    read_dir(root, root, true)
}

fn read_dir(root: &Path, dir: &Path, top: bool) -> Result<Vec<TreeNode>, CommandError> {
    let entries = fs::read_dir(dir)
        .map_err(|error| CommandError::io("Lecture du dossier impossible", dir, &error))?;
    let mut folders = Vec::new();
    let mut requests = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || (top && name == ROOT_FILE) || name == FOLDER_FILE {
            continue;
        }
        let path = entry.path();
        let relative = relative_path(root, &path);
        if path.is_dir() {
            if top && (name == ENVIRONMENTS_DIR || name == CONTRACTS_DIR) {
                continue;
            }
            let children = read_dir(root, &path, false)?;
            let request_count = children.iter().map(TreeNode::count).sum();
            folders.push(TreeNode::Folder {
                name,
                path: relative,
                children,
                request_count,
            });
        } else if is_request_file(&path) {
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            requests.push(match file::read(&path) {
                Ok(request) => TreeNode::Request {
                    name: request.name,
                    path: relative,
                    method: Some(request.method),
                    assertion_count: request.assertions.len(),
                    invalid: false,
                },
                Err(_) => TreeNode::Request {
                    name: stem,
                    path: relative,
                    method: None,
                    assertion_count: 0,
                    invalid: true,
                },
            });
        }
    }
    let key = |node: &TreeNode| match node {
        TreeNode::Folder { name, .. } | TreeNode::Request { name, .. } => sort_key(name),
    };
    folders.sort_by_key(key);
    requests.sort_by_key(key);
    folders.extend(requests);
    Ok(folders)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Chemin absolu d'un chemin relatif, refusé s'il sort du projet.
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf, CommandError> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            _ => return Err(invalid_request("Chemin hors du projet.")),
        }
    }
    Ok(path)
}

/// Nom de fichier dérivé d'un nom d'affichage (R3) : minuscules, sans accents,
/// tirets.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in name.trim().to_lowercase().chars() {
        let base = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'œ' => {
                out.push_str("oe");
                dash = false;
                continue;
            }
            other => other,
        };
        if base.is_ascii_alphanumeric() {
            out.push(base);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "requete".to_owned()
    } else {
        out
    }
}

/// Nom de dossier ou de requête normalisé (R2).
pub fn item_name(raw: &str) -> Result<String, CommandError> {
    names::project_name(raw).map_err(|error| {
        CommandError::new(error.kind, error.message.replace("nom de projet", "nom"))
    })
}

fn exists_ignoring_case(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| names::same_name(&entry.file_name().to_string_lossy(), name))
    })
}

fn name_taken(name: &str) -> CommandError {
    CommandError::new(
        CommandErrorKind::NameTaken,
        format!("« {name} » existe déjà dans ce dossier."),
    )
}

/// Une requête de `dir` (autre que `except`) porte déjà ce nom.
pub(crate) fn request_name_taken(dir: &Path, name: &str, except: Option<&Path>) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            if Some(path.as_path()) == except || !is_request_file(&path) {
                return false;
            }
            file::read_name(&path).is_some_and(|other| names::same_name(&other, name))
        })
    })
}

/// Nom de fichier libre dans `dir` pour `base` : `base.yml`, `base-2.yml`…
pub(crate) fn free_file_name(dir: &Path, base: &str) -> String {
    let mut candidate = format!("{base}.yml");
    let mut n = 2;
    while exists_ignoring_case(dir, &candidate) {
        candidate = format!("{base}-{n}.yml");
        n += 1;
    }
    candidate
}

/// Crée un dossier réel (A3). Renvoie son chemin relatif.
pub fn create_folder(root: &Path, parent: &str, name: &str) -> Result<String, CommandError> {
    let name = item_name(name)?;
    let parent_dir = resolve(root, parent)?;
    if !parent_dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Le dossier parent n'existe plus.",
        ));
    }
    if exists_ignoring_case(&parent_dir, &name)
        || (parent.is_empty()
            && (name.eq_ignore_ascii_case(ENVIRONMENTS_DIR)
                || name.eq_ignore_ascii_case(CONTRACTS_DIR)))
    {
        return Err(name_taken(&name));
    }
    let path = parent_dir.join(&name);
    fs::create_dir(&path)
        .map_err(|error| CommandError::io("Création du dossier impossible", &path, &error))?;
    Ok(relative_path(root, &path))
}

/// Crée une requête neuve dans un dossier (B1). Renvoie son chemin relatif.
pub fn create_request(
    root: &Path,
    folder: &str,
    name: Option<&str>,
) -> Result<String, CommandError> {
    let dir = resolve(root, folder)?;
    if !dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Le dossier n'existe plus.",
        ));
    }
    let name = match name {
        Some(raw) => item_name(raw)?,
        None => unique_default_name(&dir),
    };
    if request_name_taken(&dir, &name, None) {
        return Err(name_taken(&name));
    }
    let path = dir.join(free_file_name(&dir, &slug(&name)));
    file::write(&path, &Request::new(name))?;
    Ok(relative_path(root, &path))
}

fn unique_default_name(dir: &Path) -> String {
    let base = "Nouvelle requête";
    let mut name = base.to_owned();
    let mut n = 2;
    while request_name_taken(dir, &name, None) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

/// Renomme un dossier (sur disque) ou une requête (nom d'affichage, et nom de
/// fichier s'il était dérivé de l'ancien nom, R3). Renvoie le nouveau chemin.
pub fn rename(root: &Path, path: &str, name: &str) -> Result<String, CommandError> {
    let name = item_name(name)?;
    let source = resolve(root, path)?;
    let parent = source.parent().unwrap_or(root).to_path_buf();
    if source.is_dir() {
        let current = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !names::same_name(&current, &name) && exists_ignoring_case(&parent, &name) {
            return Err(name_taken(&name));
        }
        let target = parent.join(&name);
        if current != name {
            // Passage par un nom temporaire : renommer « a » en « A » sous Windows.
            let tmp = parent.join(format!(".arnod-rename-{}", uuid::Uuid::new_v4()));
            fs::rename(&source, &tmp)
                .map_err(|e| CommandError::io("Renommage impossible", &source, &e))?;
            fs::rename(&tmp, &target)
                .map_err(|e| CommandError::io("Renommage impossible", &target, &e))?;
        }
        rename_folder_settings(&target, &name);
        return Ok(relative_path(root, &target));
    }
    let request = file::read(&source)?;
    if request_name_taken(&parent, &name, Some(&source)) {
        return Err(name_taken(&name));
    }
    let old_stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let derived = {
        let base = slug(&request.name);
        old_stem == base
            || old_stem
                .strip_prefix(&format!("{base}-"))
                .is_some_and(|rest| rest.parse::<u32>().is_ok())
    };
    file::rename(&source, &name)?;
    if derived {
        let new_base = slug(&name);
        if new_base != old_stem {
            let target = parent.join(free_file_name(&parent, &new_base));
            fs::rename(&source, &target)
                .map_err(|e| CommandError::io("Renommage impossible", &target, &e))?;
            return Ok(relative_path(root, &target));
        }
    }
    Ok(relative_path(root, &source))
}

/// Déplace un dossier ou une requête dans un dossier (A6, R4).
pub fn move_to(root: &Path, path: &str, target_folder: &str) -> Result<String, CommandError> {
    let source = resolve(root, path)?;
    let target_dir = resolve(root, target_folder)?;
    if !source.exists() || !target_dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "L'élément ou le dossier cible n'existe plus.",
        ));
    }
    if source.is_dir() && (target_dir == source || target_dir.starts_with(&source)) {
        return Err(invalid_request(
            "Un dossier ne peut pas être déplacé dans lui-même.",
        ));
    }
    let file_name = source
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let target = target_dir.join(&file_name);
    if target == source {
        return Ok(relative_path(root, &source));
    }
    if exists_ignoring_case(&target_dir, &file_name.to_string_lossy()) {
        return Err(name_taken(&file_name.to_string_lossy()));
    }
    if !source.is_dir() {
        let request = file::read(&source)?;
        if request_name_taken(&target_dir, &request.name, None) {
            return Err(name_taken(&request.name));
        }
    }
    fs::rename(&source, &target)
        .map_err(|e| CommandError::io("Déplacement impossible", &target, &e))?;
    Ok(relative_path(root, &target))
}

/// Duplique une requête dans son dossier (A9) : « <nom> (copie) », nouvel `id`.
pub fn duplicate(root: &Path, path: &str) -> Result<String, CommandError> {
    let source = resolve(root, path)?;
    let dir = source.parent().unwrap_or(root).to_path_buf();
    let request = file::read(&source)?;
    let base = format!("{} (copie)", request.name);
    let mut name = base.clone();
    let mut n = 2;
    while request_name_taken(&dir, &name, None) {
        name = format!("{base} {n}");
        n += 1;
    }
    let target = dir.join(free_file_name(&dir, &slug(&name)));
    file::copy_as(&source, &target, &name)?;
    Ok(relative_path(root, &target))
}

/// Le nom d'un dossier dans son `folder.yml`, s'il en a un, suit le renommage
/// (Bruno l'affiche à la place du nom du dossier).
fn rename_folder_settings(folder: &Path, name: &str) {
    let path = folder.join(FOLDER_FILE);
    let Ok(mut value) = yaml::read(&path) else {
        return;
    };
    if let Some(info) = value
        .get_mut("info")
        .and_then(serde_json::Value::as_object_mut)
    {
        info.insert("name".to_owned(), serde_json::Value::from(name));
        // Un fichier non conforme n'est jamais réécrit (R1, R2).
        if schema::check(Part::Folder, &value).is_ok() {
            let _ = yaml::write(&path, &value);
        }
    }
}

/// Supprime un dossier ou une requête : corbeille du système, ou suppression
/// définitive si `permanent` (R5).
pub fn delete(root: &Path, path: &str, permanent: bool) -> Result<(), CommandError> {
    let target = resolve(root, path)?;
    if target == root {
        return Err(invalid_request(
            "Le projet lui-même ne se supprime pas ici.",
        ));
    }
    if !target.exists() {
        return Ok(());
    }
    if permanent {
        let result = if target.is_dir() {
            fs::remove_dir_all(&target)
        } else {
            fs::remove_file(&target)
        };
        return result.map_err(|e| CommandError::io("Suppression impossible", &target, &e));
    }
    trash::delete(&target).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("La corbeille du système n'est pas disponible ({error}). Vous pouvez supprimer définitivement."),
        )
    })
}

/// Requêtes d'un dossier (sous-dossiers compris), dans l'ordre de l'arborescence (R13).
pub fn requests_in(root: &Path, folder: &str) -> Result<Vec<String>, CommandError> {
    let tree = read(root)?;
    fn collect(nodes: &[TreeNode], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, .. } => collect(children, out),
                TreeNode::Request {
                    path,
                    invalid: false,
                    ..
                } => out.push(path.clone()),
                TreeNode::Request { .. } => {}
            }
        }
    }
    fn find<'a>(nodes: &'a [TreeNode], folder: &str) -> Option<&'a [TreeNode]> {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                if path == folder {
                    return Some(children);
                }
                if let Some(found) = find(children, folder) {
                    return Some(found);
                }
            }
        }
        None
    }
    let mut out = Vec::new();
    if folder.is_empty() {
        collect(&tree, &mut out);
    } else if let Some(children) = find(&tree, folder) {
        collect(children, &mut out);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
