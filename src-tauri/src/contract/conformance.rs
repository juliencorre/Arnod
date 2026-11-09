//! Conformité d'une réponse au contrat (C8, R9, R11).

use serde_json::Value;

use crate::contract::parse::{Contract, SpecKind, resolve};
use crate::request::assertions::ResponseData;

/// Nombre maximal d'écarts relevés, et affichés dans le message.
const MAX_ERRORS: usize = 20;
const SHOWN_ERRORS: usize = 3;
/// Profondeur maximale de validation : schémas récursifs sans fin (R3).
const MAX_DEPTH: usize = 64;

/// Vérifie la réponse reçue pour l'opération `key` : statut prévu, puis corps
/// conforme au schéma. L'erreur décrit le premier écart, en français.
pub fn check(contract: &Contract, key: &str, response: &ResponseData) -> Result<(), String> {
    let operation = contract
        .operation(key)
        .ok_or_else(|| format!("l'opération « {key} » n'est plus dans le contrat"))?;
    let Some(responses) = contract.responses(operation) else {
        return Ok(());
    };
    let status = response.status.to_string();
    let family = format!("{}XX", status.chars().next().unwrap_or('0'));
    let declared = responses
        .get(&status)
        .or_else(|| {
            responses
                .iter()
                .find(|(code, _)| code.eq_ignore_ascii_case(&family))
                .map(|(_, value)| value)
        })
        .or_else(|| responses.get("default"));
    let Some(declared) = declared else {
        let expected: Vec<&str> = responses.keys().map(String::as_str).collect();
        return Err(format!(
            "statut {status} non prévu par le contrat (prévus : {})",
            expected.join(", ")
        ));
    };
    let declared = resolve(&contract.doc, declared);
    let Some(schema) = response_schema(contract, declared) else {
        return Ok(());
    };
    if response.body.trim().is_empty() {
        return Err("corps vide, le contrat prévoit un corps JSON".to_owned());
    }
    let Ok(body) = serde_json::from_str::<Value>(&response.body) else {
        return Err("le corps n'est pas du JSON, le contrat prévoit un schéma JSON".to_owned());
    };
    let errors = validate(&contract.doc, schema, &body);
    if errors.is_empty() {
        return Ok(());
    }
    let shown: Vec<&str> = errors
        .iter()
        .take(SHOWN_ERRORS)
        .map(String::as_str)
        .collect();
    let more = errors.len().saturating_sub(SHOWN_ERRORS);
    Err(if more > 0 {
        format!("{} (+{more} autres écarts)", shown.join(" ; "))
    } else {
        shown.join(" ; ")
    })
}

/// Schéma JSON de la réponse déclarée, s'il y en a un.
fn response_schema<'a>(contract: &'a Contract, declared: &'a Value) -> Option<&'a Value> {
    match contract.kind {
        SpecKind::Swagger2 => declared.get("schema"),
        SpecKind::OpenApi3 => {
            let content = declared.get("content")?.as_object()?;
            content
                .iter()
                .find(|(media, _)| {
                    let media = media.to_lowercase();
                    media.starts_with("application/json") || media.contains("+json")
                })
                .or_else(|| content.iter().find(|(media, _)| media.as_str() == "*/*"))
                .and_then(|(_, entry)| entry.get("schema"))
        }
    }
}

/// Écarts entre une valeur et un schéma (R9), chemins préfixés par `body`.
pub fn validate(doc: &Value, schema: &Value, value: &Value) -> Vec<String> {
    validate_at(doc, schema, value, "body")
}

/// Écarts entre une valeur et un schéma, chemins préfixés par `root`. Sert
/// aussi à valider les fichiers OpenCollection contre leur schéma officiel.
pub fn validate_at(doc: &Value, schema: &Value, value: &Value, root: &str) -> Vec<String> {
    let mut errors = Vec::new();
    walk(doc, schema, value, root, &mut errors, 0);
    errors
}

fn show(value: &Value) -> String {
    match value {
        Value::String(text) => {
            let short: String = text.chars().take(40).collect();
            if short.len() < text.len() {
                format!("\"{short}…\"")
            } else {
                format!("\"{short}\"")
            }
        }
        Value::Object(_) => "un objet".to_owned(),
        Value::Array(_) => "un tableau".to_owned(),
        other => other.to_string(),
    }
}

fn type_name(kind: &str) -> &str {
    match kind {
        "string" => "texte",
        "integer" => "entier",
        "number" => "nombre",
        "boolean" => "booléen",
        "array" => "tableau",
        "object" => "objet",
        "null" => "null",
        other => other,
    }
}

fn matches_type(kind: &str, value: &Value) -> bool {
    match kind {
        "string" => value.is_string(),
        "integer" => {
            value.is_i64() || value.is_u64() || value.as_f64().is_some_and(|n| n.fract() == 0.0)
        }
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn types(schema: &Value) -> Vec<String> {
    let mut list: Vec<String> = match schema.get("type") {
        Some(Value::String(kind)) => vec![kind.clone()],
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    if !list.is_empty() && schema.get("nullable").and_then(Value::as_bool) == Some(true) {
        list.push("null".to_owned());
    }
    list
}

fn number(schema: &Value, key: &str) -> Option<f64> {
    schema.get(key).and_then(Value::as_f64)
}

fn walk(
    doc: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if errors.len() >= MAX_ERRORS || depth > MAX_DEPTH {
        return;
    }
    match schema {
        Value::Bool(true) => return,
        Value::Bool(false) => {
            errors.push(format!("{path} : valeur non autorisée par le contrat"));
            return;
        }
        Value::Object(_) => {}
        _ => return,
    }
    let schema = resolve(doc, schema);

    if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
        for part in parts {
            walk(doc, part, value, path, errors, depth + 1);
        }
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(options) = schema.get(key).and_then(Value::as_array) {
            let fits = options.iter().any(|option| {
                let mut trial = Vec::new();
                walk(doc, option, value, path, &mut trial, depth + 1);
                trial.is_empty()
            });
            if !fits {
                errors.push(format!(
                    "{path} : ne correspond à aucune des formes prévues ({key})"
                ));
            }
        }
    }

    if let Some(allowed) = schema.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        let list: Vec<String> = allowed.iter().take(5).map(show).collect();
        errors.push(format!(
            "{path} : valeur {} non prévue (attendu : {})",
            show(value),
            list.join(", ")
        ));
        return;
    }
    if let Some(expected) = schema.get("const")
        && expected != value
    {
        errors.push(format!(
            "{path} : attendu {}, reçu {}",
            show(expected),
            show(value)
        ));
        return;
    }

    let kinds = types(schema);
    if value.is_null() && schema.get("nullable").and_then(Value::as_bool) == Some(true) {
        return;
    }
    if !kinds.is_empty() && !kinds.iter().any(|kind| matches_type(kind, value)) {
        let expected: Vec<&str> = kinds
            .iter()
            .filter(|k| k.as_str() != "null")
            .map(|k| type_name(k))
            .collect();
        errors.push(format!(
            "{path} : attendu {}, reçu {}",
            expected.join(" ou "),
            show(value)
        ));
        return;
    }

    match value {
        Value::Object(map) => {
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for name in required.iter().filter_map(Value::as_str) {
                    if !map.contains_key(name) {
                        errors.push(format!("{path}.{name} : champ obligatoire absent"));
                    }
                }
            }
            let properties = schema.get("properties").and_then(Value::as_object);
            for (name, item) in map {
                let child = format!("{path}.{name}");
                if let Some(property) = properties.and_then(|p| p.get(name)) {
                    walk(doc, property, item, &child, errors, depth + 1);
                    continue;
                }
                match schema.get("additionalProperties") {
                    Some(Value::Bool(false)) => {
                        errors.push(format!("{child} : champ non prévu par le contrat"));
                    }
                    Some(extra @ Value::Object(_)) => {
                        walk(doc, extra, item, &child, errors, depth + 1);
                    }
                    _ => {}
                }
            }
        }
        Value::Array(items) => {
            if let Some(min) = schema.get("minItems").and_then(Value::as_u64)
                && (items.len() as u64) < min
            {
                errors.push(format!(
                    "{path} : au moins {min} élément(s) attendu(s), reçu {}",
                    items.len()
                ));
            }
            if let Some(max) = schema.get("maxItems").and_then(Value::as_u64)
                && (items.len() as u64) > max
            {
                errors.push(format!(
                    "{path} : au plus {max} élément(s) attendu(s), reçu {}",
                    items.len()
                ));
            }
            if let Some(item_schema) = schema.get("items").filter(|s| !s.is_array()) {
                for (index, item) in items.iter().enumerate() {
                    walk(
                        doc,
                        item_schema,
                        item,
                        &format!("{path}[{index}]"),
                        errors,
                        depth + 1,
                    );
                }
            }
        }
        Value::String(text) => {
            let length = text.chars().count() as u64;
            if let Some(min) = schema.get("minLength").and_then(Value::as_u64)
                && length < min
            {
                errors.push(format!(
                    "{path} : au moins {min} caractère(s) attendu(s), reçu {length}"
                ));
            }
            if let Some(max) = schema.get("maxLength").and_then(Value::as_u64)
                && length > max
            {
                errors.push(format!(
                    "{path} : au plus {max} caractère(s) attendu(s), reçu {length}"
                ));
            }
        }
        Value::Number(_) => {
            let Some(actual) = value.as_f64() else {
                return;
            };
            let exclusive = |key: &str| schema.get(key).and_then(Value::as_bool) == Some(true);
            if let Some(min) = number(schema, "minimum") {
                let below = if exclusive("exclusiveMinimum") {
                    actual <= min
                } else {
                    actual < min
                };
                if below {
                    errors.push(format!("{path} : minimum {min}, reçu {actual}"));
                }
            }
            if let Some(min) = number(schema, "exclusiveMinimum")
                && actual <= min
            {
                errors.push(format!(
                    "{path} : strictement supérieur à {min} attendu, reçu {actual}"
                ));
            }
            if let Some(max) = number(schema, "maximum") {
                let above = if exclusive("exclusiveMaximum") {
                    actual >= max
                } else {
                    actual > max
                };
                if above {
                    errors.push(format!("{path} : maximum {max}, reçu {actual}"));
                }
            }
            if let Some(max) = number(schema, "exclusiveMaximum")
                && actual >= max
            {
                errors.push(format!(
                    "{path} : strictement inférieur à {max} attendu, reçu {actual}"
                ));
            }
        }
        _ => {}
    }
}
