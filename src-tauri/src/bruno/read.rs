//! Lecture d'une collection Bruno sur disque : `bruno.json`, `collection.bru`,
//! `environments/*.bru`, dossiers (`folder.bru`) et requêtes `*.bru`.
//!
//! Lecture seule : les fichiers d'origine ne sont jamais modifiés. Un fichier
//! illisible est gardé avec sa raison, sans bloquer le reste.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::bruno::parse::{self, BruFile};
use crate::catalog::names::sort_key;
use crate::error::{CommandError, CommandErrorKind};

/// Fichier de la collection.
pub const MANIFEST: &str = "bruno.json";
/// Réglages de la collection.
pub const COLLECTION_FILE: &str = "collection.bru";
/// Réglages d'un dossier.
pub const FOLDER_FILE: &str = "folder.bru";
/// Dossier des environnements.
pub const ENVIRONMENTS_DIR: &str = "environments";
/// Variables du poste, lues par `{{process.env.NOM}}`.
pub const DOTENV: &str = ".env";

/// Fichier `.bru` lu, avec son chemin relatif à la collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub path: String,
    pub file: BruFile,
}

/// Dossier de la collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folder {
    /// Nom du dossier sur disque (vide pour la racine).
    pub dir: String,
    /// Chemin relatif (vide pour la racine).
    pub path: String,
    /// `folder.bru`.
    pub settings: Option<Source>,
    pub folders: Vec<Folder>,
    pub requests: Vec<Source>,
}

/// Collection lue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collection {
    pub name: String,
    /// `collection.bru`.
    pub settings: Option<Source>,
    /// Environnements, par nom de fichier.
    pub environments: Vec<(String, Source)>,
    pub tree: Folder,
    /// Variables du fichier `.env` (jamais écrites dans le projet).
    pub dotenv: BTreeMap<String, String>,
    /// Nombre de fichiers `.bru` lus.
    pub bru_files: usize,
    /// Fichiers illisibles : chemin relatif et raison.
    pub unreadable: Vec<(String, String)>,
}

/// Le dossier est une collection Bruno en fichiers `.bru`.
pub fn is_collection(folder: &Path) -> bool {
    folder.join(MANIFEST).is_file()
        && fs::read_to_string(folder.join(MANIFEST))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .is_some_and(|value| {
                value
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("collection")
                    == "collection"
            })
}

fn not_a_collection(folder: &Path, reason: &str) -> CommandError {
    CommandError::new(
        CommandErrorKind::NotAProject,
        format!(
            "Le dossier {} n'est pas une collection Bruno lisible : {reason}.",
            folder.display()
        ),
    )
}

/// Lit une collection Bruno.
pub fn read(folder: &Path) -> Result<Collection, CommandError> {
    let manifest = fs::read_to_string(folder.join(MANIFEST))
        .map_err(|_| not_a_collection(folder, &format!("{MANIFEST} absent")))?;
    let manifest: Value = serde_json::from_str(&manifest)
        .map_err(|error| not_a_collection(folder, &format!("{MANIFEST} illisible ({error})")))?;
    let name = manifest
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .or_else(|| folder.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Collection Bruno".to_owned());
    let ignore: Vec<String> = manifest
        .get("ignore")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();

    let mut collection = Collection {
        name,
        ..Collection::default()
    };
    collection.settings = read_bru(folder, COLLECTION_FILE, &mut collection);
    let env_dir = folder.join(ENVIRONMENTS_DIR);
    let mut env_files: Vec<String> = fs::read_dir(&env_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".bru"))
        .collect();
    env_files.sort_by_key(|name| sort_key(name));
    for file in env_files {
        let relative = format!("{ENVIRONMENTS_DIR}/{file}");
        if let Some(source) = read_bru(folder, &relative, &mut collection) {
            let name = file.trim_end_matches(".bru").to_owned();
            collection.environments.push((name, source));
        }
    }
    collection.dotenv = fs::read_to_string(folder.join(DOTENV))
        .map(|text| dotenv(&text))
        .unwrap_or_default();
    let mut tree = Folder::default();
    read_folder(folder, &mut tree, &ignore, &mut collection, 0)?;
    collection.tree = tree;
    Ok(collection)
}

fn read_bru(root: &Path, relative: &str, collection: &mut Collection) -> Option<Source> {
    let path = root.join(relative);
    if !path.is_file() {
        return None;
    }
    collection.bru_files += 1;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            collection
                .unreadable
                .push((relative.to_owned(), format!("lecture impossible ({error})")));
            return None;
        }
    };
    match parse::parse(&text) {
        Ok(file) => Some(Source {
            path: relative.to_owned(),
            file,
        }),
        Err(error) => {
            collection
                .unreadable
                .push((relative.to_owned(), error.to_string()));
            None
        }
    }
}

/// Profondeur maximale lue (protection contre les liens en boucle).
const MAX_DEPTH: usize = 32;

fn read_folder(
    root: &Path,
    folder: &mut Folder,
    ignore: &[String],
    collection: &mut Collection,
    depth: usize,
) -> Result<(), CommandError> {
    let dir = if folder.path.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&folder.path)
    };
    let entries = fs::read_dir(&dir)
        .map_err(|error| CommandError::io("Lecture du dossier impossible", &dir, &error))?;
    let mut names: Vec<(String, bool)> = entries
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.path().is_dir();
            (name, is_dir)
        })
        .collect();
    names.sort_by_key(|(name, _)| sort_key(name));
    for (name, is_dir) in names {
        let relative = if folder.path.is_empty() {
            name.clone()
        } else {
            format!("{}/{name}", folder.path)
        };
        if name.starts_with('.') || ignore.iter().any(|i| i == &name || i == &relative) {
            continue;
        }
        if is_dir {
            if (folder.path.is_empty() && name == ENVIRONMENTS_DIR)
                || name == "node_modules"
                || depth >= MAX_DEPTH
            {
                continue;
            }
            let mut child = Folder {
                dir: name,
                path: relative,
                ..Folder::default()
            };
            read_folder(root, &mut child, ignore, collection, depth + 1)?;
            folder.folders.push(child);
        } else if name == FOLDER_FILE {
            folder.settings = read_bru(root, &relative, collection);
        } else if name.ends_with(".bru")
            && !(folder.path.is_empty() && name == COLLECTION_FILE)
            && let Some(source) = read_bru(root, &relative, collection)
        {
            folder.requests.push(source);
        }
    }
    Ok(())
}

/// Variables d'un fichier `.env` : `NOM=valeur`, commentaires `#`, guillemets retirés.
pub fn dotenv(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (name, value) = line.split_once('=')?;
            let value = value.trim();
            let value = ['"', '\'']
                .iter()
                .find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
                .unwrap_or(value);
            Some((name.trim().to_owned(), value.to_owned()))
        })
        .filter(|(name, _)| !name.is_empty())
        .collect()
}
