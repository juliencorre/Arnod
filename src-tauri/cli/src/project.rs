//! Projet joué par `arnod run` et données du poste qu'il lit (R5, R6) : rien n'y est écrit.

use std::path::{Path, PathBuf};

use arnod_core::opencollection::collection;
use uuid::Uuid;

/// Identifiant de l'application (`tauri.conf.json`) : nom de son dossier de données.
const APP_IDENTIFIER: &str = "io.github.juliencorre.arnod";

/// Variable qui donne un autre dossier de données (tests, poste partagé).
pub const DATA_DIR_VAR: &str = "ARNOD_DONNEES";

/// Projet trouvé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
    /// Identifiant Arnod (`extensions.arnod.id`) : il relie le projet aux valeurs locales et au
    /// coffre du poste ; absent, le poste n'a rien pour lui.
    pub id: Option<Uuid>,
}

/// Projet du dossier `start` ou d'un de ses parents (celui qui contient `opencollection.yml`).
pub fn find(start: &Path) -> Result<Project, String> {
    let start = dunce_like(start);
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        if collection::exists(dir) {
            return read(dir);
        }
        if dir.join("arnod.yml").is_file() {
            return Err(format!(
                "Le projet {} est au format d'une version précédente d'Arnod : ouvrez-le une fois \
                 dans l'application pour le convertir.",
                dir.display()
            ));
        }
        current = dir.parent();
    }
    Err(format!(
        "Aucun projet Arnod dans {} ni ses dossiers parents (opencollection.yml absent). Lancez \
         la commande depuis le dossier du projet, ou donnez-le par --projet.",
        start.display()
    ))
}

/// Chemin absolu, sans le préfixe `\\?\` de Windows.
fn dunce_like(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let text = absolute.to_string_lossy();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned())
}

fn read(dir: &Path) -> Result<Project, String> {
    let root = collection::read(dir).map_err(|error| error.message)?;
    let name = collection::name(&root).unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    Ok(Project {
        root: dir.to_path_buf(),
        name,
        id: collection::meta(&root).id,
    })
}

/// Dossier de données de l'application sur ce poste (celui de Tauri, `app_local_data_dir`) :
/// `ARNOD_DONNEES`, sinon `%LOCALAPPDATA%\<identifiant>` sous Windows,
/// `$XDG_DATA_HOME/<identifiant>` ou `~/.local/share/<identifiant>` ailleurs.
pub fn data_dir(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = env(DATA_DIR_VAR).filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME")
            .filter(|d| Path::new(d).is_absolute())
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    };
    base.map(|base| base.join(APP_IDENTIFIER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r6_dossier_de_donnees_du_poste() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        assert_eq!(
            data_dir(vars(&[(DATA_DIR_VAR, "/tmp/d"), ("HOME", "/home/x")])),
            Some(PathBuf::from("/tmp/d"))
        );
        if cfg!(target_os = "linux") {
            assert_eq!(
                data_dir(vars(&[("HOME", "/home/x")])),
                Some(PathBuf::from(
                    "/home/x/.local/share/io.github.juliencorre.arnod"
                ))
            );
            assert_eq!(
                data_dir(vars(&[("XDG_DATA_HOME", "/d"), ("HOME", "/home/x")])),
                Some(PathBuf::from("/d/io.github.juliencorre.arnod"))
            );
            assert_eq!(data_dir(vars(&[])), None);
        }
    }

    #[test]
    fn lc1_projet_du_dossier_courant_ou_d_un_parent() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::from_u128(7);
        std::fs::write(
            dir.path().join("opencollection.yml"),
            collection_text("boutique-tests", id),
        )
        .unwrap();
        let sub = dir.path().join("commandes/v2");
        std::fs::create_dir_all(&sub).unwrap();
        let project = find(&sub).unwrap();
        assert_eq!(project.name, "boutique-tests");
        assert_eq!(project.id, Some(id));
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(
            find(elsewhere.path())
                .unwrap_err()
                .starts_with("Aucun projet Arnod")
        );
        std::fs::write(elsewhere.path().join("arnod.yml"), "version: 1\n").unwrap();
        assert!(
            find(elsewhere.path())
                .unwrap_err()
                .contains("version précédente")
        );
    }

    fn collection_text(name: &str, id: Uuid) -> String {
        let root = collection::new_root(name, id);
        let dir = tempfile::tempdir().unwrap();
        collection::write(dir.path(), &root).unwrap();
        std::fs::read_to_string(dir.path().join("opencollection.yml")).unwrap()
    }
}
