//! Brouillon en conflit avec le disque, réglé élément par élément (R9 à R11). Module pur.
//!
//! Une requête est découpée en **éléments** : les champs simples (nom, adresse, corps,
//! authentification…) et chaque ligne des listes (un en-tête, un paramètre, une vérification,
//! une extraction), reconnue par sa clé (nom ou cible) et son rang parmi celles de même clé.
//! [`compare`] liste les éléments où le disque et le brouillon diffèrent ; [`merge`] part du
//! brouillon et prend du disque les éléments choisis.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::request::model::Request;

/// Version gardée pour un élément.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictSide {
    Disk,
    Draft,
}

/// Élément de la requête qui diffère, ou que les deux versions ont changé de la même façon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictItem {
    /// Clé stable de l'élément (`url`, `headers:x-request-id#0`).
    pub key: String,
    /// Libellé en français (« En-tête X-Request-Id »).
    pub label: String,
    /// Valeur sur le disque, prête à afficher ; `None` : absent sur le disque.
    pub disk: Option<String>,
    /// Valeur du brouillon ; `None` : absent du brouillon.
    pub draft: Option<String>,
    /// Les deux versions sont égales (changées de la même façon depuis la version de départ).
    pub identical: bool,
    /// Version proposée : celle qui a changé quand une seule l'a fait (R10).
    pub suggested: Option<ConflictSide>,
}

/// Conflit d'une requête ouverte (`request_conflict`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestConflict {
    pub items: Vec<ConflictItem>,
    /// Date de modification du fichier, en millisecondes depuis l'époque Unix.
    pub modified_at: Option<u64>,
}

/// Champs simples comparés en entier, avec leur libellé.
const FIELDS: [(&str, &str); 11] = [
    ("name", "Nom"),
    ("description", "Description"),
    ("tags", "Étiquettes"),
    ("method", "Méthode"),
    ("url", "Adresse"),
    ("auth", "Authentification"),
    ("body", "Corps"),
    ("settings", "Réglages"),
    ("docs", "Documentation"),
    ("examples", "Réponses attendues"),
    ("seq", "Rang de l'étape"),
];

/// Listes comparées ligne par ligne, avec le libellé d'une ligne.
const LISTS: [(&str, &str); 4] = [
    ("params", "Paramètre"),
    ("headers", "En-tête"),
    ("assertions", "Vérification"),
    ("extractions", "Extraction"),
];

/// Longueur au-delà de laquelle une valeur affichée est coupée.
const SHOWN: usize = 80;

fn to_value(request: &Request) -> Map<String, Value> {
    match serde_json::to_value(request) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Ligne d'une liste telle qu'elle est comparée : sans ce qui n'est pas écrit dans le fichier
/// (identifiant et origine d'une vérification).
fn comparable(list: &str, item: &Value) -> Value {
    let mut item = item.clone();
    if list == "assertions"
        && let Some(map) = item.as_object_mut()
    {
        map.remove("id");
        map.remove("origin");
    }
    item
}

/// Nom d'une ligne, qui la reconnaît d'une version à l'autre.
fn line_name(list: &str, item: &Value) -> String {
    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or_default();
    match list {
        "assertions" => text("target").to_owned(),
        "params" => format!("{}:{}", text("kind"), text("name")),
        "headers" => text("name").to_lowercase(),
        _ => text("name").to_owned(),
    }
}

fn line_label(list: &str, item: &Value) -> String {
    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or_default();
    match list {
        "assertions" => text("target").to_owned(),
        _ => text("name").to_owned(),
    }
}

/// Lignes d'une liste par clé (`headers:x-id#0`), dans l'ordre.
fn lines(list: &str, request: &Map<String, Value>) -> Vec<(String, String, Value)> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    request
        .get(list)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let name = line_name(list, item);
                    let rank = seen.entry(name.clone()).or_default();
                    let key = format!("{list}:{name}#{rank}");
                    *rank += 1;
                    (key, line_label(list, item), item.clone())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Valeurs de chaque élément d'une requête, par clé, avec leur libellé.
fn elements(request: &Map<String, Value>) -> BTreeMap<String, (String, Value)> {
    let mut out = BTreeMap::new();
    for (field, label) in FIELDS {
        let value = request.get(field).cloned().unwrap_or(Value::Null);
        out.insert(field.to_owned(), (label.to_owned(), value));
    }
    for (list, prefix) in LISTS {
        for (key, label, item) in lines(list, request) {
            out.insert(
                key,
                (format!("{prefix} {label}"), comparable(list, &item)),
            );
        }
    }
    out
}

fn cut(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > SHOWN {
        format!("{}…", line.chars().take(SHOWN).collect::<String>())
    } else {
        line
    }
}

fn operator_label(operator: &str) -> &str {
    match operator {
        "equals" => "égal à",
        "notEquals" => "différent de",
        "exists" => "existe",
        "contains" => "contient",
        "notContains" => "ne contient pas",
        "lessThan" => "inférieur à",
        "lessOrEqual" => "inférieur ou égal à",
        "greaterThan" => "supérieur à",
        "greaterOrEqual" => "supérieur ou égal à",
        "in" => "parmi",
        "notIn" => "hors de",
        "between" => "entre",
        "startsWith" => "commence par",
        "endsWith" => "finit par",
        "length" => "a pour longueur",
        "matches" => "correspond à",
        "notMatches" => "ne correspond pas à",
        "isNull" => "est null",
        "isUndefined" => "est absent",
        "isEmpty" => "est vide",
        "isNotEmpty" => "n'est pas vide",
        "isTruthy" => "est vrai",
        "isFalsy" => "est faux",
        "isJson" => "est du JSON",
        "isNumber" => "est un nombre",
        "isText" => "est un texte",
        "isBoolean" => "est un booléen",
        "isArray" => "est un tableau",
        other => other,
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Valeur d'un élément, prête à afficher.
fn shown(key: &str, value: &Value) -> String {
    let text = |field: &str| value.get(field).map(scalar).unwrap_or_default();
    let disabled = value.get("enabled") == Some(&Value::Bool(false));
    let mut out = match key.split(':').next().unwrap_or(key) {
        "params" | "headers" => text("value"),
        "assertions" => {
            let operator = text("operator");
            let label = operator_label(&operator).to_owned();
            match value.get("value") {
                Some(v) if !v.is_null() => format!("{label} {}", scalar(v)),
                _ => label,
            }
        }
        "extractions" => text("source"),
        "tags" => value
            .as_array()
            .map(|tags| tags.iter().map(scalar).collect::<Vec<_>>().join(", "))
            .unwrap_or_default(),
        "body" => {
            let kind = text("type");
            let selected = value
                .get("selected")
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .unwrap_or(0);
            let content = value
                .get("variants")
                .and_then(Value::as_array)
                .and_then(|variants| variants.get(selected))
                .and_then(|variant| variant.get("content"))
                .map(scalar)
                .unwrap_or_default();
            if content.trim().is_empty() {
                kind
            } else {
                format!("{kind} · {content}")
            }
        }
        "auth" => {
            let mut parts = vec![text("type")];
            for field in ["username", "token", "name", "value", "other"] {
                let part = text(field);
                if !part.is_empty() {
                    parts.push(part);
                }
            }
            parts.join(" · ")
        }
        "examples" => {
            let count = value.as_array().map_or(0, Vec::len);
            match count {
                0 => "aucune".to_owned(),
                1 => "1 réponse attendue".to_owned(),
                n => format!("{n} réponses attendues"),
            }
        }
        "settings" => value
            .as_object()
            .map(|map| {
                map.iter()
                    .filter(|(_, v)| !v.is_null())
                    .map(|(k, v)| format!("{k} {}", scalar(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default(),
        _ => scalar(value),
    };
    if disabled {
        out.push_str(" (désactivé)");
    }
    cut(&out)
}

/// Éléments en conflit (R9, R10) : ceux où le disque et le brouillon diffèrent, puis ceux que
/// les deux ont changés de la même façon depuis `base` (la version de départ du brouillon),
/// marqués identiques. Ordre : champs simples, puis lignes des listes.
pub fn compare(base: &Request, disk: &Request, draft: &Request) -> Vec<ConflictItem> {
    let base = elements(&to_value(base));
    let disk = elements(&to_value(disk));
    let draft = elements(&to_value(draft));
    let mut keys: Vec<&String> = disk.keys().chain(draft.keys()).collect();
    keys.sort_by_key(|key| order(key));
    keys.dedup();
    let mut different = Vec::new();
    let mut same = Vec::new();
    for key in keys {
        let on_disk = disk.get(key);
        let in_draft = draft.get(key);
        let at_start = base.get(key).map(|(_, value)| value);
        let disk_value = on_disk.map(|(_, value)| value);
        let draft_value = in_draft.map(|(_, value)| value);
        let label = on_disk
            .or(in_draft)
            .map(|(label, _)| label.clone())
            .unwrap_or_default();
        let item = |identical: bool, suggested: Option<ConflictSide>| ConflictItem {
            key: key.clone(),
            label: label.clone(),
            disk: disk_value.map(|value| shown(key, value)),
            draft: draft_value.map(|value| shown(key, value)),
            identical,
            suggested,
        };
        if disk_value != draft_value {
            let suggested = if draft_value == at_start {
                Some(ConflictSide::Disk)
            } else if disk_value == at_start {
                Some(ConflictSide::Draft)
            } else {
                None
            };
            different.push(item(false, suggested));
        } else if disk_value != at_start {
            same.push(item(true, None));
        }
    }
    different.extend(same);
    different
}

/// Rang d'affichage d'une clé : champs simples dans l'ordre de l'éditeur, puis listes.
fn order(key: &str) -> (usize, String) {
    let field = FIELDS.iter().position(|(name, _)| *name == key);
    let list = LISTS
        .iter()
        .position(|(name, _)| key.starts_with(&format!("{name}:")));
    match (field, list) {
        (Some(index), _) => (index, String::new()),
        (None, Some(index)) => (FIELDS.len() + index, key.to_owned()),
        (None, None) => (usize::MAX, key.to_owned()),
    }
}

/// Requête réglée (R11) : le brouillon, avec les éléments choisis « disque » pris du disque.
/// Chaque élément qui diffère doit avoir un choix ; sinon le fichier a changé depuis la
/// comparaison et l'erreur le dit.
pub fn merge(
    disk: &Request,
    draft: &Request,
    choices: &BTreeMap<String, ConflictSide>,
) -> Result<Request, String> {
    let disk_map = to_value(disk);
    let mut merged = to_value(draft);
    let disk_elements = elements(&disk_map);
    let draft_elements = elements(&merged);
    let keys: BTreeSet<&String> = disk_elements.keys().chain(draft_elements.keys()).collect();
    for key in keys {
        let differs =
            disk_elements.get(key).map(|(_, v)| v) != draft_elements.get(key).map(|(_, v)| v);
        if differs && !choices.contains_key(key) {
            return Err(
                "Le fichier a encore changé sur le disque : revoyez les différences avant d'appliquer."
                    .to_owned(),
            );
        }
    }
    let from_disk = |key: &str| choices.get(key) == Some(&ConflictSide::Disk);
    for (field, _) in FIELDS {
        if from_disk(field) {
            match disk_map.get(field) {
                Some(value) => merged.insert(field.to_owned(), value.clone()),
                None => merged.remove(field),
            };
        }
    }
    for (list, _) in LISTS {
        let disk_lines = lines(list, &disk_map);
        let draft_lines = lines(list, &merged);
        let mut out: Vec<Value> = Vec::new();
        for (key, _, item) in &draft_lines {
            if !from_disk(key) {
                out.push(item.clone());
            } else if let Some((_, _, theirs)) = disk_lines.iter().find(|(k, _, _)| k == key) {
                out.push(keep_id(list, item, theirs));
            }
        }
        for (key, _, item) in &disk_lines {
            if from_disk(key) && !draft_lines.iter().any(|(k, _, _)| k == key) {
                out.push(item.clone());
            }
        }
        merged.insert(list.to_owned(), Value::Array(out));
    }
    serde_json::from_value(Value::Object(merged)).map_err(|error| error.to_string())
}

/// Une vérification prise du disque garde l'identifiant de session de celle du brouillon.
fn keep_id(list: &str, mine: &Value, theirs: &Value) -> Value {
    let mut taken = theirs.clone();
    if list == "assertions"
        && let (Some(id), Some(map)) = (mine.get("id"), taken.as_object_mut())
    {
        map.insert("id".to_owned(), id.clone());
    }
    taken
}
