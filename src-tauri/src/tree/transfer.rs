//! Copier ou déplacer une requête (lot J, écrans 4.1-f et 4.1-g) : vers un dossier du référentiel,
//! une place dans un test, une opération d'un contrat, ou un autre projet.
//!
//! Tout le contenu du fichier suit (corps et variantes, en-têtes, vérifications, étiquettes,
//! docs, champs conservés) ; seuls le lien au contrat et le rang d'étape changent selon la
//! destination. Une requête liée à un contrat n'est jamais une étape ni une requête du
//! référentiel : hors d'une opération, le lien est retiré. Les secrets sont au coffre, jamais
//! dans le fichier : ils ne sont jamais copiés. Chaque fichier touché est gardé pour « Annuler ».

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::undo::Undo;
use super::{
    TreeNode, free_file_name, read, relative_path, request_name_taken, resolve, slug, test_view,
};
use crate::contract::file as contract_file;
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::input;
use crate::request::file;
use crate::request::inherit;
use crate::request::model::{AuthKind, BodyKind, ContractLink, Request};

/// Copier (l'original reste) ou déplacer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Copy,
    Move,
}

/// Destination d'une requête copiée ou déplacée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    /// Dossier du référentiel (`""` : racine du projet).
    Folder { folder: String },
    /// Test, à la place `position` (0 : avant la première étape) ; les suivantes reculent.
    Step { test: String, position: usize },
    /// Opération d'un contrat du projet (`contract` : `contracts/…`, `operation` : `POST /orders`).
    Operation { contract: String, operation: String },
}

/// Opération d'un contrat qui peut recevoir la requête (même méthode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationTarget {
    pub contract: String,
    /// Titre et version du contrat (« API Commandes v2.4.0 »).
    pub contract_name: String,
    pub operation: String,
    /// La requête y est déjà rangée.
    pub current: bool,
}

/// Dossier du référentiel (« » : racine).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderTarget {
    pub path: String,
    /// La requête y est déjà rangée.
    pub current: bool,
}

/// Test du référentiel et ses étapes, pour choisir la place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTarget {
    pub path: String,
    pub name: String,
    /// Noms des étapes, dans l'ordre.
    pub steps: Vec<String>,
    /// Place actuelle de la requête dans ce test.
    pub current: Option<usize>,
}

/// Destinations d'une requête dans un projet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Targets {
    /// Méthode de la requête, pour le titre du dialogue.
    pub method: String,
    pub operations: Vec<OperationTarget>,
    pub folders: Vec<FolderTarget>,
    pub tests: Vec<TestTarget>,
}

/// Ce qui change pour la requête à cette destination (« Ce qui change », 4.1-g).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub title: String,
    pub detail: String,
    /// Quelque chose se perd ou manque (lien retiré, variable inconnue).
    pub warning: bool,
}

/// Requête copiée ou déplacée : son chemin dans le projet de destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transferred {
    pub path: String,
    pub name: String,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, message)
}

fn read_text(path: &Path) -> Result<String, CommandError> {
    fs::read_to_string(path).map_err(|e| CommandError::io("Lecture impossible", path, &e))
}

/// Dossier parent d'un chemin relatif (`""` à la racine).
fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// Destinations de la requête `path` de `source_root` dans le projet `target_root` : opérations
/// de même méthode, dossiers et tests du référentiel. `same` : même projet.
pub fn targets(
    source_root: &Path,
    path: &str,
    target_root: &Path,
    same: bool,
) -> Result<Targets, CommandError> {
    let request = file::read(&resolve(source_root, path)?)?;
    let method = request.method.as_str().to_uppercase();
    let link = request.contract.as_ref().filter(|_| same);
    let mut out = Targets {
        method: method.clone(),
        ..Targets::default()
    };
    for summary in contract_file::list(target_root) {
        if summary.error.is_some() {
            continue;
        }
        let Ok(contract) = contract_file::read(target_root, &summary.path) else {
            continue;
        };
        let contract_name = if contract.version.is_empty() || summary.deduced {
            contract.title.clone()
        } else {
            format!("{} v{}", contract.title, contract.version)
        };
        for operation in contract.operations.iter().filter(|op| op.method == method) {
            out.operations.push(OperationTarget {
                contract: summary.path.clone(),
                contract_name: contract_name.clone(),
                operation: operation.key.clone(),
                current: link
                    .is_some_and(|l| l.file == summary.path && l.operation == operation.key),
            });
        }
    }
    let linked_dirs = contract_file::paths(target_root);
    let tests = super::tests_view(&read(target_root)?, &linked_dirs);
    let here = (same && request.contract.is_none()).then(|| parent_of(path));
    out.folders.push(FolderTarget {
        path: String::new(),
        current: here == Some(""),
    });
    collect(&tests, here, path, &mut out);
    Ok(out)
}

fn collect(nodes: &[TreeNode], here: Option<&str>, path: &str, out: &mut Targets) {
    for node in nodes {
        let TreeNode::Folder {
            path: folder,
            name,
            children,
            test,
            ..
        } = node
        else {
            continue;
        };
        let has_folders = children
            .iter()
            .any(|c| matches!(c, TreeNode::Folder { .. }));
        if *test || (!has_folders && children.is_empty()) {
            let steps: Vec<&TreeNode> = children
                .iter()
                .filter(|c| matches!(c, TreeNode::Request { .. }))
                .collect();
            out.tests.push(TestTarget {
                path: folder.clone(),
                name: name.clone(),
                steps: steps
                    .iter()
                    .filter_map(|c| match c {
                        TreeNode::Request { name, .. } => Some(name.clone()),
                        TreeNode::Folder { .. } => None,
                    })
                    .collect(),
                current: steps.iter().position(|c| match c {
                    TreeNode::Request { path: p, .. } => p == path,
                    TreeNode::Folder { .. } => false,
                }),
            });
        }
        if !*test {
            out.folders.push(FolderTarget {
                path: folder.clone(),
                current: here == Some(folder.as_str()),
            });
        }
        collect(children, here, path, out);
    }
}

/// Dossier (relatif) où la requête est écrite, et le lien qu'elle prend.
fn place(
    target_root: &Path,
    target: &Target,
) -> Result<(String, Option<ContractLink>), CommandError> {
    match target {
        Target::Folder { folder } => Ok((folder.clone(), None)),
        Target::Step { test, .. } => {
            if test.is_empty() {
                return Err(invalid("La racine du projet n'est pas un test."));
            }
            Ok((test.clone(), None))
        }
        Target::Operation {
            contract,
            operation,
        } => {
            let read = contract_file::read(target_root, contract)?;
            let op = read
                .operations
                .iter()
                .find(|op| op.key == *operation)
                .ok_or_else(|| not_found("Cette opération n'existe plus dans le contrat."))?;
            Ok((
                contract_file::operation_dir(&read, op),
                Some(ContractLink {
                    file: contract.clone(),
                    operation: operation.clone(),
                }),
            ))
        }
    }
}

/// Nom libre dans `dir` : le même, sinon « (copie) » pour une copie à côté de l'original,
/// sinon suivi d'un numéro.
fn free_name(dir: &Path, name: &str, copy_beside: bool) -> String {
    let base = if copy_beside {
        format!("{name} (copie)")
    } else {
        name.to_owned()
    };
    let mut candidate = base.clone();
    let mut n = 2;
    while request_name_taken(dir, &candidate, None) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    candidate
}

/// Copie ou déplace la requête `path` de `source_root` vers `target` dans `target_root`.
pub fn transfer(
    source_root: &Path,
    path: &str,
    target_root: &Path,
    target: &Target,
    mode: Mode,
) -> Result<(Transferred, Undo), CommandError> {
    let source = resolve(source_root, path)?;
    if !source.is_file() {
        return Err(not_found("Cette requête n'existe plus."));
    }
    let original = read_text(&source)?;
    let request = file::read(&source)?;
    let (folder, link) = place(target_root, target)?;
    let dir = resolve(target_root, &folder)?;
    if !matches!(target, Target::Operation { .. }) && !dir.is_dir() {
        return Err(not_found("Ce dossier n'existe plus."));
    }
    if let Target::Step { .. } = target {
        let has_folder = fs::read_dir(&dir)
            .map_err(|e| CommandError::io("Lecture du dossier impossible", &dir, &e))?
            .flatten()
            .any(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'));
        if has_folder {
            return Err(invalid(
                "Ce dossier range d'autres dossiers : ce n'est pas un test.",
            ));
        }
    }
    fs::create_dir_all(&dir)
        .map_err(|e| CommandError::io("Création du dossier impossible", &dir, &e))?;

    let mut undo = Undo::default();
    let beside = source.parent() == Some(dir.as_path());
    // Déplacée dans son propre dossier : seul son rang (ou son lien) change, sur place.
    let in_place = mode == Mode::Move && beside;
    let (written, name) = if in_place {
        undo.restored(source.clone(), original.clone());
        (source.clone(), request.name.clone())
    } else {
        let name = free_name(&dir, &request.name, beside);
        let to = dir.join(free_file_name(&dir, &slug(&name)));
        file::copy_as(&source, &to, &name)?;
        undo.created(to.clone());
        (to, name)
    };

    let mut moved = file::read(&written)?;
    moved.contract = link;
    moved.seq = None;
    file::write(&written, &moved)?;

    if let Target::Step { test, position } = target {
        renumber(target_root, test, &written, *position, &mut undo)?;
    }

    if mode == Mode::Move && !in_place {
        fs::remove_file(&source)
            .map_err(|e| CommandError::io("Déplacement impossible", &source, &e))?;
        crate::watch::settle(&source);
        undo.restored(source, original);
    }
    Ok((
        Transferred {
            path: relative_path(target_root, &written),
            name,
        },
        undo,
    ))
}

/// Range `step` à la place `position` du test et renumérote `seq` de 1 à n (les fichiers
/// modifiés sont gardés pour « Annuler »).
fn renumber(
    root: &Path,
    test: &str,
    step: &Path,
    position: usize,
    undo: &mut Undo,
) -> Result<(), CommandError> {
    let view = test_view(root, test)?;
    let step_rel = relative_path(root, step);
    let mut order: Vec<PathBuf> = view
        .steps
        .iter()
        .filter(|s| s.path != step_rel && !s.invalid)
        .map(|s| resolve(root, &s.path))
        .collect::<Result<_, _>>()?;
    order.insert(position.min(order.len()), step.to_path_buf());
    for (index, path) in order.iter().enumerate() {
        let seq = (index + 1) as f64;
        let text = read_text(path)?;
        let mut request = file::read(path)?;
        if request.seq == Some(seq) {
            continue;
        }
        undo.restored(path.clone(), text);
        request.seq = Some(seq);
        file::write(path, &request)?;
    }
    Ok(())
}

fn auth_label(kind: AuthKind) -> &'static str {
    match kind {
        AuthKind::None => "aucune",
        AuthKind::Basic => "Basic",
        AuthKind::Bearer => "Bearer",
        AuthKind::ApiKey => "clé d'API",
        AuthKind::Inherit => "héritée",
        AuthKind::Other => "autre",
        AuthKind::OAuth2 => "OAuth 2",
    }
}

/// Liste lisible : « a, b et c ».
fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} et {last}", rest.join(", ")),
    }
}

/// Ce qui suit la requête, pour « Suivent la requête ».
fn follows(request: &Request) -> String {
    let mut parts = Vec::new();
    if request.body.kind != BodyKind::None {
        parts.push(if request.body.variants.len() > 1 {
            "corps et variantes".to_owned()
        } else {
            "corps".to_owned()
        });
    }
    if !request.headers.is_empty() {
        parts.push("en-têtes".to_owned());
    }
    match request.assertions.len() {
        0 => {}
        1 => parts.push("1 vérification".to_owned()),
        n => parts.push(format!("{n} vérifications")),
    }
    if !request.extractions.is_empty() {
        parts.push("extractions".to_owned());
    }
    if !request.tags.is_empty() {
        parts.push("étiquettes".to_owned());
    }
    if !request.examples.is_empty() {
        parts.push("réponses attendues".to_owned());
    }
    if !request.docs.trim().is_empty() {
        parts.push("docs".to_owned());
    }
    if parts.is_empty() {
        parts.push("l'adresse et les réglages".to_owned());
    }
    let mut text = list(&parts);
    if let Some(first) = text.get(..1) {
        text = format!("{}{}", first.to_uppercase(), &text[1..]);
    }
    format!("{text}. Les secrets restent au coffre : ils ne sont jamais copiés.")
}

/// « Ce qui change » si la requête `path` va à `target` (4.1-g). `project` : nom du projet de
/// destination quand il diffère.
pub fn preview(
    source_root: &Path,
    path: &str,
    target_root: &Path,
    target: &Target,
    mode: Mode,
    project: Option<&str>,
) -> Result<Vec<Change>, CommandError> {
    let source = resolve(source_root, path)?;
    let request = file::read(&source)?;
    let (folder, link) = place(target_root, target)?;
    let mut changes = Vec::new();
    let elsewhere = project
        .map(|p| format!(" du projet « {p} »"))
        .unwrap_or_default();

    // Destination.
    match target {
        Target::Step { test, position } => {
            let view = test_view(target_root, test)?;
            let others: Vec<&str> = view
                .steps
                .iter()
                .filter(|s| s.path != path || project.is_some())
                .map(|s| s.name.as_str())
                .collect();
            let rank = position.min(&others.len()) + 1;
            let detail = if rank <= others.len() {
                format!("L'actuelle étape {rank} devient la {}.", rank + 1)
            } else if others.is_empty() {
                "Première étape du test.".to_owned()
            } else {
                "Dernière étape du test.".to_owned()
            };
            changes.push(Change {
                title: format!("Devient l'étape {rank} de « {} »{elsewhere}", view.name),
                detail,
                warning: false,
            });
        }
        Target::Folder { folder } => {
            let place = if folder.is_empty() {
                "la racine".to_owned()
            } else {
                format!("« {folder} »")
            };
            changes.push(Change {
                title: format!("Rangée dans {place}{elsewhere}"),
                detail: "Une requête du référentiel de tests, hors de tout test.".to_owned(),
                warning: false,
            });
        }
        Target::Operation { operation, .. } => {
            let same = request
                .contract
                .as_ref()
                .is_some_and(|l| Some(l) == link.as_ref());
            if !same {
                changes.push(Change {
                    title: format!("Rangée sous {operation}{elsewhere}"),
                    detail: format!("Dans le dossier « {folder} » du contrat."),
                    warning: false,
                });
            }
        }
    }

    // Lien au contrat.
    match (&request.contract, &link, target) {
        (_, Some(new), Target::Operation { contract, .. }) => {
            let name = contract_file::read(target_root, contract)
                .map(|c| {
                    if c.version.is_empty() {
                        c.title
                    } else {
                        format!("{} v{}", c.title, c.version)
                    }
                })
                .unwrap_or_default();
            let kept = request.contract.as_ref() == Some(new) && project.is_none();
            changes.push(Change {
                title: format!(
                    "{} {}",
                    if kept { "Reste liée à" } else { "Liée à" },
                    new.operation
                ),
                detail: format!("{name} : conformité vérifiée à chaque envoi."),
                warning: false,
            });
        }
        (Some(old), None, _) => changes.push(Change {
            title: format!("N'est plus liée à {}", old.operation),
            detail: "Une étape ou une requête du référentiel n'est jamais liée à un contrat : la conformité n'est plus vérifiée.".to_owned(),
            warning: true,
        }),
        _ => {}
    }

    // Authentification.
    let at = format!(
        "{}{}x.yml",
        folder,
        if folder.is_empty() { "" } else { "/" }
    );
    let auth = if request.auth.kind == AuthKind::Inherit {
        match inherit::resolve(target_root, &at).auth {
            Some(found) => format!(
                "« Hériter » prend celle de la destination : {}, {}.",
                auth_label(found.auth.kind),
                found.source
            ),
            None => "« Hériter » : aucune authentification dans la destination.".to_owned(),
        }
    } else {
        format!("Garde la sienne : {}.", auth_label(request.auth.kind))
    };
    changes.push(Change {
        title: "Authentification".to_owned(),
        detail: auth,
        warning: false,
    });

    // Variables.
    let used = input::used_names(&request);
    if !used.is_empty() {
        let mut known: Vec<String> = Vec::new();
        for env in environment::list(target_root) {
            known.extend(env.variables.into_iter().map(|v| v.name));
            known.extend(env.locals);
        }
        known.extend(inherit::variable_names(target_root, &at));
        if let Target::Step { test, position } = target
            && let Ok(view) = test_view(target_root, test)
        {
            for step in view
                .steps
                .iter()
                .filter(|s| s.path != path || project.is_some())
                .take(*position)
            {
                known.extend(step.extractions.iter().cloned());
            }
        }
        let unknown: Vec<String> = used
            .iter()
            .filter(|name| !known.contains(name))
            .map(|name| format!("{{{{{name}}}}}"))
            .collect();
        let shown: Vec<String> = used
            .iter()
            .take(3)
            .map(|n| format!("{{{{{n}}}}}"))
            .collect();
        changes.push(if unknown.is_empty() {
            Change {
                title: "Variables".to_owned(),
                detail: format!(
                    "{} connue{} dans la destination ; aucune inconnue.",
                    list(&shown),
                    if used.len() > 1 { "s" } else { "" }
                ),
                warning: false,
            }
        } else {
            Change {
                title: "Variables".to_owned(),
                detail: format!(
                    "{} inconnue{} dans la destination.",
                    list(&unknown),
                    if unknown.len() > 1 { "s" } else { "" }
                ),
                warning: true,
            }
        });
    }

    changes.push(Change {
        title: "Suivent la requête".to_owned(),
        detail: follows(&request),
        warning: false,
    });

    let from = match &request.contract {
        Some(old) => old.operation.clone(),
        None if parent_of(path).is_empty() => "la racine".to_owned(),
        None => format!("« {} »", parent_of(path)),
    };
    changes.push(match mode {
        Mode::Copy => Change {
            title: "Copie".to_owned(),
            detail: format!("« {} » reste aussi sous {from}.", request.name),
            warning: false,
        },
        Mode::Move => Change {
            title: "Déplacement".to_owned(),
            detail: format!("« {} » quitte {from}.", request.name),
            warning: false,
        },
    });
    Ok(changes)
}

#[cfg(test)]
#[path = "transfer_tests.rs"]
mod tests;
