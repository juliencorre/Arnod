//! Recherche Ctrl+K (écrans 1.5-a et 1.5-b, `docs/plan-v2.md`, lot A, règles R1 à R3).
//!
//! Module pur : il reçoit ce que le cœur sait des projets du workspace (arborescence, contrats,
//! environnements) et rend des résultats groupés. Deux façons de chercher :
//!
//! - **par nom** : chaque mot doit paraître dans le nom (ou l'adresse d'une requête), sans casse
//!   ni accents ; `#mot` garde les éléments dont une étiquette commence par `mot`, une méthode en
//!   capitales (`POST`) garde cette méthode (R1) ;
//! - **par adresse** : `POST /orders`, `/orders/{id}` ou une URL complète se comparent aux chemins
//!   des opérations des contrats : correspondance exacte, puis les opérations de la même
//!   ressource, puis les requêtes qui appellent ce chemin (R2).
//!
//! Au plus [`PER_GROUP`] résultats par groupe, dans un ordre stable (R3).

use serde::Serialize;
use uuid::Uuid;

use crate::contract::file::ContractSummary;
use crate::tree::TreeNode;

/// Résultats montrés par groupe, au plus (R3).
pub const PER_GROUP: usize = 8;

/// Méthodes reconnues dans une saisie.
const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// Ce que le cœur sait d'un projet, pour la recherche.
#[derive(Debug, Clone, Default)]
pub struct ProjectSource {
    pub id: Uuid,
    pub name: String,
    pub nodes: Vec<TreeNode>,
    pub contracts: Vec<ContractSummary>,
    pub environments: Vec<String>,
}

/// Nature d'un résultat : ce qu'il ouvre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SearchKind {
    /// Une requête : elle s'ouvre dans son onglet.
    Request,
    /// Un test (dossier d'étapes) : sa vue de test.
    Test,
    /// Un dossier ordinaire : sa vue.
    Folder,
    /// Une opération d'un contrat : son endpoint (première requête liée), sinon le contrat.
    Operation,
    Contract,
    Environment,
    Project,
}

/// Un résultat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub kind: SearchKind,
    pub project_id: Uuid,
    pub label: String,
    /// Où il se trouve : « boutique-tests · paiements », « API Commandes v2.4.0 ».
    pub detail: String,
    pub method: Option<String>,
    /// Ce qui s'ouvre : chemin de la requête, du test, du dossier ou du contrat ; pour une
    /// opération, son endpoint s'il existe, sinon son contrat ; `None` pour un projet ou un
    /// environnement.
    pub path: Option<String>,
    /// Pour une opération : le contrat qui s'ouvre quand elle n'a pas d'endpoint.
    pub contract: Option<String>,
    /// Passages du libellé à surligner, en caractères `[début, fin)`.
    pub highlight: Vec<(usize, usize)>,
}

/// Un groupe de résultats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SearchGroup {
    pub title: String,
    pub hits: Vec<SearchHit>,
    /// Résultats trouvés, au-delà de ceux montrés.
    pub total: usize,
}

/// Résultats d'une saisie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub text: String,
    /// Recherche par adresse (R2) plutôt que par nom (R1).
    pub address: bool,
    pub groups: Vec<SearchGroup>,
}

// ------------------------------------------------------------------------- normalisation

/// Un caractère sans casse ni accent ; toujours un seul caractère, pour garder les positions.
fn fold_char(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' | 'æ' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' | 'œ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' | 'ý' => 'y',
        'ñ' => 'n',
        other => other,
    }
}

/// Texte sans casse ni accents, caractère pour caractère (R1).
pub fn fold(text: &str) -> Vec<char> {
    text.chars().map(fold_char).collect()
}

/// Position (en caractères) de `needle` dans `hay`, tous deux déjà normalisés.
fn find(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

// ------------------------------------------------------------------------- saisie

/// Saisie analysée.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Query {
    Name {
        words: Vec<Vec<char>>,
        tags: Vec<Vec<char>>,
        method: Option<String>,
    },
    Address {
        method: Option<String>,
        path: String,
    },
}

fn parse(text: &str) -> Option<Query> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut parts = text.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let (method, rest) = if METHODS.contains(&first) {
        (Some(first.to_owned()), parts.next())
    } else {
        (None, Some(first))
    };
    if let Some(rest) = rest
        && parts.clone().next().is_none()
        && let Some(path) = address_path(rest)
    {
        return Some(Query::Address { method, path });
    }
    let mut words = Vec::new();
    let mut tags = Vec::new();
    let mut method = None;
    for part in text.split_whitespace() {
        if let Some(tag) = part.strip_prefix('#') {
            if !tag.is_empty() {
                tags.push(fold(tag));
            }
        } else if METHODS.contains(&part) {
            method = Some(part.to_owned());
        } else {
            words.push(fold(part));
        }
    }
    Some(Query::Name {
        words,
        tags,
        method,
    })
}

/// Chemin d'une adresse saisie : `/orders/{id}`, ou celui d'une URL complète, sans requête.
fn address_path(text: &str) -> Option<String> {
    if text.starts_with('/') {
        return Some(strip_query(text).to_owned());
    }
    if text.starts_with("http://") || text.starts_with("https://") {
        let url = reqwest::Url::parse(text).ok()?;
        return Some(url.path().to_owned());
    }
    None
}

fn strip_query(path: &str) -> &str {
    path.split(['?', '#']).next().unwrap_or(path)
}

// ------------------------------------------------------------------------- chemins

/// Segment variable : `{id}`, `{{id}}`, `:id`.
fn is_variable(segment: &str) -> bool {
    segment.starts_with(':') || (segment.starts_with('{') && segment.ends_with('}'))
}

fn segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Deux chemins se correspondent : mêmes segments, un segment variable valant n'importe lequel.
fn same_path(a: &str, b: &str) -> bool {
    let (a, b) = (segments(a), segments(b));
    a.len() == b.len()
        && a.iter()
            .zip(&b)
            .all(|(x, y)| x == y || is_variable(x) || is_variable(y))
}

/// Le chemin saisi suit le modèle d'une opération : un segment variable du modèle vaut
/// n'importe quel segment, un segment variable saisi ne vaut qu'un segment variable.
fn follows(template: &str, path: &str) -> bool {
    let (t, p) = (segments(template), segments(path));
    t.len() == p.len() && t.iter().zip(&p).all(|(t, p)| t == p || is_variable(t))
}

/// Le chemin saisi correspond au modèle d'une opération, lui ou une de ses fins (une URL peut
/// porter le préfixe du serveur : `/v2/orders/42` pour `/orders/{id}`).
fn path_served(template: &str, path: &str) -> bool {
    let segs = segments(path);
    (0..segs.len().max(1)).any(|skip| {
        let tail = format!("/{}", segs.get(skip..).unwrap_or_default().join("/"));
        follows(template, &tail)
    })
}

/// Chemin d'une adresse de requête : `{{baseUrl}}/orders/{{id}}?x=1` donne `/orders/{{id}}`.
fn request_path(url: &str) -> String {
    let url = strip_query(url.trim());
    let rest = if let Some(after) = url.split_once("://").map(|(_, rest)| rest) {
        after.find('/').map_or("", |i| &after[i..])
    } else if url.starts_with("{{") {
        url.find("}}").map_or("", |i| &url[i + 2..])
    } else {
        url
    };
    if rest.starts_with('/') {
        rest.to_owned()
    } else {
        format!("/{rest}")
    }
}

/// Ressource d'un chemin : son premier segment (`/orders`).
fn resource(path: &str) -> String {
    format!("/{}", segments(path).first().copied().unwrap_or_default())
}

// ------------------------------------------------------------------------- index

/// Élément cherchable, avant le tri.
struct Entry {
    group: usize,
    hit: SearchHit,
    /// Autres textes où chercher les mots (adresse d'une requête, chemin d'une opération).
    extra: Vec<String>,
    tags: Vec<String>,
}

const TESTS: usize = 0;
const FOLDERS: usize = 1;
const ENDPOINTS: usize = 2;
const CONTRACTS: usize = 3;
const ENVIRONMENTS: usize = 4;
const PROJECTS: usize = 5;
const TITLES: [&str; 6] = [
    "Tests",
    "Dossiers",
    "Endpoints",
    "Contrats",
    "Environnements",
    "Projets",
];

fn hit(kind: SearchKind, project: &ProjectSource, label: &str, detail: String) -> SearchHit {
    SearchHit {
        kind,
        project_id: project.id,
        label: label.to_owned(),
        detail,
        method: None,
        path: None,
        contract: None,
        highlight: Vec::new(),
    }
}

/// « boutique-tests · paiements » : projet et dossier parent.
fn place(project: &ProjectSource, path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => format!("{} · {}", project.name, folder.replace('/', " / ")),
        None => project.name.clone(),
    }
}

fn walk(project: &ProjectSource, nodes: &[TreeNode], out: &mut Vec<Entry>) {
    for node in nodes {
        match node {
            TreeNode::Folder {
                name,
                path,
                children,
                test,
                ..
            } => {
                let (kind, group) = if *test {
                    (SearchKind::Test, TESTS)
                } else {
                    (SearchKind::Folder, FOLDERS)
                };
                let mut h = hit(kind, project, name, place(project, path));
                h.path = Some(path.clone());
                out.push(Entry {
                    group,
                    hit: h,
                    extra: Vec::new(),
                    tags: Vec::new(),
                });
                walk(project, children, out);
            }
            TreeNode::Request {
                name,
                path,
                method,
                contract,
                url,
                tags,
                invalid,
                ..
            } => {
                // Une requête liée est montrée sous son opération (groupe Endpoints).
                if *invalid || contract.is_some() {
                    continue;
                }
                let mut h = hit(SearchKind::Request, project, name, place(project, path));
                h.method = method.map(|m| m.as_str().to_owned());
                h.path = Some(path.clone());
                out.push(Entry {
                    group: TESTS,
                    hit: h,
                    extra: vec![url.clone()],
                    tags: tags.clone(),
                });
            }
        }
    }
}

fn index(projects: &[ProjectSource]) -> Vec<Entry> {
    let mut out = Vec::new();
    for project in projects {
        walk(project, &project.nodes, &mut out);
        for contract in &project.contracts {
            let title = if contract.deduced || contract.version.is_empty() {
                contract.name.clone()
            } else {
                format!("{} v{}", contract.name, contract.version)
            };
            for op in contract.resources.iter().flat_map(|r| &r.operations) {
                let label = op
                    .summary
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| op.path.clone());
                let mut h = hit(
                    SearchKind::Operation,
                    project,
                    &label,
                    format!("{} · {title}", op.path),
                );
                h.method = Some(op.method.to_uppercase());
                h.path = op.requests.first().map(|r| r.path.clone());
                h.contract = Some(contract.path.clone());
                let mut extra = vec![op.path.clone(), op.key.clone()];
                extra.extend(op.requests.iter().map(|r| r.name.clone()));
                out.push(Entry {
                    group: ENDPOINTS,
                    hit: h,
                    extra,
                    tags: Vec::new(),
                });
            }
            let mut h = hit(
                SearchKind::Contract,
                project,
                &title,
                format!("{} · {}", project.name, contract.path),
            );
            h.path = Some(contract.path.clone());
            out.push(Entry {
                group: CONTRACTS,
                hit: h,
                extra: Vec::new(),
                tags: Vec::new(),
            });
        }
        for env in &project.environments {
            out.push(Entry {
                group: ENVIRONMENTS,
                hit: hit(SearchKind::Environment, project, env, project.name.clone()),
                extra: Vec::new(),
                tags: Vec::new(),
            });
        }
        out.push(Entry {
            group: PROJECTS,
            hit: hit(
                SearchKind::Project,
                project,
                &project.name,
                "Projet".to_owned(),
            ),
            extra: Vec::new(),
            tags: Vec::new(),
        });
    }
    out
}

// ------------------------------------------------------------------------- recherche

/// Rang d'un résultat trouvé par nom : libellé exact, puis début, puis début d'un mot, puis
/// ailleurs dans le libellé, puis seulement dans l'adresse (R3).
fn rank_by_name(entry: &Entry, words: &[Vec<char>]) -> Option<(u8, Vec<(usize, usize)>)> {
    let label = fold(&entry.hit.label);
    let extra: Vec<Vec<char>> = entry.extra.iter().map(|e| fold(e)).collect();
    let mut highlight = Vec::new();
    let mut rank = 0u8;
    for word in words {
        match find(&label, word) {
            Some(at) => {
                highlight.push((at, at + word.len()));
                let r = if label == *word {
                    0
                } else if at == 0 {
                    1
                } else if !label[at - 1].is_alphanumeric() {
                    2
                } else {
                    3
                };
                rank = rank.max(r);
            }
            None if extra.iter().any(|e| find(e, word).is_some()) => rank = rank.max(4),
            None => return None,
        }
    }
    highlight.sort_unstable();
    Some((rank, highlight))
}

fn groups_of(mut found: Vec<(usize, u8, SearchHit)>, titles: &[&str]) -> Vec<SearchGroup> {
    found.sort_by(|a, b| {
        (a.0, a.1)
            .cmp(&(b.0, b.1))
            .then_with(|| fold(&a.2.label).cmp(&fold(&b.2.label)))
            .then_with(|| a.2.detail.cmp(&b.2.detail))
    });
    titles
        .iter()
        .enumerate()
        .filter_map(|(i, title)| {
            let all: Vec<SearchHit> = found
                .iter()
                .filter(|(g, _, _)| *g == i)
                .map(|(_, _, h)| h.clone())
                .collect();
            (!all.is_empty()).then(|| SearchGroup {
                title: (*title).to_owned(),
                total: all.len(),
                hits: all.into_iter().take(PER_GROUP).collect(),
            })
        })
        .collect()
}

fn by_name(
    entries: Vec<Entry>,
    words: &[Vec<char>],
    tags: &[Vec<char>],
    method: Option<&str>,
) -> Vec<SearchGroup> {
    let mut found = Vec::new();
    for entry in entries {
        if !tags.is_empty()
            && !tags
                .iter()
                .all(|t| entry.tags.iter().any(|own| fold(own).starts_with(t)))
        {
            continue;
        }
        if let Some(method) = method
            && entry.hit.method.as_deref() != Some(method)
        {
            continue;
        }
        let Some((rank, highlight)) = rank_by_name(&entry, words) else {
            continue;
        };
        let mut h = entry.hit;
        h.highlight = highlight;
        found.push((entry.group, rank, h));
    }
    groups_of(found, &TITLES)
}

fn by_address(entries: Vec<Entry>, method: Option<&str>, path: &str) -> Vec<SearchGroup> {
    let wanted = resource(path);
    let method_ok = |m: Option<&str>| method.is_none_or(|want| m == Some(want));
    let mut exact_templates = Vec::new();
    let mut found = Vec::new();
    for entry in &entries {
        if entry.hit.kind != SearchKind::Operation {
            continue;
        }
        let template = entry.extra.first().map(String::as_str).unwrap_or_default();
        let m = entry.hit.method.as_deref();
        if path_served(template, path) && method_ok(m) {
            exact_templates.push(template.to_owned());
            found.push((0, 0, entry.hit.clone()));
        } else if resource(template) == wanted {
            found.push((1, 0, entry.hit.clone()));
        }
    }
    for entry in entries {
        if entry.hit.kind != SearchKind::Request || !method_ok(entry.hit.method.as_deref()) {
            continue;
        }
        let called = request_path(entry.extra.first().map(String::as_str).unwrap_or_default());
        if same_path(&called, path) || exact_templates.iter().any(|t| same_path(t, &called)) {
            found.push((2, 0, entry.hit));
        }
    }
    let exact_title = "Correspondance exacte".to_owned();
    let same_title = format!("Endpoints de {wanted}");
    let calls_title = "Tests qui l'appellent".to_owned();
    groups_of(found, &[&exact_title, &same_title, &calls_title])
}

/// Résultats d'une saisie dans les projets donnés (R1 à R3).
pub fn search(projects: &[ProjectSource], text: &str) -> SearchResults {
    let Some(query) = parse(text) else {
        return SearchResults {
            text: text.to_owned(),
            address: false,
            groups: Vec::new(),
        };
    };
    let entries = index(projects);
    match query {
        Query::Name {
            words,
            tags,
            method,
        } => SearchResults {
            text: text.to_owned(),
            address: false,
            groups: by_name(entries, &words, &tags, method.as_deref()),
        },
        Query::Address { method, path } => SearchResults {
            text: text.to_owned(),
            address: true,
            groups: by_address(entries, method.as_deref(), &path),
        },
    }
}

#[cfg(test)]
mod tests;
