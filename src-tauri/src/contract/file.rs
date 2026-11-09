//! Contrats d'un projet sur disque : dossier `contracts/`, requêtes liées,
//! création des requêtes (R2, R4, R5, R8, R10, R11, R12).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::catalog::names::{same_name, sort_key};
use crate::contract::conformance;
use crate::contract::generate::{request_for, safe_name, supported};
use crate::contract::parse::{Contract, SpecKind, load};
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::request::assertions::{AssertionResult, ResponseData};
use crate::request::file as request_file;
use crate::request::model::{ContractLink, Request};
use crate::tree::{self, ENVIRONMENTS_DIR, TreeNode};

/// Dossier des contrats, à la racine du projet (réservé comme `environments/`).
pub const CONTRACTS_DIR: &str = "contracts";

/// Variable de l'adresse de base des requêtes (R8).
const BASE_URL: &str = "baseUrl";

/// Contrat de la barre latérale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSummary {
    /// Chemin relatif au projet : `contracts/petstore.yaml`.
    pub path: String,
    pub name: String,
    pub version: String,
    pub operation_count: usize,
    /// Fichier illisible (R10) : la raison, et le contrat reste listé.
    pub error: Option<String>,
}

/// Opération affichée dans l'onglet du contrat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub key: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub operation_id: Option<String>,
    pub tag: Option<String>,
    pub deprecated: bool,
    /// Méthode envoyable (TRACE ne l'est pas).
    pub supported: bool,
    pub external_refs: bool,
    /// Requête liée à cette opération (R4), s'il y en a une.
    pub request: Option<String>,
}

/// Requête liée dont l'opération n'est plus dans le contrat (R11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanView {
    pub path: String,
    pub name: String,
    pub operation: String,
}

/// Contrat affiché dans son onglet (C5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractView {
    pub path: String,
    pub name: String,
    pub version: String,
    pub kind: Option<SpecKind>,
    pub spec_version: String,
    pub description: Option<String>,
    pub servers: Vec<String>,
    /// Texte du fichier, tel quel (C9).
    pub source: String,
    pub error: Option<String>,
    pub operations: Vec<OperationView>,
    pub orphans: Vec<OrphanView>,
}

/// Origine d'un contrat ajouté (C2 à C4).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContractSource {
    /// Texte collé.
    Text { text: String },
    /// Adresse lue par le cœur.
    Url { url: String },
    /// Contrat vide, à partir d'un modèle.
    Template { name: String },
}

/// Opération pour laquelle aucune requête n'a été créée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedOperation {
    pub operation: String,
    pub reason: String,
}

/// `baseUrl` renseignée à la création des requêtes (R8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseUrlSet {
    pub environment: String,
    pub value: String,
}

/// Bilan de « Créer les requêtes » (C6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReport {
    /// Dossier des requêtes créées.
    pub folder: String,
    pub created: Vec<String>,
    pub skipped: Vec<SkippedOperation>,
    pub base_url: Option<BaseUrlSet>,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn not_a_contract(reason: &str) -> CommandError {
    invalid(format!("Ce texte n'est pas un contrat lisible : {reason}"))
}

fn dir(root: &Path) -> PathBuf {
    root.join(CONTRACTS_DIR)
}

/// Fichier de contrat : JSON ou YAML.
pub fn is_contract_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["json", "yaml", "yml"]
                .iter()
                .any(|e| ext.eq_ignore_ascii_case(e))
        })
}

/// Chemin absolu d'un contrat, refusé hors de `contracts/`.
fn resolve_path(root: &Path, path: &str) -> Result<PathBuf, CommandError> {
    let absolute = tree::resolve(root, path)?;
    if absolute.parent() != Some(dir(root).as_path()) || !is_contract_file(&absolute) {
        return Err(invalid("Ce fichier n'est pas un contrat du projet."));
    }
    Ok(absolute)
}

fn relative(path: &Path) -> String {
    format!(
        "{CONTRACTS_DIR}/{}",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    )
}

fn read_text(path: &Path) -> Result<String, CommandError> {
    fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            CommandError::new(CommandErrorKind::NotFound, "Ce contrat n'existe plus.")
        } else {
            CommandError::io("Lecture du contrat impossible", path, &error)
        }
    })
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Contrats du projet, par nom (C5) ; un fichier illisible est listé avec son erreur.
pub fn list(root: &Path) -> Vec<ContractSummary> {
    let Ok(entries) = fs::read_dir(dir(root)) else {
        return Vec::new();
    };
    let mut list: Vec<ContractSummary> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_contract_file(path))
        .filter(|path| !stem(path).starts_with('.'))
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_default();
            match load(&text) {
                Ok(contract) => ContractSummary {
                    path: relative(&path),
                    name: contract.title,
                    version: contract.version,
                    operation_count: contract.operations.len(),
                    error: None,
                },
                Err(reason) => ContractSummary {
                    path: relative(&path),
                    name: stem(&path),
                    version: String::new(),
                    operation_count: 0,
                    error: Some(reason),
                },
            }
        })
        .collect();
    list.sort_by_key(|summary| (sort_key(&summary.name), summary.path.clone()));
    list
}

/// Contrat lu depuis le projet.
pub fn read(root: &Path, path: &str) -> Result<Contract, CommandError> {
    let absolute = resolve_path(root, path)?;
    load(&read_text(&absolute)?)
        .map_err(|reason| invalid(format!("Le contrat {path} est illisible : {reason}")))
}

/// Requête liée à un contrat.
#[derive(Debug, Clone)]
pub struct Linked {
    pub path: String,
    pub name: String,
    pub link: ContractLink,
}

/// Requêtes du projet liées à un contrat, dans l'ordre de l'arborescence.
pub fn linked(root: &Path) -> Vec<Linked> {
    fn visit(root: &Path, nodes: &[TreeNode], out: &mut Vec<Linked>) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, .. } => visit(root, children, out),
                TreeNode::Request {
                    path,
                    invalid: false,
                    ..
                } => {
                    let request = tree::resolve(root, path).and_then(|p| request_file::read(&p));
                    if let Ok(Request {
                        name,
                        contract: Some(link),
                        ..
                    }) = request
                    {
                        out.push(Linked {
                            path: path.clone(),
                            name,
                            link,
                        });
                    }
                }
                TreeNode::Request { .. } => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(nodes) = tree::read(root) {
        visit(root, &nodes, &mut out);
    }
    out
}

/// Chemins des requêtes liées au contrat `path`, dans l'ordre de l'arborescence (R12).
pub fn linked_paths(root: &Path, path: &str) -> Vec<String> {
    linked(root)
        .into_iter()
        .filter(|l| l.link.file == path)
        .map(|l| l.path)
        .collect()
}

/// Vue du contrat pour son onglet (C5, C12). Un fichier illisible donne une vue
/// avec son erreur et sa source.
pub fn view(root: &Path, path: &str) -> Result<ContractView, CommandError> {
    let absolute = resolve_path(root, path)?;
    let source = read_text(&absolute)?;
    let links: Vec<Linked> = linked(root)
        .into_iter()
        .filter(|l| l.link.file == path)
        .collect();
    Ok(match load(&source) {
        Ok(contract) => view_of(path, &contract, source, &links),
        Err(reason) => ContractView {
            path: path.to_owned(),
            name: stem(&absolute),
            version: String::new(),
            kind: None,
            spec_version: String::new(),
            description: None,
            servers: Vec::new(),
            source,
            error: Some(reason),
            operations: Vec::new(),
            orphans: Vec::new(),
        },
    })
}

fn view_of(path: &str, contract: &Contract, source: String, links: &[Linked]) -> ContractView {
    let by_operation: HashMap<&str, &Linked> = links
        .iter()
        .rev()
        .map(|l| (l.link.operation.as_str(), l))
        .collect();
    let operations = contract
        .operations
        .iter()
        .map(|operation| OperationView {
            key: operation.key.clone(),
            method: operation.method.clone(),
            path: operation.path.clone(),
            summary: operation.summary.clone(),
            operation_id: operation.operation_id.clone(),
            tag: operation.tag.clone(),
            deprecated: operation.deprecated,
            supported: supported(operation),
            external_refs: operation.external_refs,
            request: by_operation
                .get(operation.key.as_str())
                .map(|l| l.path.clone()),
        })
        .collect();
    let orphans = links
        .iter()
        .filter(|l| contract.operation(&l.link.operation).is_none())
        .map(|l| OrphanView {
            path: l.path.clone(),
            name: l.name.clone(),
            operation: l.link.operation.clone(),
        })
        .collect();
    ContractView {
        path: path.to_owned(),
        name: contract.title.clone(),
        version: contract.version.clone(),
        kind: Some(contract.kind),
        spec_version: contract.spec_version.clone(),
        description: contract.description.clone(),
        servers: contract.servers.clone(),
        source,
        error: None,
        operations,
        orphans,
    }
}

/// Texte d'un contrat vide (C4) : OpenAPI 3.0 avec une opération d'exemple.
pub fn template(name: &str) -> String {
    let title = name.trim().replace('"', "'");
    format!(
        r#"openapi: 3.0.3
info:
  title: "{title}"
  version: 1.0.0
  description: Décrivez ici le service.
servers:
  - url: http://localhost:8080
paths:
  /sante:
    get:
      summary: État du service
      operationId: sante
      responses:
        "200":
          description: Le service répond.
          content:
            application/json:
              schema:
                type: object
                required: [statut]
                properties:
                  statut:
                    type: string
                    example: ok
"#
    )
}

fn is_json_text(text: &str) -> bool {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

/// Nom de fichier libre dans `contracts/` pour `base.ext`.
fn free_name(dir: &Path, base: &str, ext: &str) -> String {
    let taken = |candidate: &str| {
        fs::read_dir(dir).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                // Même nom, quelle que soit l'extension : un seul `petstore.*`.
                same_name(
                    Path::new(&name)
                        .file_stem()
                        .map_or(&*name, |s| s.to_str().unwrap_or("")),
                    candidate,
                )
            })
        })
    };
    let mut candidate = base.to_owned();
    let mut n = 2;
    while taken(&candidate) {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    format!("{candidate}.{ext}")
}

/// Ajoute un contrat au projet (R1, R2) : vérifié, puis écrit tel quel dans
/// `contracts/<nom>.yaml` ou `.json`. Renvoie son chemin relatif.
pub fn add(root: &Path, text: &str) -> Result<String, CommandError> {
    let contract = load(text).map_err(|reason| not_a_contract(&reason))?;
    let folder = dir(root);
    fs::create_dir_all(&folder)
        .map_err(|error| CommandError::io("Création du dossier impossible", &folder, &error))?;
    let ext = if is_json_text(text) { "json" } else { "yaml" };
    let file = folder.join(free_name(&folder, &tree::slug(&contract.title), ext));
    write_text(&file, text)?;
    Ok(relative(&file))
}

fn write_text(path: &Path, text: &str) -> Result<(), CommandError> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text).map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        CommandError::io("Écriture impossible", path, &error)
    })
}

/// Enregistre la source d'un contrat, seulement si elle reste lisible (R10).
pub fn save(root: &Path, path: &str, text: &str) -> Result<(), CommandError> {
    let absolute = resolve_path(root, path)?;
    load(text).map_err(|reason| not_a_contract(&reason))?;
    if fs::read_to_string(&absolute).is_ok_and(|current| current == text) {
        return Ok(());
    }
    write_text(&absolute, text)
}

/// Met un contrat à la corbeille, ou le supprime définitivement (C11).
pub fn delete(root: &Path, path: &str, permanent: bool) -> Result<(), CommandError> {
    let absolute = resolve_path(root, path)?;
    if !absolute.exists() {
        return Ok(());
    }
    if permanent {
        return fs::remove_file(&absolute)
            .map_err(|e| CommandError::io("Suppression impossible", &absolute, &e));
    }
    trash::delete(&absolute).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("La corbeille du système n'est pas disponible ({error}). Vous pouvez supprimer définitivement."),
        )
    })
}

/// Dossier d'un nom à la racine, hors des dossiers réservés.
fn folder_name(title: &str) -> String {
    let name = safe_name(title, "Contrat");
    if [CONTRACTS_DIR, ENVIRONMENTS_DIR]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        format!("{name} (contrat)")
    } else {
        name
    }
}

/// Nom libre pour une requête dans `dir` (R5).
fn free_request_name(dir: &Path, base: &str) -> String {
    let mut name = base.to_owned();
    let mut n = 2;
    while tree::request_name_taken(dir, &name, None) {
        name = format!("{base} ({n})");
        n += 1;
    }
    name
}

/// Crée les requêtes des opérations `keys` du contrat `path` (C6, R4 à R8).
pub fn generate(
    root: &Path,
    path: &str,
    keys: &[String],
    environment_name: Option<&str>,
) -> Result<GenerateReport, CommandError> {
    let contract = read(root, path)?;
    let existing: Vec<String> = linked(root)
        .into_iter()
        .filter(|l| l.link.file == path)
        .map(|l| l.link.operation)
        .collect();
    let folder = folder_name(&contract.title);
    let mut created = Vec::new();
    let mut skipped = Vec::new();
    for operation in contract
        .operations
        .iter()
        .filter(|operation| keys.contains(&operation.key))
    {
        if existing.contains(&operation.key) {
            skipped.push(SkippedOperation {
                operation: operation.key.clone(),
                reason: "une requête existe déjà pour cette opération".to_owned(),
            });
            continue;
        }
        let mut request = match request_for(&contract, operation, path) {
            Ok(request) => request,
            Err(reason) => {
                skipped.push(SkippedOperation {
                    operation: operation.key.clone(),
                    reason,
                });
                continue;
            }
        };
        let mut relative_dir = folder.clone();
        if let Some(tag) = &operation.tag {
            relative_dir = format!("{relative_dir}/{}", safe_name(tag, "Autres"));
        }
        let target = tree::resolve(root, &relative_dir)?;
        fs::create_dir_all(&target)
            .map_err(|error| CommandError::io("Création du dossier impossible", &target, &error))?;
        request.name = free_request_name(&target, &request.name);
        let file = target.join(tree::free_file_name(&target, &tree::slug(&request.name)));
        request_file::write(&file, &request)?;
        created.push(format!(
            "{relative_dir}/{}",
            file.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ));
    }
    let base_url = if created.is_empty() {
        None
    } else {
        environment_name.and_then(|name| fill_base_url(root, name, &contract))
    };
    Ok(GenerateReport {
        folder,
        created,
        skipped,
        base_url,
    })
}

/// Renseigne `baseUrl` dans l'environnement `name` si elle est vide ou absente,
/// avec le premier serveur absolu du contrat (R8). Une variable locale n'est
/// jamais modifiée.
fn fill_base_url(root: &Path, name: &str, contract: &Contract) -> Option<BaseUrlSet> {
    let server = contract
        .servers
        .iter()
        .find(|url| url.starts_with("http://") || url.starts_with("https://"))?;
    let mut environment = environment::list(root)
        .into_iter()
        .find(|e| same_name(&e.name, name))?;
    if environment.locals.iter().any(|local| local == BASE_URL) {
        return None;
    }
    match environment
        .variables
        .iter_mut()
        .find(|v| v.name == BASE_URL)
    {
        Some(variable) if !variable.value.trim().is_empty() => return None,
        Some(variable) => variable.value.clone_from(server),
        None => environment.variables.push(environment::Variable {
            name: BASE_URL.to_owned(),
            value: server.clone(),
        }),
    }
    let previous = environment.name.clone();
    environment::save(root, &environment, Some(&previous)).ok()?;
    Some(BaseUrlSet {
        environment: environment.name,
        value: server.clone(),
    })
}

/// Résultat de la vérification « est conforme au contrat » (C8, R11).
pub fn conformance_result(
    root: &Path,
    link: Option<&ContractLink>,
    id: &str,
    response: &ResponseData,
) -> AssertionResult {
    let outcome = match link {
        None => Err("aucun contrat associé à cette requête".to_owned()),
        Some(link) => match read(root, &link.file) {
            Err(error) if error.kind == CommandErrorKind::NotFound => {
                Err(format!("le contrat {} est introuvable", link.file))
            }
            Err(_) => Err(format!("le contrat {} est illisible", link.file)),
            Ok(contract) => conformance::check(&contract, &link.operation, response),
        },
    };
    AssertionResult {
        id: id.to_owned(),
        passed: outcome.is_ok(),
        message: outcome.err(),
        actual: Some(response.status.to_string()),
    }
}
