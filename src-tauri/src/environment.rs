//! Environnements d'un projet : fichiers, chaîne ordonnée, variables (N1 à N4, R7).
//!
//! Un fichier par environnement dans `environments/` ; seules les variables
//! **partagées** y figurent. Les noms des variables locales y sont listés
//! (`locals`) pour que l'équipe voie ce qu'il faut renseigner, jamais leurs valeurs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::catalog::names::same_name;
use crate::error::{CommandError, CommandErrorKind};
use crate::tree::{ENVIRONMENTS_DIR, item_name, slug};

/// Version courante du format.
pub const ENVIRONMENT_VERSION: u32 = 1;

const ORDER_FILE: &str = "_order.yml";

/// Variable partagée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

fn default_color() -> String {
    "gray".to_owned()
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Contenu d'un fichier d'environnement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub version: u32,
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub protected: bool,
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// Noms des variables locales (valeurs hors du projet).
    #[serde(default)]
    pub locals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Order {
    version: u32,
    order: Vec<String>,
}

fn dir(root: &Path) -> PathBuf {
    root.join(ENVIRONMENTS_DIR)
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn write_yaml<T: Serialize>(path: &Path, header: &str, value: &T) -> Result<(), CommandError> {
    let body = serde_saphyr::to_string(value)
        .map_err(|error| invalid(format!("Écriture impossible ({error}).")))?;
    fs::write(path, format!("{header}{body}"))
        .map_err(|error| CommandError::io("Écriture impossible", path, &error))
}

fn read_order(root: &Path) -> Vec<String> {
    fs::read_to_string(dir(root).join(ORDER_FILE))
        .ok()
        .and_then(|text| serde_saphyr::from_str::<Order>(&text).ok())
        .map(|order| order.order)
        .unwrap_or_default()
}

fn write_order(root: &Path, order: &[String]) -> Result<(), CommandError> {
    write_yaml(
        &dir(root).join(ORDER_FILE),
        "# Ordre des environnements Arnod\n",
        &Order {
            version: ENVIRONMENT_VERSION,
            order: order.to_vec(),
        },
    )
}

/// Environnements du projet dans l'ordre de la chaîne ; ceux qui n'y figurent
/// pas suivent par nom. Un fichier illisible est ignoré.
pub fn list(root: &Path) -> Vec<Environment> {
    let mut found: Vec<Environment> = fs::read_dir(dir(root))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.ends_with(".yml") && name != ORDER_FILE
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_saphyr::from_str::<Environment>(&text).ok())
        .collect();
    let order = read_order(root);
    found.sort_by_key(|env| {
        (
            order
                .iter()
                .position(|name| same_name(name, &env.name))
                .unwrap_or(usize::MAX),
            env.name.to_lowercase(),
        )
    });
    found
}

fn file_for(root: &Path, name: &str) -> PathBuf {
    dir(root).join(format!("{}.yml", slug(name)))
}

/// Crée `environments/` avec « Local » et la variable `baseUrl` vide (N1).
pub fn init(root: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    if list(root).is_empty() {
        let local = Environment {
            version: ENVIRONMENT_VERSION,
            name: "Local".to_owned(),
            color: "gray".to_owned(),
            protected: false,
            variables: vec![Variable {
                name: "baseUrl".to_owned(),
                value: String::new(),
            }],
            locals: Vec::new(),
        };
        write_yaml(&file_for(root, "Local"), "# Environnement Arnod\n", &local)?;
        write_order(root, &["Local".to_owned()])?;
    }
    Ok(())
}

/// Enregistre un environnement ; `previous` est son ancien nom en cas de renommage.
pub fn save(
    root: &Path,
    environment: &Environment,
    previous: Option<&str>,
) -> Result<(), CommandError> {
    let name = item_name(&environment.name)?;
    let mut environment = environment.clone();
    environment.name = name.clone();
    environment.version = ENVIRONMENT_VERSION;
    let others = list(root);
    let renamed_from = previous.filter(|previous| !same_name(previous, &name) || *previous != name);
    if others.iter().any(|other| {
        same_name(&other.name, &name) && previous.is_none_or(|p| !same_name(p, &other.name))
    }) {
        return Err(CommandError::new(
            CommandErrorKind::NameTaken,
            format!("Un environnement « {name} » existe déjà dans ce projet."),
        ));
    }
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    write_yaml(
        &file_for(root, &name),
        "# Environnement Arnod\n",
        &environment,
    )?;
    let mut order: Vec<String> = others.iter().map(|other| other.name.clone()).collect();
    if let Some(previous) = renamed_from {
        if slug(previous) != slug(&name) {
            let _ = fs::remove_file(file_for(root, previous));
        }
        for entry in &mut order {
            if same_name(entry, previous) {
                entry.clone_from(&name);
            }
        }
    }
    if !order.iter().any(|entry| same_name(entry, &name)) {
        order.push(name);
    }
    write_order(root, &order)
}

/// Supprime un environnement ; le dernier ne peut pas l'être.
pub fn delete(root: &Path, name: &str) -> Result<(), CommandError> {
    let all = list(root);
    if all.len() <= 1 {
        return Err(invalid("Un projet garde au moins un environnement."));
    }
    let _ = fs::remove_file(file_for(root, name));
    let order: Vec<String> = all
        .iter()
        .filter(|env| !same_name(&env.name, name))
        .map(|env| env.name.clone())
        .collect();
    write_order(root, &order)
}

/// Fixe l'ordre de la chaîne.
pub fn reorder(root: &Path, names: &[String]) -> Result<(), CommandError> {
    write_order(root, names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Environment {
        Environment {
            version: 1,
            name: name.to_owned(),
            color: "blue".to_owned(),
            protected: false,
            variables: vec![Variable {
                name: "baseUrl".into(),
                value: "https://x".into(),
            }],
            locals: vec!["token".into()],
        }
    }

    #[test]
    fn init_cree_local_avec_base_url_vide() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        let all = list(dir.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Local");
        assert_eq!(all[0].variables[0].name, "baseUrl");
        init(dir.path()).unwrap();
        assert_eq!(list(dir.path()).len(), 1);
    }

    #[test]
    fn ordre_de_la_chaine_et_reordonnancement() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        save(dir.path(), &env("Recette"), None).unwrap();
        save(dir.path(), &env("Pré-production"), None).unwrap();
        let names = |root: &Path| list(root).into_iter().map(|e| e.name).collect::<Vec<_>>();
        assert_eq!(names(dir.path()), ["Local", "Recette", "Pré-production"]);
        reorder(
            dir.path(),
            &["Pré-production".into(), "Local".into(), "Recette".into()],
        )
        .unwrap();
        assert_eq!(names(dir.path()), ["Pré-production", "Local", "Recette"]);
    }

    #[test]
    fn noms_uniques_sans_casse() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        let error = save(dir.path(), &env("local"), None).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NameTaken);
    }

    #[test]
    fn renommer_deplace_le_fichier_et_garde_la_place() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        save(dir.path(), &env("Recette"), None).unwrap();
        save(dir.path(), &env("Recette fonctionnelle"), Some("Recette")).unwrap();
        let names: Vec<_> = list(dir.path()).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Local", "Recette fonctionnelle"]);
        assert!(!file_for(dir.path(), "Recette").exists());
    }

    #[test]
    fn le_dernier_environnement_reste() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        assert_eq!(
            delete(dir.path(), "Local").unwrap_err().kind,
            CommandErrorKind::InvalidRequest
        );
        save(dir.path(), &env("Recette"), None).unwrap();
        delete(dir.path(), "Local").unwrap();
        assert_eq!(list(dir.path()).len(), 1);
    }

    #[test]
    fn la_protection_et_les_noms_de_variables_locales_sont_gardes() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        let mut e = env("Pré-production");
        e.protected = true;
        save(dir.path(), &e, None).unwrap();
        let back = list(dir.path())
            .into_iter()
            .find(|x| x.name == "Pré-production")
            .unwrap();
        assert!(back.protected);
        assert_eq!(back.locals, ["token"]);
    }
}
