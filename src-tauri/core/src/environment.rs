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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "EnvVariable"))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "EnvironmentDoc"))]
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

/// Fichiers d'environnement présents, lisibles ou non.
fn env_paths(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir(root))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
        .collect()
}

/// Fichier où écrire l'environnement `name` (ES2) : `<slug>.yml`, ou `<slug>-2.yml`… quand ce
/// fichier appartient à un autre environnement, ou est illisible (R3). `own` est le fichier
/// actuel de l'environnement, qu'il réécrit.
fn free_file(root: &Path, name: &str, own: Option<&Path>) -> PathBuf {
    let base = slug(name);
    (1..)
        .map(|n| {
            if n == 1 {
                dir(root).join(format!("{base}.yml"))
            } else {
                dir(root).join(format!("{base}-{n}.yml"))
            }
        })
        .find(|path| own == Some(path.as_path()) || !path.exists())
        .unwrap_or_else(|| file_for(root, name))
}

fn files(root: &Path) -> Vec<EnvFile> {
    let protected = collection::read_meta(root).protected;
    env_paths(root)
        .into_iter()
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
/// Environnement lu depuis le texte d'un fichier (version venue de Git, V2 lot E).
pub fn parse_text(text: &str) -> Option<Environment> {
    let value = crate::opencollection::yaml::parse(text).ok()?;
    from_value(&value, false)
}

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

/// Environnement créé avec chaque projet (N1).
pub const LOCAL: &str = "Local";

/// Crée `environments/` avec « Local » et la variable `baseUrl` vide (N1), seulement s'il n'y a
/// aucun fichier d'environnement : un fichier illisible (conflit Git…) n'est jamais remplacé (ES1).
pub fn init(root: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    if env_paths(root).is_empty() {
        let local = Environment {
            name: LOCAL.to_owned(),
            color: default_color(),
            protected: false,
            variables: vec![Variable {
                name: "baseUrl".to_owned(),
                value: String::new(),
            }],
            locals: Vec::new(),
        };
        write_file(&file_for(root, LOCAL), &into_value(&local, None))?;
        collection::update_meta(root, |meta| meta.environment_order = vec![LOCAL.to_owned()])?;
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
    let target = free_file(root, &name, old.map(|f| f.path.as_path()));
    write_file(&target, &into_value(&environment, old.map(|f| &f.value)))?;
    if let Some(old) = old
        && old.path != target
    {
        let _ = fs::remove_file(&old.path);
        crate::watch::settle(&old.path);
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

/// Nouvel environnement (3.3-b, AM34) : nom, adresse de base, variables reprises d'un autre
/// environnement, place dans la chaîne et protection.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NewEnvironment {
    pub name: String,
    /// Adresse de base, qui devient la variable `baseUrl` ; vide : aucune.
    #[serde(default)]
    pub base_url: String,
    /// Environnement dont les variables partagées et les noms des variables locales sont
    /// repris ; jamais les valeurs locales ni les secrets, qui restent sur le poste.
    #[serde(default)]
    pub copy_from: Option<String>,
    /// Environnement après lequel il prend place dans la chaîne ; `None` : en tête.
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub protected: bool,
}

/// Crée l'environnement `new` (AM34) et le place dans la chaîne.
pub fn create(root: &Path, new: &NewEnvironment) -> Result<(), CommandError> {
    let all = list(root);
    let source = match new.copy_from.as_deref() {
        Some(from) => Some(
            all.iter()
                .find(|env| same_name(&env.name, from))
                .cloned()
                .ok_or_else(|| invalid(format!("L'environnement « {from} » n'existe plus.")))?,
        ),
        None => None,
    };
    let mut variables = source
        .as_ref()
        .map(|env| env.variables.clone())
        .unwrap_or_default();
    let base_url = new.base_url.trim();
    if !base_url.is_empty() {
        match variables.iter_mut().find(|v| v.name == BASE_URL) {
            Some(variable) => base_url.clone_into(&mut variable.value),
            None => variables.insert(
                0,
                Variable {
                    name: BASE_URL.to_owned(),
                    value: base_url.to_owned(),
                },
            ),
        }
    }
    let environment = Environment {
        name: new.name.trim().to_owned(),
        color: default_color(),
        protected: new.protected,
        variables,
        locals: source.map(|env| env.locals).unwrap_or_default(),
    };
    save(root, &environment, None)?;
    let name = item_name(&environment.name)?;
    let mut names: Vec<String> = list(root)
        .into_iter()
        .map(|env| env.name)
        .filter(|other| !same_name(other, &name))
        .collect();
    let at = new.after.as_deref().map_or(0, |after| {
        names
            .iter()
            .position(|other| same_name(other, after))
            .map_or(names.len(), |index| index + 1)
    });
    names.insert(at, name);
    reorder(root, &names)
}

/// Nom de la variable d'adresse de base d'un environnement.
const BASE_URL: &str = "baseUrl";

/// Ce que la suppression d'un environnement retire du projet, pour « Annuler » (ER1) : le texte
/// de ses fichiers, sa place dans la chaîne et sa protection (`extensions.arnod`).
#[derive(Debug, Clone)]
pub struct Removed {
    name: String,
    files: Vec<(PathBuf, String)>,
    position: Option<usize>,
    protected: bool,
}

impl Removed {
    /// Nom de l'environnement supprimé, tel qu'il était écrit.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Supprime un environnement ; le dernier ne peut pas l'être. Rend de quoi le remettre.
pub fn delete(root: &Path, name: &str) -> Result<Removed, CommandError> {
    let all = files(root);
    if all.len() <= 1 {
        return Err(invalid("Un projet garde au moins un environnement."));
    }
    let meta = collection::read_meta(root);
    let mut removed = Removed {
        name: all
            .iter()
            .find(|f| same_name(&f.environment.name, name))
            .map_or_else(|| name.to_owned(), |f| f.environment.name.clone()),
        files: Vec::new(),
        position: meta
            .environment_order
            .iter()
            .position(|entry| same_name(entry, name)),
        protected: meta.protected.iter().any(|entry| same_name(entry, name)),
    };
    for file in all.iter().filter(|f| same_name(&f.environment.name, name)) {
        if let Ok(text) = fs::read_to_string(&file.path) {
            removed.files.push((file.path.clone(), text));
        }
        let _ = fs::remove_file(&file.path);
        crate::watch::settle(&file.path);
    }
    collection::update_meta(root, |meta| {
        meta.environment_order
            .retain(|entry| !same_name(entry, name));
        meta.protected.retain(|entry| !same_name(entry, name));
    })?;
    Ok(removed)
}

/// Remet un environnement supprimé (« Annuler », ER1) : ses fichiers tels qu'ils étaient (sous un
/// autre nom de fichier si le sien a été repris), sa place dans la chaîne et sa protection. Un
/// environnement du même nom créé depuis n'est jamais écrasé.
pub fn restore(root: &Path, removed: &Removed) -> Result<(), CommandError> {
    if let Some(taken) = list(root)
        .into_iter()
        .find(|env| same_name(&env.name, &removed.name))
    {
        return Err(invalid(format!(
            "Un environnement « {} » existe de nouveau dans ce projet : la suppression ne peut plus être annulée.",
            taken.name
        )));
    }
    fs::create_dir_all(dir(root))
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir(root), &error))?;
    for (path, text) in &removed.files {
        let target = if path.exists() {
            free_file(root, &removed.name, None)
        } else {
            path.clone()
        };
        crate::contract::file::write_text(&target, text)?;
    }
    collection::update_meta(root, |meta| {
        if let Some(position) = removed.position
            && !meta
                .environment_order
                .iter()
                .any(|entry| same_name(entry, &removed.name))
        {
            let at = position.min(meta.environment_order.len());
            meta.environment_order.insert(at, removed.name.clone());
        }
        if removed.protected && !meta.protected.iter().any(|p| same_name(p, &removed.name)) {
            meta.protected.push(removed.name.clone());
        }
    })
}

/// Fixe l'ordre de la chaîne.
pub fn reorder(root: &Path, names: &[String]) -> Result<(), CommandError> {
    collection::update_meta(root, |meta| meta.environment_order = names.to_vec())
}

/// « Copier depuis un autre projet… » (3.3-a, AM43) : les environnements `names` du projet
/// `from` sont ajoutés en fin de chaîne de `root`, avec leur couleur, leur protection, leurs
/// variables partagées et les noms de leurs variables locales ; jamais les valeurs locales ni
/// les secrets, qui restent sur le poste. Un nom déjà pris reçoit un numéro (« Recette 2 »).
/// Rend les noms créés.
pub fn copy_from(root: &Path, from: &Path, names: &[String]) -> Result<Vec<String>, CommandError> {
    let source = list(from);
    let mut created = Vec::new();
    for wanted in names {
        let environment = source
            .iter()
            .find(|env| same_name(&env.name, wanted))
            .ok_or_else(|| invalid(format!("L'environnement « {wanted} » n'existe plus.")))?;
        let taken = list(root);
        let free = |candidate: &str| !taken.iter().any(|env| same_name(&env.name, candidate));
        let mut name = environment.name.clone();
        let mut n = 2;
        while !free(&name) {
            name = format!("{} {n}", environment.name);
            n += 1;
        }
        let copy = Environment {
            name: name.clone(),
            ..environment.clone()
        };
        save(root, &copy, None)?;
        created.push(name);
    }
    Ok(created)
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

    /// AM43 (3.3-a) : la copie depuis un autre projet reprend variables partagées, noms des
    /// locales, couleur et protection, en fin de chaîne ; un nom pris reçoit un numéro.
    #[test]
    fn am43_copie_depuis_un_autre_projet() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        init(from.path()).unwrap();
        init(to.path()).unwrap();
        let mut recette = env("Recette");
        recette.protected = true;
        save(from.path(), &recette, None).unwrap();
        let created = copy_from(
            to.path(),
            from.path(),
            &["Recette".to_owned(), "local".to_owned()],
        )
        .unwrap();
        assert_eq!(created, ["Recette", "Local 2"]);
        let all = list(to.path());
        let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Local", "Recette", "Local 2"]);
        let copied = &all[1];
        assert_eq!((copied.color.as_str(), copied.protected), ("blue", true));
        assert_eq!(copied.variables[0].value, "https://x");
        assert_eq!(copied.locals, ["token"]);
        assert_eq!(
            copy_from(to.path(), from.path(), &["Absent".to_owned()])
                .unwrap_err()
                .kind,
            CommandErrorKind::InvalidRequest
        );
    }

    /// ER1 : « Annuler » remet l'environnement supprimé, à sa place dans la chaîne et protégé ;
    /// un environnement du même nom créé depuis n'est jamais écrasé.
    #[test]
    fn er1_environnement_supprime_remis_a_sa_place() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        save(dir.path(), &env("Recette"), None).unwrap();
        let mut prod = env("Production");
        prod.protected = true;
        save(dir.path(), &prod, None).unwrap();
        let names = |root: &Path| list(root).into_iter().map(|e| e.name).collect::<Vec<_>>();
        reorder(
            dir.path(),
            &["Local".into(), "Production".into(), "Recette".into()],
        )
        .unwrap();
        let before = fs::read_to_string(file_for(dir.path(), "Production")).unwrap();

        let removed = delete(dir.path(), "production").unwrap();
        assert_eq!(names(dir.path()), ["Local", "Recette"]);
        restore(dir.path(), &removed).unwrap();
        assert_eq!(names(dir.path()), ["Local", "Production", "Recette"]);
        assert!(list(dir.path())[1].protected);
        assert_eq!(
            fs::read_to_string(file_for(dir.path(), "Production")).unwrap(),
            before
        );

        let removed = delete(dir.path(), "Production").unwrap();
        save(dir.path(), &env("Production"), None).unwrap();
        assert_eq!(
            restore(dir.path(), &removed).unwrap_err().kind,
            CommandErrorKind::InvalidRequest
        );
        assert!(!list(dir.path()).iter().any(|e| e.protected));
    }

    /// AM34 : un nouvel environnement reprend les variables partagées et les noms des variables
    /// locales d'un autre, reçoit son adresse de base et prend sa place dans la chaîne.
    #[test]
    fn am34_creation_reprend_les_variables_et_prend_sa_place() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        save(dir.path(), &env("Recette"), None).unwrap();
        save(dir.path(), &env("Production"), None).unwrap();
        create(
            dir.path(),
            &NewEnvironment {
                name: "Recette client".into(),
                base_url: "https://client.test".into(),
                copy_from: Some("recette".into()),
                after: Some("Recette".into()),
                protected: true,
            },
        )
        .unwrap();
        let all = list(dir.path());
        let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Local", "Recette", "Recette client", "Production"]);
        let created = &all[2];
        assert!(created.protected);
        assert_eq!(created.locals, ["token"]);
        assert_eq!(
            created.variables,
            [Variable {
                name: "baseUrl".into(),
                value: "https://client.test".into()
            }]
        );
        // Sans position : en tête ; un nom pris est refusé, rien n'est écrit.
        create(
            dir.path(),
            &NewEnvironment {
                name: "Démo".into(),
                base_url: String::new(),
                copy_from: None,
                after: None,
                protected: false,
            },
        )
        .unwrap();
        assert_eq!(list(dir.path())[0].name, "Démo");
        let taken = create(
            dir.path(),
            &NewEnvironment {
                name: "production".into(),
                base_url: String::new(),
                copy_from: None,
                after: None,
                protected: false,
            },
        );
        assert_eq!(taken.unwrap_err().kind, CommandErrorKind::NameTaken);
        assert_eq!(list(dir.path()).len(), 5);
    }

    /// ES1 : un environnement illisible (marqueurs de fusion Git) n'est jamais remplacé par un
    /// « Local » vide.
    #[test]
    fn es1_environnement_illisible_jamais_reecrit() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(ENVIRONMENTS_DIR)).unwrap();
        let file = dir.path().join(ENVIRONMENTS_DIR).join("local.yml");
        let edited = "<<<<<<< HEAD\nname: Local\n=======\nname: Local\n>>>>>>> main\n";
        fs::write(&file, edited).unwrap();
        init(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), edited);
        // Un nouvel environnement « Local » ne l'écrase pas non plus.
        save(dir.path(), &env("Local"), None).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), edited);
        assert_eq!(list(dir.path()).len(), 1);
    }

    /// ES2 : deux environnements de noms différents mais de même nom de fichier coexistent.
    #[test]
    fn es2_environnement_de_meme_fichier_n_ecrase_pas_l_autre() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        save(dir.path(), &env("Recette 1"), None).unwrap();
        save(dir.path(), &env("Recette-1"), None).unwrap();
        let names: Vec<String> = list(dir.path()).into_iter().map(|e| e.name).collect();
        assert!(names.contains(&"Recette 1".to_owned()), "{names:?}");
        assert!(names.contains(&"Recette-1".to_owned()), "{names:?}");
        // Renommer l'un garde l'autre ; réenregistrer réécrit son propre fichier.
        save(dir.path(), &env("Recette 1"), Some("Recette 1")).unwrap();
        save(dir.path(), &env("Recette un"), Some("Recette-1")).unwrap();
        let names: Vec<String> = list(dir.path()).into_iter().map(|e| e.name).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names.contains(&"Recette 1".to_owned()));
        assert!(names.contains(&"Recette un".to_owned()));
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
