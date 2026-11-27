//! Templates (`docs/plan-v4.md`, lot F, R16 à R19) : un test, un dossier ou une arborescence
//! rangé sous `templates/`, avec ses paramètres, réutilisé par des utilisations.
//!
//! Tout est écrit en OpenCollection : le template est un dossier marqué par l'étiquette
//! [`TEMPLATE_MARK`], ses paramètres sont les `request.variables` de son `folder.yml` (valeur par
//! défaut, description ; sans valeur par défaut, un paramètre est obligatoire, Q6) ; une
//! utilisation est un dossier marqué `template:<chemin>`, ses valeurs sont ses `request.variables`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CommandError, CommandErrorKind};
use crate::flow::store as flows;
use crate::opencollection::FOLDER_FILE;
use crate::request::file;
use crate::request::inherit;
use crate::request::model::{AuthKind, BodyKind, ParamKind, Request};
use crate::request::resolve::variable_names;
use crate::tree::{self, TEMPLATE_TAG, TEMPLATES_DIR, TreeNode};

#[cfg(test)]
mod tests;

/// Étiquette du dossier racine d'un template (R16).
pub const TEMPLATE_MARK: &str = "arnod:template";

/// Nature d'un template, déduite de son contenu (R16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum TemplateKind {
    /// Un test : des étapes.
    Test,
    /// Un dossier de tests.
    Folder,
    /// Des dossiers dans des dossiers.
    Tree,
}

/// Paramètre d'un template (4.8-c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TemplateParam {
    pub name: String,
    /// Valeur par défaut ; vide : paramètre obligatoire (Q6).
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub description: String,
}

impl TemplateParam {
    pub fn required(&self) -> bool {
        self.default.trim().is_empty()
    }
}

/// Template listé dans la barre latérale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    /// Chemin du dossier, relatif au projet (`templates/paiements/payer`).
    pub path: String,
    pub name: String,
    pub kind: TemplateKind,
    /// Dossier de rangement sous `templates/` (`paiements`), vide à la racine.
    pub category: String,
    pub params: usize,
    pub folders: usize,
    pub tests: usize,
    pub assertions: usize,
}

/// Utilisation d'un template (4.8-c, « Utilisations »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TemplateUse {
    /// Dossier de l'utilisation, relatif au projet.
    pub path: String,
    pub name: String,
    /// Valeurs saisies dans l'utilisation (les autres reprennent la valeur héritée ou par défaut).
    pub values: BTreeMap<String, String>,
}

/// Template ouvert dans un onglet (4.8-c).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TemplateView {
    #[serde(flatten)]
    pub summary: TemplateSummary,
    pub parameters: Vec<TemplateParam>,
    pub content: Vec<TreeNode>,
    pub uses: Vec<TemplateUse>,
}

/// Origine de la valeur d'un paramètre dans une utilisation (4.8-d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ValueOrigin {
    /// « saisi ici ».
    Here,
    /// « propriété de commandes/v2 ».
    Property { folder: String },
    /// « valeur par défaut ».
    Default,
    /// Obligatoire, sans valeur.
    Missing,
}

/// Valeur d'un paramètre dans une utilisation (4.8-d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ParamValue {
    pub name: String,
    /// Valeur saisie dans l'utilisation, vide sinon.
    pub value: String,
    /// Valeur qui part : saisie, héritée ou par défaut.
    pub effective: String,
    pub origin: ValueOrigin,
    pub default: String,
    pub description: String,
}

/// Utilisation ouverte dans un onglet (4.8-d).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InstanceView {
    pub path: String,
    pub name: String,
    /// Template utilisé ; `None` s'il n'existe plus.
    pub template: Option<TemplateSummary>,
    pub template_path: String,
    pub values: Vec<ParamValue>,
    pub content: Vec<TreeNode>,
}

/// Variable proposée comme paramètre à la création d'un template (4.8-b, R18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TemplateCandidate {
    pub name: String,
    pub value: String,
    /// « propriété du dossier », « hérité de commandes », « environnement ».
    pub origin: String,
    /// Cochée par défaut : propriété du dossier ou héritée ; pas une variable d'environnement.
    pub checked: bool,
}

/// Valeur écrite d'un endpoint, proposée comme paramètre (3.2-d, R19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Literal {
    pub name: String,
    pub value: String,
    /// Où elle est écrite : `body.items[0].sku`, `query.limit`.
    pub location: String,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, message)
}

/// Le dossier `dir` est la racine d'un template.
fn is_template_dir(dir: &Path) -> bool {
    tree::folder_tags_raw(dir)
        .iter()
        .any(|t| t == TEMPLATE_MARK)
}

/// Nœud de chemin `path` dans `nodes`.
pub fn find<'a>(nodes: &'a [TreeNode], path: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        match node {
            TreeNode::Folder {
                path: p, children, ..
            } => {
                if p == path {
                    return Some(node);
                }
                if let Some(found) = find(children, path) {
                    return Some(found);
                }
            }
            TreeNode::Request { path: p, .. } if p == path => return Some(node),
            TreeNode::Request { .. } => {}
        }
    }
    None
}

/// Nœud d'un chemin du projet, templates compris.
pub fn node(root: &Path, path: &str) -> Option<TreeNode> {
    let nodes = if path == TEMPLATES_DIR || path.starts_with(&format!("{TEMPLATES_DIR}/")) {
        tree::read_at(root, TEMPLATES_DIR).ok()?
    } else {
        tree::read(root).ok()?
    };
    find(&nodes, path).cloned()
}

/// Nature, dossiers, tests et assertions d'un contenu.
fn measure(children: &[TreeNode], is_test: bool) -> (TemplateKind, usize, usize, usize) {
    fn walk(nodes: &[TreeNode], folders: &mut usize, tests: &mut usize, assertions: &mut usize) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, test, .. } => {
                    *folders += 1;
                    if *test {
                        *tests += 1;
                    }
                    walk(children, folders, tests, assertions);
                }
                TreeNode::Request {
                    assertion_count, ..
                } => *assertions += assertion_count,
            }
        }
    }
    let (mut folders, mut tests, mut assertions) = (0, 0, 0);
    walk(children, &mut folders, &mut tests, &mut assertions);
    // Une requête hors d'un test compte comme un test (4.8-c : « 4 tests »).
    fn loose(nodes: &[TreeNode]) -> usize {
        nodes
            .iter()
            .map(|n| match n {
                TreeNode::Request { .. } => 1,
                TreeNode::Folder { children, test, .. } => {
                    if *test {
                        0
                    } else {
                        loose(children)
                    }
                }
            })
            .sum()
    }
    let kind = if is_test {
        TemplateKind::Test
    } else if children.iter().any(|c| {
        matches!(c, TreeNode::Folder { children, .. }
            if children.iter().any(|g| matches!(g, TreeNode::Folder { .. })))
    }) {
        TemplateKind::Tree
    } else {
        TemplateKind::Folder
    };
    if is_test {
        tests = 1;
    } else {
        tests += loose(children);
    }
    (kind, folders, tests, assertions)
}

fn summary_of(root: &Path, node: &TreeNode) -> Option<TemplateSummary> {
    let TreeNode::Folder {
        path,
        children,
        test,
        ..
    } = node
    else {
        return None;
    };
    let dir = tree::resolve(root, path).ok()?;
    let name = folder_name(&dir);
    let (kind, folders, tests, assertions) = measure(children, *test);
    let category = path
        .strip_prefix(&format!("{TEMPLATES_DIR}/"))
        .and_then(|rest| rest.rsplit_once('/').map(|(c, _)| c.to_owned()))
        .unwrap_or_default();
    Some(TemplateSummary {
        path: path.clone(),
        name,
        kind,
        category,
        params: tree::folder_variables_raw(&dir).len(),
        folders,
        tests,
        assertions,
    })
}

/// Nom d'un dossier : `info.name` de son `folder.yml`, sinon son nom sur disque.
fn folder_name(dir: &Path) -> String {
    crate::opencollection::yaml::read(&dir.join(FOLDER_FILE))
        .ok()
        .and_then(|v| v.pointer("/info/name")?.as_str().map(str::to_owned))
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
}

/// Templates du projet, dans l'ordre de `templates/` (R16).
pub fn list(root: &Path) -> Vec<TemplateSummary> {
    fn walk(root: &Path, nodes: &[TreeNode], out: &mut Vec<TemplateSummary>) {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                let marked = tree::resolve(root, path).is_ok_and(|dir| is_template_dir(&dir));
                if marked {
                    if let Some(summary) = summary_of(root, node) {
                        out.push(summary);
                    }
                } else {
                    walk(root, children, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(nodes) = tree::read_at(root, TEMPLATES_DIR) {
        walk(root, &nodes, &mut out);
    }
    out
}

/// Dossiers de rangement de `templates/` (« Ranger dans »), sans les templates.
pub fn categories(root: &Path) -> Vec<String> {
    fn walk(root: &Path, nodes: &[TreeNode], out: &mut Vec<String>) {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                if tree::resolve(root, path).is_ok_and(|dir| is_template_dir(&dir)) {
                    continue;
                }
                if let Some(rest) = path.strip_prefix(&format!("{TEMPLATES_DIR}/")) {
                    out.push(rest.to_owned());
                }
                walk(root, children, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(nodes) = tree::read_at(root, TEMPLATES_DIR) {
        walk(root, &nodes, &mut out);
    }
    out
}

/// Paramètres d'un template (R16).
pub fn parameters(root: &Path, path: &str) -> Vec<TemplateParam> {
    tree::resolve(root, path)
        .map(|dir| {
            tree::folder_variables_raw(&dir)
                .into_iter()
                .map(|(name, default, description)| TemplateParam {
                    name,
                    default,
                    description,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Utilisations d'un template dans le référentiel (R17).
pub fn uses(root: &Path, path: &str) -> Vec<TemplateUse> {
    fn walk(root: &Path, nodes: &[TreeNode], template: &str, out: &mut Vec<TemplateUse>) {
        for node in nodes {
            if let TreeNode::Folder {
                path,
                name,
                children,
                template: link,
                ..
            } = node
            {
                if link.as_deref() == Some(template) {
                    let values = tree::resolve(root, path)
                        .map(|dir| {
                            tree::folder_variables_raw(&dir)
                                .into_iter()
                                .filter(|(_, v, _)| !v.trim().is_empty())
                                .map(|(n, v, _)| (n, v))
                                .collect()
                        })
                        .unwrap_or_default();
                    out.push(TemplateUse {
                        path: path.clone(),
                        name: name.clone(),
                        values,
                    });
                }
                walk(root, children, template, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(nodes) = tree::read(root) {
        walk(root, &nodes, path, &mut out);
    }
    out
}

/// Template ouvert (4.8-c).
pub fn read(root: &Path, path: &str) -> Result<TemplateView, CommandError> {
    let node = node(root, path).ok_or_else(|| not_found("Ce template n'existe plus."))?;
    let summary = summary_of(root, &node).ok_or_else(|| not_found("Ce template n'existe plus."))?;
    let content = match node {
        TreeNode::Folder { children, .. } => children,
        TreeNode::Request { .. } => Vec::new(),
    };
    Ok(TemplateView {
        summary,
        parameters: parameters(root, path),
        content,
        uses: uses(root, path),
    })
}

/// Enregistre les paramètres d'un template (4.8-c, « + Paramètre »).
pub fn save_parameters(
    root: &Path,
    path: &str,
    params: &[TemplateParam],
) -> Result<TemplateView, CommandError> {
    let list: Vec<(String, String, String)> = params
        .iter()
        .map(|p| (p.name.clone(), p.default.clone(), p.description.clone()))
        .collect();
    tree::set_folder_variables(root, path, &list)?;
    read(root, path)
}

/// Noms des variables lues par une requête (adresse, paramètres, en-têtes, corps,
/// authentification), sans les valeurs dynamiques.
pub fn used_names(request: &Request) -> BTreeSet<String> {
    let mut texts: Vec<&str> = vec![&request.url];
    for pair in request.params.iter().chain(&request.headers) {
        texts.push(&pair.name);
        texts.push(&pair.value);
    }
    texts.push(request.body.content());
    for field in request.body.fields() {
        texts.push(&field.name);
        texts.push(&field.value);
    }
    if request.auth.kind != AuthKind::Inherit {
        let auth = &request.auth;
        texts.extend([
            auth.username.as_str(),
            auth.password.as_str(),
            auth.token.as_str(),
            auth.name.as_str(),
            auth.value.as_str(),
        ]);
    }
    texts
        .into_iter()
        .flat_map(variable_names)
        .filter(|name| !name.starts_with('$') && !name.is_empty())
        .collect()
}

/// Requêtes d'un ensemble de nœuds.
fn requests(nodes: &[TreeNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            TreeNode::Folder { children, .. } => requests(children, out),
            TreeNode::Request {
                path,
                invalid: false,
                ..
            } => out.push(path.clone()),
            TreeNode::Request { .. } => {}
        }
    }
}

/// Variables proposées comme paramètres d'un template créé depuis `folder` (4.8-b, R18) : celles
/// que lisent ses requêtes, données par une propriété du dossier, héritées d'un dossier parent
/// (cochées) ou de l'environnement `environment` (non cochées). Les valeurs extraites par ses
/// étapes n'en sont pas.
pub fn candidates(
    root: &Path,
    folder: &str,
    environment: &BTreeMap<String, String>,
    environment_name: &str,
) -> Result<Vec<TemplateCandidate>, CommandError> {
    let node = node(root, folder).ok_or_else(|| not_found("Ce dossier n'existe plus."))?;
    let TreeNode::Folder { children, .. } = &node else {
        return Err(invalid("Seul un dossier devient un template."));
    };
    let mut paths = Vec::new();
    requests(children, &mut paths);
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut extracted: BTreeSet<String> = BTreeSet::new();
    for path in &paths {
        if let Ok(request) = tree::resolve(root, path).and_then(|p| file::read(&p)) {
            used.extend(used_names(&request));
            extracted.extend(
                request
                    .extractions
                    .iter()
                    .filter(|e| e.enabled)
                    .map(|e| e.name.trim().to_owned()),
            );
        }
    }
    let inherited = inherit::folder_variables(root, folder);
    let mut out: Vec<TemplateCandidate> = Vec::new();
    // Propriétés et héritées d'abord, dans leur ordre, puis l'environnement.
    for variable in &inherited {
        if !used.contains(&variable.name) || extracted.contains(&variable.name) {
            continue;
        }
        let origin = if variable.folder == folder {
            "propriété du dossier".to_owned()
        } else if variable.folder.is_empty() {
            "variable du projet".to_owned()
        } else {
            format!(
                "hérité de {}",
                variable
                    .folder
                    .rsplit('/')
                    .next()
                    .unwrap_or(&variable.folder)
            )
        };
        out.push(TemplateCandidate {
            name: variable.name.clone(),
            value: variable.value.clone(),
            origin,
            checked: true,
        });
    }
    for name in &used {
        if extracted.contains(name) || out.iter().any(|c| &c.name == name) {
            continue;
        }
        if let Some(value) = environment.get(name) {
            out.push(TemplateCandidate {
                name: name.clone(),
                value: value.clone(),
                origin: format!("environnement {environment_name}"),
                checked: false,
            });
        }
    }
    Ok(out)
}

/// Dossier `templates/<category>` créé au besoin, relatif au projet.
fn category_dir(root: &Path, category: &str) -> Result<String, CommandError> {
    let mut path = TEMPLATES_DIR.to_owned();
    fs::create_dir_all(root.join(TEMPLATES_DIR)).map_err(|error| {
        CommandError::io(
            "Création du dossier impossible",
            &root.join(TEMPLATES_DIR),
            &error,
        )
    })?;
    for part in category.split('/').map(str::trim).filter(|p| !p.is_empty()) {
        let name = tree::item_name(part)?;
        let candidate = format!("{path}/{name}");
        let dir = root.join(&candidate);
        if !dir.is_dir() {
            fs::create_dir_all(&dir).map_err(|error| {
                CommandError::io("Création du dossier impossible", &dir, &error)
            })?;
        }
        path = candidate;
    }
    Ok(path)
}

/// Dossier neuf `name` dans `parent` (relatif au projet), au nom libre.
fn new_dir(root: &Path, parent: &str, name: &str) -> Result<String, CommandError> {
    let name = tree::item_name(name)?;
    let parent_dir = tree::resolve(root, parent)?;
    if fs::read_dir(&parent_dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name))
        })
        .unwrap_or(false)
    {
        return Err(CommandError::new(
            CommandErrorKind::NameTaken,
            format!("« {name} » existe déjà à cet endroit : choisissez un autre nom."),
        ));
    }
    let path = if parent.is_empty() {
        name.clone()
    } else {
        format!("{parent}/{name}")
    };
    let dir = root.join(&path);
    fs::create_dir_all(&dir)
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir, &error))?;
    Ok(path)
}

/// Marque `path` comme racine d'un template nommé `name`, avec ses paramètres.
fn mark_template(
    root: &Path,
    path: &str,
    name: &str,
    params: &[TemplateParam],
) -> Result<(), CommandError> {
    let dir = tree::resolve(root, path)?;
    let mut tags: Vec<String> = tree::folder_tags_raw(&dir)
        .into_iter()
        .filter(|t| t != TEMPLATE_MARK && !t.starts_with(TEMPLATE_TAG))
        .collect();
    tags.push(TEMPLATE_MARK.to_owned());
    let name = name.trim().to_owned();
    tree::edit_folder(root, path, |map| {
        let info = map.entry("info").or_insert_with(|| serde_json::json!({}));
        if let Some(info) = info.as_object_mut() {
            info.insert("name".to_owned(), Value::from(name));
            info.insert("type".to_owned(), Value::from("folder"));
            info.insert("tags".to_owned(), serde_json::json!(tags));
        }
    })?;
    let list: Vec<(String, String, String)> = params
        .iter()
        .map(|p| (p.name.clone(), p.default.clone(), p.description.clone()))
        .collect();
    tree::set_folder_variables(root, path, &list)
}

/// Crée le template `name` depuis le dossier `folder` (4.8-b, R18) : son contenu est copié sous
/// `templates/<category>` ; `params` deviennent ses paramètres, leurs valeurs actuelles les
/// valeurs par défaut. `replace` fait du dossier une utilisation du template, au même résultat.
pub fn create(
    root: &Path,
    folder: &str,
    name: &str,
    category: &str,
    params: &[TemplateParam],
    replace: bool,
) -> Result<TemplateView, CommandError> {
    let source = tree::resolve(root, folder)?;
    if !source.is_dir() || source == root {
        return Err(invalid(
            "Seul un dossier du référentiel devient un template.",
        ));
    }
    let parent = category_dir(root, category)?;
    let target = new_dir(root, &parent, name)?;
    let target_dir = root.join(&target);
    crate::tree::batch::copy_dir(&source, &target_dir)?;
    flows::copy(root, folder, &target)?;
    // Les propriétés du dossier deviennent les paramètres du template (R18).
    mark_template(root, &target, name, params)?;
    if replace {
        // Le contenu part à la corbeille ; le dossier garde son nom et devient une utilisation.
        if let Ok(entries) = fs::read_dir(&source) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().is_some_and(|n| n == FOLDER_FILE) {
                    continue;
                }
                crate::watch::settle(&path);
                trash::delete(&path).map_err(|error| {
                    CommandError::new(
                        CommandErrorKind::Io,
                        format!("Mise à la corbeille impossible : {error}"),
                    )
                })?;
            }
        }
        flows::forget(root, folder)?;
        let mut tags: Vec<String> = tree::folder_tags_raw(&source)
            .into_iter()
            .filter(|t| !t.starts_with(TEMPLATE_TAG) && t != TEMPLATE_MARK)
            .collect();
        tags.push(format!("{TEMPLATE_TAG}{target}"));
        tree::set_folder_tags_raw(root, folder, &tags)?;
        // Les valeurs viennent du template : rien à saisir dans l'utilisation.
        let kept: Vec<(String, String, String)> = tree::folder_variables_raw(&source)
            .into_iter()
            .filter(|(n, _, _)| !params.iter().any(|p| &p.name == n))
            .collect();
        tree::set_folder_variables(root, folder, &kept)?;
    }
    read(root, &target)
}

/// Utilise le template `template` dans le dossier `folder`, sous le nom `name` (4.8-d) : un
/// dossier marqué `template:<chemin>`, sans valeur saisie.
pub fn use_in(
    root: &Path,
    template: &str,
    folder: &str,
    name: &str,
) -> Result<String, CommandError> {
    if node(root, template).is_none() {
        return Err(not_found("Ce template n'existe plus."));
    }
    if folder == TEMPLATES_DIR || folder.starts_with(&format!("{TEMPLATES_DIR}/")) {
        return Err(invalid(
            "Une utilisation se range dans le référentiel de tests, pas parmi les templates.",
        ));
    }
    let path = new_dir(root, folder, name)?;
    let display = name.trim().to_owned();
    tree::edit_folder(root, &path, |map| {
        map.insert(
            "info".to_owned(),
            serde_json::json!({
                "name": display,
                "type": "folder",
                "tags": [format!("{TEMPLATE_TAG}{template}")],
            }),
        );
    })?;
    Ok(path)
}

/// Valeurs des paramètres de l'utilisation `path` (R17) : saisie ici, sinon propriété des
/// dossiers parents, sinon valeur par défaut.
pub fn values(root: &Path, path: &str, template: &str) -> Vec<ParamValue> {
    let here: BTreeMap<String, String> = tree::resolve(root, path)
        .map(|dir| {
            tree::folder_variables_raw(&dir)
                .into_iter()
                .map(|(n, v, _)| (n, v))
                .collect()
        })
        .unwrap_or_default();
    let parent = path.rsplit_once('/').map_or("", |(p, _)| p);
    let inherited = inherit::folder_variables(root, parent);
    parameters(root, template)
        .into_iter()
        .map(|param| {
            let value = here.get(&param.name).cloned().unwrap_or_default();
            let (effective, origin) = if !value.trim().is_empty() {
                (value.clone(), ValueOrigin::Here)
            } else if let Some(found) = inherited
                .iter()
                .find(|v| v.name == param.name && !v.value.trim().is_empty())
            {
                (
                    found.value.clone(),
                    ValueOrigin::Property {
                        folder: found.folder.clone(),
                    },
                )
            } else if !param.default.trim().is_empty() {
                (param.default.clone(), ValueOrigin::Default)
            } else {
                (String::new(), ValueOrigin::Missing)
            };
            ParamValue {
                name: param.name,
                value,
                effective,
                origin,
                default: param.default,
                description: param.description,
            }
        })
        .collect()
}

/// Utilisation ouverte (4.8-d).
pub fn instance(root: &Path, path: &str) -> Result<InstanceView, CommandError> {
    let dir = tree::resolve(root, path)?;
    let template = tree::folder_template(&dir)
        .ok_or_else(|| not_found("Ce dossier n'utilise pas de template."))?;
    let node = node(root, &template);
    let summary = node.as_ref().and_then(|n| summary_of(root, n));
    let content = match node {
        Some(TreeNode::Folder { children, .. }) => children,
        _ => Vec::new(),
    };
    Ok(InstanceView {
        path: path.to_owned(),
        name: folder_name(&dir),
        template: summary,
        template_path: template.clone(),
        values: values(root, path, &template),
        content,
    })
}

/// Enregistre les valeurs saisies dans une utilisation (4.8-d) ; une valeur vide n'est pas
/// écrite (elle reprend la valeur héritée ou par défaut).
pub fn set_values(
    root: &Path,
    path: &str,
    values: &BTreeMap<String, String>,
) -> Result<InstanceView, CommandError> {
    let list: Vec<(String, String, String)> = values
        .iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(n, v)| (n.clone(), v.clone(), String::new()))
        .collect();
    tree::set_folder_variables(root, path, &list)?;
    instance(root, path)
}

/// Valeurs qui partent pour une utilisation ou une étape template : la valeur donnée, sinon celle
/// des propriétés des dossiers parents (`inherited`), sinon la valeur par défaut. Un paramètre
/// obligatoire sans valeur est nommé dans l'erreur (R17).
pub fn effective_values(
    root: &Path,
    template: &str,
    given: &BTreeMap<String, String>,
    inherited: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let mut missing = Vec::new();
    for param in parameters(root, template) {
        let value = given
            .get(&param.name)
            .filter(|v| !v.trim().is_empty())
            .or_else(|| inherited.get(&param.name).filter(|v| !v.trim().is_empty()))
            .cloned()
            .unwrap_or_else(|| param.default.clone());
        if value.trim().is_empty() {
            missing.push(format!("« {} »", param.name));
        } else {
            out.insert(param.name, value);
        }
    }
    if missing.is_empty() {
        Ok(out)
    } else {
        Err(format!(
            "Paramètre{} obligatoire{} sans valeur : {}.",
            if missing.len() > 1 { "s" } else { "" },
            if missing.len() > 1 { "s" } else { "" },
            missing.join(", ")
        ))
    }
}

/// Feuilles d'une valeur JSON : chemin et valeur, chaînes, nombres et booléens seulement.
fn leaves(value: &Value, path: &str, key: &str, out: &mut Vec<(String, String, Value)>) {
    match value {
        Value::Object(map) => {
            for (k, child) in map {
                leaves(child, &format!("{path}.{k}"), k, out);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                leaves(child, &format!("{path}[{i}]"), key, out);
            }
        }
        Value::String(text) if text.contains("{{") => {}
        Value::String(_) | Value::Number(_) | Value::Bool(_) => {
            out.push((path.to_owned(), key.to_owned(), value.clone()));
        }
        Value::Null => {}
    }
}

fn unique(name: &str, taken: &[Literal]) -> String {
    let base = if name.trim().is_empty() {
        "valeur".to_owned()
    } else {
        name.to_owned()
    };
    let mut candidate = base.clone();
    let mut n = 2;
    while taken.iter().any(|l| l.name == candidate) {
        candidate = format!("{base}{n}");
        n += 1;
    }
    candidate
}

fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Valeurs écrites d'une requête (3.2-d, R19) : feuilles du corps JSON et paramètres d'adresse
/// qui ne sont pas des variables.
pub fn literals(request: &Request) -> Vec<Literal> {
    let mut out: Vec<Literal> = Vec::new();
    if request.body.kind == BodyKind::Json
        && let Ok(value) = serde_json::from_str::<Value>(request.body.content())
    {
        let mut found = Vec::new();
        leaves(&value, "body", "", &mut found);
        for (location, key, value) in found {
            let name = unique(&key, &out);
            out.push(Literal {
                name,
                value: text_of(&value),
                location,
            });
        }
    }
    for pair in request
        .params
        .iter()
        .filter(|p| p.enabled && p.kind == ParamKind::Query)
    {
        if pair.value.contains("{{") || pair.name.trim().is_empty() {
            continue;
        }
        let name = unique(pair.name.trim(), &out);
        out.push(Literal {
            name,
            value: pair.value.clone(),
            location: format!("query.{}", pair.name.trim()),
        });
    }
    out
}

/// Remplace dans la requête chaque valeur choisie par `{{nom}}` (R19).
fn parameterize(request: &mut Request, chosen: &[Literal]) {
    if request.body.kind == BodyKind::Json
        && let Ok(mut value) = serde_json::from_str::<Value>(request.body.content())
    {
        let mut markers: Vec<(String, String, bool)> = Vec::new();
        for literal in chosen.iter().filter(|l| l.location.starts_with("body")) {
            let marker = format!("__arnod_param_{}__", markers.len());
            if replace_at(&mut value, &literal.location, &marker) {
                let quoted = serde_json::from_str::<Value>(request.body.content())
                    .ok()
                    .and_then(|v| leaf_at(&v, &literal.location))
                    .is_some_and(|v| v.is_string());
                markers.push((marker, literal.name.clone(), quoted));
            }
        }
        if let Ok(mut text) = serde_json::to_string_pretty(&value) {
            for (marker, name, quoted) in markers {
                let replacement = if quoted {
                    format!("\"{{{{{name}}}}}\"")
                } else {
                    format!("{{{{{name}}}}}")
                };
                text = text.replace(&format!("\"{marker}\""), &replacement);
            }
            let selected = request.body.selected;
            if let Some(variant) = request.body.variants.get_mut(selected) {
                variant.content = text;
            }
        }
    }
    for literal in chosen.iter().filter(|l| l.location.starts_with("query.")) {
        let key = literal.location.trim_start_matches("query.");
        for pair in request
            .params
            .iter_mut()
            .filter(|p| p.kind == ParamKind::Query && p.name.trim() == key)
        {
            pair.value = format!("{{{{{}}}}}", literal.name);
        }
    }
}

/// Segments d'un chemin `body.a[0].b`.
fn segments(location: &str) -> Vec<String> {
    let rest = location.strip_prefix("body").unwrap_or(location);
    let mut out = Vec::new();
    for part in rest.split('.').filter(|p| !p.is_empty()) {
        let mut current = part;
        while let Some(open) = current.find('[') {
            if open > 0 {
                out.push(current[..open].to_owned());
            }
            let close = current[open..]
                .find(']')
                .map_or(current.len(), |c| open + c);
            out.push(format!("#{}", &current[open + 1..close]));
            current = &current[(close + 1).min(current.len())..];
        }
        if !current.is_empty() {
            out.push(current.to_owned());
        }
    }
    out
}

fn leaf_at(value: &Value, location: &str) -> Option<Value> {
    let mut current = value;
    for segment in segments(location) {
        current = match segment.strip_prefix('#') {
            Some(index) => current.get(index.parse::<usize>().ok()?)?,
            None => current.get(segment.as_str())?,
        };
    }
    Some(current.clone())
}

fn replace_at(value: &mut Value, location: &str, marker: &str) -> bool {
    let mut current = value;
    for segment in segments(location) {
        let next = match segment.strip_prefix('#') {
            Some(index) => match index.parse::<usize>() {
                Ok(i) => current.get_mut(i),
                Err(_) => None,
            },
            None => current.get_mut(segment.as_str()),
        };
        let Some(next) = next else { return false };
        current = next;
    }
    *current = Value::String(marker.to_owned());
    true
}

/// Transfère l'endpoint `path` vers les templates (3.2-d, R19) : un template test `name` sous
/// `templates/<category>`, dont `chosen` (noms des valeurs écrites) deviennent les paramètres ;
/// la requête quitte le contrat et le référentiel.
pub fn from_request(
    root: &Path,
    path: &str,
    name: &str,
    category: &str,
    chosen: &[String],
) -> Result<TemplateView, CommandError> {
    let source = tree::resolve(root, path)?;
    let mut request = file::read(&source)?;
    let all = literals(&request);
    let picked: Vec<Literal> = all
        .iter()
        .filter(|l| chosen.iter().any(|c| c == &l.name))
        .cloned()
        .collect();
    let parent = category_dir(root, category)?;
    let target = new_dir(root, &parent, name)?;
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{}.yml", tree::slug(&request.name)));
    let target_file = root.join(&target).join(&file_name);
    file::copy_as(&source, &target_file, &request.name)?;
    parameterize(&mut request, &picked);
    // Une étape n'est jamais liée à un contrat ; le template non plus (R19).
    request.contract = None;
    file::write(&target_file, &request)?;
    let params: Vec<TemplateParam> = picked
        .iter()
        .map(|l| TemplateParam {
            name: l.name.clone(),
            default: l.value.clone(),
            description: String::new(),
        })
        .collect();
    mark_template(root, &target, name, &params)?;
    crate::watch::settle(&source);
    trash::delete(&source).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Mise à la corbeille impossible : {error}"),
        )
    })?;
    read(root, &target)
}
