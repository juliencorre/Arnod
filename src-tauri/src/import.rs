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
use serde_json::{Value, json};
use uuid::Uuid;

use crate::bruno::convert::{self, Converted, SecretValue};
use crate::bruno::read as bruno;
use crate::bruno::{ConversionExample, ImportCounts, ItemStatus, ReportItem};
use crate::catalog::names::same_name;
use crate::catalog::snapshot::CatalogSnapshot;
use crate::contract;
use crate::environment::{self, Environment, Variable};
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::migrate::LEGACY_MANIFEST;
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{FOLDER_FILE, collection, yaml};
use crate::state::AppState;
use crate::tree::{ENVIRONMENTS_DIR, item_name, slug};
use crate::workbench::ProjectCtx;

/// Fichier du dernier rapport d'import d'un projet, dans ses données locales.
const REPORT_FILE: &str = "import.json";

/// Format reconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Où va l'import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        });
    }
    Err(unknown(path))
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
        if let Err(error) = store(secret) {
            for item in report.items.iter_mut().filter(|item| {
                item.status == ItemStatus::Stored
                    && item.name == secret.name
                    && item.place == secret.environment
            }) {
                item.status = ItemStatus::ToFill;
                item.detail = format!("Variable locale à renseigner : {}", error.message);
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
            let converted = convert::convert(&read);
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
