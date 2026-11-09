//! Schéma officiel OpenCollection 1.0.0, embarqué (R1, R2).
//!
//! Source : <https://schema.opencollection.com/opencollection/v1.0.0.json>,
//! conservé tel quel dans `docs/opencollection/`.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::contract::conformance::validate_at;

const SCHEMA_TEXT: &str =
    include_str!("../../../docs/opencollection/opencollection-v1.0.0.schema.json");

/// Nombre d'écarts repris dans le message.
const SHOWN: usize = 3;

/// Partie du format à valider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// `opencollection.yml`.
    Root,
    /// Fichier d'une requête HTTP.
    HttpRequest,
    /// `folder.yml`.
    Folder,
    /// Fichier d'un environnement.
    Environment,
}

fn schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_TEXT).unwrap_or(Value::Bool(true)))
}

/// Écarts d'une valeur à la partie `part` du schéma, chemins préfixés par `fichier`.
pub fn errors(part: Part, value: &Value) -> Vec<String> {
    let doc = schema();
    let target = match part {
        Part::Root => doc.clone(),
        Part::HttpRequest => json!({ "$ref": "#/$defs/HttpRequest" }),
        Part::Folder => json!({ "$ref": "#/$defs/Folder" }),
        Part::Environment => json!({ "$ref": "#/$defs/Environment" }),
    };
    validate_at(doc, &target, value, "fichier")
}

/// Valide une valeur ; l'erreur résume les premiers écarts, en français.
pub fn check(part: Part, value: &Value) -> Result<(), String> {
    let list = errors(part, value);
    if list.is_empty() {
        return Ok(());
    }
    let shown: Vec<&str> = list.iter().take(SHOWN).map(String::as_str).collect();
    let more = list.len().saturating_sub(SHOWN);
    Err(format!(
        "non conforme à OpenCollection 1.0.0 : {}{}",
        shown.join(" ; "),
        if more > 0 {
            format!(" (+{more} autres écarts)")
        } else {
            String::new()
        }
    ))
}
