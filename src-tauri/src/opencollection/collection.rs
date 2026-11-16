//! Racine d'une collection : `opencollection.yml` et données propres à Arnod
//! (S1, S4, S5, R5).
//!
//! Ce qui n'a pas de place dans le schéma va dans `extensions.arnod` de la
//! racine, seul point d'extension prévu par OpenCollection (Q1) :
//!
//! ```yaml
//! extensions:
//!   arnod:
//!     version: 1
//!     id: 6c1f4e0a-…
//!     environments:
//!       order: [Local, Recette]
//!       protected: [Recette]
//! ```

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{ROOT_FILE, SPEC_VERSION, yaml};

/// Version des données `extensions.arnod`.
pub const ARNOD_VERSION: u64 = 1;

/// Données propres à Arnod, rangées dans `extensions.arnod`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArnodMeta {
    /// Identifiant du projet (P12).
    pub id: Option<Uuid>,
    /// Ordre de la chaîne des environnements.
    pub environment_order: Vec<String>,
    /// Environnements protégés (R8 du premier usage).
    pub protected: Vec<String>,
}

/// Chemin de `opencollection.yml`.
pub fn root_file(folder: &Path) -> PathBuf {
    folder.join(ROOT_FILE)
}

/// Le dossier est une collection OpenCollection (R5).
pub fn exists(folder: &Path) -> bool {
    root_file(folder).is_file()
}

/// Racine d'une nouvelle collection.
pub fn new_root(name: &str, id: Uuid) -> Value {
    let mut root = json!({
        "opencollection": SPEC_VERSION,
        "info": { "name": name },
    });
    set_meta(
        &mut root,
        &ArnodMeta {
            id: Some(id),
            ..ArnodMeta::default()
        },
    );
    root
}

/// Version majeure de la spécification d'une racine.
pub fn spec_major(root: &Value) -> Option<u64> {
    root.get("opencollection")
        .map(|v| match v {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .and_then(|text| text.split('.').next().and_then(|m| m.parse().ok()))
}

/// Nom de la collection.
pub fn name(root: &Value) -> Option<String> {
    root.pointer("/info/name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Données Arnod d'une racine.
pub fn meta(root: &Value) -> ArnodMeta {
    let arnod = root.pointer("/extensions/arnod");
    ArnodMeta {
        id: arnod
            .and_then(|a| a.get("id"))
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok()),
        environment_order: strings(arnod.and_then(|a| a.pointer("/environments/order"))),
        protected: strings(arnod.and_then(|a| a.pointer("/environments/protected"))),
    }
}

/// Écrit les données Arnod dans une racine ; le reste de `extensions` est gardé.
pub fn set_meta(root: &mut Value, meta: &ArnodMeta) {
    let Some(map) = root.as_object_mut() else {
        return;
    };
    let extensions = map
        .entry("extensions")
        .or_insert_with(|| Value::Object(Map::new()));
    if !extensions.is_object() {
        *extensions = Value::Object(Map::new());
    }
    let Some(extensions) = extensions.as_object_mut() else {
        return;
    };
    let mut arnod = Map::new();
    arnod.insert("version".to_owned(), Value::from(ARNOD_VERSION));
    if let Some(id) = meta.id {
        arnod.insert("id".to_owned(), Value::from(id.to_string()));
    }
    let mut environments = Map::new();
    if !meta.environment_order.is_empty() {
        environments.insert("order".to_owned(), json!(meta.environment_order));
    }
    if !meta.protected.is_empty() {
        environments.insert("protected".to_owned(), json!(meta.protected));
    }
    if !environments.is_empty() {
        arnod.insert("environments".to_owned(), Value::Object(environments));
    }
    extensions.insert("arnod".to_owned(), Value::Object(arnod));
}

/// Racine lue (R5) : absente, illisible ou non conforme → `NotAProject`.
pub fn read(folder: &Path) -> Result<Value, CommandError> {
    let path = root_file(folder);
    if !path.is_file() {
        return Err(CommandError::new(
            CommandErrorKind::NotAProject,
            format!(
                "Le dossier {} ne contient pas de projet ({ROOT_FILE} absent).",
                folder.display()
            ),
        ));
    }
    let root = yaml::read(&path)
        .map_err(|error| CommandError::new(CommandErrorKind::NotAProject, error.message))?;
    let Some(major) = spec_major(&root) else {
        return Err(CommandError::new(
            CommandErrorKind::NotAProject,
            format!(
                "{} n'indique pas de version d'OpenCollection.",
                path.display()
            ),
        ));
    };
    if major > 1 {
        return Err(CommandError::new(
            CommandErrorKind::NewerVersion,
            format!(
                "Le projet {} utilise une version plus récente d'OpenCollection.",
                folder.display()
            ),
        ));
    }
    schema::check(Part::Root, &root).map_err(|reason| {
        CommandError::new(
            CommandErrorKind::NotAProject,
            format!("{} : {reason}", path.display()),
        )
    })?;
    if root.get("bundled").and_then(Value::as_bool) == Some(true) {
        return Err(CommandError::new(
            CommandErrorKind::NotAProject,
            format!(
                "La collection {} est regroupée en un seul fichier : Arnod ne la prend pas en charge.",
                folder.display()
            ),
        ));
    }
    Ok(root)
}

/// Écrit une racine, valide pour le schéma (R1).
pub fn write(folder: &Path, root: &Value) -> Result<(), CommandError> {
    schema::check(Part::Root, root)
        .map_err(|reason| CommandError::new(CommandErrorKind::Io, reason))?;
    yaml::write(&root_file(folder), root)
}

/// Données Arnod d'un projet ; une racine absente ou illisible n'en a pas.
pub fn read_meta(folder: &Path) -> ArnodMeta {
    read(folder).map(|root| meta(&root)).unwrap_or_default()
}

/// Modifie les données Arnod d'un projet ; une racine absente est créée, au
/// nom du dossier.
pub fn update_meta(folder: &Path, edit: impl FnOnce(&mut ArnodMeta)) -> Result<(), CommandError> {
    let mut root = match read(folder) {
        Ok(root) => root,
        Err(error) if error.kind == CommandErrorKind::NotAProject && !exists(folder) => {
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Projet".to_owned());
            let mut root = json!({ "opencollection": SPEC_VERSION, "info": { "name": name } });
            set_meta(&mut root, &ArnodMeta::default());
            root
        }
        Err(error) => return Err(error),
    };
    let mut current = meta(&root);
    edit(&mut current);
    set_meta(&mut root, &current);
    write(folder, &root)
}
