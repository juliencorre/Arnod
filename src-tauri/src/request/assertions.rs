//! Évaluation des vérifications et vérifications proposées (R10, R11).

use serde::Serialize;
use serde_json::Value;

use crate::request::model::{Assertion, DEFAULT_MAX_DURATION_MS, Operator, Origin, Scalar};

/// Ce que les vérifications lisent dans une réponse.
#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub duration_ms: u64,
    pub headers: Vec<(String, String)>,
    /// Corps en texte (vide pour un corps binaire).
    pub body: String,
}

/// Résultat d'une vérification (V3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResult {
    pub id: String,
    pub passed: bool,
    /// Détail de l'échec, en français.
    pub message: Option<String>,
    /// Valeur reçue, en texte.
    pub actual: Option<String>,
}

/// Valeur lue dans la réponse pour une cible.
fn lookup(target: &str, response: &ResponseData, json: Option<&Value>) -> Option<Value> {
    if target == "status" {
        return Some(Value::from(response.status));
    }
    if target == "responseTime" {
        return Some(Value::from(response.duration_ms));
    }
    if let Some(name) = target.strip_prefix("headers.") {
        return response
            .headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| Value::from(value.clone()));
    }
    if let Some(path) = target.strip_prefix("body") {
        let path = path.strip_prefix('.').unwrap_or(path);
        let mut current = json?;
        if path.is_empty() {
            return Some(current.clone());
        }
        for segment in split_path(path) {
            current = match segment {
                Segment::Key(key) => current.get(key.as_str())?,
                Segment::Index(index) => current.get(index)?,
            };
        }
        return Some(current.clone());
    }
    None
}

enum Segment {
    Key(String),
    Index(usize),
}

/// `a.b[0].c` → [a, b, 0, c].
fn split_path(path: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    for part in path.split('.') {
        let mut rest = part;
        if let Some(bracket) = rest.find('[') {
            if bracket > 0 {
                segments.push(Segment::Key(rest[..bracket].to_owned()));
            }
            rest = &rest[bracket..];
            while let Some(stripped) = rest.strip_prefix('[') {
                let Some(end) = stripped.find(']') else { break };
                if let Ok(index) = stripped[..end].parse() {
                    segments.push(Segment::Index(index));
                }
                rest = &stripped[end + 1..];
            }
        } else if !rest.is_empty() {
            segments.push(Segment::Key(rest.to_owned()));
        }
    }
    segments
}

fn show(value: &Value) -> String {
    match value {
        Value::String(text) => format!("\"{text}\""),
        other => other.to_string(),
    }
}

fn scalar_text(value: &Scalar) -> String {
    match value {
        Scalar::Bool(value) => value.to_string(),
        Scalar::Int(value) => value.to_string(),
        Scalar::Float(value) => value.to_string(),
        Scalar::Text(value) => format!("\"{value}\""),
    }
}

fn as_f64(scalar: &Scalar) -> Option<f64> {
    match scalar {
        Scalar::Int(value) => Some(*value as f64),
        Scalar::Float(value) => Some(*value),
        Scalar::Text(text) => text.trim().parse().ok(),
        Scalar::Bool(_) => None,
    }
}

fn equals(actual: &Value, expected: &Scalar) -> bool {
    match (actual, expected) {
        (Value::Bool(a), Scalar::Bool(b)) => a == b,
        (Value::Number(a), expected) => as_f64(expected)
            .zip(a.as_f64())
            .is_some_and(|(b, a)| (a - b).abs() < f64::EPSILON),
        (Value::String(a), Scalar::Text(b)) => a == b,
        (Value::String(a), Scalar::Int(b)) => *a == b.to_string(),
        (Value::Bool(a), Scalar::Text(b)) => a.to_string() == *b,
        _ => false,
    }
}

fn number_of(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

fn expected_text(expected: Option<&Scalar>) -> String {
    match expected {
        Some(Scalar::Text(text)) => text.clone(),
        Some(Scalar::Bool(flag)) => flag.to_string(),
        Some(Scalar::Int(n)) => n.to_string(),
        Some(Scalar::Float(n)) => n.to_string(),
        None => String::new(),
    }
}

/// Éléments d'une liste attendue : `a,b`, ou `[a, b]` (opérateurs `in`, `between`).
fn list_items(expected: Option<&Scalar>) -> Vec<Scalar> {
    let text = expected_text(expected);
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(text.trim());
    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(crate::opencollection::request::scalar_from)
        .collect()
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::String(text) => text.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(map) => map.is_empty(),
        Value::Null => true,
        _ => false,
    }
}

/// Vrai au sens de JavaScript, comme dans Bruno.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0 && !n.is_nan()),
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}

/// Évalue une vérification (R11, R4 du format OpenCollection).
pub fn evaluate(
    assertion: &Assertion,
    response: &ResponseData,
    json: Option<&Value>,
) -> AssertionResult {
    let raw = lookup(&assertion.target, response, json);
    let found = raw.clone().filter(|value| !value.is_null());
    let actual = found.as_ref().map(show);
    let expected = assertion.value.as_ref();
    let expected_text = expected.map(scalar_text).unwrap_or_default();
    let missing = || {
        if assertion.target.starts_with("headers.") {
            "en-tête absent de la réponse".to_owned()
        } else {
            "valeur absente de la réponse".to_owned()
        }
    };
    let fail = |ok: bool, message: String| (ok, (!ok).then_some(message));

    let (passed, message) = match (assertion.operator, &found) {
        (Operator::Exists, found) => (found.is_some(), found.is_none().then(missing)),
        (Operator::IsUndefined, _) => fail(
            raw.is_none(),
            format!(
                "devait être absent, reçu {}",
                raw.as_ref().map(show).unwrap_or_default()
            ),
        ),
        (Operator::IsNull, _) => fail(
            raw.as_ref().is_some_and(Value::is_null),
            format!(
                "attendu null, reçu {}",
                raw.as_ref().map_or_else(|| "rien".to_owned(), show)
            ),
        ),
        (Operator::IsTruthy, _) => fail(
            truthy(found.as_ref()),
            format!(
                "attendu une valeur vraie, reçu {}",
                actual.clone().unwrap_or_default()
            ),
        ),
        (Operator::IsFalsy, _) => fail(
            !truthy(found.as_ref()),
            format!(
                "attendu une valeur fausse, reçu {}",
                actual.clone().unwrap_or_default()
            ),
        ),
        (Operator::Matches | Operator::NotMatches, _) => (
            false,
            Some("opérateur non évalué par Arnod (expression régulière)".to_owned()),
        ),
        (Operator::NotEquals | Operator::NotContains | Operator::NotIn, None) => (true, None),
        (_, None) => (false, Some(missing())),
        (Operator::Equals, Some(value)) => fail(
            expected.is_some_and(|expected| equals(value, expected)),
            format!("attendu {expected_text}, reçu {}", show(value)),
        ),
        (Operator::NotEquals, Some(value)) => fail(
            !expected.is_some_and(|expected| equals(value, expected)),
            format!("ne devait pas valoir {expected_text}"),
        ),
        (Operator::Contains | Operator::NotContains, Some(value)) => {
            let contains = match (value, expected) {
                (Value::String(text), Some(part)) => {
                    text.contains(self::expected_text(Some(part)).as_str())
                }
                (Value::Array(items), Some(expected)) => {
                    items.iter().any(|item| equals(item, expected))
                }
                _ => false,
            };
            if assertion.operator == Operator::Contains {
                fail(
                    contains,
                    format!("devait contenir {expected_text}, reçu {}", show(value)),
                )
            } else {
                fail(
                    !contains,
                    format!(
                        "ne devait pas contenir {expected_text}, reçu {}",
                        show(value)
                    ),
                )
            }
        }
        (
            Operator::LessThan
            | Operator::LessOrEqual
            | Operator::GreaterThan
            | Operator::GreaterOrEqual,
            Some(value),
        ) => match (number_of(value), expected.and_then(as_f64)) {
            (Some(number), Some(limit)) => {
                let (ok, word) = match assertion.operator {
                    Operator::LessThan => (number < limit, "inférieur à"),
                    Operator::LessOrEqual => (number <= limit, "inférieur ou égal à"),
                    Operator::GreaterThan => (number > limit, "supérieur à"),
                    _ => (number >= limit, "supérieur ou égal à"),
                };
                fail(
                    ok,
                    format!("attendu {word} {expected_text}, reçu {}", show(value)),
                )
            }
            _ => (
                false,
                Some(format!("« {} » n'est pas un nombre", show(value))),
            ),
        },
        (Operator::Between, Some(value)) => {
            let bounds: Vec<f64> = list_items(expected).iter().filter_map(as_f64).collect();
            match (number_of(value), bounds.as_slice()) {
                (Some(number), [min, max]) => fail(
                    *min <= number && number <= *max,
                    format!("attendu entre {min} et {max}, reçu {}", show(value)),
                ),
                _ => (
                    false,
                    Some("bornes attendues sous la forme min,max".to_owned()),
                ),
            }
        }
        (Operator::In | Operator::NotIn, Some(value)) => {
            let listed = list_items(expected).iter().any(|item| equals(value, item));
            if assertion.operator == Operator::In {
                fail(
                    listed,
                    format!(
                        "attendu parmi {}, reçu {}",
                        self::expected_text(expected),
                        show(value)
                    ),
                )
            } else {
                fail(
                    !listed,
                    format!("ne devait pas être parmi {}", self::expected_text(expected)),
                )
            }
        }
        (Operator::StartsWith | Operator::EndsWith, Some(value)) => {
            let part = self::expected_text(expected);
            let ok = value.as_str().is_some_and(|text| {
                if assertion.operator == Operator::StartsWith {
                    text.starts_with(&part)
                } else {
                    text.ends_with(&part)
                }
            });
            let word = if assertion.operator == Operator::StartsWith {
                "commencer"
            } else {
                "finir"
            };
            fail(
                ok,
                format!("devait {word} par {expected_text}, reçu {}", show(value)),
            )
        }
        (Operator::Length, Some(value)) => {
            let length = match value {
                Value::String(text) => Some(text.chars().count()),
                Value::Array(items) => Some(items.len()),
                Value::Object(map) => Some(map.len()),
                _ => None,
            };
            let wanted = expected.and_then(as_f64);
            fail(
                length
                    .zip(wanted)
                    .is_some_and(|(l, w)| (l as f64 - w).abs() < f64::EPSILON),
                format!(
                    "longueur attendue {expected_text}, reçue {}",
                    length.map_or_else(|| "aucune".to_owned(), |l| l.to_string())
                ),
            )
        }
        (Operator::IsEmpty, Some(value)) => fail(
            is_empty_value(value),
            format!("attendu vide, reçu {}", show(value)),
        ),
        (Operator::IsNotEmpty, Some(value)) => fail(
            !is_empty_value(value),
            "attendu non vide, reçu une valeur vide".to_owned(),
        ),
        (Operator::IsJson, Some(value)) => fail(
            value.is_object() || value.is_array(),
            format!("attendu un objet JSON, reçu {}", show(value)),
        ),
        (Operator::IsNumber, Some(value)) => fail(
            value.is_number(),
            format!("attendu number, reçu {}", show(value)),
        ),
        (Operator::IsText, Some(value)) => fail(
            value.is_string(),
            format!("attendu texte, reçu {}", show(value)),
        ),
        (Operator::IsBoolean, Some(value)) => fail(
            value.is_boolean(),
            format!("attendu un booléen, reçu {}", show(value)),
        ),
        (Operator::IsArray, Some(value)) => fail(
            value.is_array(),
            format!("attendu un tableau, reçu {}", show(value)),
        ),
    };
    AssertionResult {
        id: assertion.id.clone(),
        passed,
        message,
        actual,
    }
}

/// Évalue les vérifications actives d'une réponse (R11) ; une vérification
/// désactivée n'a pas de résultat.
pub fn evaluate_all(assertions: &[Assertion], response: &ResponseData) -> Vec<AssertionResult> {
    let json: Option<Value> = serde_json::from_str(&response.body).ok();
    assertions
        .iter()
        .filter(|assertion| assertion.enabled)
        .map(|assertion| evaluate(assertion, response, json.as_ref()))
        .collect()
}

/// Une requête réussit si toutes ses vérifications réussissent ; sans aucune,
/// si le statut est inférieur à 400 (R11).
pub fn request_passed(results: &[AssertionResult], status: u16) -> bool {
    if results.is_empty() {
        status < 400
    } else {
        results.iter().all(|result| result.passed)
    }
}

/// Nom de champ dont la valeur varie d'un appel à l'autre (R10).
fn is_volatile(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower == "id"
        || lower.ends_with("id")
        || lower.ends_with("_id")
        || lower.ends_with("at")
        || lower.contains("date")
        || lower.contains("time")
        || lower.contains("token")
        || lower.contains("uuid")
}

/// Nombre maximal de vérifications proposées pour une réponse.
const MAX_PROPOSED: usize = 8;

/// Vérifications proposées d'après la réponse reçue (R10).
pub fn propose(response: &ResponseData) -> Vec<Assertion> {
    let mut proposed = vec![Assertion {
        id: "status".to_owned(),
        target: "status".to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Int(i64::from(response.status))),
        origin: Origin::Proposed,
        enabled: true,
    }];
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&response.body) {
        for (key, value) in &map {
            if proposed.len() >= MAX_PROPOSED {
                break;
            }
            let target = format!("body.{key}");
            let scalar = match value {
                Value::String(text) => Some(Scalar::Text(text.clone())),
                Value::Bool(flag) => Some(Scalar::Bool(*flag)),
                Value::Number(number) => number
                    .as_i64()
                    .map(Scalar::Int)
                    .or_else(|| number.as_f64().map(Scalar::Float)),
                _ => None,
            };
            let (operator, value) = match scalar {
                Some(scalar) if !is_volatile(key) => (Operator::Equals, Some(scalar)),
                _ => (Operator::Exists, None),
            };
            proposed.push(Assertion {
                id: format!("body-{key}"),
                target,
                operator,
                value,
                origin: Origin::Proposed,
                enabled: true,
            });
        }
    }
    proposed
}

/// Seuil de la vérification de durée par défaut, pour R16.
pub fn duration_limit(assertions: &[Assertion]) -> Option<i64> {
    assertions
        .iter()
        .find(|a| a.enabled && a.target == "responseTime" && a.operator == Operator::LessThan)
        .and_then(|a| match a.value {
            Some(Scalar::Int(limit)) => Some(limit),
            Some(Scalar::Float(limit)) => Some(limit as i64),
            _ => None,
        })
}

/// Seuil de durée appliqué par défaut (V5).
pub const DEFAULT_LIMIT: i64 = DEFAULT_MAX_DURATION_MS;

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: u16, body: &str) -> ResponseData {
        ResponseData {
            status,
            duration_ms: 42,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: body.to_owned(),
        }
    }

    fn assertion(target: &str, operator: Operator, value: Option<Scalar>) -> Assertion {
        Assertion {
            id: "a".into(),
            target: target.into(),
            operator,
            value,
            origin: Origin::Manual,
            enabled: true,
        }
    }

    fn check(a: &Assertion, r: &ResponseData) -> AssertionResult {
        evaluate_all(std::slice::from_ref(a), r).remove(0)
    }

    #[test]
    fn statut_egal() {
        let r = response(201, "{}");
        assert!(
            check(
                &assertion("status", Operator::Equals, Some(Scalar::Int(201))),
                &r
            )
            .passed
        );
        let failed = check(
            &assertion("status", Operator::Equals, Some(Scalar::Int(200))),
            &r,
        );
        assert!(!failed.passed);
        assert_eq!(failed.message.as_deref(), Some("attendu 200, reçu 201"));
    }

    #[test]
    fn chemin_dans_le_corps_avec_index() {
        let r = response(200, r#"{"items":[{"sku":"A","qty":2}],"total":59.8}"#);
        assert!(
            check(
                &assertion(
                    "body.items[0].sku",
                    Operator::Equals,
                    Some(Scalar::Text("A".into()))
                ),
                &r
            )
            .passed
        );
        assert!(
            check(
                &assertion("body.items[0].qty", Operator::IsNumber, None),
                &r
            )
            .passed
        );
        assert!(
            check(
                &assertion("body.total", Operator::GreaterThan, Some(Scalar::Int(50))),
                &r
            )
            .passed
        );
        assert!(!check(&assertion("body.nope", Operator::Exists, None), &r).passed);
    }

    #[test]
    fn en_tete_insensible_a_la_casse() {
        let r = response(200, "{}");
        assert!(
            check(
                &assertion(
                    "headers.content-type",
                    Operator::Contains,
                    Some(Scalar::Text("json".into()))
                ),
                &r
            )
            .passed
        );
        let missing = check(&assertion("headers.Location", Operator::Exists, None), &r);
        assert!(!missing.passed);
        assert_eq!(
            missing.message.as_deref(),
            Some("en-tête absent de la réponse")
        );
    }

    #[test]
    fn type_attendu() {
        let r = response(200, r#"{"amount":"129.90"}"#);
        let failed = check(&assertion("body.amount", Operator::IsNumber, None), &r);
        assert!(!failed.passed);
        assert_eq!(
            failed.message.as_deref(),
            Some("attendu number, reçu \"129.90\"")
        );
        assert!(check(&assertion("body.amount", Operator::IsText, None), &r).passed);
    }

    #[test]
    fn duree() {
        let r = response(200, "{}");
        assert!(
            check(
                &assertion("responseTime", Operator::LessThan, Some(Scalar::Int(500))),
                &r
            )
            .passed
        );
        assert!(
            !check(
                &assertion("responseTime", Operator::LessThan, Some(Scalar::Int(10))),
                &r
            )
            .passed
        );
    }

    #[test]
    fn corps_non_json_donne_valeur_absente() {
        let r = response(200, "texte");
        assert!(!check(&assertion("body.a", Operator::Exists, None), &r).passed);
        assert!(
            check(
                &assertion("body.a", Operator::NotEquals, Some(Scalar::Int(1))),
                &r
            )
            .passed
        );
    }

    #[test]
    fn reussite_de_la_requete() {
        let a = assertion("status", Operator::Equals, Some(Scalar::Int(200)));
        let r = response(200, "{}");
        let results = evaluate_all(std::slice::from_ref(&a), &r);
        assert!(request_passed(&results, 200));
        assert!(request_passed(&[], 399));
        assert!(!request_passed(&[], 404));
        // Désactivée : pas de résultat, et le statut décide.
        let mut off = a.clone();
        off.enabled = false;
        assert!(evaluate_all(std::slice::from_ref(&off), &r).is_empty());
    }

    #[test]
    fn r4_operateurs_d_opencollection() {
        let r = response(
            200,
            r#"{ "nom": "Alice", "age": 30, "tags": ["a", "b"], "vide": "", "rien": null, "ok": true }"#,
        );
        let text = |t: &str| Some(Scalar::Text(t.to_owned()));
        let cases = [
            (
                assertion("body.age", Operator::GreaterOrEqual, Some(Scalar::Int(30))),
                true,
            ),
            (
                assertion("body.age", Operator::LessOrEqual, Some(Scalar::Int(29))),
                false,
            ),
            (
                assertion("body.age", Operator::Between, text("18,65")),
                true,
            ),
            (
                assertion("body.age", Operator::Between, text("[40, 65]")),
                false,
            ),
            (assertion("body.nom", Operator::In, text("Alice,Bob")), true),
            (
                assertion("body.nom", Operator::NotIn, text("[Alice, Bob]")),
                false,
            ),
            (
                assertion("body.nom", Operator::StartsWith, text("Al")),
                true,
            ),
            (assertion("body.nom", Operator::EndsWith, text("ce")), true),
            (
                assertion("body.nom", Operator::NotContains, text("lic")),
                false,
            ),
            (
                assertion("body.tags", Operator::Length, Some(Scalar::Int(2))),
                true,
            ),
            (assertion("body.vide", Operator::IsEmpty, None), true),
            (assertion("body.tags", Operator::IsNotEmpty, None), true),
            (assertion("body.rien", Operator::IsNull, None), true),
            (assertion("body.absent", Operator::IsUndefined, None), true),
            (assertion("body.nom", Operator::IsUndefined, None), false),
            (assertion("body.ok", Operator::IsTruthy, None), true),
            (assertion("body.vide", Operator::IsFalsy, None), true),
            (assertion("body", Operator::IsJson, None), true),
            (assertion("body.ok", Operator::IsBoolean, None), true),
            (assertion("body.tags", Operator::IsArray, None), true),
            (assertion("body.nom", Operator::Matches, text("^A")), false),
        ];
        for (a, expected) in cases {
            assert_eq!(
                check(&a, &r).passed,
                expected,
                "{:?} {:?}",
                a.target,
                a.operator
            );
        }
        let regex = check(&assertion("body.nom", Operator::Matches, text("^A")), &r);
        assert_eq!(
            regex.message.as_deref(),
            Some("opérateur non évalué par Arnod (expression régulière)")
        );
    }

    #[test]
    fn propositions_sans_champs_variables() {
        let r = response(
            201,
            r#"{"id":"ord_1","status":"pending","createdAt":"2026","total":59.8,"items":[1],"ok":true}"#,
        );
        let proposed = propose(&r);
        let by_target = |t: &str| proposed.iter().find(|a| a.target == t).unwrap();
        assert_eq!(proposed[0].target, "status");
        assert_eq!(by_target("body.status").operator, Operator::Equals);
        assert_eq!(by_target("body.id").operator, Operator::Exists);
        assert_eq!(by_target("body.createdAt").operator, Operator::Exists);
        assert_eq!(by_target("body.items").operator, Operator::Exists);
        assert_eq!(by_target("body.ok").value, Some(Scalar::Bool(true)));
        assert!(proposed.iter().all(|a| a.origin == Origin::Proposed));
        // les propositions passent sur la réponse dont elles viennent
        assert!(
            evaluate_all(&proposed, &r)
                .iter()
                .all(|result| result.passed)
        );
    }

    #[test]
    fn propositions_limitees() {
        let body: String = format!(
            "{{{}}}",
            (0..20)
                .map(|i| format!("\"k{i}\":\"v\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(propose(&response(200, &body)).len(), 8);
    }
}
