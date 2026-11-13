//! Filtre JSONPath du corps (R5 à R7) : correspondances avec leur chemin canonique et leur valeur,
//! vérification proposée pour chacune, expression invalide expliquée en français.
//!
//! L'évaluation suit la RFC 9535 (crate `serde_json_path`).

use serde::Serialize;
use serde_json::Value;
use serde_json_path::{JsonPath, PathElement};

use super::path::{self, Part};
use crate::request::model::{Assertion, Operator, Origin, Scalar};

/// Nombre maximal de correspondances renvoyées ; `total` les compte toutes.
pub const MAX_MATCHES: usize = 500;

/// Longueur maximale de la valeur montrée d'une correspondance.
const VALUE_PREVIEW: usize = 200;

/// Correspondance du filtre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterMatch {
    /// Chemin canonique, celui des jetons de la mise en forme (`$.items[0].sku`).
    pub path: String,
    /// Valeur en JSON compact, raccourcie au-delà de 200 caractères.
    pub value: String,
    /// Vérification tirée de cette valeur (R6), absente si un nom de membre ne s'écrit pas en
    /// notation pointée.
    pub assertion: Option<Assertion>,
}

/// Résultat du filtre : correspondances, ou erreur en français (R7).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterResult {
    pub matches: Vec<FilterMatch>,
    pub total: usize,
    pub error: Option<String>,
}

impl FilterResult {
    fn error(message: impl Into<String>) -> Self {
        Self {
            matches: Vec::new(),
            total: 0,
            error: Some(message.into()),
        }
    }
}

/// Évalue `expression` sur le corps JSON `body` (R5). Une expression vide ne donne rien.
pub fn filter(body: &str, expression: &str) -> FilterResult {
    let expression = expression.trim();
    if expression.is_empty() {
        return FilterResult {
            matches: Vec::new(),
            total: 0,
            error: None,
        };
    }
    let query = match JsonPath::parse(expression) {
        Ok(query) => query,
        Err(error) => return FilterResult::error(explain(expression, error.position())),
    };
    if body.trim().is_empty() {
        return FilterResult::error("La réponse n'a pas de corps à filtrer.");
    }
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return FilterResult::error(
            "Le filtre JSONPath s'applique à un corps JSON : celui-ci n'en est pas.",
        );
    };
    let located = query.query_located(&json).all();
    let total = located.len();
    let matches = located
        .into_iter()
        .take(MAX_MATCHES)
        .map(|node| {
            let parts: Vec<Part<'_>> = node
                .location()
                .iter()
                .map(|element| match element {
                    PathElement::Name(name) => Part::Name(name),
                    PathElement::Index(index) => Part::Index(*index),
                })
                .collect();
            let value = node.node();
            let assertion = path::assertion_target(parts.iter().cloned())
                .map(|target| assertion_for(target, value));
            FilterMatch {
                path: path::canonical(parts),
                value: shorten(&value.to_string()),
                assertion,
            }
        })
        .collect();
    FilterResult {
        matches,
        total,
        error: None,
    }
}

fn shorten(text: &str) -> String {
    if text.chars().count() <= VALUE_PREVIEW {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(VALUE_PREVIEW).collect();
    short.push('…');
    short
}

/// Vérification tirée d'une valeur (R6) : égale pour une valeur simple, nulle, longueur d'un
/// tableau, présence d'un objet.
pub fn assertion_for(target: String, value: &Value) -> Assertion {
    let (operator, expected) = match value {
        Value::String(text) => (Operator::Equals, Some(Scalar::Text(text.clone()))),
        Value::Bool(flag) => (Operator::Equals, Some(Scalar::Bool(*flag))),
        Value::Number(number) => (
            Operator::Equals,
            number
                .as_i64()
                .map(Scalar::Int)
                .or_else(|| number.as_f64().map(Scalar::Float)),
        ),
        Value::Null => (Operator::IsNull, None),
        Value::Array(items) => (
            Operator::Length,
            Some(Scalar::Int(i64::try_from(items.len()).unwrap_or(i64::MAX))),
        ),
        Value::Object(_) => (Operator::Exists, None),
    };
    let id = format!(
        "filtre-{}",
        target
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );
    Assertion {
        id,
        target,
        operator,
        value: expected,
        origin: Origin::Proposed,
        enabled: true,
    }
}

/// Explication en français d'une expression refusée (R7).
fn explain(expression: &str, position: usize) -> String {
    if !expression.starts_with('$') {
        return "Une expression JSONPath commence par $ (par exemple $.items[*].sku).".to_owned();
    }
    let mut brackets = 0i32;
    let mut parens = 0i32;
    let mut quote: Option<char> = None;
    for c in expression.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '\'' | '"' => quote = Some(c),
                '[' => brackets += 1,
                ']' => brackets -= 1,
                '(' => parens += 1,
                ')' => parens -= 1,
                _ => {}
            },
        }
    }
    if quote.is_some() {
        return "Guillemet non fermé dans l'expression.".to_owned();
    }
    if brackets > 0 {
        return "Crochet « [ » non fermé dans l'expression.".to_owned();
    }
    if parens > 0 {
        return "Parenthèse « ( » non fermée dans l'expression.".to_owned();
    }
    if expression.ends_with('.') {
        return "Un nom de membre est attendu après le dernier point.".to_owned();
    }
    let at = expression
        .char_indices()
        .position(|(byte, _)| byte >= position)
        .unwrap_or(expression.chars().count());
    let excerpt: String = expression.chars().skip(at).take(12).collect();
    if excerpt.is_empty() {
        "Expression JSONPath incomplète.".to_owned()
    } else {
        format!(
            "Expression JSONPath invalide au caractère {} : « {excerpt} ».",
            at + 1
        )
    }
}
