//! De quoi remettre des fichiers d'un projet comme avant (« Annuler ») : rangement des essais
//! (2.2-d), copie ou déplacement d'une requête (lot J), actions sur une sélection (4.1-a quater),
//! suppression d'un contrat ou d'un environnement (ER1). `UndoBook` garde les dernières
//! annulations d'un domaine, retrouvées par leur jeton.

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use crate::contract::file as contract_file;
use crate::error::CommandError;

/// Éléments déplacés, à remettre ; fichiers et dossiers créés, à retirer ; fichiers retirés ou
/// modifiés, avec leur texte d'avant.
#[derive(Debug, Clone, Default)]
pub struct Undo {
    created: Vec<PathBuf>,
    created_dirs: Vec<PathBuf>,
    moved: Vec<(PathBuf, PathBuf)>,
    restored: Vec<(PathBuf, String)>,
}

impl Undo {
    /// Retient un fichier créé.
    pub fn created(&mut self, path: PathBuf) {
        self.created.push(path);
    }

    /// Retient un dossier créé (retiré avec son contenu).
    pub fn created_dir(&mut self, path: PathBuf) {
        self.created_dirs.push(path);
    }

    /// Retient un élément déplacé de `from` vers `to`.
    pub fn moved(&mut self, from: PathBuf, to: PathBuf) {
        self.moved.push((from, to));
    }

    /// Retient le texte d'un fichier avant qu'il soit retiré ou modifié (le premier gagne ; un
    /// fichier créé est seulement retiré).
    pub fn restored(&mut self, path: PathBuf, text: String) {
        if !self.created.contains(&path) && !self.restored.iter().any(|(p, _)| *p == path) {
            self.restored.push((path, text));
        }
    }

    /// Remet les fichiers : les déplacés reviennent, les créés sont retirés, les autres
    /// retrouvent leur texte.
    pub fn apply(&self) -> Result<(), CommandError> {
        for (from, to) in self.moved.iter().rev() {
            if to.exists() && !from.exists() {
                fs::rename(to, from)
                    .map_err(|e| CommandError::io("Annulation impossible", from, &e))?;
                crate::watch::settle(to);
                crate::watch::settle(from);
            }
        }
        for path in &self.created {
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|e| CommandError::io("Annulation impossible", path, &e))?;
                crate::watch::settle(path);
            }
        }
        for path in &self.created_dirs {
            if path.exists() {
                fs::remove_dir_all(path)
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

/// Annulations en attente d'un domaine (sélection, transfert, contrat, environnement) : les
/// `KEPT` dernières, chacune retrouvée une fois par son jeton. Constructible en `static`.
#[derive(Debug)]
pub struct UndoBook<T> {
    entries: VecDeque<(Uuid, T)>,
}

impl<T> UndoBook<T> {
    /// Nombre d'annulations gardées ; la plus ancienne part la première.
    pub const KEPT: usize = 20;

    pub const fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    /// Garde `entry` et rend son jeton.
    pub fn push(&mut self, entry: T) -> Uuid {
        let token = Uuid::new_v4();
        self.entries.push_back((token, entry));
        while self.entries.len() > Self::KEPT {
            self.entries.pop_front();
        }
        token
    }

    /// Reprend l'annulation du jeton `token` : `None` si elle est oubliée ou déjà faite.
    pub fn take(&mut self, token: Uuid) -> Option<T> {
        let index = self.entries.iter().position(|(t, _)| *t == token)?;
        self.entries.remove(index).map(|(_, entry)| entry)
    }
}

impl<T> Default for UndoBook<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ER1 : un jeton ne sert qu'une fois, et seules les `KEPT` dernières annulations restent.
    #[test]
    fn er1_carnet_jeton_unique_et_vingt_dernieres() {
        let mut book = UndoBook::new();
        let first = book.push(0);
        let tokens: Vec<Uuid> = (1..=UndoBook::<i32>::KEPT as i32)
            .map(|n| book.push(n))
            .collect();
        assert_eq!(book.take(first), None, "la plus ancienne est oubliée");
        let last = *tokens.last().unwrap();
        assert_eq!(book.take(last), Some(UndoBook::<i32>::KEPT as i32));
        assert_eq!(book.take(last), None, "un jeton ne sert qu'une fois");
        assert_eq!(book.take(tokens[0]), Some(1));
    }
}
