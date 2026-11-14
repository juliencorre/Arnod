//! De quoi remettre des fichiers d'un projet comme avant (« Annuler ») : rangement des essais
//! (2.2-d), copie ou déplacement d'une requête (lot J).

use std::fs;
use std::path::PathBuf;

use crate::contract::file as contract_file;
use crate::error::CommandError;

/// Fichiers créés, à retirer, et fichiers retirés ou modifiés, avec leur texte d'avant.
#[derive(Debug, Clone, Default)]
pub struct Undo {
    created: Vec<PathBuf>,
    restored: Vec<(PathBuf, String)>,
}

impl Undo {
    /// Retient un fichier créé.
    pub fn created(&mut self, path: PathBuf) {
        self.created.push(path);
    }

    /// Retient le texte d'un fichier avant qu'il soit retiré ou modifié (le premier gagne ; un
    /// fichier créé est seulement retiré).
    pub fn restored(&mut self, path: PathBuf, text: String) {
        if !self.created.contains(&path) && !self.restored.iter().any(|(p, _)| *p == path) {
            self.restored.push((path, text));
        }
    }

    /// Remet les fichiers : les créés sont retirés, les autres retrouvent leur texte.
    pub fn apply(&self) -> Result<(), CommandError> {
        for path in &self.created {
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|e| CommandError::io("Annulation impossible", path, &e))?;
                crate::watch::settle(path);
            }
        }
        for (path, text) in &self.restored {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| CommandError::io("Annulation impossible", parent, &e))?;
            }
            contract_file::write_text(path, text)?;
        }
        Ok(())
    }
}
