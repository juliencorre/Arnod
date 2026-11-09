//! Environnements d'un projet : fichiers, chaîne ordonnée, variables (N1 à N4, R7).
//!
//! Un fichier OpenCollection par environnement dans `environments/`
//! (`$defs/Environment`, docs/plan-opencollection.md, S4) : les variables
//! **partagées** y ont leur valeur ; les variables locales y sont des
//! variables secrètes (`secret: true`), sans valeur. L'ordre de la chaîne et la
//! protection sont dans `extensions.arnod` de la racine. Ce qu'Arnod ne gère
//! pas (description, `extends`, certificats, variables désactivées) est gardé.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::catalog::names::same_name;
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::collection;
use crate::opencollection::schema::{self, Part};
use crate::opencollection::yaml;
use crate::tree::{ENVIRONMENTS_DIR, item_name, slug};

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

/// Environnement, tel que l'interface l'édite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// Noms des variables locales (valeurs hors du projet).
    #[serde(default)]
    pub locals: Vec<String>,
}

fn dir(root: &Path) -> PathBuf {
    root.join(ENVIRONMENTS_DIR)
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn file_for(root: &Path, name: &str) -> PathBuf {
    dir(root).join(format!("{}.yml", slug(name)))
}

/// Texte d'une valeur de variable (texte ou valeur typée `{ type, data }`).
fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Object(typed)) => typed
            .get("data")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    }
}

/// Fichier d'environnement lu : environnement, et chemin pour le réécrire.
struct EnvFile {
    path: PathBuf,
    value: Value,
    environment: Environment,
}

fn from_value(value: &Value, protected: bool) -> Option<Environment> {
    schema::check(Part::Environment, value).ok()?;
    let name = value.get("name")?.as_str()?.to_owned();
    let mut variables = Vec::new();
    let mut locals = Vec::new();
    for item in value
        .get("variables")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(var_name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        if item.get("disabled").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if item.get("secret").and_then(Value::as_bool) == Some(true) {
            locals.push(var_name.to_owned());
        } else {
            variables.push(Variable {
                name: var_name.to_owned(),
                value: value_text(item.get("value")),
            });
        }
    }
    Some(Environment {
        name,
        color: value
            .get("color")
            .and_then(Value::as_str)
            .map_or_else(default_color, str::to_owned),
        protected,
        variables,
        locals,
    })
}

fn files(root: &Path) -> Vec<EnvFile> {
    let protected = collection::read_meta(root).protected;
    fs::read_dir(dir(root))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
        .filter_map(|path| {
            let value = yaml::read(&path).ok()?;
            let name = value.get("name").and_then(Value::as_str).unwrap_or("");
            let is_protected = protected.iter().any(|p| same_name(p, name));
            let environment = from_value(&value, is_protected)?;
            Some(EnvFile {
                path,
                value,
                environment,
            })
        })
        .collect()
}

/// Environnements du projet dans l'ordre de la chaîne ; ceux qui n'y figurent
/// pas suivent par nom. Un fichier illisible ou non conforme est ignoré.
pub fn list(root: &Path) -> Vec<Environment> {
    let order = collection::read_meta(root).environment_order;
    let mut found: Vec<Environment> = files(root).into_iter().map(|f| f.environment).collect();
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

/// Fichier d'un environnement : `previous` (le fichier actuel) dont seuls les
/// champs gérés par Arnod sont remplacés.
fn into_value(environment: &Environment, previous: Option<&Value>) -> Value {
    let mut map = previous
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    map.insert("name".to_owned(), Value::from(environment.name.clone()));
    map.insert("color".to_owned(), Value::from(environment.color.clone()));
    let old: Vec<Value> = previous
        .and_then(|p| p.get("variables"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let same = |item: &Value, name: &str| item.get("name").and_then(Value::as_str) == Some(name);
    let mut variables = Vec::new();
    for variable in &environment.variables {
        let mut item = Map::new();
        item.insert("name".to_owned(), Value::from(variable.name.clone()));
        let kept = old.iter().find(|v| {
            same(v, &variable.name) && v.get("secret").is_none() && v.get("disabled").is_none()
        });
        // Une valeur typée dont le texte n'a pas changé reste typée.
        let value = kept
            .and_then(|v| v.get("value"))
            .filter(|v| value_text(Some(v)) == variable.value)
            .cloned()
            .unwrap_or_else(|| Value::from(variable.value.clone()));
        item.insert("value".to_owned(), value);
        if let Some(description) = kept.and_then(|v| v.get("description")) {
            item.insert("description".to_owned(), description.clone());
        }
        variables.push(Value::Object(item));
    }
    for local in &environment.locals {
        let kept = old
            .iter()
            .find(|v| same(v, local) && v.get("secret").is_some());
        variables.push(
            kept.cloned()
                .unwrap_or_else(|| json!({ "name": local, "secret": true })),
        );
    }
    // Variables désactivées : non gérées, gardées.
    for item in &old {
        if item.get("disabled").and_then(Value::as_bool) == Some(true) {
            variables.push(item.clone());
        }
    }
    if variables.is_empty() {
        map.remove("variables");
    } else {
        map.insert("variables".to_owned(), Value::Array(variables));
    }
    Value::Object(map)
}

fn write_file(path: &Path, value: &Value) -> Result<(), CommandError> {
    schema::check(Part::Environment, value).map_err(invalid)?;
    yaml::write(path, value)
}

/// Crée `environments/` avec « Local » et la variable `baseUrl` vide (N1).
pub fn init(root: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    if list(root).is_empty() {
        let local = Environment {
            name: "Local".to_owned(),
            color: default_color(),
            protected: false,
            variables: vec![Variable {
                name: "baseUrl".to_owned(),
                value: String::new(),
            }],
            locals: Vec::new(),
        };
        write_file(&file_for(root, "Local"), &into_value(&local, None))?;
        collection::update_meta(root, |meta| {
            meta.environment_order = vec!["Local".to_owned()]
        })?;
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
    let existing = files(root);
    if existing.iter().any(|other| {
        same_name(&other.environment.name, &name)
            && previous.is_none_or(|p| !same_name(p, &other.environment.name))
    }) {
        return Err(CommandError::new(
            CommandErrorKind::NameTaken,
            format!("Un environnement « {name} » existe déjà dans ce projet."),
        ));
    }
    let old = previous.and_then(|p| existing.iter().find(|f| same_name(&f.environment.name, p)));
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    let target = file_for(root, &name);
    write_file(&target, &into_value(&environment, old.map(|f| &f.value)))?;
    if let Some(old) = old
        && old.path != target
    {
        let _ = fs::remove_file(&old.path);
    }
    let order: Vec<String> = list(root).into_iter().map(|env| env.name).collect();
    collection::update_meta(root, |meta| {
        let mut chain = if meta.environment_order.is_empty() {
            order.clone()
        } else {
            meta.environment_order.clone()
        };
        if let Some(previous) = previous {
            for entry in &mut chain {
                if same_name(entry, previous) {
                    entry.clone_from(&name);
                }
            }
            meta.protected.retain(|p| !same_name(p, previous));
        }
        if !chain.iter().any(|entry| same_name(entry, &name)) {
            chain.push(name.clone());
        }
        chain.retain(|entry| order.iter().any(|o| same_name(o, entry)));
        meta.environment_order = chain;
        meta.protected.retain(|p| !same_name(p, &name));
        if environment.protected {
            meta.protected.push(name.clone());
        }
    })
}

/// Supprime un environnement ; le dernier ne peut pas l'être.
pub fn delete(root: &Path, name: &str) -> Result<(), CommandError> {
    let all = files(root);
    if all.len() <= 1 {
        return Err(invalid("Un projet garde au moins un environnement."));
    }
    for file in all.iter().filter(|f| same_name(&f.environment.name, name)) {
        let _ = fs::remove_file(&file.path);
    }
    collection::update_meta(root, |meta| {
        meta.environment_order
            .retain(|entry| !same_name(entry, name));
        meta.protected.retain(|entry| !same_name(entry, name));
    })
}

/// Fixe l'ordre de la chaîne.
pub fn reorder(root: &Path, names: &[String]) -> Result<(), CommandError> {
    collection::update_meta(root, |meta| meta.environment_order = names.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Environment {
        Environment {
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
        // Protection et ordre sont à la racine, jamais dans le fichier (S4).
        let text = fs::read_to_string(file_for(dir.path(), "Pré-production")).unwrap();
        assert!(!text.contains("protected"), "{text}");
        assert!(text.contains("secret: true"), "{text}");
        let meta = collection::read_meta(dir.path());
        assert_eq!(meta.protected, ["Pré-production"]);
    }

    #[test]
    fn le_contenu_non_gere_est_garde() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        let path = file_for(dir.path(), "Local");
        fs::write(
            &path,
            "name: Local\ncolor: gray\ndescription: Poste du développeur\nvariables:\n  - name: baseUrl\n    value: ''\n    description: Adresse du service\n  - name: ancien\n    value: x\n    disabled: true\n  - name: port\n    value:\n      type: number\n      data: '8080'\n",
        )
        .unwrap();
        let mut local = list(dir.path()).remove(0);
        assert_eq!(local.variables.len(), 2, "variable désactivée non gérée");
        local.variables[0].value = "http://localhost".into();
        save(dir.path(), &local, Some("Local")).unwrap();
        let value = yaml::read(&path).unwrap();
        assert_eq!(value["description"], "Poste du développeur");
        assert_eq!(value["variables"][0]["description"], "Adresse du service");
        assert_eq!(value["variables"][1]["value"]["type"], "number");
        assert_eq!(value["variables"][2]["disabled"], true);
        assert!(schema::check(Part::Environment, &value).is_ok());
    }
}
