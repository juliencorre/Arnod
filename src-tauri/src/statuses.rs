//! Statut de la dernière exécution de chaque requête
//! (`docs/plan-fin-premier-usage.md`, R4, R5).
//!
//! `projects/<id>/statuses.json` dans le dossier de données de l'application,
//! jamais dans le projet : un résultat dépend du poste et de l'environnement.
//! La barre latérale en tire ses ✓ / ✕ et les compteurs des dossiers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::local;

/// Version du format de `statuses.json`.
pub const VERSION: u32 = 1;

/// Dernière exécution d'une requête.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestStatus {
    pub passed: bool,
    /// Fin de l'exécution, en millisecondes depuis l'époque Unix.
    pub at: u64,
}

/// Statuts d'un projet, par chemin de requête (relatif au projet, `/`).
pub type Statuses = BTreeMap<String, RequestStatus>;

/// Statuts d'un projet, tels que gardés en mémoire par l'établi.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusBook {
    pub requests: Statuses,
    /// Fichier d'une version plus récente : jamais réécrit (R5).
    pub read_only: bool,
}

/// Le chemin `path` est `base` ou se trouve sous le dossier `base`.
fn under(path: &str, base: &str) -> bool {
    path == base
        || path
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/'))
}

impl StatusBook {
    /// Fin d'une exécution (R4).
    pub fn record(&mut self, path: &str, passed: bool, at: u64) {
        self.requests
            .insert(path.to_owned(), RequestStatus { passed, at });
    }

    /// Une requête ou un dossier renommé ou déplacé garde ses statuts (R5).
    /// Renvoie vrai si quelque chose a changé.
    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        let moved: Vec<String> = self
            .requests
            .keys()
            .filter(|path| under(path, from))
            .cloned()
            .collect();
        for path in &moved {
            if let Some(status) = self.requests.remove(path) {
                let target = format!("{to}{}", &path[from.len()..]);
                self.requests.insert(target, status);
            }
        }
        !moved.is_empty()
    }

    /// Une requête ou un dossier supprimé perd ses statuts (R5).
    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.requests.len();
        self.requests.retain(|key, _| !under(key, path));
        self.requests.len() != before
    }
}

/// Contenu de `statuses.json`.
#[derive(Debug, Serialize, Deserialize)]
struct StatusFile {
    version: u32,
    #[serde(default)]
    requests: Statuses,
}

fn file(data_dir: &Path, project: Uuid) -> PathBuf {
    local::project_dir(data_dir, project).join("statuses.json")
}

/// Lit les statuts d'un projet. Absent ou illisible : aucun statut (R5).
pub fn load(data_dir: &Path, project: Uuid) -> StatusBook {
    let Some(content) = fs::read_to_string(file(data_dir, project))
        .ok()
        .and_then(|text| serde_json::from_str::<StatusFile>(&text).ok())
    else {
        return StatusBook::default();
    };
    if content.version > VERSION {
        return StatusBook {
            requests: Statuses::new(),
            read_only: true,
        };
    }
    StatusBook {
        requests: content.requests,
        read_only: false,
    }
}

/// Écrit les statuts de façon atomique ; un fichier d'une version plus récente
/// est laissé tel quel (R5).
pub fn save(data_dir: &Path, project: Uuid, book: &StatusBook) -> Result<(), CommandError> {
    if book.read_only {
        return Ok(());
    }
    let dir = local::project_dir(data_dir, project);
    fs::create_dir_all(&dir)
        .map_err(|error| CommandError::io("Création du dossier local impossible", &dir, &error))?;
    let path = file(data_dir, project);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string(&StatusFile {
        version: VERSION,
        requests: book.requests.clone(),
    })
    .map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture impossible : {error}"),
        )
    })?;
    fs::write(&tmp, text).map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, &path).map_err(|error| CommandError::io("Écriture impossible", &path, &error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(paths: &[&str]) -> StatusBook {
        let mut book = StatusBook::default();
        for (index, path) in paths.iter().enumerate() {
            book.record(path, index % 2 == 0, index as u64);
        }
        book
    }

    #[test]
    fn une_execution_remplace_la_precedente() {
        let mut book = StatusBook::default();
        book.record("a.yml", false, 1);
        book.record("a.yml", true, 2);
        assert_eq!(
            book.requests.get("a.yml"),
            Some(&RequestStatus {
                passed: true,
                at: 2
            })
        );
    }

    #[test]
    fn renommer_un_dossier_deplace_ses_requetes_seulement() {
        let mut book = book(&["cmd/a.yml", "cmd/sous/b.yml", "cmdes/c.yml", "d.yml"]);
        assert!(book.rename("cmd", "commandes"));
        let paths: Vec<&str> = book.requests.keys().map(String::as_str).collect();
        assert_eq!(
            paths,
            [
                "cmdes/c.yml",
                "commandes/a.yml",
                "commandes/sous/b.yml",
                "d.yml"
            ]
        );
        assert!(book.rename("d.yml", "cmdes/d.yml"));
        assert!(!book.rename("absent", "ailleurs"));
    }

    #[test]
    fn supprimer_retire_la_requete_ou_le_dossier() {
        let mut book = book(&["cmd/a.yml", "cmd/sous/b.yml", "cmdes/c.yml"]);
        assert!(book.remove("cmd"));
        assert_eq!(book.requests.len(), 1);
        assert!(book.remove("cmdes/c.yml"));
        assert!(!book.remove("cmdes/c.yml"));
    }

    #[test]
    fn aller_retour_sur_disque() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        assert_eq!(load(dir.path(), id), StatusBook::default());
        let book = book(&["a.yml", "b/c.yml"]);
        save(dir.path(), id, &book).unwrap();
        assert_eq!(load(dir.path(), id), book);
        let text = fs::read_to_string(file(dir.path(), id)).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            serde_json::json!({
                "version": 1,
                "requests": {
                    "a.yml": { "passed": true, "at": 0 },
                    "b/c.yml": { "passed": false, "at": 1 }
                }
            })
        );
    }

    #[test]
    fn fichier_illisible_vaut_aucun_statut() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        fs::create_dir_all(local::project_dir(dir.path(), id)).unwrap();
        fs::write(file(dir.path(), id), "{").unwrap();
        assert_eq!(load(dir.path(), id), StatusBook::default());
    }

    #[test]
    fn version_plus_recente_jamais_reecrite() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        fs::create_dir_all(local::project_dir(dir.path(), id)).unwrap();
        let newer = r#"{"version":2,"requests":{"a.yml":{"passed":true,"at":1}},"autre":1}"#;
        fs::write(file(dir.path(), id), newer).unwrap();
        let mut book = load(dir.path(), id);
        assert!(book.read_only);
        assert!(book.requests.is_empty());
        book.record("b.yml", true, 2);
        save(dir.path(), id, &book).unwrap();
        assert_eq!(fs::read_to_string(file(dir.path(), id)).unwrap(), newer);
    }
}
