//! Ce qu'un fichier changé représente pour l'utilisateur (V2, lot E, R14) : un élément partagé
//! (étape, requête, endpoint, dossier, contrat, environnement, réglages du projet), décrit en
//! clair (« assertion body.installments modifiée »), jamais un chemin de fichier ni un commit.
//! Module pur.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

use crate::environment::{self, Environment};
use crate::request::file;
use crate::request::model::Request;
use crate::watch::conflict;

use super::merge::is_request;

/// Nature d'un élément partagé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SharedItemKind"))]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    /// Étape d'un test.
    Step,
    /// Requête hors d'un test.
    Request,
    /// Requête liée à une opération d'un contrat.
    Endpoint,
    Folder,
    Contract,
    Environment,
    /// Réglages du projet (`opencollection.yml`).
    Project,
    Other,
}

/// Élément partagé changé, d'un côté ou de l'autre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SharedItem {
    /// Clé de l'élément, pour le choisir (« Mes modifications ») ; jamais affichée.
    pub key: String,
    pub kind: ItemKind,
    pub label: String,
    /// Où il se range, en clair (« commandes / v2 »), vide à la racine.
    pub place: String,
    /// Ce qui a changé (« 2 assertions modifiées », « nouvelle étape »).
    pub detail: String,
    /// Auteur (ce qui arrive des collègues), sinon `None` (mes modifications).
    pub author: Option<String>,
    /// Date du changement, en millisecondes depuis l'époque Unix.
    pub at: Option<u64>,
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn place_of(dir: &str) -> String {
    dir.split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

fn stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    Path::new(name)
        .file_stem()
        .map_or_else(|| name.to_owned(), |s| s.to_string_lossy().into_owned())
}

fn request_of(path: &str, bytes: Option<&[u8]>) -> Option<Request> {
    let text = std::str::from_utf8(bytes?).ok()?;
    file::parse(text, Path::new(path)).ok()
}

fn environment_of(bytes: Option<&[u8]>) -> Option<Environment> {
    environment::parse_text(std::str::from_utf8(bytes?).ok()?)
}

/// Genre et nombre d'un élément de requête, pour accorder « modifié ».
#[derive(Clone, Copy, PartialEq, Eq)]
enum Agreement {
    Masculine,
    Feminine,
    MasculinePlural,
    FemininePlural,
}

fn agree(word: &str, agreement: Agreement) -> String {
    match agreement {
        Agreement::Masculine => word.to_owned(),
        Agreement::Feminine => format!("{word}e"),
        Agreement::MasculinePlural => format!("{word}s"),
        Agreement::FemininePlural => format!("{word}es"),
    }
}

/// Nom d'un élément de requête en clair, son accord, et sa famille (pour regrouper).
fn element_name(key: &str, label: &str) -> (String, Agreement, &'static str, &'static str) {
    use Agreement::{Feminine, FemininePlural, Masculine, MasculinePlural};
    let line = label.split_once(' ').map_or("", |(_, rest)| rest);
    match key.split(':').next().unwrap_or(key) {
        "name" => ("nom".into(), Masculine, "", ""),
        "description" => ("description".into(), Feminine, "", ""),
        "tags" => ("étiquettes".into(), FemininePlural, "", ""),
        "method" => ("méthode".into(), Feminine, "", ""),
        "url" => ("adresse".into(), Feminine, "", ""),
        "auth" => ("authentification".into(), Feminine, "", ""),
        "body" => ("corps".into(), Masculine, "", ""),
        "settings" => ("réglages".into(), MasculinePlural, "", ""),
        "docs" => ("documentation".into(), Feminine, "", ""),
        "examples" => ("réponses attendues".into(), FemininePlural, "", ""),
        "seq" => ("rang".into(), Masculine, "", ""),
        "params" => (
            format!("paramètre {line}"),
            Masculine,
            "paramètre",
            "paramètres",
        ),
        "headers" => (format!("en-tête {line}"), Masculine, "en-tête", "en-têtes"),
        "assertions" => (
            format!("assertion {line}"),
            Feminine,
            "assertion",
            "assertions",
        ),
        "extractions" => (
            format!("extraction {line}"),
            Feminine,
            "extraction",
            "extractions",
        ),
        _ => (label.to_lowercase(), Masculine, "", ""),
    }
}

/// « assertion body.installments modifiée · en-tête X-Id ajouté », ou « 3 assertions
/// modifiées » quand une même famille revient.
fn request_detail(old: &Request, new: &Request) -> String {
    let items = conflict::compare(old, old, new);
    let mut phrases: Vec<(String, &'static str, &'static str, Agreement)> = Vec::new();
    for item in items.iter().filter(|i| !i.identical) {
        let (name, agreement, _, family) = element_name(&item.key, &item.label);
        let action = match (&item.disk, &item.draft) {
            (None, Some(_)) => "ajouté",
            (Some(_), None) => "retiré",
            _ => "modifié",
        };
        phrases.push((name, action, family, agreement));
    }
    if phrases.is_empty() {
        return "mise en forme du fichier".to_owned();
    }
    let mut out: Vec<String> = Vec::new();
    let mut done: BTreeSet<usize> = BTreeSet::new();
    for (index, (name, action, family, agreement)) in phrases.iter().enumerate() {
        if done.contains(&index) {
            continue;
        }
        let same: Vec<usize> = phrases
            .iter()
            .enumerate()
            .filter(|(_, (_, a, f, _))| !family.is_empty() && f == family && a == action)
            .map(|(i, _)| i)
            .collect();
        if same.len() > 1 {
            done.extend(same.iter().copied());
            let plural = if *agreement == Agreement::Feminine {
                Agreement::FemininePlural
            } else {
                Agreement::MasculinePlural
            };
            out.push(format!("{} {family} {}", same.len(), agree(action, plural)));
        } else {
            done.insert(index);
            out.push(format!("{name} {}", agree(action, *agreement)));
        }
    }
    if out.len() > 3 {
        let rest = out.len() - 2;
        out.truncate(2);
        out.push(format!("{rest} autres modifications"));
    }
    out.join(" · ")
}

fn environment_detail(old: Option<&Environment>, new: Option<&Environment>) -> String {
    let (Some(old), Some(new)) = (old, new) else {
        return match (old, new) {
            (None, Some(_)) => "nouvel environnement".to_owned(),
            (Some(_), None) => "environnement supprimé".to_owned(),
            _ => "environnement modifié".to_owned(),
        };
    };
    let mut out = Vec::new();
    for variable in &new.variables {
        match old.variables.iter().find(|v| v.name == variable.name) {
            None => out.push(format!("variable {} ajoutée", variable.name)),
            Some(before) if before.value != variable.value => {
                out.push(format!("variable {}", variable.name));
            }
            _ => {}
        }
    }
    for variable in &old.variables {
        if !new.variables.iter().any(|v| v.name == variable.name) {
            out.push(format!("variable {} retirée", variable.name));
        }
    }
    for local in &new.locals {
        if !old.locals.contains(local) {
            out.push(format!("variable locale {local} déclarée"));
        }
    }
    if out.is_empty() {
        "réglages de l'environnement modifiés".to_owned()
    } else {
        out.join(" · ")
    }
}

fn status_word(old: Option<&[u8]>, new: Option<&[u8]>, masculine: &str, feminine: bool) -> String {
    let word = match (old, new) {
        (None, Some(_)) => "ajouté",
        (Some(_), None) => "supprimé",
        _ => "modifié",
    };
    let word = if feminine {
        format!("{word}e")
    } else {
        word.to_owned()
    };
    format!("{masculine} {word}")
}

/// Élément partagé d'un fichier changé ; `tests` : dossiers qui sont des tests.
pub fn describe(
    path: &str,
    old: Option<&[u8]>,
    new: Option<&[u8]>,
    tests: &BTreeSet<String>,
) -> SharedItem {
    let dir = parent(path);
    let name = path.rsplit('/').next().unwrap_or(path);
    let item = |kind: ItemKind, label: String, place: String, detail: String| SharedItem {
        key: path.to_owned(),
        kind,
        label,
        place,
        detail,
        author: None,
        at: None,
    };
    if path == "opencollection.yml" {
        return item(
            ItemKind::Project,
            "Réglages du projet".to_owned(),
            String::new(),
            "réglages du projet modifiés".to_owned(),
        );
    }
    if name == "folder.yml" {
        let label = dir.rsplit('/').next().unwrap_or(dir).to_owned();
        return item(
            ItemKind::Folder,
            label,
            place_of(parent(dir)),
            "réglages du dossier modifiés".to_owned(),
        );
    }
    if let Some(rest) = path.strip_prefix("environments/") {
        let old_env = environment_of(old);
        let new_env = environment_of(new);
        let label = new_env
            .as_ref()
            .or(old_env.as_ref())
            .map_or_else(|| stem(rest), |e| e.name.clone());
        return item(
            ItemKind::Environment,
            label,
            String::new(),
            environment_detail(old_env.as_ref(), new_env.as_ref()),
        );
    }
    if path.starts_with("contracts/") {
        return item(
            ItemKind::Contract,
            stem(path),
            String::new(),
            status_word(old, new, "contrat", false),
        );
    }
    if is_request(path) {
        let old_req = request_of(path, old);
        let new_req = request_of(path, new);
        let shown = new_req.as_ref().or(old_req.as_ref());
        let label = shown.map_or_else(|| stem(path), |r| r.name.clone());
        let in_test = tests.contains(dir);
        let kind = if shown.is_some_and(|r| r.contract.is_some()) {
            ItemKind::Endpoint
        } else if in_test {
            ItemKind::Step
        } else {
            ItemKind::Request
        };
        let noun = match kind {
            ItemKind::Step => ("étape", true),
            ItemKind::Endpoint => ("endpoint", false),
            _ => ("requête", true),
        };
        let detail = match (&old_req, &new_req) {
            (Some(old), Some(new)) => request_detail(old, new),
            (None, Some(_)) => {
                if noun.1 {
                    format!("nouvelle {}", noun.0)
                } else {
                    format!("nouvel {}", noun.0)
                }
            }
            (Some(_), None) => status_word(old, new, noun.0, noun.1)
                .replacen(noun.0, "", 1)
                .trim()
                .to_owned(),
            (None, None) => status_word(old, new, "fichier", false),
        };
        return item(kind, label, place_of(dir), detail);
    }
    item(
        ItemKind::Other,
        name.to_owned(),
        place_of(dir),
        status_word(old, new, "fichier", false),
    )
}

/// Initiale d'un auteur, pour sa pastille (CF4).
pub fn initial(author: &str) -> String {
    author
        .chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "?".to_owned(), |c| c.to_uppercase().collect())
}
