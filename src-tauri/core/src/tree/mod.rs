//! Arborescence d'un projet : dossiers réels et fichiers de requête (R1 à R5).
//!
//! Module sans Tauri ni état : il travaille sur le dossier du projet. Les
//! chemins échangés sont relatifs à la racine du projet, séparés par `/`.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::catalog::names::{self, sort_key};
use crate::contract::CONTRACTS_DIR;
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::request::is_internal_tag;
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{FOLDER_FILE, ROOT_FILE, yaml};
use crate::request::file;
use crate::request::model::{Method, Request};

/// Dossier réservé aux environnements, à la racine du projet.
pub const ENVIRONMENTS_DIR: &str = "environments";
/// Dossier réservé aux templates, à la racine du projet (`docs/plan-v4.md`, R20).
pub const TEMPLATES_DIR: &str = "templates";
/// Dossier réservé aux jeux de données d'Arnod, à la racine du projet (R11, R20).
pub const DATA_DIR: &str = "données";

/// Dossiers réservés à la racine du projet : absents de l'arborescence, refusés comme nom.
pub const RESERVED_DIRS: [&str; 4] = [ENVIRONMENTS_DIR, CONTRACTS_DIR, TEMPLATES_DIR, DATA_DIR];

/// Nœud de l'arborescence affichée dans la barre latérale (A1).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TreeNode {
    Folder {
        name: String,
        path: String,
        children: Vec<TreeNode>,
        /// Nombre de requêtes dans le dossier, sous-dossiers compris.
        request_count: usize,
        /// Test : au moins une requête et aucun sous-dossier
        /// (`docs/plan-mvp-e-tests-etapes.md`, R1).
        test: bool,
        /// Étiquettes du dossier (`info.tags` de son `folder.yml`) : elles valent pour tout ce
        /// qu'il contient (`docs/plan-v2.md`, R4).
        tags: Vec<String>,
        /// Utilisation d'un template : son chemin (étiquette `template:`, `docs/plan-v4.md`, R17).
        template: Option<String>,
    },
    Request {
        name: String,
        path: String,
        method: Option<Method>,
        assertion_count: usize,
        /// Fichier illisible : listé sans bloquer le reste (A10).
        invalid: bool,
        /// Contrat lié (`contracts/petstore.yaml`), d'après l'étiquette `contract:`.
        contract: Option<String>,
        /// Adresse de la requête, pour le filtre de la barre latérale
        /// (docs/plan-mvp-d-barre-laterale.md, R1) ; vide pour un fichier illisible.
        url: String,
        /// Étiquettes effectives, sans celle du lien au contrat (R1) : les siennes, puis celles
        /// de ses dossiers parents (`docs/plan-v2.md`, R4).
        tags: Vec<String>,
    },
}

impl TreeNode {
    fn count(&self) -> usize {
        match self {
            Self::Folder { request_count, .. } => *request_count,
            Self::Request { .. } => 1,
        }
    }
}

fn invalid_request(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn is_request_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

/// Lit l'arborescence du projet (R1) : dossiers d'abord, puis requêtes, chacun
/// par nom sans tenir compte de la casse ; éléments cachés, racine de la
/// collection et réglages de dossier (`folder.yml`) ignorés (R7 du format).
pub fn read(root: &Path) -> Result<Vec<TreeNode>, CommandError> {
    // Requêtes propres à une étape (Avant / Après, attente) : absentes de l'arborescence (R14).
    let owned = crate::flow::store::Extensions::read(root).owned_files();
    let mut nodes = read_dir(root, root, true, &mut vec![canonical(root)], &owned)?;
    inherit_tags(&mut nodes, &[]);
    Ok(nodes)
}

/// Arborescence d'un dossier réservé ou d'un sous-dossier (`templates/…`, V4) : chemins relatifs
/// au projet, étiquettes héritées depuis ce dossier.
pub fn read_at(root: &Path, folder: &str) -> Result<Vec<TreeNode>, CommandError> {
    let dir = resolve(root, folder)?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let owned = crate::flow::store::Extensions::read(root).owned_files();
    let mut nodes = read_dir(root, &dir, false, &mut vec![canonical(&dir)], &owned)?;
    inherit_tags(&mut nodes, &[]);
    Ok(nodes)
}

/// Étiquettes effectives (`docs/plan-v2.md`, R4) : une requête garde les siennes, puis prend
/// celles de chacun de ses dossiers parents, sans doublon (sans tenir compte de la casse).
fn inherit_tags(nodes: &mut [TreeNode], inherited: &[String]) {
    for node in nodes {
        match node {
            TreeNode::Folder { children, tags, .. } => {
                let mut below = inherited.to_vec();
                add_tags(&mut below, tags);
                inherit_tags(children, &below);
            }
            TreeNode::Request { tags, .. } => add_tags(tags, inherited),
        }
    }
}

fn add_tags(into: &mut Vec<String>, more: &[String]) {
    for tag in more {
        if !into.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            into.push(tag.clone());
        }
    }
}

/// Étiquettes écrites d'un dossier : `info.tags` de son `folder.yml`, internes comprises.
pub fn folder_tags_raw(dir: &Path) -> Vec<String> {
    yaml::read(&dir.join(FOLDER_FILE))
        .ok()
        .and_then(|value| {
            value.get("info")?.get("tags")?.as_array().map(|tags| {
                tags.iter()
                    .filter_map(|t| t.as_str().map(str::to_owned))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Étiquettes d'un dossier, sans celles propres à Arnod (R20 du plan V4).
fn folder_tags(dir: &Path) -> Vec<String> {
    folder_tags_raw(dir)
        .into_iter()
        .filter(|tag| !is_internal_tag(tag))
        .collect()
}

/// Template d'une utilisation : étiquette `template:<chemin>` du dossier (R17 du plan V4).
pub fn folder_template(dir: &Path) -> Option<String> {
    folder_tags_raw(dir)
        .into_iter()
        .find_map(|tag| tag.strip_prefix(TEMPLATE_TAG).map(str::to_owned))
}

/// Étiquette d'une utilisation de template.
pub const TEMPLATE_TAG: &str = "template:";

/// Étiquettes d'un dossier (`docs/plan-v2.md`, R4, ET1) : écrites dans `info.tags` de son
/// `folder.yml`, créé conforme au besoin ; sans étiquette, la clé est retirée. Les étiquettes
/// sont nettoyées (espaces, doublons sans tenir compte de la casse, lien au contrat refusé).
pub fn set_folder_tags(root: &Path, path: &str, tags: &[String]) -> Result<(), CommandError> {
    let dir = resolve(root, path)?;
    if !dir.is_dir() || dir == root {
        return Err(invalid_request(
            "Seul un dossier du projet reçoit des étiquettes.",
        ));
    }
    let mut clean: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() || is_internal_tag(tag) {
            continue;
        }
        add_tags(&mut clean, &[tag.to_owned()]);
    }
    // Les étiquettes propres à Arnod (template, utilisation) restent (R20 du plan V4).
    clean.extend(
        folder_tags_raw(&dir)
            .into_iter()
            .filter(|t| is_internal_tag(t)),
    );
    let file = dir.join(FOLDER_FILE);
    let mut value = if file.exists() {
        yaml::read(&file)?
    } else {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        serde_json::json!({ "info": { "name": name, "type": "folder" } })
    };
    let Some(info) = value
        .as_object_mut()
        .map(|object| {
            object
                .entry("info")
                .or_insert_with(|| serde_json::json!({}))
        })
        .and_then(serde_json::Value::as_object_mut)
    else {
        return Err(invalid_request(
            "Le fichier folder.yml de ce dossier est illisible.",
        ));
    };
    if clean.is_empty() {
        info.remove("tags");
    } else {
        info.insert("tags".to_owned(), serde_json::json!(clean));
    }
    // Un contenu non conforme n'est jamais écrit (format OpenCollection strict).
    schema::check(Part::Folder, &value).map_err(|error| {
        invalid_request(format!(
            "Étiquettes non conformes au format OpenCollection : {error}"
        ))
    })?;
    yaml::write(&file, &value)
}

/// Chemin réel d'un dossier, liens suivis ; tel quel s'il ne se résout pas.
fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `ancestors` : chemins réels du dossier lu et de ses parents, pour ne pas
/// suivre un lien qui y ramène (boucle sans fin).
fn read_dir(
    root: &Path,
    dir: &Path,
    top: bool,
    ancestors: &mut Vec<PathBuf>,
    owned: &HashSet<String>,
) -> Result<Vec<TreeNode>, CommandError> {
    let entries = fs::read_dir(dir)
        .map_err(|error| CommandError::io("Lecture du dossier impossible", dir, &error))?;
    // Dossiers et requêtes avec leur rang (`info.seq` d'une requête ou du `folder.yml` d'un
    // dossier) : il décide de l'ordre des étapes (R1) et de celui de la vue d'un dossier (AM38).
    let mut folders: Vec<(Option<f64>, TreeNode)> = Vec::new();
    let mut requests: Vec<(Option<f64>, TreeNode)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || (top && name == ROOT_FILE) || name == FOLDER_FILE {
            continue;
        }
        let path = entry.path();
        let relative = relative_path(root, &path);
        if path.is_dir() {
            if top && RESERVED_DIRS.contains(&name.as_str()) {
                continue;
            }
            // Seul un lien demande une résolution ; un dossier ordinaire prolonge son parent.
            let real = match ancestors.last() {
                Some(parent) if !entry.file_type().is_ok_and(|kind| kind.is_symlink()) => {
                    parent.join(entry.file_name())
                }
                _ => canonical(&path),
            };
            if ancestors.contains(&real) {
                continue;
            }
            ancestors.push(real);
            let children = read_dir(root, &path, false, ancestors, owned);
            ancestors.pop();
            let children = children?;
            let request_count = children.iter().map(TreeNode::count).sum();
            let test = is_test(&children);
            folders.push((
                folder_seq(&path),
                TreeNode::Folder {
                    name,
                    path: relative,
                    children,
                    request_count,
                    test,
                    tags: folder_tags(&path),
                    template: folder_template(&path),
                },
            ));
        } else if is_request_file(&path) && !owned.contains(&relative) {
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            requests.push(match file::read(&path) {
                Ok(request) => (
                    request.seq,
                    TreeNode::Request {
                        name: request.name,
                        path: relative,
                        method: Some(request.method),
                        assertion_count: request.assertions.len(),
                        invalid: false,
                        contract: request.contract.map(|link| link.file),
                        url: request.url,
                        tags: request.tags,
                    },
                ),
                Err(_) => (
                    None,
                    TreeNode::Request {
                        name: stem,
                        path: relative,
                        method: None,
                        assertion_count: 0,
                        invalid: true,
                        contract: None,
                        url: String::new(),
                        tags: Vec::new(),
                    },
                ),
            });
        }
    }
    let key = |node: &TreeNode| match node {
        TreeNode::Folder { name, .. } | TreeNode::Request { name, .. } => sort_key(name),
    };
    // Rang écrit d'abord, puis les dossiers avant les requêtes, puis le nom (R1, AM38) : sans
    // rang, les dossiers puis les requêtes, dans l'ordre alphabétique.
    let mut all: Vec<(Option<f64>, u8, TreeNode)> = folders
        .into_iter()
        .map(|(seq, node)| (seq, 0, node))
        .chain(requests.into_iter().map(|(seq, node)| (seq, 1, node)))
        .collect();
    all.sort_by(|(a, ka, x), (b, kb, y)| {
        a.unwrap_or(f64::INFINITY)
            .total_cmp(&b.unwrap_or(f64::INFINITY))
            .then_with(|| ka.cmp(kb))
            .then_with(|| key(x).cmp(&key(y)))
    });
    Ok(all.into_iter().map(|(_, _, node)| node).collect())
}

/// Rang d'un dossier : `info.seq` de son `folder.yml`, s'il en a un.
fn folder_seq(dir: &Path) -> Option<f64> {
    let value = yaml::read(&dir.join(FOLDER_FILE)).ok()?;
    value.get("info")?.get("seq")?.as_f64()
}

/// Réordonne le contenu du dossier `folder` (4.1-a ter, AM38) : `order` donne les chemins de
/// ses éléments dans le nouvel ordre, qui devient leur rang de 1 à n (`info.seq` d'une requête,
/// du `folder.yml` d'un dossier, créé au besoin). Un élément absent de `order` garde sa place
/// après eux ; un chemin qui n'est pas dans le dossier est refusé, rien n'est écrit.
pub fn reorder(root: &Path, folder: &str, order: &[String]) -> Result<undo::Undo, CommandError> {
    let dir = resolve(root, folder)?;
    if !dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Ce dossier n'existe plus.",
        ));
    }
    let mut targets = Vec::with_capacity(order.len());
    for path in order {
        let target = resolve(root, path)?;
        if target.parent() != Some(dir.as_path()) || !target.exists() {
            return Err(invalid_request(
                "Un élément à ranger n'est plus dans ce dossier.",
            ));
        }
        targets.push(target);
    }
    let mut undo = undo::Undo::default();
    for (index, target) in targets.iter().enumerate() {
        let seq = (index + 1) as f64;
        if target.is_dir() {
            set_folder_seq(target, seq, &mut undo)?;
        } else if let Ok(mut request) = file::read(target) {
            // Un fichier illisible garde sa place : il n'est jamais réécrit.
            if request.seq == Some(seq) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(target) {
                undo.restored(target.clone(), text);
            }
            request.seq = Some(seq);
            file::write(target, &request)?;
        }
    }
    Ok(undo)
}

/// Écrit le rang d'un dossier dans son `folder.yml`, créé avec son nom s'il n'existe pas.
fn set_folder_seq(dir: &Path, seq: f64, undo: &mut undo::Undo) -> Result<(), CommandError> {
    let path = dir.join(FOLDER_FILE);
    let mut value = if path.exists() {
        let text = fs::read_to_string(&path)
            .map_err(|error| CommandError::io("Lecture impossible", &path, &error))?;
        let value = yaml::read(&path)?;
        if value
            .get("info")
            .and_then(|i| i.get("seq"))
            .and_then(serde_json::Value::as_f64)
            == Some(seq)
        {
            return Ok(());
        }
        undo.restored(path.clone(), text);
        value
    } else {
        undo.created(path.clone());
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        serde_json::json!({ "info": { "name": name, "type": "folder" } })
    };
    let Some(info) = value.as_object_mut().map(|object| {
        object
            .entry("info")
            .or_insert_with(|| serde_json::json!({}))
    }) else {
        return Err(invalid_request(
            "Le fichier folder.yml de ce dossier est illisible.",
        ));
    };
    let Some(info) = info.as_object_mut() else {
        return Err(invalid_request(
            "Le fichier folder.yml de ce dossier est illisible.",
        ));
    };
    info.insert("seq".to_owned(), serde_json::Value::from(seq));
    // Un contenu non conforme n'est jamais écrit (format OpenCollection strict).
    schema::check(Part::Folder, &value).map_err(|reason| {
        invalid_request(format!(
            "Le rang du dossier n'a pas pu être écrit : {reason}"
        ))
    })?;
    yaml::write(&path, &value)
}

/// Référentiel de tests (docs/plan-requetes-dans-contrats.md, R1, R5, R6) :
/// l'arborescence sans les requêtes liées à un contrat présent dans `contracts`.
/// Un dossier qui contenait des requêtes et n'en garde aucune disparaît ; un
/// dossier vide reste.
pub fn tests_view(nodes: &[TreeNode], contracts: &HashSet<String>) -> Vec<TreeNode> {
    nodes
        .iter()
        .filter_map(|node| match node {
            TreeNode::Request {
                contract: Some(file),
                ..
            } if contracts.contains(file) => None,
            TreeNode::Request { .. } => Some(node.clone()),
            TreeNode::Folder {
                name,
                path,
                children,
                request_count,
                test,
                tags,
                template,
            } => {
                let kept = tests_view(children, contracts);
                let count: usize = kept.iter().map(TreeNode::count).sum();
                if *request_count > 0 && count == 0 && kept.is_empty() {
                    return None;
                }
                Some(TreeNode::Folder {
                    name: name.clone(),
                    path: path.clone(),
                    children: kept,
                    request_count: count,
                    test: *test,
                    tags: tags.clone(),
                    template: template.clone(),
                })
            }
        })
        .collect()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Chemin absolu d'un chemin relatif, refusé s'il sort du projet.
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf, CommandError> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            _ => return Err(invalid_request("Chemin hors du projet.")),
        }
    }
    Ok(path)
}

/// Nom de fichier dérivé d'un nom d'affichage (R3) : minuscules, sans accents,
/// tirets.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in name.trim().to_lowercase().chars() {
        let base = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'œ' => {
                out.push_str("oe");
                dash = false;
                continue;
            }
            other => other,
        };
        if base.is_ascii_alphanumeric() {
            out.push(base);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "requete".to_owned()
    } else if names::is_reserved(&out) {
        // « con.yml » ou « nul.yml » ne s'ouvrent pas sous Windows (R10).
        format!("_{out}")
    } else {
        out
    }
}

/// Nom de dossier ou de requête normalisé (R2).
pub fn item_name(raw: &str) -> Result<String, CommandError> {
    names::project_name(raw).map_err(|error| {
        CommandError::new(error.kind, error.message.replace("nom de projet", "nom"))
    })
}

fn exists_ignoring_case(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| names::same_name(&entry.file_name().to_string_lossy(), name))
    })
}

fn name_taken(name: &str) -> CommandError {
    CommandError::new(
        CommandErrorKind::NameTaken,
        format!("« {name} » existe déjà dans ce dossier."),
    )
}

/// Une requête de `dir` (autre que `except`) porte déjà ce nom.
pub(crate) fn request_name_taken(dir: &Path, name: &str, except: Option<&Path>) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            if Some(path.as_path()) == except || !is_request_file(&path) {
                return false;
            }
            file::read_name(&path).is_some_and(|other| names::same_name(&other, name))
        })
    })
}

/// Nom de fichier libre dans `dir` pour `base` : `base.yml`, `base-2.yml`…
pub(crate) fn free_file_name(dir: &Path, base: &str) -> String {
    let mut candidate = format!("{base}.yml");
    let mut n = 2;
    while exists_ignoring_case(dir, &candidate) {
        candidate = format!("{base}-{n}.yml");
        n += 1;
    }
    candidate
}

/// Un dossier nommé `name` dans `parent_dir` reste visible dans l'arborescence (PU4) : ni caché
/// (`.…`), ni `folder.yml`, ni, à la racine du projet, un dossier réservé (`environments`,
/// `contracts`) ou le fichier racine. Vaut pour créer, renommer et déplacer.
fn check_folder_name(root: &Path, parent_dir: &Path, name: &str) -> Result<(), CommandError> {
    if name.starts_with('.') || name.eq_ignore_ascii_case(FOLDER_FILE) {
        return Err(CommandError::new(
            CommandErrorKind::InvalidName,
            format!("« {name} » serait caché dans l'arborescence : choisissez un autre nom."),
        ));
    }
    let top = parent_dir == root;
    if top
        && RESERVED_DIRS
            .iter()
            .chain(std::iter::once(&ROOT_FILE))
            .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        return Err(CommandError::new(
            CommandErrorKind::NameTaken,
            format!("« {name} » est réservé à la racine du projet : choisissez un autre nom."),
        ));
    }
    Ok(())
}

/// Crée un dossier réel (A3). Renvoie son chemin relatif.
pub fn create_folder(root: &Path, parent: &str, name: &str) -> Result<String, CommandError> {
    let name = item_name(name)?;
    let parent_dir = resolve(root, parent)?;
    if !parent_dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Le dossier parent n'existe plus.",
        ));
    }
    check_folder_name(root, &parent_dir, &name)?;
    if exists_ignoring_case(&parent_dir, &name) {
        return Err(name_taken(&name));
    }
    let path = parent_dir.join(&name);
    fs::create_dir(&path)
        .map_err(|error| CommandError::io("Création du dossier impossible", &path, &error))?;
    crate::watch::settle(&path);
    Ok(relative_path(root, &path))
}

/// Crée une requête neuve dans un dossier (B1). Renvoie son chemin relatif.
pub fn create_request(
    root: &Path,
    folder: &str,
    name: Option<&str>,
) -> Result<String, CommandError> {
    let dir = resolve(root, folder)?;
    if !dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Le dossier n'existe plus.",
        ));
    }
    let name = match name {
        Some(raw) => item_name(raw)?,
        None => unique_default_name(&dir),
    };
    if request_name_taken(&dir, &name, None) {
        return Err(name_taken(&name));
    }
    let path = dir.join(free_file_name(&dir, &slug(&name)));
    file::write(&path, &Request::new(name))?;
    Ok(relative_path(root, &path))
}

fn unique_default_name(dir: &Path) -> String {
    let base = "Nouvelle requête";
    let mut name = base.to_owned();
    let mut n = 2;
    while request_name_taken(dir, &name, None) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

/// Renomme un dossier (sur disque) ou une requête (nom d'affichage, et nom de
/// fichier s'il était dérivé de l'ancien nom, R3). Renvoie le nouveau chemin.
pub fn rename(root: &Path, path: &str, name: &str) -> Result<String, CommandError> {
    let name = item_name(name)?;
    let source = resolve(root, path)?;
    let parent = source.parent().unwrap_or(root).to_path_buf();
    if source.is_dir() {
        let current = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        check_folder_name(root, &parent, &name)?;
        if !names::same_name(&current, &name) && exists_ignoring_case(&parent, &name) {
            return Err(name_taken(&name));
        }
        let target = parent.join(&name);
        if current != name {
            // Passage par un nom temporaire : renommer « a » en « A » sous Windows.
            let tmp = parent.join(format!(".arnod-rename-{}", uuid::Uuid::new_v4()));
            fs::rename(&source, &tmp)
                .map_err(|e| CommandError::io("Renommage impossible", &source, &e))?;
            if let Err(error) = fs::rename(&tmp, &target) {
                // Le dossier ne reste jamais caché sous son nom temporaire (PU5).
                let _ = fs::rename(&tmp, &source);
                return Err(CommandError::io("Renommage impossible", &target, &error));
            }
            crate::watch::settle(&source);
            crate::watch::settle(&target);
        }
        rename_folder_settings(&target, &name);
        return Ok(relative_path(root, &target));
    }
    let request = file::read(&source)?;
    if request_name_taken(&parent, &name, Some(&source)) {
        return Err(name_taken(&name));
    }
    let old_stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let derived = {
        let base = slug(&request.name);
        old_stem == base
            || old_stem
                .strip_prefix(&format!("{base}-"))
                .is_some_and(|rest| rest.parse::<u32>().is_ok())
    };
    file::rename(&source, &name)?;
    if derived {
        let new_base = slug(&name);
        if new_base != old_stem {
            let target = parent.join(free_file_name(&parent, &new_base));
            fs::rename(&source, &target)
                .map_err(|e| CommandError::io("Renommage impossible", &target, &e))?;
            crate::watch::settle(&source);
            crate::watch::settle(&target);
            return Ok(relative_path(root, &target));
        }
    }
    Ok(relative_path(root, &source))
}

/// Déplace un dossier ou une requête dans un dossier (A6, R4).
pub fn move_to(root: &Path, path: &str, target_folder: &str) -> Result<String, CommandError> {
    let source = resolve(root, path)?;
    let target_dir = resolve(root, target_folder)?;
    if !source.exists() || !target_dir.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "L'élément ou le dossier cible n'existe plus.",
        ));
    }
    if source.is_dir() && (target_dir == source || target_dir.starts_with(&source)) {
        return Err(invalid_request(
            "Un dossier ne peut pas être déplacé dans lui-même.",
        ));
    }
    let file_name = source
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let target = target_dir.join(&file_name);
    if target == source {
        return Ok(relative_path(root, &source));
    }
    if source.is_dir() {
        check_folder_name(root, &target_dir, &file_name.to_string_lossy())?;
    }
    if exists_ignoring_case(&target_dir, &file_name.to_string_lossy()) {
        return Err(name_taken(&file_name.to_string_lossy()));
    }
    if !source.is_dir() {
        let request = file::read(&source)?;
        if request_name_taken(&target_dir, &request.name, None) {
            return Err(name_taken(&request.name));
        }
    }
    fs::rename(&source, &target)
        .map_err(|e| CommandError::io("Déplacement impossible", &target, &e))?;
    crate::watch::settle(&source);
    crate::watch::settle(&target);
    Ok(relative_path(root, &target))
}

/// Duplique une requête dans son dossier (A9) : « <nom> (copie) », nouvel `id`.
pub fn duplicate(root: &Path, path: &str) -> Result<String, CommandError> {
    let source = resolve(root, path)?;
    let dir = source.parent().unwrap_or(root).to_path_buf();
    let request = file::read(&source)?;
    let base = format!("{} (copie)", request.name);
    let mut name = base.clone();
    let mut n = 2;
    while request_name_taken(&dir, &name, None) {
        name = format!("{base} {n}");
        n += 1;
    }
    let target = dir.join(free_file_name(&dir, &slug(&name)));
    file::copy_as(&source, &target, &name)?;
    Ok(relative_path(root, &target))
}

/// Le nom d'un dossier dans son `folder.yml`, s'il en a un, suit le renommage
/// (Bruno l'affiche à la place du nom du dossier).
fn rename_folder_settings(folder: &Path, name: &str) {
    let path = folder.join(FOLDER_FILE);
    let Ok(mut value) = yaml::read(&path) else {
        return;
    };
    if let Some(info) = value
        .get_mut("info")
        .and_then(serde_json::Value::as_object_mut)
    {
        info.insert("name".to_owned(), serde_json::Value::from(name));
        // Un fichier non conforme n'est jamais réécrit (R1, R2).
        if schema::check(Part::Folder, &value).is_ok() {
            let _ = yaml::write(&path, &value);
        }
    }
}

/// Supprime un dossier ou une requête : corbeille du système, ou suppression
/// définitive si `permanent` (R5).
pub fn delete(root: &Path, path: &str, permanent: bool) -> Result<(), CommandError> {
    let target = resolve(root, path)?;
    if target == root {
        return Err(invalid_request(
            "Le projet lui-même ne se supprime pas ici.",
        ));
    }
    if !target.exists() {
        return Ok(());
    }
    let result = if permanent {
        let removed = if target.is_dir() {
            fs::remove_dir_all(&target)
        } else {
            fs::remove_file(&target)
        };
        removed.map_err(|e| CommandError::io("Suppression impossible", &target, &e))
    } else {
        trash::delete(&target).map_err(|error| {
            CommandError::new(
                CommandErrorKind::Io,
                format!("La corbeille du système n'est pas disponible ({error}). Vous pouvez supprimer définitivement."),
            )
        })
    };
    crate::watch::settle(&target);
    result
}

/// Requêtes d'un dossier (sous-dossiers compris), dans l'ordre de l'arborescence (R13).
pub fn requests_in(root: &Path, folder: &str) -> Result<Vec<String>, CommandError> {
    let tree = read(root)?;
    fn collect(nodes: &[TreeNode], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, .. } => collect(children, out),
                TreeNode::Request {
                    path,
                    invalid: false,
                    ..
                } => out.push(path.clone()),
                TreeNode::Request { .. } => {}
            }
        }
    }
    fn find<'a>(nodes: &'a [TreeNode], folder: &str) -> Option<&'a [TreeNode]> {
        for node in nodes {
            if let TreeNode::Folder { path, children, .. } = node {
                if path == folder {
                    return Some(children);
                }
                if let Some(found) = find(children, folder) {
                    return Some(found);
                }
            }
        }
        None
    }
    let mut out = Vec::new();
    if folder.is_empty() {
        collect(&tree, &mut out);
    } else if let Some(children) = find(&tree, folder) {
        collect(children, &mut out);
    }
    Ok(out)
}

pub mod batch;
pub mod transfer;
pub mod undo;

mod test;
pub use test::{
    CreatedTest, StepView, TestView, VariableUser, add_step, create_test, create_test_named,
    extracted_before, is_test, test_view, variable_users,
};

#[cfg(test)]
mod tests;
