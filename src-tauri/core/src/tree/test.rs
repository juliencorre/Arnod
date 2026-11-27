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
use crate::flow::edit::{FlowStepInfo, info as flow_info, test_flow};
use crate::flow::store::Extensions;
use crate::flow::{Action, Flow};
use crate::request::file;
use crate::request::model::{Assertion, BodyKind, Extraction, Method, Request};
use crate::request::resolve::variable_names;

/// Nom d'un test créé par « Nouveau test » (R11).
pub const NEW_TEST: &str = "Nouveau test";

/// Le dossier de ces enfants est un test (R1).
pub fn is_test(children: &[TreeNode]) -> bool {
    // Une requête liée à un contrat n'est jamais une étape (R1) : le dossier d'un serveur de
    // « Non classés », qui ne range que des essais, n'est pas un test.
    children
        .iter()
        .any(|node| matches!(node, TreeNode::Request { contract: None, .. }))
        && !children
            .iter()
            .any(|node| matches!(node, TreeNode::Folder { .. }))
}

/// Test créé : son dossier et son étape 1 (R11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CreatedTest {
    pub folder: String,
    pub step: String,
}

/// Étape d'un test, pour son onglet (4.2-a).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub path: String,
    /// Nom du fichier dans le dossier du test : l'étape requête du schéma qui le joue (V4).
    pub file: String,
    pub name: String,
    pub method: Option<Method>,
    pub url: String,
    pub assertion_count: usize,
    /// Noms des variables extraites par l'étape.
    pub extractions: Vec<String>,
    /// Assertions actives et extractions, pour le panneau de l'étape (4.3-c).
    pub assertions: Vec<Assertion>,
    pub extracts: Vec<Extraction>,
    /// Fichier illisible.
    pub invalid: bool,
}

/// Test ouvert dans un onglet (4.2-a, 4.3-a).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TestView {
    pub path: String,
    pub name: String,
    /// Le dossier est un test ; sinon il range des dossiers.
    pub test: bool,
    /// Requêtes du test, celles de ses attentes comprises.
    pub steps: Vec<StepView>,
    /// Schéma réconcilié (`docs/plan-v4.md`, R2) ; vide pour un dossier qui n'est pas un test.
    pub flow: Flow,
    /// Ce que chaque étape du schéma lit et extrait (4.3-a, 4.3-c).
    pub info: Vec<FlowStepInfo>,
}

/// Requête qui lit une variable (onglet « Variables », EX5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// Étape d'un fichier de requête.
pub fn step_view(root: &Path, path: &str, fallback: &str) -> StepView {
    let request = resolve(root, path).and_then(|p| file::read(&p)).ok();
    let file = path.rsplit('/').next().unwrap_or(path).to_owned();
    match request {
        Some(request) => StepView {
            path: path.to_owned(),
            file,
            name: request.name.clone(),
            method: Some(request.method),
            url: request.url.clone(),
            assertion_count: request.assertions.len(),
            extractions: request
                .extractions
                .iter()
                .filter(|e| e.enabled)
                .map(|e| e.name.clone())
                .collect(),
            assertions: request
                .assertions
                .iter()
                .filter(|a| a.enabled)
                .cloned()
                .collect(),
            extracts: request.extractions.clone(),
            invalid: false,
        },
        None => StepView {
            path: path.to_owned(),
            file,
            name: fallback.to_owned(),
            method: None,
            url: String::new(),
            assertion_count: 0,
            extractions: Vec::new(),
            assertions: Vec::new(),
            extracts: Vec::new(),
            invalid: true,
        },
    }
}

/// Onglet du test `path` : ses étapes dans l'ordre (R1), son schéma (V4).
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
    let mut steps: Vec<StepView> = children
        .iter()
        .filter_map(|node| match node {
            TreeNode::Request { name, path, .. } => Some(step_view(root, path, name)),
            TreeNode::Folder { .. } => None,
        })
        .collect();
    let (flow, info) = if *test || children.is_empty() {
        let ext = Extensions::read(root);
        let flow = test_flow(root, &ext, path, children);
        // Requêtes des attentes : absentes de l'arborescence (R14), montrées dans le test.
        for step in &flow.steps {
            if let Action::Wait {
                file: Some(file), ..
            } = &step.action
                && !steps.iter().any(|s| &s.file == file)
            {
                steps.push(step_view(root, &format!("{path}/{file}"), file));
            }
        }
        let info = flow_info(root, path, &flow);
        (flow, info)
    } else {
        (Flow::default(), Vec::new())
    };
    Ok(TestView {
        path: path.to_owned(),
        name: name.clone(),
        test: *test,
        steps,
        flow,
        info,
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
    create_test_named(root, parent, None)
}

/// « + Test » de la vue d'un dossier (4.1-a bis, AM37) : le test reçoit le nom saisi sur place ;
/// un nom déjà pris est refusé et rien n'est créé. Sans nom, « Nouveau test », « Nouveau test 2 »…
pub fn create_test_named(
    root: &Path,
    parent: &str,
    name: Option<&str>,
) -> Result<CreatedTest, CommandError> {
    let parent_dir = resolve(root, parent)?;
    if !parent_dir.is_dir() {
        return Err(not_found("Le dossier parent n'existe plus."));
    }
    let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => name.to_owned(),
        None => unique_folder_name(&parent_dir),
    };
    let folder = create_folder(root, parent, &name)?;
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

/// Variables extraites par les étapes qui précèdent `path` dans son test
/// (`docs/plan-mvp-e-tests-etapes.md`, R5) : connues de l'étape, jamais inconnues.
pub fn extracted_before(root: &Path, path: &str) -> Vec<String> {
    let Some((folder, _)) = path.rsplit_once('/') else {
        return Vec::new();
    };
    let Ok(view) = test_view(root, folder) else {
        return Vec::new();
    };
    if !view.test {
        return Vec::new();
    }
    let mut names: Vec<String> = Vec::new();
    for step in view.steps.iter().take_while(|s| s.path != path) {
        for name in &step.extractions {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names
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
