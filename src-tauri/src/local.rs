//! Données locales d'un projet, propres au poste et jamais versionnées (§ 6.1).
//!
//! `projects/<id>/local.json` dans le dossier de données de l'application :
//! dernier environnement utilisé (R7), dossiers repliés (A1) et valeurs des
//! variables locales non secrètes. Les secrets vont dans le coffre du système.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};

/// Variable locale : sa valeur est ici, sauf si elle est secrète (coffre).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalVar {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub secret: bool,
}

/// Contenu de `local.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalData {
    #[serde(default)]
    pub last_environment: Option<String>,
    /// Chemins des dossiers repliés dans l'arborescence.
    #[serde(default)]
    pub collapsed: Vec<String>,
    /// Variables locales, par environnement.
    #[serde(default)]
    pub locals: BTreeMap<String, Vec<LocalVar>>,
}

/// Dossier des données locales d'un projet.
pub fn project_dir(data_dir: &Path, project: Uuid) -> PathBuf {
    data_dir.join("projects").join(project.to_string())
}

fn file(data_dir: &Path, project: Uuid) -> PathBuf {
    project_dir(data_dir, project).join("local.json")
}

/// Charge les données locales ; absentes ou illisibles : valeurs par défaut.
pub fn load(data_dir: &Path, project: Uuid) -> LocalData {
    fs::read_to_string(file(data_dir, project))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Écrit les données locales de façon atomique.
pub fn save(data_dir: &Path, project: Uuid, data: &LocalData) -> Result<(), CommandError> {
    let dir = project_dir(data_dir, project);
    fs::create_dir_all(&dir)
        .map_err(|error| CommandError::io("Création du dossier local impossible", &dir, &error))?;
    let path = file(data_dir, project);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(data).map_err(|error| {
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

    #[test]
    fn aller_retour_et_valeurs_par_defaut() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        assert_eq!(load(dir.path(), id), LocalData::default());
        let mut data = LocalData {
            last_environment: Some("Recette".into()),
            ..LocalData::default()
        };
        data.collapsed.push("a/b".into());
        data.locals.insert(
            "Recette".into(),
            vec![LocalVar {
                name: "token".into(),
                value: None,
                secret: true,
            }],
        );
        save(dir.path(), id, &data).unwrap();
        assert_eq!(load(dir.path(), id), data);
    }

    #[test]
    fn fichier_illisible_donne_les_valeurs_par_defaut() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        fs::create_dir_all(project_dir(dir.path(), id)).unwrap();
        fs::write(file(dir.path(), id), "{").unwrap();
        assert_eq!(load(dir.path(), id), LocalData::default());
    }
}
