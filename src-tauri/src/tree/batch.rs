//! Actions sur une sélection de l'arborescence (écran 4.1-a quater) : déplacer, dupliquer ou
//! mettre à la corbeille plusieurs requêtes et dossiers d'un coup, avec « Annuler ». Un élément
//! dont un dossier parent est aussi choisi est pris avec lui, pas deux fois.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::undo::Undo;
use super::{
    duplicate, exists_ignoring_case, invalid_request, move_to, relative_path,
    rename_folder_settings, resolve,
};
use crate::contract::file as contract_file;
use crate::error::CommandError;

/// Action sur la sélection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BatchAction {
    /// Vers le dossier `target` (`""` : racine du projet).
    Move {
        target: String,
    },
    Duplicate,
    /// Corbeille du système (ou définitivement) ; « Annuler » remet les fichiers.
    Delete {
        #[serde(default)]
        permanent: bool,
    },
}

/// Ce que l'action a fait : chemins d'avant et d'après (absent après une suppression).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDone {
    pub items: Vec<(String, Option<String>)>,
}

/// Chemins distincts, sans ceux qu'un dossier choisi contient déjà, dans l'ordre reçu.
pub fn normalize(paths: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for path in paths {
        let path = path.trim_matches('/');
        if path.is_empty() || kept.iter().any(|k| k == path) {
            continue;
        }
        let inside = |parent: &str, child: &str| child.starts_with(&format!("{parent}/"));
        if paths
            .iter()
            .any(|other| inside(other.trim_matches('/'), path))
        {
            continue;
        }
        kept.push(path.to_owned());
    }
    kept
}

/// Textes de tous les fichiers sous `path` (le fichier lui-même s'il n'est pas un dossier).
fn texts_under(path: &Path, out: &mut Vec<(PathBuf, String)>) -> Result<(), CommandError> {
    if path.is_dir() {
        let entries =
            fs::read_dir(path).map_err(|e| CommandError::io("Lecture impossible", path, &e))?;
        for entry in entries.flatten() {
            texts_under(&entry.path(), out)?;
        }
    } else if let Ok(text) = fs::read_to_string(path) {
        out.push((path.to_path_buf(), text));
    }
    Ok(())
}

/// Copie le dossier `source` en `target`, fichier par fichier.
fn copy_dir(source: &Path, target: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(target)
        .map_err(|e| CommandError::io("Création du dossier impossible", target, &e))?;
    let entries =
        fs::read_dir(source).map_err(|e| CommandError::io("Lecture impossible", source, &e))?;
    for entry in entries.flatten() {
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            let text = fs::read_to_string(&from)
                .map_err(|e| CommandError::io("Lecture impossible", &from, &e))?;
            contract_file::write_text(&to, &text)?;
        }
    }
    Ok(())
}

/// Duplique un dossier à côté de lui : « <nom> (copie) ».
fn duplicate_folder(root: &Path, path: &str, undo: &mut Undo) -> Result<String, CommandError> {
    let source = resolve(root, path)?;
    let parent = source.parent().unwrap_or(root).to_path_buf();
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base = format!("{name} (copie)");
    let mut candidate = base.clone();
    let mut n = 2;
    while exists_ignoring_case(&parent, &candidate) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    let target = parent.join(&candidate);
    undo.created_dir(target.clone());
    copy_dir(&source, &target)?;
    rename_folder_settings(&target, &candidate);
    Ok(relative_path(root, &target))
}

fn run(
    root: &Path,
    paths: &[String],
    action: &BatchAction,
    undo: &mut Undo,
    items: &mut Vec<(String, Option<String>)>,
) -> Result<(), CommandError> {
    for path in paths {
        let source = resolve(root, path)?;
        if source == root {
            return Err(invalid_request("Le projet lui-même ne se modifie pas ici."));
        }
        match action {
            BatchAction::Move { target } => {
                let moved = move_to(root, path, target)?;
                if moved != *path {
                    undo.moved(source, resolve(root, &moved)?);
                }
                items.push((path.clone(), Some(moved)));
            }
            BatchAction::Duplicate => {
                let copy = if source.is_dir() {
                    duplicate_folder(root, path, undo)?
                } else {
                    let copy = duplicate(root, path)?;
                    undo.created(resolve(root, &copy)?);
                    copy
                };
                items.push((path.clone(), Some(copy)));
            }
            BatchAction::Delete { permanent } => {
                let mut texts = Vec::new();
                texts_under(&source, &mut texts)?;
                super::delete(root, path, *permanent)?;
                for (file, text) in texts {
                    undo.restored(file, text);
                }
                items.push((path.clone(), None));
            }
        }
    }
    Ok(())
}

/// Applique `action` à la sélection `paths` du projet `root`. Un échec remet ce qui a déjà été
/// fait : la sélection change d'un coup ou pas du tout.
pub fn apply(
    root: &Path,
    paths: &[String],
    action: &BatchAction,
) -> Result<(BatchDone, Undo), CommandError> {
    let paths = normalize(paths);
    if paths.is_empty() {
        return Err(invalid_request("Aucun élément choisi."));
    }
    let mut undo = Undo::default();
    let mut items = Vec::new();
    if let Err(error) = run(root, &paths, action, &mut undo, &mut items) {
        let _ = undo.apply();
        return Err(error);
    }
    Ok((BatchDone { items }, undo))
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
