//! Dossier d'un projet et sa racine `opencollection.yml` (T8, S1, S5).
//!
//! Un projet est une collection OpenCollection : un dossier qui contient
//! `opencollection.yml` (docs/plan-opencollection.md). Son identifiant Arnod est
//! dans `extensions.arnod.id` : il permet de reconnaître le dossier après un
//! déplacement ou un clone (P12). Un projet au format précédent (`arnod.yml`)
//! est converti à la lecture (R6). Arnod n'initialise pas Git.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::ROOT_FILE;
use crate::opencollection::collection;
use crate::opencollection::migrate::{self, LEGACY_MANIFEST};

/// Fichier qui fait d'un dossier un projet.
pub const MANIFEST_FILE_NAME: &str = ROOT_FILE;

/// Version majeure d'OpenCollection lue et écrite.
pub const MANIFEST_VERSION: u32 = 1;

/// Identité d'un projet, lue dans `opencollection.yml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Version majeure de la spécification.
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
    /// ce qu'Arnod y a écrit. Au mieux : un échec ici laisse au pire un dossier
    /// vide ou une racine orpheline, sans conséquence pour le référentiel.
    pub fn rollback(self) {
        let _ = if self.created_dir {
            fs::remove_dir_all(&self.folder)
        } else {
            let _ = fs::remove_dir_all(self.folder.join(crate::tree::ENVIRONMENTS_DIR));
            fs::remove_file(self.folder.join(MANIFEST_FILE_NAME))
        };
    }
}

/// Crée le dossier d'un projet et sa racine `opencollection.yml`.
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
    if let Err(error) =
        collection::write(folder, &collection::new_root(&manifest.name, manifest.id))
    {
        created.rollback();
        return Err(error);
    }
    Ok(created)
}

/// Lit l'identité d'un projet (P12) ; un projet au format précédent est
/// d'abord converti (R6). Une collection sans identifiant Arnod en reçoit un,
/// écrit dans `extensions.arnod` (Q4).
pub fn read_manifest(folder: &Path) -> Result<Manifest, CommandError> {
    migrate::run(folder)?;
    let root = collection::read(folder)?;
    let name = collection::name(&root).unwrap_or_else(|| {
        folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let id = match collection::meta(&root).id {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            collection::update_meta(folder, |meta| meta.id = Some(id))?;
            id
        }
    };
    Ok(Manifest {
        version: collection::spec_major(&root).map_or(MANIFEST_VERSION, |m| m as u32),
        id,
        name,
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
            if folder.join(MANIFEST_FILE_NAME).exists() || folder.join(LEGACY_MANIFEST).exists() {
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
        assert!(text.starts_with("opencollection: \"1.0.0\""), "{text}");
        assert!(text.contains("name: boutique-tests"), "{text}");
        assert!(
            text.contains("id: 00000000-0000-0000-0000-000000000007"),
            "{text}"
        );
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
            error.message.ends_with("(opencollection.yml absent)."),
            "{}",
            error.message
        );

        fs::write(dir.path().join(MANIFEST_FILE_NAME), "name: [incomplet").unwrap();
        let error = read_manifest(dir.path()).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotAProject);
        assert!(error.message.contains("est illisible"), "{}", error.message);

        fs::write(dir.path().join(MANIFEST_FILE_NAME), "info:\n  name: x\n").unwrap();
        let error = read_manifest(dir.path()).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotAProject);
        assert!(
            error.message.contains("pas de version"),
            "{}",
            error.message
        );

        fs::write(
            dir.path().join(MANIFEST_FILE_NAME),
            "opencollection: 1.0.0\ninfo:\n  nom: x\n",
        )
        .unwrap();
        let error = read_manifest(dir.path()).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotAProject);
        assert!(error.message.contains("non conforme"), "{}", error.message);
    }

    #[test]
    fn version_majeure_superieure_refusee() {
        let dir = workdir();
        fs::write(
            dir.path().join(MANIFEST_FILE_NAME),
            "opencollection: 2.0.0\n",
        )
        .unwrap();
        assert_eq!(
            read_manifest(dir.path()).unwrap_err().kind,
            CommandErrorKind::NewerVersion
        );
    }

    #[test]
    fn s5_une_collection_sans_identifiant_en_recoit_un() {
        let dir = workdir();
        let original = "opencollection: 1.0.0\ninfo:\n  name: API Bruno\nextensions:\n  bruno:\n    theme: sombre\n";
        fs::write(dir.path().join(MANIFEST_FILE_NAME), original).unwrap();
        let first = read_manifest(dir.path()).unwrap();
        assert_eq!(first.name, "API Bruno");
        assert_eq!(
            read_manifest(dir.path()).unwrap().id,
            first.id,
            "identifiant écrit"
        );
        let text = fs::read_to_string(dir.path().join(MANIFEST_FILE_NAME)).unwrap();
        assert!(
            text.contains("theme: sombre"),
            "extension d'un autre outil gardée : {text}"
        );
    }
}
