//! Tests : dossiers d'étapes (`docs/plan-mvp-e-tests-etapes.md`, R1, R11).
//!
//! Un test est un dossier qui contient au moins une requête et aucun
//! sous-dossier ; ses requêtes sont ses étapes, dans l'ordre du dossier
//! (`info.seq`, puis le nom).

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{
    TreeNode, create_folder, exists_ignoring_case, free_file_name, read, relative_path,
    request_name_taken, requests_in, resolve, slug,
};
use crate::error::{CommandError, CommandErrorKind};
use crate::request::file;
use crate::request::model::{BodyKind, Method, Request};
use crate::request::resolve::variable_names;

/// Nom d'un test créé par « Nouveau test » (R11).
pub const NEW_TEST: &str = "Nouveau test";

/// Le dossier de ces enfants est un test (R1).
pub fn is_test(children: &[TreeNode]) -> bool {
    children
        .iter()
        .any(|node| matches!(node, TreeNode::Request { .. }))
        && !children
            .iter()
            .any(|node| matches!(node, TreeNode::Folder { .. }))
}

/// Test créé : son dossier et son étape 1 (R11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedTest {
    pub folder: String,
    pub step: String,
}

/// Étape d'un test, pour son onglet (4.2-a).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub path: String,
    pub name: String,
    pub method: Option<Method>,
    pub url: String,
    pub assertion_count: usize,
    /// Noms des variables extraites par l'étape.
    pub extractions: Vec<String>,
    /// Fichier illisible.
    pub invalid: bool,
}

/// Test ouvert dans un onglet (4.2-a).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestView {
    pub path: String,
    pub name: String,
    /// Le dossier est un test ; sinon il range des dossiers.
    pub test: bool,
    pub steps: Vec<StepView>,
}

/// Requête qui lit une variable (onglet « Variables », EX5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableUser {
    pub path: String,
    pub name: String,
    pub method: Method,
    pub url: String,
    /// Variables lues parmi celles demandées.
    pub variables: Vec<String>,
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, message)
}

fn find<'a>(nodes: &'a [TreeNode], folder: &str) -> Option<&'a TreeNode> {
    nodes.iter().find_map(|node| match node {
        TreeNode::Folder { path, children, .. } => {
            if path == folder {
                Some(node)
            } else {
                find(children, folder)
            }
        }
        TreeNode::Request { .. } => None,
    })
}

/// Onglet du test `path` : ses étapes dans l'ordre (R1).
pub fn test_view(root: &Path, path: &str) -> Result<TestView, CommandError> {
    let tree = read(root)?;
    let Some(TreeNode::Folder {
        name,
        children,
        test,
        ..
    }) = find(&tree, path)
    else {
        return Err(not_found("Ce test n'existe plus."));
    };
    let steps = children
        .iter()
        .filter_map(|node| match node {
            TreeNode::Request {
                name,
                path,
                method,
                assertion_count,
                invalid,
                ..
            } => {
                let request = resolve(root, path).and_then(|p| file::read(&p)).ok();
                Some(StepView {
                    path: path.clone(),
                    name: name.clone(),
                    method: *method,
                    url: request.as_ref().map(|r| r.url.clone()).unwrap_or_default(),
                    assertion_count: *assertion_count,
                    extractions: request
                        .map(|r| {
                            r.extractions
                                .into_iter()
                                .filter(|e| e.enabled)
                                .map(|e| e.name)
                                .collect()
                        })
                        .unwrap_or_default(),
                    invalid: *invalid,
                })
            }
            TreeNode::Folder { .. } => None,
        })
        .collect();
    Ok(TestView {
        path: path.to_owned(),
        name: name.clone(),
        test: *test,
        steps,
    })
}

fn unique_folder_name(dir: &Path) -> String {
    let mut name = NEW_TEST.to_owned();
    let mut n = 2;
    while exists_ignoring_case(dir, &name) {
        name = format!("{NEW_TEST} {n}");
        n += 1;
    }
    name
}

/// Écrit une étape neuve `name` de rang `seq` dans `dir`.
fn write_step(root: &Path, dir: &Path, name: &str, seq: f64) -> Result<String, CommandError> {
    let mut request = Request::new(name);
    request.seq = Some(seq);
    let path = dir.join(free_file_name(dir, &slug(name)));
    file::write(&path, &request)?;
    Ok(relative_path(root, &path))
}

/// « Nouveau test » (R11) : dossier « Nouveau test » (nom libre suivant) dans
/// `parent`, et son étape 1.
pub fn create_test(root: &Path, parent: &str) -> Result<CreatedTest, CommandError> {
    let parent_dir = resolve(root, parent)?;
    if !parent_dir.is_dir() {
        return Err(not_found("Le dossier parent n'existe plus."));
    }
    let folder = create_folder(root, parent, &unique_folder_name(&parent_dir))?;
    let dir = resolve(root, &folder)?;
    let step = write_step(root, &dir, "Étape 1", 1.0)?;
    Ok(CreatedTest { folder, step })
}

/// « + Étape » (R11) : « Étape N » après la dernière étape du test. Les étapes
/// sans rang reçoivent le leur, dans l'ordre actuel, pour que la nouvelle
/// reste la dernière.
pub fn add_step(root: &Path, test: &str) -> Result<String, CommandError> {
    let dir = resolve(root, test)?;
    if !dir.is_dir() || test.is_empty() {
        return Err(not_found("Ce test n'existe plus."));
    }
    let has_folder = fs::read_dir(&dir)
        .map_err(|e| CommandError::io("Lecture du dossier impossible", &dir, &e))?
        .flatten()
        .any(|entry| {
            entry.path().is_dir() && !entry.file_name().to_string_lossy().starts_with('.')
        });
    if has_folder {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Ce dossier range d'autres dossiers : ce n'est pas un test.",
        ));
    }
    let view = test_view(root, test)?;
    let mut last = 0.0_f64;
    let missing = view.steps.iter().any(|step| {
        resolve(root, &step.path)
            .and_then(|p| file::read(&p))
            .is_ok_and(|r| r.seq.is_none())
    });
    for (index, step) in view.steps.iter().enumerate() {
        let path = resolve(root, &step.path)?;
        let Ok(mut request) = file::read(&path) else {
            continue;
        };
        if missing {
            request.seq = Some((index + 1) as f64);
            file::write(&path, &request)?;
        }
        last = last.max(request.seq.unwrap_or(0.0));
    }
    let mut n = view.steps.len() + 1;
    let mut name = format!("Étape {n}");
    while request_name_taken(&dir, &name, None) {
        n += 1;
        name = format!("Étape {n}");
    }
    write_step(root, &dir, &name, last.floor() + 1.0)
}

/// Textes d'une requête où une variable peut être lue.
fn texts(request: &Request) -> Vec<&str> {
    let mut out = vec![request.url.as_str()];
    for pair in request.params.iter().chain(&request.headers) {
        out.push(&pair.name);
        out.push(&pair.value);
    }
    let auth = &request.auth;
    out.extend([
        auth.username.as_str(),
        auth.password.as_str(),
        auth.token.as_str(),
        auth.name.as_str(),
        auth.value.as_str(),
    ]);
    if request.body.kind == BodyKind::Form {
        for pair in request.body.fields() {
            out.push(&pair.name);
            out.push(&pair.value);
        }
    } else {
        out.push(request.body.content());
    }
    out
}

/// Requêtes du projet, hors `path`, qui lisent l'une des variables `names` (EX5).
pub fn variable_users(
    root: &Path,
    path: &str,
    names: &[String],
) -> Result<Vec<VariableUser>, CommandError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut users = Vec::new();
    for other in requests_in(root, "")? {
        if other == path {
            continue;
        }
        let Ok(request) = resolve(root, &other).and_then(|p| file::read(&p)) else {
            continue;
        };
        let mut found: Vec<String> = Vec::new();
        for text in texts(&request) {
            for name in variable_names(text) {
                if names.contains(&name) && !found.contains(&name) {
                    found.push(name);
                }
            }
        }
        if !found.is_empty() {
            users.push(VariableUser {
                path: other,
                name: request.name,
                method: request.method,
                url: request.url,
                variables: found,
            });
        }
    }
    Ok(users)
}
