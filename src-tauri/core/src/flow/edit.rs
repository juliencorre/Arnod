//! Modifications d'un schéma depuis l'onglet du test (`docs/plan-v4.md`, R3) : insérer une étape
//! sur un lien, en fin de liste ou libre ; lire le schéma d'un dossier ; ce que chaque étape lit et
//! extrait (4.3-a « Données », 4.3-c « Entrées »).

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::reconcile::{normalize, reconcile};
use super::store::Extensions;
use super::{Action, Check, END, Flow, FolderHooks, HookPhase, Step, StepKind};
use crate::error::{CommandError, CommandErrorKind};
use crate::request::file;
use crate::request::resolve::variable_names;
use crate::template::used_names;
use crate::tree::{self, TreeNode};

/// Lien sur lequel une étape est insérée (4.3-b bis, 4.3-b ter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "StepPlacement"))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Placement {
    /// À la fin de la liste, à la suite de la dernière étape (« + Requête » de la liste).
    End,
    /// Dans le vide : une étape libre, à relier (4.3-b ter).
    Free,
    /// Entre « Départ » et la première étape.
    Start,
    /// Sur la suite de l'étape `after` (« puis », « si oui ») ou sur son « sinon ».
    Link { after: String, otherwise: bool },
}

/// Entrée d'une étape : une variable qu'elle lit, et l'étape qui la donne (4.3-c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StepInput {
    pub name: String,
    /// Étape qui la donne ; `None` : l'environnement ou une propriété.
    pub from: Option<String>,
    /// `body.id`, « valeur », « élément de la boucle ».
    pub source: String,
}

/// Ce qu'une étape lit et extrait (4.3-a « utilise … · extrait … »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FlowStepInfo {
    pub id: String,
    pub inputs: Vec<StepInput>,
    pub extracts: Vec<String>,
}

/// Fichiers de requête candidats d'un test, dans l'ordre du dossier (sans contrat).
pub fn files_of(children: &[TreeNode]) -> Vec<String> {
    children
        .iter()
        .filter_map(|node| match node {
            TreeNode::Request {
                path,
                contract: None,
                invalid: false,
                ..
            } => Some(path.rsplit('/').next().unwrap_or(path).to_owned()),
            _ => None,
        })
        .collect()
}

/// Schéma réconcilié du test `path` (R2) ; les requêtes de ses attentes, absentes de
/// l'arborescence (R14), restent les siennes.
pub fn test_flow(root: &Path, ext: &Extensions, path: &str, children: &[TreeNode]) -> Flow {
    let stored = ext.flow(path);
    let mut files = files_of(children);
    if let Some(stored) = &stored {
        for step in &stored.steps {
            if let Action::Wait {
                file: Some(file), ..
            } = &step.action
                && !files.contains(file)
                && tree::resolve(root, &format!("{path}/{file}")).is_ok_and(|p| p.is_file())
            {
                files.push(file.clone());
            }
        }
    }
    reconcile(stored.as_ref(), &files, &ext.hook_files(path))
}

/// Étape neuve d'une nature, à configurer (4.3-b quater) ; `file` pour une requête ou une
/// attente.
pub fn new_step(flow: &Flow, kind: StepKind, file: Option<String>) -> Step {
    let action = match kind {
        StepKind::Request => Action::Request {
            file: file.clone().unwrap_or_default(),
        },
        StepKind::Condition => Action::Condition {
            check: Check::new(),
            otherwise: None,
        },
        StepKind::Loop => Action::Loop {
            count: Some(3),
            list: None,
            item: "item".to_owned(),
            until: None,
        },
        StepKind::Wait => Action::Wait {
            file,
            check: Check::new(),
            interval_ms: 2_000,
            attempts: 5,
        },
        StepKind::Variable => Action::Variable {
            variable: String::new(),
            value: String::new(),
        },
        StepKind::Script => Action::Script {
            code: "// vars, response et assert() sont à disposition.\nassert(response.status === 200, \"statut 200\");\n".to_owned(),
        },
        StepKind::Test => Action::Test {
            test: String::new(),
        },
        StepKind::Template => Action::Template {
            template: String::new(),
            values: BTreeMap::new(),
        },
    };
    let name = match kind {
        StepKind::Request => String::new(),
        StepKind::Condition => "Nouvelle condition".to_owned(),
        StepKind::Loop => "Nouvelle boucle".to_owned(),
        StepKind::Wait => "Nouvelle attente".to_owned(),
        StepKind::Variable => "Nouvelle variable".to_owned(),
        StepKind::Script => "Nouveau script".to_owned(),
        StepKind::Test => "Test existant".to_owned(),
        StepKind::Template => "Template".to_owned(),
    };
    Step {
        id: flow.free_id(),
        name,
        action,
        next: None,
        on_failure: None,
    }
}

/// Insère `step` dans `flow` (R3) : sur un lien, le lien est reconnecté (l'étape d'avant renvoie
/// vers la nouvelle, la nouvelle vers celle d'après) ; en fin de liste, à la suite de la dernière ;
/// libre, sans rien changer aux autres.
pub fn insert(flow: &mut Flow, mut step: Step, placement: &Placement) {
    match placement {
        Placement::Start => {
            flow.steps.insert(0, step);
        }
        Placement::End => flow.steps.push(step),
        Placement::Free => {
            // La dernière étape garde sa fin ; la nouvelle ne suit aucune autre.
            if let Some(last) = flow.steps.last_mut()
                && last.next.is_none()
            {
                last.next = Some(END.to_owned());
            }
            step.next = Some(END.to_owned());
            flow.steps.push(step);
        }
        Placement::Link { after, otherwise } => {
            let Some(at) = flow.position(after) else {
                flow.steps.push(step);
                normalize(flow);
                return;
            };
            let id = step.id.clone();
            let following = flow.following(at);
            if *otherwise {
                let old = match &flow.steps[at].action {
                    Action::Condition {
                        otherwise: Some(target),
                        ..
                    } => target.clone(),
                    _ => following.clone(),
                };
                let source = &mut flow.steps[at];
                // « Si oui » garde sa suite, écrite puisque la nouvelle étape vient juste après.
                source.next = Some(following);
                if let Action::Condition { otherwise, .. } = &mut source.action {
                    *otherwise = Some(id);
                }
                step.next = Some(old);
            } else {
                let source = &mut flow.steps[at];
                if source.next.is_some() {
                    source.next = Some(id);
                }
                step.next = Some(following);
            }
            flow.steps.insert(at + 1, step);
        }
    }
    normalize(flow);
}

/// Retire l'étape `id` : ses renvois passent à sa suite (R2, R3).
pub fn remove(flow: &mut Flow, id: &str) {
    let Some(at) = flow.position(id) else { return };
    let following = flow.following(at);
    flow.steps.remove(at);
    for step in &mut flow.steps {
        if step.next.as_deref() == Some(id) {
            step.next = Some(following.clone());
        }
        match &mut step.action {
            Action::Condition {
                otherwise: Some(target),
                ..
            } if target == id => *target = following.clone(),
            Action::Loop {
                until: Some(target),
                ..
            } if target == id => {
                *target = following.clone();
            }
            _ => {}
        }
    }
    normalize(flow);
}

/// Ce que chaque étape lit et extrait, dans l'ordre du schéma (4.3-a, 4.3-c).
pub fn info(root: &Path, folder: &str, flow: &Flow) -> Vec<FlowStepInfo> {
    // Variable → (étape qui la donne, source).
    let mut provided: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut out = Vec::new();
    for step in &flow.steps {
        let mut reads: Vec<String> = Vec::new();
        let mut extracts: Vec<(String, String)> = Vec::new();
        if let Some(file_name) = step.file()
            && let Ok(request) =
                tree::resolve(root, &format!("{folder}/{file_name}")).and_then(|p| file::read(&p))
        {
            reads.extend(used_names(&request));
            extracts.extend(
                request
                    .extractions
                    .iter()
                    .filter(|e| e.enabled && !e.name.trim().is_empty())
                    .map(|e| (e.name.trim().to_owned(), e.source.trim().to_owned())),
            );
        }
        match &step.action {
            Action::Variable { variable, value } => {
                reads.extend(variable_names(value));
                if !variable.trim().is_empty() {
                    extracts.push((variable.trim().to_owned(), "valeur".to_owned()));
                }
            }
            Action::Condition { check, .. } | Action::Wait { check, .. } => {
                if let Some(name) = super::check::variable_of(&check.target) {
                    reads.push(name.to_owned());
                }
            }
            Action::Loop { list, item, .. } => {
                if let Some(name) = list.as_deref().and_then(super::check::variable_of) {
                    reads.push(name.to_owned());
                }
                let name = if item.trim().is_empty() {
                    "item"
                } else {
                    item.trim()
                };
                extracts.push((name.to_owned(), "élément de la boucle".to_owned()));
            }
            Action::Template { values, .. } => {
                for value in values.values() {
                    reads.extend(variable_names(value));
                }
            }
            _ => {}
        }
        let mut seen = HashSet::new();
        let inputs: Vec<StepInput> = reads
            .into_iter()
            .filter(|name| !name.starts_with('$') && seen.insert(name.clone()))
            .filter_map(|name| {
                provided.get(&name).map(|(from, source)| StepInput {
                    name: name.clone(),
                    from: Some(from.clone()),
                    source: source.clone(),
                })
            })
            .collect();
        for (name, source) in &extracts {
            provided.insert(name.clone(), (step.id.clone(), source.clone()));
        }
        out.push(FlowStepInfo {
            id: step.id.clone(),
            inputs,
            extracts: extracts.into_iter().map(|(name, _)| name).collect(),
        });
    }
    out
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Fichiers d'un dossier de test, attentes comprises, d'après le dossier sur disque.
fn folder_files(
    root: &Path,
    ext: &Extensions,
    path: &str,
) -> Result<(Vec<String>, Vec<TreeNode>), CommandError> {
    let nodes = tree::read(root)?;
    let children = match crate::template::find(&nodes, path) {
        Some(TreeNode::Folder { children, .. }) => children.clone(),
        _ => {
            return Err(CommandError::new(
                CommandErrorKind::NotFound,
                "Ce test n'existe plus.",
            ));
        }
    };
    let mut files = files_of(&children);
    for (_, step) in ext.hooks(path).steps() {
        if let Some(file) = step.file() {
            files.retain(|f| f != file);
        }
    }
    if let Some(stored) = ext.flow(path) {
        for step in &stored.steps {
            if let Action::Wait {
                file: Some(file), ..
            } = &step.action
                && !files.contains(file)
                && tree::resolve(root, &format!("{path}/{file}")).is_ok_and(|p| p.is_file())
            {
                files.push(file.clone());
            }
        }
    }
    Ok((files, children))
}

/// Enregistre le schéma du test `path` (R1) : vérifié, ses requêtes rangées dans son ordre
/// (`info.seq`), écrit dans `extensions.arnod` s'il n'est pas linéaire.
pub fn save(root: &Path, path: &str, flow: &Flow) -> Result<(), CommandError> {
    let ext = Extensions::read(root);
    let (files, _) = folder_files(root, &ext, path)?;
    let mut flow = flow.clone();
    normalize(&mut flow);
    super::reconcile::validate(&flow, &files).map_err(invalid)?;
    let order: Vec<String> = super::reconcile::files_in_order(&flow)
        .into_iter()
        .map(|file| format!("{path}/{file}"))
        .collect();
    tree::reorder(root, path, &order)?;
    let requests: Vec<String> = flow
        .steps
        .iter()
        .filter_map(|s| match &s.action {
            Action::Request { file } => Some(file.clone()),
            _ => None,
        })
        .collect();
    let linear = flow.is_linear(&requests);
    super::store::save_flow(root, path, (!linear).then_some(&flow))
}

/// Étape ajoutée et son identifiant.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StepAdded {
    pub step: String,
    /// Requête créée pour l'étape (requête, attente), relative au projet.
    pub request: Option<String>,
}

/// Nom d'une requête créée pour une étape.
fn request_name(kind: StepKind, name: Option<&str>) -> String {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => name.to_owned(),
        None if kind == StepKind::Wait => "Requête de l'attente".to_owned(),
        None => "Nouvelle requête".to_owned(),
    }
}

/// Requête neuve au nom libre dans `folder`.
fn create_free_request(root: &Path, folder: &str, base: &str) -> Result<String, CommandError> {
    let mut name = base.to_owned();
    let mut n = 2;
    loop {
        match tree::create_request(root, folder, Some(&name)) {
            Ok(path) => return Ok(path),
            Err(error) if error.kind == CommandErrorKind::NameTaken && n < 100 => {
                name = format!("{base} {n}");
                n += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

/// Ajoute une étape de nature `kind` au test `path` (4.3-e, 4.3-b bis) : une requête ou une
/// attente reçoit sa requête neuve dans le dossier ; le lien choisi est reconnecté (R3).
pub fn add(
    root: &Path,
    path: &str,
    kind: StepKind,
    placement: &Placement,
    name: Option<&str>,
) -> Result<StepAdded, CommandError> {
    let ext = Extensions::read(root);
    let (_, children) = folder_files(root, &ext, path)?;
    let mut flow = test_flow(root, &ext, path, &children);
    let request = match kind {
        StepKind::Request | StepKind::Wait => {
            Some(create_free_request(root, path, &request_name(kind, name))?)
        }
        _ => None,
    };
    let file = request
        .as_deref()
        .map(|p| p.rsplit('/').next().unwrap_or(p).to_owned());
    let mut step = new_step(&flow, kind, file);
    if kind == StepKind::Wait
        && let Some(name) = name.map(str::trim).filter(|n| !n.is_empty())
    {
        step.name = name.to_owned();
    }
    let id = step.id.clone();
    insert(&mut flow, step, placement);
    save(root, path, &flow)?;
    Ok(StepAdded { step: id, request })
}

/// Retire l'étape `id` du test `path` (« Supprimer l'étape », « Annuler l'insertion », 4.3-b
/// quater) : ses renvois passent à sa suite (R3) ; sa requête va à la corbeille.
pub fn remove_step(root: &Path, path: &str, id: &str) -> Result<(), CommandError> {
    let ext = Extensions::read(root);
    let (_, children) = folder_files(root, &ext, path)?;
    let mut flow = test_flow(root, &ext, path, &children);
    let Some(step) = flow.steps.iter().find(|s| s.id == id).cloned() else {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Cette étape n'existe plus.",
        ));
    };
    remove(&mut flow, id);
    save(root, path, &flow)?;
    if let Some(file) = step.file() {
        let request = format!("{path}/{file}");
        if tree::resolve(root, &request).is_ok_and(|p| p.is_file()) {
            tree::delete(root, &request, false)?;
        }
    }
    Ok(())
}

/// Retire l'étape `id` de la phase `phase` du dossier `path` ; sa requête va à la corbeille.
pub fn remove_hook(
    root: &Path,
    path: &str,
    phase: HookPhase,
    id: &str,
) -> Result<HooksView, CommandError> {
    let mut hooks = Extensions::read(root).hooks(path);
    let steps = hooks.phase_mut(phase);
    let Some(at) = steps.iter().position(|s| s.id == id) else {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Cette étape n'existe plus.",
        ));
    };
    let step = steps.remove(at);
    let view = save_hooks(root, path, &hooks)?;
    if let Some(file) = step.file() {
        let request = format!("{path}/{file}");
        if tree::resolve(root, &request).is_ok_and(|p| p.is_file()) {
            tree::delete(root, &request, false)?;
        }
    }
    Ok(HooksView {
        requests: view
            .requests
            .into_iter()
            .filter(|r| Some(r.file.as_str()) != step.file())
            .collect(),
        ..view
    })
}

/// Avant / Après d'un dossier, pour sa vue (4.7-c).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HooksView {
    pub folder: String,
    pub hooks: FolderHooks,
    /// Requêtes jouées par les étapes (rangées dans le dossier, absentes de l'arborescence).
    pub requests: Vec<tree::StepView>,
}

/// Avant / Après du dossier `path` (4.7-c).
pub fn hooks_view(root: &Path, path: &str) -> HooksView {
    let hooks = Extensions::read(root).hooks(path);
    let requests = hooks
        .steps()
        .filter_map(|(_, step)| step.file().map(str::to_owned))
        .map(|file| tree::step_view(root, &format!("{path}/{file}"), &file))
        .collect();
    HooksView {
        folder: path.to_owned(),
        hooks,
        requests,
    }
}

/// Enregistre les Avant / Après du dossier `path` : chaque étape vérifiée comme dans un test.
pub fn save_hooks(root: &Path, path: &str, hooks: &FolderHooks) -> Result<HooksView, CommandError> {
    let dir = tree::resolve(root, path)?;
    if !dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Ce dossier n'existe plus.",
        ));
    }
    for phase in HookPhase::ALL {
        let steps = hooks.phase(phase);
        let files: Vec<String> = steps
            .iter()
            .filter_map(|s| s.file().map(str::to_owned))
            .filter(|f| dir.join(f).is_file())
            .collect();
        let flow = Flow {
            on_failure: Default::default(),
            steps: steps
                .iter()
                .cloned()
                .map(|mut s| {
                    s.next = None;
                    s
                })
                .collect(),
        };
        super::reconcile::validate(&flow, &files)
            .map_err(|reason| invalid(format!("{} : {reason}", phase.label())))?;
    }
    let mut clean = hooks.clone();
    for phase in HookPhase::ALL {
        for step in clean.phase_mut(phase) {
            step.next = None;
            step.on_failure = None;
        }
    }
    super::store::save_hooks(root, path, &clean)?;
    Ok(hooks_view(root, path))
}

/// Ajoute une étape de nature `kind` à la phase `phase` du dossier `path` (« + Étape », 4.7-c).
pub fn add_hook(
    root: &Path,
    path: &str,
    phase: HookPhase,
    kind: StepKind,
    name: Option<&str>,
) -> Result<HooksView, CommandError> {
    let mut hooks = Extensions::read(root).hooks(path);
    let request = match kind {
        StepKind::Request | StepKind::Wait => {
            Some(create_free_request(root, path, &request_name(kind, name))?)
        }
        _ => None,
    };
    let file = request
        .as_deref()
        .map(|p| p.rsplit('/').next().unwrap_or(p).to_owned());
    let taken: Vec<String> = hooks.steps().map(|(_, s)| s.id.clone()).collect();
    let probe = Flow {
        on_failure: Default::default(),
        steps: hooks.steps().map(|(_, s)| s.clone()).collect(),
    };
    let mut step = new_step(&probe, kind, file);
    let mut n = 1;
    while taken.contains(&step.id) {
        step.id = format!("h{n}");
        n += 1;
    }
    hooks.phase_mut(phase).push(step);
    save_hooks(root, path, &hooks)
}

/// Origine d'une propriété (4.8-a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PropertyOrigin {
    /// « défini ici ».
    Here,
    /// « hérité de commandes ».
    Folder { folder: String },
    /// Variable de la collection.
    Project,
    /// « environnement Recette fonctionnelle ».
    Environment { name: String },
}

/// Propriété d'un dossier, avec son origine (4.8-a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FolderProperty {
    pub name: String,
    pub value: String,
    pub origin: PropertyOrigin,
}

/// Propriétés du dossier `path` (4.8-a) : définies ici, puis héritées (du dossier le plus proche
/// à la collection), puis les variables de l'environnement `environment` que lisent ses requêtes.
pub fn properties(
    root: &Path,
    path: &str,
    environment: &str,
    environment_values: &BTreeMap<String, String>,
) -> Vec<FolderProperty> {
    let mut out: Vec<FolderProperty> = Vec::new();
    for variable in crate::request::inherit::folder_variables(root, path) {
        let origin = if variable.folder == path {
            PropertyOrigin::Here
        } else if variable.folder.is_empty() {
            PropertyOrigin::Project
        } else {
            PropertyOrigin::Folder {
                folder: variable.folder.clone(),
            }
        };
        out.push(FolderProperty {
            name: variable.name,
            value: variable.value,
            origin,
        });
    }
    // Variables de l'environnement lues par les requêtes du dossier.
    let mut used = std::collections::BTreeSet::new();
    if let Some(node) = crate::template::node(root, path) {
        let mut paths = Vec::new();
        fn walk(nodes: &[TreeNode], out: &mut Vec<String>) {
            for node in nodes {
                match node {
                    TreeNode::Folder { children, .. } => walk(children, out),
                    TreeNode::Request { path, .. } => out.push(path.clone()),
                }
            }
        }
        walk(std::slice::from_ref(&node), &mut paths);
        for request_path in paths {
            if let Ok(request) = tree::resolve(root, &request_path).and_then(|p| file::read(&p)) {
                used.extend(used_names(&request));
            }
        }
    }
    for name in used {
        if out.iter().any(|p| p.name == name) {
            continue;
        }
        if let Some(value) = environment_values.get(&name) {
            out.push(FolderProperty {
                name,
                value: value.clone(),
                origin: PropertyOrigin::Environment {
                    name: environment.to_owned(),
                },
            });
        }
    }
    out
}
