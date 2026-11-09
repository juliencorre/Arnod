//! Persistance de l'état de l'application dans `arnod.json` (§ 6.1 du plan).
//!
//! Le fichier vit dans `app_local_data_dir()`, local au poste car les chemins
//! des projets le sont. Il est écrit de façon atomique : fichier temporaire
//! dans le même dossier, puis renommage. Un fichier illisible est mis de côté,
//! jamais écrasé ; un fichier écrit par une version plus récente d'Arnod n'est
//! ni lu ni réécrit.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::catalog::model::{CatalogFile, CatalogFileV1};
use crate::catalog::{Catalog, FILE_VERSION};
use crate::error::{CommandError, CommandErrorKind};

/// Nom du fichier d'état dans le dossier de données.
pub const STATE_FILE_NAME: &str = "arnod.json";

/// Résultat de la lecture de l'état au démarrage.
#[derive(Debug)]
pub enum Loaded {
    /// Pas encore de fichier : premier lancement (W1).
    FirstLaunch(Catalog),
    /// Fichier lu.
    Existing(Catalog),
    /// Fichier illisible, renommé en `backup` ; Arnod repart d'un état neuf.
    Recovered {
        catalog: Catalog,
        backup: PathBuf,
        reason: String,
    },
}

/// Chemin du fichier d'état dans le dossier de données `dir`.
pub fn state_file(dir: &Path) -> PathBuf {
    dir.join(STATE_FILE_NAME)
}

/// Lit l'état. Échoue si le fichier ne peut pas être lu, s'il a été écrit par
/// une version plus récente, ou si un fichier illisible ne peut pas être mis de côté.
pub fn load(dir: &Path) -> Result<Loaded, CommandError> {
    let path = state_file(dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Loaded::FirstLaunch(Catalog::first_launch()));
        }
        Err(error) => {
            return Err(CommandError::io(
                "Lecture de l'état d'Arnod impossible",
                &path,
                &error,
            ));
        }
    };
    match parse(&bytes) {
        Ok(catalog) => Ok(Loaded::Existing(catalog)),
        Err(Invalid::Newer(version)) => Err(CommandError::new(
            CommandErrorKind::NewerVersion,
            format!(
                "L'état d'Arnod ({}) a été écrit par une version plus récente (format {version}). \
                 Mettez Arnod à jour ; rien n'a été modifié.",
                path.display()
            ),
        )),
        Err(Invalid::Corrupt(reason)) => Ok(Loaded::Recovered {
            catalog: Catalog::first_launch(),
            backup: set_aside(&path)?,
            reason,
        }),
    }
}

/// Écrit l'état de façon atomique, en créant le dossier de données au besoin.
pub fn save(dir: &Path, catalog: &Catalog) -> Result<(), CommandError> {
    let path = state_file(dir);
    let json = serde_json::to_vec_pretty(&catalog.to_file()).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Enregistrement de l'état d'Arnod impossible : {error}"),
        )
    })?;
    fs::create_dir_all(dir).map_err(|error| {
        CommandError::io("Création du dossier de données impossible", dir, &error)
    })?;
    let temporary = path.with_file_name(format!("{STATE_FILE_NAME}.tmp"));
    if let Err(error) = write_synced(&temporary, &json).and_then(|()| fs::rename(&temporary, &path))
    {
        // Le fichier temporaire est inutile ; l'état précédent reste intact.
        let _ = fs::remove_file(&temporary);
        return Err(CommandError::io(
            "Enregistrement de l'état d'Arnod impossible",
            &path,
            &error,
        ));
    }
    Ok(())
}

/// Raison pour laquelle un fichier d'état n'est pas utilisable.
enum Invalid {
    /// Format plus récent que [`FILE_VERSION`].
    Newer(u64),
    /// Fichier illisible, avec la raison.
    Corrupt(String),
}

fn parse(bytes: &[u8]) -> Result<Catalog, Invalid> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| Invalid::Corrupt(format!("JSON invalide : {error}")))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| Invalid::Corrupt("version absente".to_owned()))?;
    if version > u64::from(FILE_VERSION) {
        return Err(Invalid::Newer(version));
    }
    let invalid =
        |error: serde_json::Error| Invalid::Corrupt(format!("contenu invalide : {error}"));
    let file: CatalogFile = match version {
        // Version 1 : workspace par défaut, migré à la lecture (R12). Le
        // fichier n'est réécrit en version 2 qu'à la prochaine modification.
        1 => Catalog::migrate_v1(serde_json::from_value::<CatalogFileV1>(value).map_err(invalid)?),
        2 => serde_json::from_value(value).map_err(invalid)?,
        _ => return Err(Invalid::Corrupt(format!("version {version} inconnue"))),
    };
    Catalog::from_file(file).map_err(Invalid::Corrupt)
}

/// Renomme un fichier d'état illisible en `arnod.json.corrompu-<horodatage>`.
fn set_aside(path: &Path) -> Result<PathBuf, CommandError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let backup = path.with_file_name(format!("{STATE_FILE_NAME}.corrompu-{stamp}"));
    fs::rename(path, &backup).map_err(|error| {
        CommandError::io(
            "Mise de côté de l'état illisible d'Arnod impossible",
            path,
            &error,
        )
    })?;
    Ok(backup)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn data_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_state(dir: &Path, content: &str) {
        fs::write(state_file(dir), content).unwrap();
    }

    fn expect_recovered(dir: &Path) -> (PathBuf, String) {
        match load(dir).unwrap() {
            Loaded::Recovered { backup, reason, .. } => (backup, reason),
            other => panic!("fichier illisible attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn premier_lancement_sans_fichier() {
        let dir = data_dir();

        let Loaded::FirstLaunch(catalog) = load(dir.path()).unwrap() else {
            panic!("premier lancement attendu");
        };

        assert!(catalog.snapshot(|_| true).workspaces.is_empty());
        assert_eq!(catalog.last_workspace_id(), None);
        assert!(!state_file(dir.path()).exists(), "la lecture n'écrit rien");
    }

    #[test]
    fn aller_retour_par_le_disque() {
        let dir = data_dir();
        let mut catalog = Catalog::first_launch();
        let boutique = catalog.create_workspace("Boutique").unwrap();
        catalog.open_workspace(Some(boutique)).unwrap();

        save(dir.path(), &catalog).unwrap();

        let Loaded::Existing(loaded) = load(dir.path()).unwrap() else {
            panic!("fichier existant attendu");
        };
        assert_eq!(loaded, catalog);
    }

    #[test]
    fn ecriture_atomique_sans_fichier_temporaire_restant() {
        let dir = data_dir();
        let catalog = Catalog::first_launch();
        save(dir.path(), &catalog).unwrap();
        save(dir.path(), &catalog).unwrap();

        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [STATE_FILE_NAME]);
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(state_file(dir.path())).unwrap()).unwrap();
        assert_eq!(json["version"], 2);
    }

    #[test]
    fn cree_le_dossier_de_donnees() {
        let dir = data_dir();
        let nested = dir.path().join("io.github.juliencorre.arnod");

        save(&nested, &Catalog::first_launch()).unwrap();

        assert!(state_file(&nested).is_file());
    }

    #[test]
    fn echec_d_ecriture_signale_sans_toucher_l_etat_precedent() {
        let dir = data_dir();
        let catalog = Catalog::first_launch();
        save(dir.path(), &catalog).unwrap();
        let before = fs::read(state_file(dir.path())).unwrap();
        // Un dossier à la place du fichier temporaire fait échouer l'écriture.
        fs::create_dir(dir.path().join(format!("{STATE_FILE_NAME}.tmp"))).unwrap();

        let error = save(dir.path(), &Catalog::first_launch()).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::Io);
        assert!(
            error
                .message
                .starts_with("Enregistrement de l'état d'Arnod impossible")
        );
        assert_eq!(fs::read(state_file(dir.path())).unwrap(), before);
    }

    #[test]
    fn fichier_illisible_mis_de_cote() {
        let dir = data_dir();
        write_state(dir.path(), "pas du JSON");

        let (backup, reason) = expect_recovered(dir.path());

        assert!(reason.starts_with("JSON invalide"), "{reason}");
        assert!(!state_file(dir.path()).exists());
        assert_eq!(fs::read_to_string(&backup).unwrap(), "pas du JSON");
        let backup_name = backup.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            backup_name.starts_with("arnod.json.corrompu-"),
            "{backup_name}"
        );
    }

    #[test]
    fn version_absente_ou_inconnue_mise_de_cote() {
        for (content, expected) in [
            (r#"{ "projects": [] }"#, "version absente"),
            (r#"{ "version": 0 }"#, "version 0 inconnue"),
        ] {
            let dir = data_dir();
            write_state(dir.path(), content);
            assert_eq!(expect_recovered(dir.path()).1, expected);
        }
    }

    #[test]
    fn contenu_ou_catalogue_invalide_mis_de_cote() {
        let dir = data_dir();
        write_state(dir.path(), r#"{ "version": 1, "projects": [] }"#);
        assert!(
            expect_recovered(dir.path())
                .1
                .starts_with("contenu invalide")
        );

        let dir = data_dir();
        let mut catalog = Catalog::first_launch();
        catalog.create_workspace("Boutique").unwrap();
        let mut file = catalog.to_file();
        file.workspaces.push(file.workspaces[0].clone());
        write_state(dir.path(), &serde_json::to_string(&file).unwrap());
        assert!(expect_recovered(dir.path()).1.contains("en double"));
    }

    #[test]
    fn version_plus_recente_ni_lue_ni_touchee() {
        let dir = data_dir();
        let content = r#"{ "version": 3, "nouveau": true }"#;
        write_state(dir.path(), content);

        let error = load(dir.path()).unwrap_err();

        assert_eq!(error.kind, CommandErrorKind::NewerVersion);
        assert!(
            error.message.contains("version plus récente (format 3)"),
            "{}",
            error.message
        );
        assert_eq!(fs::read_to_string(state_file(dir.path())).unwrap(), content);
    }

    /// Fichier version 1 : « Mes projets » par défaut avec tous les projets,
    /// « Boutique » ouvert avec un seul.
    fn v1_state(default_has_all: bool) -> String {
        let (p1, p2) = (
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000002",
        );
        let (mes, boutique) = (
            "00000000-0000-0000-0000-00000000000a",
            "00000000-0000-0000-0000-00000000000b",
        );
        let default_projects = if default_has_all {
            format!(r#"["{p1}", "{p2}"]"#)
        } else {
            format!(r#"["{p1}"]"#)
        };
        format!(
            r#"{{
              "version": 1,
              "projects": [
                {{ "id": "{p1}", "name": "boutique-tests", "path": "/dev/boutique-tests" }},
                {{ "id": "{p2}", "name": "auth-sso", "path": "/dev/auth-sso" }}
              ],
              "workspaces": [
                {{ "id": "{mes}", "name": "Mes projets", "color": "gray", "projectIds": {default_projects} }},
                {{ "id": "{boutique}", "name": "Boutique", "color": "orange", "projectIds": ["{p1}"] }}
              ],
              "defaultWorkspaceId": "{mes}",
              "lastWorkspaceId": "{mes}"
            }}"#
        )
    }

    #[test]
    fn r12_version_1_migree_sans_le_workspace_par_defaut_redondant() {
        let dir = data_dir();
        write_state(dir.path(), &v1_state(true));

        let Loaded::Existing(catalog) = load(dir.path()).unwrap() else {
            panic!("fichier existant attendu");
        };

        let snapshot = catalog.snapshot(|_| true);
        let names: Vec<&str> = snapshot
            .workspaces
            .iter()
            .map(|w| w.name.as_str())
            .collect();
        assert_eq!(names, ["Boutique"]);
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(
            snapshot.last_workspace_id, None,
            "l'onglet ouvert devient « Tous les projets »"
        );
        // Lecture seule : le fichier reste en version 1 jusqu'à la prochaine écriture.
        let raw = fs::read_to_string(state_file(dir.path())).unwrap();
        assert!(raw.contains("defaultWorkspaceId"));

        save(dir.path(), &catalog).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(state_file(dir.path())).unwrap()).unwrap();
        assert_eq!(json["version"], 2);
        assert!(json.get("defaultWorkspaceId").is_none());
    }

    #[test]
    fn r12_workspace_par_defaut_partiel_garde_comme_workspace_ordinaire() {
        let dir = data_dir();
        write_state(dir.path(), &v1_state(false));

        let Loaded::Existing(catalog) = load(dir.path()).unwrap() else {
            panic!("fichier existant attendu");
        };

        let snapshot = catalog.snapshot(|_| true);
        let names: Vec<&str> = snapshot
            .workspaces
            .iter()
            .map(|w| w.name.as_str())
            .collect();
        assert_eq!(names, ["Mes projets", "Boutique"]);
        assert_eq!(snapshot.last_workspace_id, Some(snapshot.workspaces[0].id));
    }
}
