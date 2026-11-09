//! Exemples tirés des schémas : corps et paramètres des requêtes créées (R6).

use serde_json::{Map, Value};

use crate::contract::parse::{pointer, resolve};

/// Profondeur maximale d'un exemple : les schémas récursifs s'arrêtent là (R3).
const MAX_DEPTH: usize = 8;

/// Exemple d'une valeur conforme au schéma, pour une requête : les propriétés
/// en lecture seule sont omises.
pub fn sample(doc: &Value, schema: &Value) -> Value {
    walk(doc, schema, 0, &mut Vec::new())
}

/// Exemple déclaré d'un paramètre ou d'un contenu (`example`, puis `examples`).
pub fn declared_example(doc: &Value, holder: &Value) -> Option<Value> {
    if let Some(example) = holder.get("example") {
        return Some(example.clone());
    }
    match holder.get("examples")? {
        // OpenAPI 3 : { nom: { value } } (référence possible).
        Value::Object(map) => map.values().find_map(|example| {
            let example = resolve(doc, example);
            example.get("value").cloned()
        }),
        // JSON Schema 2020-12 (OpenAPI 3.1) : liste de valeurs.
        Value::Array(items) => items.first().cloned(),
        _ => None,
    }
}

/// Texte d'une valeur, pour un paramètre d'adresse ou d'en-tête.
pub fn as_param(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Array(items) => items.iter().map(as_param).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

fn first_type(schema: &Value) -> Option<&str> {
    match schema.get("type")? {
        Value::String(kind) => Some(kind.as_str()),
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|kind| *kind != "null"),
        _ => None,
    }
}

fn walk(doc: &Value, schema: &Value, depth: usize, seen: &mut Vec<String>) -> Value {
    if depth > MAX_DEPTH {
        return Value::Null;
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if seen.iter().any(|r| r == reference) {
            return Value::Null;
        }
        let Some(target) = pointer(doc, reference) else {
            return Value::Null;
        };
        seen.push(reference.to_owned());
        let value = walk(doc, target, depth + 1, seen);
        seen.pop();
        return value;
    }
    if let Some(example) = declared_example(doc, schema) {
        return example;
    }
    for key in ["default", "const"] {
        if let Some(value) = schema.get(key) {
            return value.clone();
        }
    }
    if let Some(first) = schema
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|e| e.first())
    {
        return first.clone();
    }
    if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
        let mut merged = Map::new();
        let mut other = None;
        for part in parts {
            match walk(doc, part, depth + 1, seen) {
                Value::Object(map) => merged.extend(map),
                Value::Null => {}
                value => other = Some(value),
            }
        }
        return if merged.is_empty() {
            other.unwrap_or(Value::Null)
        } else {
            Value::Object(merged)
        };
    }
    for key in ["oneOf", "anyOf"] {
        if let Some(first) = schema
            .get(key)
            .and_then(Value::as_array)
            .and_then(|l| l.first())
        {
            return walk(doc, first, depth + 1, seen);
        }
    }
    let kind = first_type(schema).or_else(|| {
        if schema.get("properties").is_some() {
            Some("object")
        } else if schema.get("items").is_some() {
            Some("array")
        } else {
            None
        }
    });
    match kind {
        Some("object") => {
            let mut map = Map::new();
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for (name, property) in properties {
                    let target = resolve(doc, property);
                    if target.get("readOnly").and_then(Value::as_bool) == Some(true) {
                        continue;
                    }
                    map.insert(name.clone(), walk(doc, property, depth + 1, seen));
                }
            } else if let Some(extra) = schema.get("additionalProperties").filter(|v| v.is_object())
            {
                map.insert("cle".to_owned(), walk(doc, extra, depth + 1, seen));
            }
            Value::Object(map)
        }
        Some("array") => match schema.get("items") {
            Some(items) => {
                let item = walk(doc, items, depth + 1, seen);
                if item.is_null() {
                    Value::Array(Vec::new())
                } else {
                    Value::Array(vec![item])
                }
            }
            None => Value::Array(Vec::new()),
        },
        Some("string") => Value::from(string_sample(schema)),
        Some("integer") => {
            let min = schema
                .get("minimum")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .max(1);
            Value::from(min)
        }
        Some("number") => schema
            .get("minimum")
            .and_then(Value::as_f64)
            .map_or_else(|| Value::from(1.5), Value::from),
        Some("boolean") => Value::Bool(true),
        _ => Value::Null,
    }
}

fn string_sample(schema: &Value) -> String {
    let format = schema.get("format").and_then(Value::as_str).unwrap_or("");
    match format {
        "date" => "2026-01-01".to_owned(),
        "date-time" => "2026-01-01T00:00:00Z".to_owned(),
        "time" => "12:00:00".to_owned(),
        "email" => "nom@exemple.fr".to_owned(),
        "uuid" => "00000000-0000-0000-0000-000000000001".to_owned(),
        "uri" | "url" => "https://exemple.fr".to_owned(),
        "hostname" => "exemple.fr".to_owned(),
        "ipv4" => "192.0.2.1".to_owned(),
        "byte" => "dGV4dGU=".to_owned(),
        "password" => "{{motDePasse}}".to_owned(),
        _ => {
            let min = schema.get("minLength").and_then(Value::as_u64).unwrap_or(0);
            let mut text = "texte".to_owned();
            while (text.chars().count() as u64) < min {
                text.push('x');
            }
            text
        }
    }
}
