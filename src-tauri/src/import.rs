//! Import (docs/plan-mvp-c-import-bruno.md) : reconnaissance du format d'un
//! dossier ou d'un fichier, écriture d'une collection Bruno convertie dans un
//! nouveau projet ou dans un dossier d'un projet, rapport d'import.
//!
//! Les fichiers d'origine ne sont jamais modifiés (R1). Le rapport est gardé
//! sur le poste, dans `projects/<id>/import.json`, jamais dans le projet.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::bruno::convert::{self, Converted, SecretValue};
use crate::bruno::read as bruno;
use crate::bruno::{ConversionExample, ImportCounts, ItemKind, ItemStatus, ReportItem};
use crate::catalog::names::same_name;
use crate::catalog::snapshot::CatalogSnapshot;
use crate::contract;
use crate::environment::{self, Environment, Variable};
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::migrate::LEGACY_MANIFEST;
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{FOLDER_FILE, collection, yaml};
use crate::state::AppState;
use crate::tree::undo::Undo;
use crate::tree::{ENVIRONMENTS_DIR, item_name, resolve, slug};
use crate::workbench::ProjectCtx;

/// Fichier du dernier rapport d'import d'un projet, dans ses données locales.
const REPORT_FILE: &str = "import.json";

/// Format reconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename = "ImportFormat"))]
#[serde(rename_all = "camelCase")]
pub enum SourceFormat {
    /// Collection Bruno en fichiers `.bru`.
    Bruno,
    /// Projet Arnod ou collection OpenCollection (Bruno 3) : ajouté tel quel.
    OpenCollection,
    /// Contrat OpenAPI ou Swagger : rangé dans `contracts/`.
    OpenApi,
}

/// Ce que l'import a reconnu, avant toute écriture (R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    /// Dossier (ou fichier, pour un contrat) à importer.
    pub path: String,
    pub format: SourceFormat,
    /// Nom de la collection, du projet ou titre du contrat.
    pub name: String,
    /// Ce que la conversion reprendra (Bruno) ; `requests` compte les
    /// opérations d'un contrat.
    pub counts: ImportCounts,
    /// Fichiers `.bru` lus.
    pub files: usize,
    /// Environnements de la collection (Bruno), dans son ordre (2.3-b, AM41).
    pub environments: Vec<SourceEnvironment>,
    /// Variables partagées dont le nom ressemble à un secret (jeton, mot de passe, clé
    /// d'API…) : avec `secretsLocal`, elles deviennent des variables locales, valeur au coffre.
    pub secrets: Vec<String>,
}

/// Environnement d'une collection, tel que l'import le reprendra (2.3-b, AM41).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SourceEnvironment {
    pub name: String,
    /// Noms de ses variables actives, partagées et locales.
    pub variables: Vec<String>,
}

/// Choix de la fenêtre « Importer » (2.3-b, AM41).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    /// Environnement du projet qui reçoit chaque environnement de la collection ; un
    /// environnement sans choix garde son nom (complété s'il existe, sinon créé).
    pub environments: Vec<EnvironmentChoice>,
    /// Valeurs qui ressemblent à des secrets en variables locales, au coffre du système
    /// (oui par défaut, Q5 du plan).
    pub secrets_local: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            environments: Vec::new(),
            secrets_local: true,
        }
    }
}

/// Un environnement de la collection (`from`) et celui du projet qui le reçoit (`to`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentChoice {
    pub from: String,
    pub to: String,
}

/// Où va l'import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportTarget {
    /// Nouveau projet `<location>/<nom>`, dans `workspace_id` s'il est donné.
    NewProject {
        location: Option<String>,
        workspace_id: Option<Uuid>,
    },
    /// Un nouveau dossier (ou contrat) dans un projet existant.
    Project { project_id: Uuid },
}

/// Rapport d'import (2.3-f).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub collection: String,
    pub format: SourceFormat,
    /// Dossier d'origine.
    pub source: String,
    /// Fichiers `.bru` lus.
    pub source_files: usize,
    pub project_id: Uuid,
    pub project_name: String,
    /// Dossier créé dans un projet existant ; `None` pour un nouveau projet.
    pub folder: Option<String>,
    /// Date de l'import, en millisecondes depuis 1970.
    pub imported_at: u64,
    pub counts: ImportCounts,
    pub items: Vec<ReportItem>,
    pub examples: Vec<ConversionExample>,
}

/// Réponse de `import_run`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportDone {
    pub project_id: Uuid,
    /// Nouvel instantané si le catalogue a changé (projet créé ou ajouté).
    pub snapshot: Option<CatalogSnapshot>,
    /// Rapport d'une collection Bruno.
    pub report: Option<ImportReport>,
    /// Contrat ajouté (OpenAPI), relatif au projet.
    pub contract: Option<String>,
}

fn unknown(path: &Path) -> CommandError {
    CommandError::new(
        CommandErrorKind::NotAProject,
        format!(
            "Arnod ne reconnaît pas {} : choisissez une collection Bruno (dossier avec bruno.json), un projet Arnod ou OpenCollection, ou un contrat OpenAPI.",
            path.display()
        ),
    )
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Reconnaît le format d'un dossier ou d'un fichier, sans rien écrire (R2).
pub fn detect(path: &Path) -> Result<ImportSource, CommandError> {
    if !path.is_absolute() {
        return Err(CommandError::new(
            CommandErrorKind::Io,
            format!("Le chemin doit être absolu : {}.", path.display()),
        ));
    }
    if path.is_file() {
        let file_name = dir_name(path);
        let parent = path.parent().unwrap_or(path);
        if file_name == bruno::MANIFEST || file_name.ends_with(".bru") {
            let folder = if file_name.ends_with(".bru") && !bruno::is_collection(parent) {
                // Requête d'un sous-dossier : la collection est plus haut.
                parent
                    .ancestors()
                    .find(|dir| bruno::is_collection(dir))
                    .ok_or_else(|| unknown(path))?
            } else {
                parent
            };
            return detect(folder);
        }
        if file_name == crate::opencollection::ROOT_FILE || file_name == LEGACY_MANIFEST {
            return detect(parent);
        }
        let text = fs::read_to_string(path)
            .map_err(|error| CommandError::io("Lecture du fichier impossible", path, &error))?;
        let contract = contract::parse::load(&text).map_err(|_| unknown(path))?;
        return Ok(ImportSource {
            path: path.display().to_string(),
            format: SourceFormat::OpenApi,
            name: contract.title,
            counts: ImportCounts {
                requests: contract.operations.len(),
                ..ImportCounts::default()
            },
            files: 1,
            environments: Vec::new(),
            secrets: Vec::new(),
        });
    }
    if !path.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            format!("{} est introuvable.", path.display()),
        ));
    }
    if collection::exists(path) || path.join(LEGACY_MANIFEST).is_file() {
        let name = collection::read(path)
            .ok()
            .and_then(|root| collection::name(&root))
            .unwrap_or_else(|| dir_name(path));
        return Ok(ImportSource {
            path: path.display().to_string(),
            format: SourceFormat::OpenCollection,
            name,
            counts: ImportCounts::default(),
            files: 0,
            environments: Vec::new(),
            secrets: Vec::new(),
        });
    }
    if bruno::is_collection(path) {
        let read = bruno::read(path)?;
        let converted = convert::convert(&read);
        return Ok(ImportSource {
            path: path.display().to_string(),
            format: SourceFormat::Bruno,
            name: read.name,
            counts: converted.counts,
            files: read.bru_files,
            environments: converted
                .environments
                .iter()
                .map(|env| SourceEnvironment {
                    name: env_name(env),
                    variables: active_variables(env)
                        .filter_map(|v| v.get("name").and_then(Value::as_str))
                        .map(str::to_owned)
                        .collect(),
                })
                .collect(),
            secrets: secret_names(&converted),
        });
    }
    Err(unknown(path))
}

/// Nom d'un environnement converti (`$defs/Environment`).
fn env_name(env: &Value) -> String {
    env.get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Variables actives d'un environnement converti.
fn active_variables(env: &Value) -> impl Iterator<Item = &Value> {
    env.get("variables")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|v| v.get("disabled").and_then(Value::as_bool) != Some(true))
}

/// Un nom de variable qui ressemble à un secret : jeton, mot de passe, clé d'API, secret
/// client… sauf ce qui le décrit sans le contenir (`tokenUrl`, `passwordPolicy`…).
pub fn looks_secret(name: &str) -> bool {
    const CONTAINS: [&str; 9] = [
        "token",
        "password",
        "passwd",
        "secret",
        "apikey",
        "privatekey",
        "accesskey",
        "credential",
        "bearer",
    ];
    const EXACT: [&str; 6] = ["pwd", "pass", "key", "pin", "jwt", "auth"];
    const DESCRIBES: [&str; 10] = [
        "url", "uri", "endpoint", "host", "path", "type", "name", "header", "policy", "expiry",
    ];
    let key: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if DESCRIBES.iter().any(|suffix| key.ends_with(suffix)) {
        return false;
    }
    EXACT.contains(&key.as_str()) || CONTAINS.iter().any(|part| key.contains(part))
}

/// Valeur d'une variable partagée qui ressemble à un secret ; aucune pour une variable déjà
/// locale, vide ou qui n'est qu'une référence `{{…}}`.
fn secret_value(variable: &Value) -> Option<String> {
    if variable.get("secret").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let name = variable.get("name").and_then(Value::as_str)?;
    let value = variable.get("value").and_then(Value::as_str)?.trim();
    let reference = value.starts_with("{{") && value.ends_with("}}");
    (looks_secret(name) && !value.is_empty() && !reference).then(|| value.to_owned())
}

/// Noms des variables partagées qui ressemblent à des secrets, sans doublon.
fn secret_names(converted: &Converted) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for env in &converted.environments {
        for variable in active_variables(env) {
            if secret_value(variable).is_none() {
                continue;
            }
            let name = variable
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
    }
    names
}

/// Applique les choix de la fenêtre « Importer » à la collection convertie (2.3-b, AM41) :
/// avec `secrets_local`, une valeur partagée qui ressemble à un secret devient une variable
/// locale dont la valeur va au coffre, jamais dans le projet ; puis chaque environnement prend
/// le nom de sa cible (deux environnements vers la même cible n'en font qu'un, le premier
/// gardant ses valeurs).
pub fn apply_options(converted: &mut Converted, options: &ImportOptions) {
    if options.secrets_local {
        localize_secrets(converted);
    }
    rename_environments(converted, &options.environments);
}

fn localize_secrets(converted: &mut Converted) {
    for env in &mut converted.environments {
        let place = env_name(env);
        let Some(variables) = env.get_mut("variables").and_then(Value::as_array_mut) else {
            continue;
        };
        for variable in variables
            .iter_mut()
            .filter(|v| v.get("disabled").and_then(Value::as_bool) != Some(true))
        {
            let Some(value) = secret_value(variable) else {
                continue;
            };
            let name = variable
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let mut local = Map::new();
            local.insert("name".to_owned(), Value::from(name.clone()));
            local.insert("secret".to_owned(), Value::Bool(true));
            *variable = Value::Object(local);
            converted.counts.secrets += 1;
            converted.secrets.push(SecretValue {
                environment: place.clone(),
                name: name.clone(),
                value,
            });
            converted.items.push(ReportItem {
                kind: ItemKind::Env,
                status: ItemStatus::Stored,
                name,
                place: place.clone(),
                file: format!("environments/{place}.bru"),
                block: Some("vars".to_owned()),
                request: None,
                detail: "Sa valeur ressemble à un secret : variable locale, jamais dans le projet."
                    .to_owned(),
                code: None,
            });
        }
    }
}

fn rename_environments(converted: &mut Converted, choices: &[EnvironmentChoice]) {
    let target = |name: &str| -> String {
        choices
            .iter()
            .find(|choice| choice.from == name)
            .map(|choice| choice.to.trim())
            .filter(|to| !to.is_empty())
            .unwrap_or(name)
            .to_owned()
    };
    // Une seule passe, sur les noms d'origine : un échange de noms reste juste.
    for secret in &mut converted.secrets {
        secret.environment = target(&secret.environment);
    }
    for item in converted
        .items
        .iter_mut()
        .filter(|item| item.kind == ItemKind::Env)
    {
        item.place = target(&item.place);
    }
    let mut merged: Vec<Value> = Vec::new();
    for mut env in std::mem::take(&mut converted.environments) {
        let to = target(&env_name(&env));
        if let Some(map) = env.as_object_mut() {
            map.insert("name".to_owned(), Value::from(to.clone()));
        }
        let Some(first) = merged.iter_mut().find(|e| same_name(&env_name(e), &to)) else {
            merged.push(env);
            continue;
        };
        let incoming: Vec<Value> = env
            .get("variables")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(map) = first.as_object_mut() else {
            continue;
        };
        let list = map
            .entry("variables")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(list) = list.as_array_mut() {
            for variable in incoming {
                let name = variable.get("name").cloned();
                if !list.iter().any(|v| v.get("name") == name.as_ref()) {
                    list.push(variable);
                }
            }
        }
    }
    converted.environments = merged;
}

/// Écrit un fichier OpenCollection, validé (R3 du format).
fn write(path: &Path, part: Part, value: &Value) -> Result<(), CommandError> {
    schema::check(part, value).map_err(|reason| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("{} : {reason}", path.display()),
        )
    })?;
    yaml::write(path, value)
}

fn create_dir(path: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(path)
        .map_err(|error| CommandError::io("Création du dossier impossible", path, &error))
}

/// Dossiers, requêtes et `folder.yml` convertis, sous `base`.
fn write_tree(base: &Path, converted: &Converted) -> Result<(), CommandError> {
    create_dir(base)?;
    for folder in &converted.folders {
        create_dir(&base.join(folder))?;
    }
    for file in &converted.files {
        write(&base.join(&file.path), file.part, &file.value)?;
    }
    Ok(())
}

/// Environnements d'un nouveau projet, dans l'ordre de la collection ;
/// « Local » seul s'il n'y en a aucun.
fn write_environments(root: &Path, converted: &Converted) -> Result<(), CommandError> {
    if converted.environments.is_empty() {
        return environment::init(root);
    }
    let dir = root.join(ENVIRONMENTS_DIR);
    create_dir(&dir)?;
    let mut order = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for value in &converted.environments {
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Environnement")
            .to_owned();
        let mut stem = slug(&name);
        let mut n = 2;
        while taken.contains(&stem) {
            stem = format!("{}-{n}", slug(&name));
            n += 1;
        }
        taken.push(stem.clone());
        write(&dir.join(format!("{stem}.yml")), Part::Environment, value)?;
        order.push(name);
    }
    collection::update_meta(root, |meta| meta.environment_order = order)
}

/// Variables partagées et locales d'un environnement converti.
fn environment_of(value: &Value) -> Environment {
    let mut env = Environment {
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        color: "gray".to_owned(),
        protected: false,
        variables: Vec::new(),
        locals: Vec::new(),
    };
    for item in value
        .get("variables")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("disabled").and_then(Value::as_bool) != Some(true))
    {
        let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
        if item.get("secret").and_then(Value::as_bool) == Some(true) {
            env.locals.push(name.to_owned());
        } else {
            env.variables.push(Variable {
                name: name.to_owned(),
                value: item
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
    }
    env
}

/// Environnements ajoutés à un projet existant : un environnement de même nom
/// est complété (les variables déjà présentes gardent leur valeur), sinon créé.
fn merge_environments(root: &Path, converted: &Converted) -> Result<(), CommandError> {
    for value in &converted.environments {
        let incoming = environment_of(value);
        let existing = environment::list(root)
            .into_iter()
            .find(|env| same_name(&env.name, &incoming.name));
        match existing {
            Some(mut env) => {
                let known = |env: &Environment, name: &str| {
                    env.variables.iter().any(|v| v.name == name)
                        || env.locals.iter().any(|l| l == name)
                };
                for variable in incoming.variables {
                    if !known(&env, &variable.name) {
                        env.variables.push(variable);
                    }
                }
                for local in incoming.locals {
                    if !known(&env, &local) {
                        env.locals.push(local);
                    }
                }
                let previous = env.name.clone();
                environment::save(root, &env, Some(&previous))?;
            }
            None => environment::save(root, &incoming, None)?,
        }
    }
    Ok(())
}

/// Nom de dossier libre à la racine d'un projet pour la collection.
fn free_folder(root: &Path, name: &str) -> Result<String, CommandError> {
    let base = item_name(name).unwrap_or_else(|_| slug(name));
    let taken = |candidate: &str| {
        same_name(candidate, ENVIRONMENTS_DIR)
            || same_name(candidate, contract::CONTRACTS_DIR)
            || fs::read_dir(root).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|e| same_name(&e.file_name().to_string_lossy(), candidate))
            })
    };
    let mut candidate = base.clone();
    let mut n = 2;
    while taken(&candidate) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    Ok(candidate)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Range les valeurs connues des variables locales (`.env`) dans le coffre ;
/// une valeur non rangée reste « à renseigner ».
fn store_secrets(
    report: &mut ImportReport,
    secrets: &[SecretValue],
    store: &dyn Fn(&SecretValue) -> Result<(), CommandError>,
) {
    for secret in secrets {
        // Le message du système (souvent en anglais) n'est pas montré : seule la suite compte.
        if store(secret).is_err() {
            for item in report.items.iter_mut().filter(|item| {
                item.status == ItemStatus::Stored
                    && item.name == secret.name
                    && item.place == secret.environment
            }) {
                item.status = ItemStatus::ToFill;
                item.detail = "Le coffre du système n'a pas gardé sa valeur : renseignez-la dans les environnements du projet.".to_owned();
            }
        }
    }
}

fn report_of(
    converted: &Converted,
    source: &Path,
    project_id: Uuid,
    project_name: &str,
    folder: Option<&str>,
) -> ImportReport {
    let prefix = |path: &str| match folder {
        Some(folder) => format!("{folder}/{path}"),
        None => path.to_owned(),
    };
    ImportReport {
        collection: converted.name.clone(),
        format: SourceFormat::Bruno,
        source: source.display().to_string(),
        source_files: converted.bru_files,
        project_id,
        project_name: project_name.to_owned(),
        folder: folder.map(str::to_owned),
        imported_at: now_ms(),
        counts: converted.counts,
        items: converted
            .items
            .iter()
            .cloned()
            .map(|mut item| {
                item.request = item.request.as_deref().map(prefix);
                item
            })
            .collect(),
        examples: converted.examples.clone(),
    }
}

/// Importe `source` vers `target`. `store_secret` range une valeur connue dans
/// le coffre (branché sur l'établi par la commande).
pub fn run(
    state: &AppState,
    source: &Path,
    target: &ImportTarget,
    options: &ImportOptions,
    store_secret: &dyn Fn(&ProjectCtx, &SecretValue) -> Result<(), CommandError>,
) -> Result<ImportDone, CommandError> {
    let detected = detect(source)?;
    let source = PathBuf::from(&detected.path);
    match detected.format {
        SourceFormat::OpenCollection => {
            let workspace = match target {
                ImportTarget::NewProject { workspace_id, .. } => *workspace_id,
                ImportTarget::Project { .. } => None,
            };
            let created = state.add_existing_project(&detected.path, workspace)?;
            Ok(ImportDone {
                project_id: created.project_id,
                snapshot: Some(created.snapshot),
                report: None,
                contract: None,
            })
        }
        SourceFormat::OpenApi => {
            let text = fs::read_to_string(&source).map_err(|error| {
                CommandError::io("Lecture du fichier impossible", &source, &error)
            })?;
            match target {
                ImportTarget::NewProject {
                    location,
                    workspace_id,
                } => {
                    let mut added = None;
                    let created = state.create_project_with(
                        &detected.name,
                        location.as_deref(),
                        *workspace_id,
                        |root| {
                            environment::init(root)?;
                            added = Some(contract::file::add(root, &text)?);
                            Ok(())
                        },
                    )?;
                    Ok(ImportDone {
                        project_id: created.project_id,
                        snapshot: Some(created.snapshot),
                        report: None,
                        contract: added,
                    })
                }
                ImportTarget::Project { project_id } => {
                    let ctx = state.project_ctx(*project_id)?;
                    let path = contract::file::add(&ctx.root, &text)?;
                    Ok(ImportDone {
                        project_id: *project_id,
                        snapshot: None,
                        report: None,
                        contract: Some(path),
                    })
                }
            }
        }
        SourceFormat::Bruno => {
            let read = bruno::read(&source)?;
            let mut converted = convert::convert(&read);
            apply_options(&mut converted, options);
            let (project_id, snapshot, folder) = match target {
                ImportTarget::NewProject {
                    location,
                    workspace_id,
                } => {
                    let created = state.create_project_with(
                        &converted.name,
                        location.as_deref(),
                        *workspace_id,
                        |root| write_new_project(root, &converted),
                    )?;
                    (created.project_id, Some(created.snapshot), None)
                }
                ImportTarget::Project { project_id } => {
                    let ctx = state.project_ctx(*project_id)?;
                    let folder = free_folder(&ctx.root, &converted.name)?;
                    write_into_project(&ctx.root, &folder, &converted)?;
                    (*project_id, None, Some(folder))
                }
            };
            let ctx = state.project_ctx(project_id)?;
            let project_name = state
                .snapshot()?
                .projects
                .into_iter()
                .find(|p| p.id == project_id)
                .map(|p| p.name)
                .unwrap_or_else(|| converted.name.clone());
            let mut report = report_of(
                &converted,
                &source,
                project_id,
                &project_name,
                folder.as_deref(),
            );
            store_secrets(&mut report, &converted.secrets, &|secret| {
                store_secret(&ctx, secret)
            });
            save_report(state.data_dir(), &report)?;
            Ok(ImportDone {
                project_id,
                snapshot,
                report: Some(report),
                contract: None,
            })
        }
    }
}

/// Nouveau projet : réglages de la collection à la racine, puis dossiers,
/// requêtes et environnements.
pub fn write_new_project(root: &Path, converted: &Converted) -> Result<(), CommandError> {
    let mut value = collection::read(root)?;
    if let Some(map) = value.as_object_mut() {
        if !converted.defaults.is_empty() {
            map.insert(
                "request".to_owned(),
                Value::Object(converted.defaults.clone()),
            );
        }
        if let Some(docs) = &converted.docs {
            map.insert("docs".to_owned(), Value::from(docs.clone()));
        }
    }
    collection::write(root, &value)?;
    write_tree(root, converted)?;
    write_environments(root, converted)
}

/// Projet existant : un dossier `folder` à la racine, dont `folder.yml` porte
/// les réglages de la collection ; environnements complétés.
pub fn write_into_project(
    root: &Path,
    folder: &str,
    converted: &Converted,
) -> Result<(), CommandError> {
    let base = root.join(folder);
    create_dir(&base)?;
    let mut value = json!({ "info": { "name": folder, "type": "folder" } });
    if let Some(map) = value.as_object_mut() {
        if !converted.defaults.is_empty() {
            map.insert(
                "request".to_owned(),
                Value::Object(converted.defaults.clone()),
            );
        }
        if let Some(docs) = &converted.docs {
            map.insert("docs".to_owned(), Value::from(docs.clone()));
        }
    }
    let written = write(&base.join(FOLDER_FILE), Part::Folder, &value)
        .and_then(|()| write_tree(&base, converted))
        .and_then(|()| merge_environments(root, converted));
    if written.is_err() {
        // Rien de partiel : le dossier créé est retiré.
        let _ = fs::remove_dir_all(&base);
    }
    written
}

// --- Scripts Bruno repris (2.3-f, AM42) ------------------------------------------------------

/// Suite donnée à un script Bruno du rapport (2.3-f, AM42).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ScriptChoice {
    /// « Accepter sans ce script » : retiré du fichier, la requête s'exécute sans lui.
    Accept,
    /// « Remplacer par une assertion… » : retiré du fichier, une assertion le remplace.
    Replace,
}

/// Réponse de `import_script` : le rapport à jour et le jeton de « Annuler ».
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ScriptResolved {
    pub report: ImportReport,
    pub undo_token: Uuid,
}

/// De quoi annuler un script repris : le fichier, et la suite d'avant dans le rapport.
#[derive(Debug, Clone)]
pub struct ScriptUndo {
    pub project: Uuid,
    pub index: usize,
    status: ItemStatus,
    detail: String,
    files: Undo,
}

/// Type OpenCollection du script d'un élément du rapport ; aucun pour un élément qui n'en est pas.
fn script_type(kind: ItemKind) -> Option<&'static str> {
    match kind {
        ItemKind::Tests => Some("tests"),
        ItemKind::PreRequest => Some("before-request"),
        ItemKind::PostResponse => Some("after-response"),
        _ => None,
    }
}

/// Retire le script `(kind, code)` de la liste `scripts` de `holder` ; une liste vide disparaît.
fn take_script(holder: &mut Value, kind: &str, code: &str) -> bool {
    let Some(map) = holder.as_object_mut() else {
        return false;
    };
    let Some(list) = map.get_mut("scripts").and_then(Value::as_array_mut) else {
        return false;
    };
    let Some(at) = list.iter().position(|script| {
        script.get("type").and_then(Value::as_str) == Some(kind)
            && script.get("code").and_then(Value::as_str).map(str::trim) == Some(code.trim())
    }) else {
        return false;
    };
    list.remove(at);
    if list.is_empty() {
        map.remove("scripts");
    }
    true
}

/// `folder.yml` sous `dir`, à toute profondeur, dans un ordre stable.
fn folder_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let file = dir.join(FOLDER_FILE);
    if file.is_file() {
        out.push(file);
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            let name = dir_name(path);
            !name.starts_with('.')
                && !same_name(&name, ENVIRONMENTS_DIR)
                && !same_name(&name, contract::CONTRACTS_DIR)
        })
        .collect();
    dirs.sort();
    for sub in dirs {
        folder_files(&sub, out);
    }
}

/// Fichiers où chercher le script d'un élément : sa requête ; sinon les `folder.yml` de
/// l'import, puis la racine du projet créé par l'import.
fn script_files(
    root: &Path,
    report: &ImportReport,
    item: &ReportItem,
) -> Result<Vec<(PathBuf, Part, &'static str)>, CommandError> {
    if let Some(request) = &item.request {
        return Ok(vec![(
            resolve(root, request)?,
            Part::HttpRequest,
            "runtime",
        )]);
    }
    let base = match &report.folder {
        Some(folder) => resolve(root, folder)?,
        None => root.to_path_buf(),
    };
    let mut folders = Vec::new();
    folder_files(&base, &mut folders);
    let mut files: Vec<(PathBuf, Part, &'static str)> = folders
        .into_iter()
        .map(|path| (path, Part::Folder, "request"))
        .collect();
    if report.folder.is_none() {
        files.push((
            root.join(crate::opencollection::ROOT_FILE),
            Part::Root,
            "request",
        ));
    }
    Ok(files)
}

/// Reprend le script de l'élément `index` du dernier rapport (2.3-f, AM42) : il est retiré du
/// fichier où l'import l'a gardé, et l'élément est marqué accepté ou remplacé. Un script déjà
/// retiré ou modifié à la main est refusé, rien n'est écrit.
pub fn resolve_script(
    data_dir: &Path,
    root: &Path,
    project: Uuid,
    index: usize,
    choice: ScriptChoice,
) -> Result<(ImportReport, ScriptUndo), CommandError> {
    let mut report = load_report(data_dir, project).ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::NotFound,
            "Le rapport d'import de ce projet n'est plus sur ce poste.",
        )
    })?;
    let not_script = || {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Cet élément du rapport n'est pas un script à reprendre.",
        )
    };
    let item = report.items.get(index).ok_or_else(not_script)?;
    let kind = script_type(item.kind)
        .filter(|_| item.status == ItemStatus::NotRun)
        .ok_or_else(not_script)?;
    let code = item.code.clone().unwrap_or_default();
    let mut files = Undo::default();
    let mut found = false;
    for (path, part, key) in script_files(root, &report, item)? {
        let (Ok(text), Ok(mut value)) = (fs::read_to_string(&path), yaml::read(&path)) else {
            continue;
        };
        let Some(holder) = value.get_mut(key) else {
            continue;
        };
        if !take_script(holder, kind, &code) {
            continue;
        }
        if holder.as_object().is_some_and(Map::is_empty)
            && let Some(map) = value.as_object_mut()
        {
            map.remove(key);
        }
        write(&path, part, &value)?;
        files.restored(path, text);
        found = true;
        break;
    }
    if !found {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Ce script n'est plus dans le projet : il a été retiré ou modifié depuis l'import.",
        ));
    }
    let undo = ScriptUndo {
        project,
        index,
        status: item.status,
        detail: item.detail.clone(),
        files,
    };
    if let Some(item) = report.items.get_mut(index) {
        (item.status, item.detail) = match choice {
            ScriptChoice::Accept => (
                ItemStatus::Accepted,
                "Retiré du fichier : la requête s'exécute sans ce script.".to_owned(),
            ),
            ScriptChoice::Replace => (
                ItemStatus::Replaced,
                "Retiré du fichier : une assertion de la requête le remplace.".to_owned(),
            ),
        };
    }
    save_report(data_dir, &report)?;
    Ok((report, undo))
}

/// « Annuler » : le fichier retrouve son script, l'élément sa suite d'avant.
pub fn undo_script(data_dir: &Path, undo: &ScriptUndo) -> Result<ImportReport, CommandError> {
    undo.files.apply()?;
    let mut report = load_report(data_dir, undo.project).ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::NotFound,
            "Le rapport d'import de ce projet n'est plus sur ce poste.",
        )
    })?;
    if let Some(item) = report.items.get_mut(undo.index) {
        item.status = undo.status;
        item.detail.clone_from(&undo.detail);
    }
    save_report(data_dir, &report)?;
    Ok(report)
}

fn report_path(data_dir: &Path, project: Uuid) -> PathBuf {
    crate::local::project_dir(data_dir, project).join(REPORT_FILE)
}

/// Garde le rapport du dernier import d'un projet, sur le poste.
pub fn save_report(data_dir: &Path, report: &ImportReport) -> Result<(), CommandError> {
    let dir = crate::local::project_dir(data_dir, report.project_id);
    create_dir(&dir)?;
    let path = report_path(data_dir, report.project_id);
    let text = serde_json::to_string_pretty(report).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture impossible : {error}"),
        )
    })?;
    contract::file::write_text(&path, &text)
}

/// Dernier rapport d'import d'un projet ; aucun s'il est absent ou illisible.
pub fn load_report(data_dir: &Path, project: Uuid) -> Option<ImportReport> {
    let text = fs::read_to_string(report_path(data_dir, project)).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
