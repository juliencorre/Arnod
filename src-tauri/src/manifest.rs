//! Dossier d'un projet et son manifeste `arnod.yml` (T8).
//!
//! Un projet est un dossier qui contient `arnod.yml`. Le manifeste porte
//! l'identifiant du projet : il permettra de reconnaître le dossier après un
//! déplacement ou un clone (P12). Arnod n'initialise pas Git.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};

/// Nom du manifeste à la racine du dossier d'un projet.
pub const MANIFEST_FILE_NAME: &str = "arnod.yml";

/// Version courante du format du manifeste.
pub const MANIFEST_VERSION: u32 = 1;

/// Commentaire placé en tête du manifeste.
const HEADER: &str = "# Projet Arnod : ce fichier identifie le dossier comme un projet.\n";

/// Contenu de `arnod.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub id: Uuid,
    pub name: String,
}

impl Manifest {
    /// Manifeste au format courant.
    pub fn new(id: Uuid, name: impl Into<String>) -> Self {
        Self {
            version: MANIFEST_VERSION,
            id,
            name: name.into(),
        }
    }
}

/// Dossier de projet préparé par Arnod, à défaire si l'inscription échoue ensuite.
#[derive(Debug)]
#[must_use = "défaire la création avec `rollback` si l'inscription échoue"]
pub struct CreatedFolder {
    folder: PathBuf,
    created_dir: bool,
}

impl CreatedFolder {
    /// Dossier du projet.
    pub fn path(&self) -> &Path {
        &self.folder
    }

    /// Défait la création : supprime le dossier s'il a été créé par Arnod, sinon
    /// le seul manifeste. Au mieux : un échec ici laisse au pire un dossier vide
    /// ou un manifeste orphelin, sans conséquence pour le référentiel.
    pub fn rollback(self) {
        let _ = if self.created_dir {
            fs::remove_dir_all(&self.folder)
        } else {
            fs::remove_file(self.folder.join(MANIFEST_FILE_NAME))
        };
    }
}

/// Crée le dossier d'un projet et son manifeste.
///
/// Le dossier peut déjà exister s'il est vide ; ses dossiers parents manquants
/// sont créés. Un dossier non vide, ou qui contient déjà un projet, est refusé.
pub fn create_project_folder(
    folder: &Path,
    manifest: &Manifest,
) -> Result<CreatedFolder, CommandError> {
    let created = CreatedFolder {
        folder: folder.to_path_buf(),
        created_dir: prepare_folder(folder)?,
    };
    if let Err(error) = write_manifest(folder, manifest) {
        created.rollback();
        return Err(error);
    }
    Ok(created)
}

/// Lit le manifeste d'un dossier de projet.
pub fn read_manifest(folder: &Path) -> Result<Manifest, CommandError> {
    let path = folder.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CommandError::new(
                CommandErrorKind::NotAProject,
                format!(
                    "Le dossier {} ne contient pas de projet Arnod ({MANIFEST_FILE_NAME} absent).",
                    folder.display()
                ),
            )
        } else {
            CommandError::io("Lecture du manifeste impossible", &path, &error)
        }
    })?;
    serde_saphyr::from_str(&text).map_err(|error| {
        CommandError::new(
            CommandErrorKind::NotAProject,
            format!("Le manifeste {} est illisible : {error}", path.display()),
        )
    })
}

/// Vérifie que `folder` peut accueillir un projet ; renvoie `true` s'il a fallu le créer.
fn prepare_folder(folder: &Path) -> Result<bool, CommandError> {
    let not_empty = |message: String| CommandError::new(CommandErrorKind::FolderNotEmpty, message);
    if folder.is_file() {
        return Err(not_empty(format!(
            "Un fichier porte déjà ce nom : {}.",
            folder.display()
        )));
    }
    match fs::read_dir(folder) {
        Ok(mut entries) => {
            if folder.join(MANIFEST_FILE_NAME).exists() {
                Err(not_empty(format!(
                    "Le dossier {} contient déjà un projet Arnod.",
                    folder.display()
                )))
            } else if entries.next().is_some() {
                Err(not_empty(format!(
                    "Le dossier {} existe déjà et n'est pas vide.",
                    folder.display()
                )))
            } else {
                Ok(false)
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(folder).map_err(|error| {
                CommandError::io("Création du dossier du projet impossible", folder, &error)
            })?;
            Ok(true)
        }
        Err(error) => Err(CommandError::io(
            "Lecture du dossier du projet impossible",
            folder,
            &error,
        )),
    }
}

fn write_manifest(folder: &Path, manifest: &Manifest) -> Result<(), CommandError> {
    let path = folder.join(MANIFEST_FILE_NAME);
    let yaml = serde_saphyr::to_string(manifest).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture du manifeste impossible : {error}"),
        )
    })?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| {
            file.write_all(HEADER.as_bytes())?;
            file.write_all(yaml.as_bytes())?;
            file.sync_all()
        })
        .map_err(|error| CommandError::io("Écriture du manifeste impossible", &path, &error))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn manifest() -> Manifest {
        Manifest::new(Uuid::from_u128(7), "boutique-tests")
    }

    fn workdir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn cree_le_dossier_et_un_manifeste_lisible() {
        let dir = workdir();
        let folder = dir.path().join("arnod").join("boutique-tests");

        let created = create_project_folder(&folder, &manifest()).unwrap();

        assert_eq!(created.path(), folder);
        let text = fs::read_to_string(folder.join(MANIFEST_FILE_NAME)).unwrap();
        assert!(text.starts_with(HEADER), "{text}");
        assert!(text.contains("name: boutique-tests"), "{text}");
        assert!(text.contains("version: 1"), "{text}");
        assert_eq!(read_manifest(&folder).unwrap(), manifest());
    }

    #[test]
    fn accepte_un_dossier_vide_existant() {
        let dir = workdir();

        let created = create_project_folder(dir.path(), &manifest()).unwrap();

        assert_eq!(read_manifest(dir.path()).unwrap(), manifest());
        created.rollback();
        assert!(dir.path().is_dir(), "le dossier préexistant est conservé");
        assert!(!dir.path().join(MANIFEST_FILE_NAME).exists());
    }

    #[test]
    fn annuler_supprime_le_dossier_cree() {
        let dir = workdir();
        let folder = dir.path().join("boutique-tests");

        create_project_folder(&folder, &manifest())
            .unwrap()
            .rollback();

        assert!(!folder.exists());
        assert!(dir.path().is_dir());
    }

    #[test]
    fn refuse_un_dossier_non_vide_sans_le_toucher() {
        let dir = workdir();
        fs::write(dir.path().join("notes.txt"), "à garder").unwrap();

        let error = create_project_folder(dir.path(), &manifest()).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::FolderNotEmpty);
        assert!(
            error.message.ends_with("existe déjà et n'est pas vide."),
            "{}",
            error.message
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("notes.txt")).unwrap(),
            "à garder"
        );
        assert!(!dir.path().join(MANIFEST_FILE_NAME).exists());
    }

    #[test]
    fn refuse_un_dossier_contenant_deja_un_projet() {
        let dir = workdir();
        let _first = create_project_folder(dir.path(), &manifest()).unwrap();

        let error =
            create_project_folder(dir.path(), &Manifest::new(Uuid::new_v4(), "autre")).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::FolderNotEmpty);
        assert!(
            error.message.ends_with("contient déjà un projet Arnod."),
            "{}",
            error.message
        );
        assert_eq!(read_manifest(dir.path()).unwrap(), manifest());
    }

    #[test]
    fn refuse_un_fichier_du_meme_nom() {
        let dir = workdir();
        let file = dir.path().join("boutique-tests");
        fs::write(&file, "").unwrap();

        let error = create_project_folder(&file, &manifest()).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::FolderNotEmpty);
        assert!(error.message.starts_with("Un fichier porte déjà ce nom"));
    }

    #[test]
    fn manifeste_absent_ou_illisible() {
        let dir = workdir();
        let error = read_manifest(dir.path()).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotAProject);
        assert!(
            error.message.ends_with("(arnod.yml absent)."),
            "{}",
            error.message
        );

        fs::write(dir.path().join(MANIFEST_FILE_NAME), "name: [incomplet").unwrap();
        let error = read_manifest(dir.path()).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotAProject);
        assert!(error.message.contains("est illisible"), "{}", error.message);
    }
}
