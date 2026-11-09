//! Requête Arnod ↔ `HttpRequest` d'OpenCollection 1.0.0 (S2, S3, S7, R3, R4).
//!
//! La lecture traduit les champs gérés par Arnod ; l'écriture part du fichier
//! existant et ne remplace que ces champs : scripts, variables, exemples,
//! docs, descriptions, réglages et autres authentifications ou corps restent
//! tels quels (R3).

use serde_json::{Map, Value, json};

use crate::opencollection::schema::{self, Part};
use crate::request::model::{
    ApiKeyLocation, Assertion, Auth, AuthKind, Body, BodyKind, ContractLink, Method, Operator,
    Origin, Pair, ParamKind, Request, Scalar, default_timeout,
};

/// Préfixe de l'étiquette qui lie une requête à une opération de contrat (S7).
pub const CONTRACT_TAG: &str = "contract:";

/// Opérateurs et leur nom OpenCollection (et Bruno).
pub const OPERATORS: [(Operator, &str); 28] = [
    (Operator::Equals, "eq"),
    (Operator::NotEquals, "neq"),
    (Operator::Exists, "isDefined"),
    (Operator::Contains, "contains"),
    (Operator::NotContains, "notContains"),
    (Operator::LessThan, "lt"),
    (Operator::LessOrEqual, "lte"),
    (Operator::GreaterThan, "gt"),
    (Operator::GreaterOrEqual, "gte"),
    (Operator::In, "in"),
    (Operator::NotIn, "notIn"),
    (Operator::Between, "between"),
    (Operator::StartsWith, "startsWith"),
    (Operator::EndsWith, "endsWith"),
    (Operator::Length, "length"),
    (Operator::Matches, "matches"),
    (Operator::NotMatches, "notMatches"),
    (Operator::IsNull, "isNull"),
    (Operator::IsUndefined, "isUndefined"),
    (Operator::IsEmpty, "isEmpty"),
    (Operator::IsNotEmpty, "isNotEmpty"),
    (Operator::IsTruthy, "isTruthy"),
    (Operator::IsFalsy, "isFalsy"),
    (Operator::IsJson, "isJson"),
    (Operator::IsNumber, "isNumber"),
    (Operator::IsText, "isString"),
    (Operator::IsBoolean, "isBoolean"),
    (Operator::IsArray, "isArray"),
];

/// Nom OpenCollection d'un opérateur.
pub fn operator_name(operator: Operator) -> &'static str {
    OPERATORS
        .iter()
        .find(|(op, _)| *op == operator)
        .map_or("eq", |(_, name)| name)
}

/// Opérateur d'un nom OpenCollection.
pub fn operator_from(name: &str) -> Option<Operator> {
    OPERATORS
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(op, _)| *op)
}

/// L'opérateur compare à une valeur attendue.
pub fn takes_value(operator: Operator) -> bool {
    matches!(
        operator,
        Operator::Equals
            | Operator::NotEquals
            | Operator::Contains
            | Operator::NotContains
            | Operator::LessThan
            | Operator::LessOrEqual
            | Operator::GreaterThan
            | Operator::GreaterOrEqual
            | Operator::In
            | Operator::NotIn
            | Operator::Between
            | Operator::StartsWith
            | Operator::EndsWith
            | Operator::Length
            | Operator::Matches
            | Operator::NotMatches
    )
}

/// Expression OpenCollection d'une cible Arnod.
pub fn expression(target: &str) -> String {
    match target {
        "status" => "res.status".to_owned(),
        "responseTime" => "res.responseTime".to_owned(),
        _ => {
            if let Some(name) = target.strip_prefix("headers.") {
                format!("res.headers[\"{}\"]", name.to_lowercase())
            } else if target == "body" || target.starts_with("body.") || target.starts_with("body[")
            {
                format!("res.{target}")
            } else {
                target.to_owned()
            }
        }
    }
}

/// Cible Arnod d'une expression OpenCollection ; une expression inconnue est
/// gardée telle quelle.
pub fn target_from(expression: &str) -> String {
    let expression = expression.trim();
    match expression {
        "res.status" => return "status".to_owned(),
        "res.responseTime" => return "responseTime".to_owned(),
        _ => {}
    }
    if let Some(rest) = expression.strip_prefix("res.headers") {
        let name = rest.strip_prefix('.').map(str::to_owned).or_else(|| {
            rest.strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .map(|r| r.trim_matches(|c| c == '"' || c == '\'').to_owned())
        });
        if let Some(name) = name.filter(|n| !n.is_empty()) {
            return format!("headers.{name}");
        }
    }
    if let Some(rest) = expression.strip_prefix("res.body")
        && (rest.is_empty() || rest.starts_with('.') || rest.starts_with('['))
    {
        return format!("body{rest}");
    }
    expression.to_owned()
}

fn looks_typed(text: &str) -> bool {
    matches!(text, "true" | "false" | "null")
        || text.parse::<f64>().is_ok()
        || text.is_empty()
        || text != text.trim()
        || text.starts_with('"')
        || text.starts_with('\'')
}

/// Valeur attendue écrite en texte : un texte qui ressemblerait à un nombre,
/// un booléen ou `null` est mis entre guillemets.
pub fn value_text(value: &Scalar) -> String {
    match value {
        Scalar::Bool(value) => value.to_string(),
        Scalar::Int(value) => value.to_string(),
        Scalar::Float(value) => value.to_string(),
        Scalar::Text(text) if looks_typed(text) => format!("\"{text}\""),
        Scalar::Text(text) => text.clone(),
    }
}

/// Valeur attendue lue d'un texte.
pub fn scalar_from(text: &str) -> Scalar {
    let trimmed = text.trim();
    for quote in ['"', '\''] {
        if trimmed.len() >= 2 && trimmed.starts_with(quote) && trimmed.ends_with(quote) {
            return Scalar::Text(trimmed[1..trimmed.len() - 1].to_owned());
        }
    }
    match trimmed {
        "true" => Scalar::Bool(true),
        "false" => Scalar::Bool(false),
        _ => trimmed
            .parse::<i64>()
            .map(Scalar::Int)
            .or_else(|_| trimmed.parse::<f64>().map(Scalar::Float))
            .unwrap_or_else(|_| Scalar::Text(text.to_owned())),
    }
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn disabled(item: &Value) -> bool {
    item.get("disabled").and_then(Value::as_bool) == Some(true)
}

fn pairs(list: Option<&Value>) -> Vec<Pair> {
    list.and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| Pair {
                    name: text(item.get("name")),
                    value: text(item.get("value")),
                    enabled: !disabled(item),
                    kind: if item.get("type").and_then(Value::as_str) == Some("path") {
                        ParamKind::Path
                    } else {
                        ParamKind::Query
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

fn method_from(name: &str) -> Option<Method> {
    Some(match name.to_uppercase().as_str() {
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "PATCH" => Method::Patch,
        "DELETE" => Method::Delete,
        "HEAD" => Method::Head,
        "OPTIONS" => Method::Options,
        _ => return None,
    })
}

fn auth_from(value: Option<&Value>) -> Auth {
    let Some(value) = value else {
        return Auth::default();
    };
    if value.as_str() == Some("inherit") {
        return Auth {
            kind: AuthKind::Inherit,
            ..Auth::default()
        };
    }
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "basic" => Auth {
            kind: AuthKind::Basic,
            username: text(value.get("username")),
            password: text(value.get("password")),
            ..Auth::default()
        },
        "bearer" => Auth {
            kind: AuthKind::Bearer,
            token: text(value.get("token")),
            ..Auth::default()
        },
        "apikey" => Auth {
            kind: AuthKind::ApiKey,
            name: text(value.get("key")),
            value: text(value.get("value")),
            location: if value.get("placement").and_then(Value::as_str) == Some("query") {
                ApiKeyLocation::Query
            } else {
                ApiKeyLocation::Header
            },
            ..Auth::default()
        },
        other => Auth {
            kind: AuthKind::Other,
            other: other.to_owned(),
            ..Auth::default()
        },
    }
}

fn body_from(value: Option<&Value>) -> Body {
    let Some(value) = value else {
        return Body::default();
    };
    if value.is_array() {
        return Body {
            kind: BodyKind::Other,
            other: "variantes".to_owned(),
            ..Body::default()
        };
    }
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    let data = value.get("data");
    match kind {
        "json" => Body {
            kind: BodyKind::Json,
            content: text(data),
            ..Body::default()
        },
        "text" => Body {
            kind: BodyKind::Text,
            content: text(data),
            ..Body::default()
        },
        "xml" => Body {
            kind: BodyKind::Xml,
            content: text(data),
            ..Body::default()
        },
        "form-urlencoded" => Body {
            kind: BodyKind::Form,
            form: pairs(data),
            ..Body::default()
        },
        other => Body {
            kind: BodyKind::Other,
            other: other.to_owned(),
            ..Body::default()
        },
    }
}

/// Lien au contrat porté par une étiquette `contract:<fichier>#<opération>`.
fn contract_from(tags: Option<&Value>) -> Option<ContractLink> {
    tags?.as_array()?.iter().find_map(|tag| {
        let rest = tag.as_str()?.strip_prefix(CONTRACT_TAG)?;
        let (file, operation) = rest.split_once('#')?;
        Some(ContractLink {
            file: file.to_owned(),
            operation: operation.to_owned(),
        })
    })
}

/// Étiquette d'un lien au contrat.
pub fn contract_tag(link: &ContractLink) -> String {
    format!("{CONTRACT_TAG}{}#{}", link.file, link.operation)
}

/// Parties du fichier conservées sans être gérées par Arnod.
fn preserved(value: &Value) -> Vec<String> {
    let mut list = Vec::new();
    let present = |pointer: &str| {
        value
            .pointer(pointer)
            .is_some_and(|v| !(v.is_null() || v.as_array().is_some_and(Vec::is_empty)))
    };
    for (pointer, label) in [
        ("/runtime/scripts", "scripts"),
        ("/runtime/variables", "variables de requête"),
        ("/runtime/actions", "actions"),
        ("/examples", "exemples"),
        ("/docs", "documentation"),
    ] {
        if present(pointer) {
            list.push(label.to_owned());
        }
    }
    let other_settings = value
        .get("settings")
        .and_then(Value::as_object)
        .is_some_and(|settings| settings.keys().any(|k| k != "timeout"));
    if other_settings {
        list.push("réglages".to_owned());
    }
    list
}

/// Nom de la requête d'un fichier, s'il en a un.
pub fn name_of(value: &Value) -> Option<String> {
    value
        .pointer("/info/name")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Requête lue d'un fichier OpenCollection (R2). `fallback` nomme une requête
/// sans `info.name`.
pub fn from_value(value: &Value, fallback: &str) -> Result<Request, String> {
    let kind = value
        .pointer("/info/type")
        .and_then(Value::as_str)
        .unwrap_or("http");
    if kind != "http" || value.get("http").is_none() && value.get("info").is_none() {
        return Err(format!("requête {kind} non prise en charge par Arnod"));
    }
    schema::check(Part::HttpRequest, value)?;
    let http = value.get("http");
    let method_name = http
        .and_then(|h| h.get("method"))
        .and_then(Value::as_str)
        .unwrap_or("GET");
    let method = method_from(method_name)
        .ok_or_else(|| format!("méthode {method_name} non prise en charge"))?;
    let mut assertions = Vec::new();
    if let Some(list) = value
        .pointer("/runtime/assertions")
        .and_then(Value::as_array)
    {
        for (index, item) in list.iter().enumerate() {
            let name = item.get("operator").and_then(Value::as_str).unwrap_or("");
            let operator =
                operator_from(name).ok_or_else(|| format!("opérateur « {name} » inconnu"))?;
            let value = item
                .get("value")
                .and_then(Value::as_str)
                .filter(|_| takes_value(operator))
                .map(scalar_from);
            assertions.push(Assertion {
                id: format!("a{index}"),
                target: target_from(&text(item.get("expression"))),
                operator,
                value,
                origin: Origin::Manual,
                enabled: !disabled(item),
            });
        }
    }
    let timeout_ms = value
        .pointer("/settings/timeout")
        .and_then(Value::as_f64)
        .filter(|ms| *ms >= 0.0)
        .map_or_else(default_timeout, |ms| ms as u64);
    Ok(Request {
        name: name_of(value).unwrap_or_else(|| fallback.to_owned()),
        method,
        url: text(http.and_then(|h| h.get("url"))),
        params: pairs(http.and_then(|h| h.get("params"))),
        headers: pairs(http.and_then(|h| h.get("headers"))),
        auth: auth_from(http.and_then(|h| h.get("auth"))),
        body: body_from(http.and_then(|h| h.get("body"))),
        timeout_ms,
        assertions,
        contract: contract_from(value.pointer("/info/tags")),
        preserved: preserved(value),
    })
}

/// Applique `edit` à l'objet `key` de `map`, créé vide s'il manque ou n'est
/// pas un objet ; sa place dans le fichier est gardée.
fn with_object<T>(
    map: &mut Map<String, Value>,
    key: &str,
    edit: impl FnOnce(&mut Map<String, Value>) -> T,
) -> Option<T> {
    let entry = map
        .entry(key.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    entry.as_object_mut().map(edit)
}

/// Remplace `key` par `value`, ou le retire si `value` est `None`.
fn set(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            map.insert(key.to_owned(), value);
        }
        None => {
            map.remove(key);
        }
    }
}

/// Retire `key` de `map` s'il désigne un objet vide.
fn prune(map: &mut Map<String, Value>, key: &str) {
    if map
        .get(key)
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        map.remove(key);
    }
}

/// Description d'origine d'une ligne de même clé, conservée (R3).
fn description(previous: Option<&Value>, matches: impl Fn(&Value) -> bool) -> Option<Value> {
    previous?
        .as_array()?
        .iter()
        .find(|item| matches(item))?
        .get("description")
        .cloned()
}

fn pair_values(list: &[Pair], previous: Option<&Value>, params: bool) -> Option<Value> {
    if list.is_empty() {
        return None;
    }
    let items = list
        .iter()
        .map(|pair| {
            let mut item = Map::new();
            item.insert("name".to_owned(), Value::from(pair.name.clone()));
            item.insert("value".to_owned(), Value::from(pair.value.clone()));
            let kind = if pair.kind == ParamKind::Path {
                "path"
            } else {
                "query"
            };
            if let Some(text) = description(previous, |p| {
                p.get("name").and_then(Value::as_str) == Some(pair.name.as_str())
            }) {
                item.insert("description".to_owned(), text);
            }
            if params {
                item.insert("type".to_owned(), Value::from(kind));
            }
            if !pair.enabled {
                item.insert("disabled".to_owned(), Value::Bool(true));
            }
            Value::Object(item)
        })
        .collect();
    Some(Value::Array(items))
}

fn auth_value(auth: &Auth, previous: Option<&Value>) -> Option<Value> {
    match auth.kind {
        AuthKind::None => None,
        AuthKind::Inherit => Some(Value::from("inherit")),
        AuthKind::Other => previous.cloned(),
        AuthKind::Basic => Some(json!({
            "type": "basic",
            "username": auth.username,
            "password": auth.password,
        })),
        AuthKind::Bearer => Some(json!({ "type": "bearer", "token": auth.token })),
        AuthKind::ApiKey => Some(json!({
            "type": "apikey",
            "key": auth.name,
            "value": auth.value,
            "placement": if auth.location == ApiKeyLocation::Query { "query" } else { "header" },
        })),
    }
}

fn body_value(body: &Body, previous: Option<&Value>) -> Option<Value> {
    let raw = |kind: &str| Some(json!({ "type": kind, "data": body.content }));
    match body.kind {
        BodyKind::None => None,
        BodyKind::Other => previous.cloned(),
        BodyKind::Json => raw("json"),
        BodyKind::Text => raw("text"),
        BodyKind::Xml => raw("xml"),
        BodyKind::Form => Some(json!({
            "type": "form-urlencoded",
            "data": pair_values(&body.form, previous.and_then(|p| p.get("data")), false)
                .unwrap_or_else(|| Value::Array(Vec::new())),
        })),
    }
}

fn assertion_values(list: &[Assertion], previous: Option<&Value>) -> Option<Value> {
    if list.is_empty() {
        return None;
    }
    let items = list
        .iter()
        .map(|assertion| {
            let expression = expression(&assertion.target);
            let operator = operator_name(assertion.operator);
            let mut item = Map::new();
            item.insert("expression".to_owned(), Value::from(expression.clone()));
            item.insert("operator".to_owned(), Value::from(operator));
            if takes_value(assertion.operator) {
                let text = assertion.value.as_ref().map(value_text).unwrap_or_default();
                item.insert("value".to_owned(), Value::from(text));
            }
            if !assertion.enabled {
                item.insert("disabled".to_owned(), Value::Bool(true));
            }
            if let Some(text) = description(previous, |p| {
                p.get("expression").and_then(Value::as_str) == Some(expression.as_str())
                    && p.get("operator").and_then(Value::as_str) == Some(operator)
            }) {
                item.insert("description".to_owned(), text);
            }
            Value::Object(item)
        })
        .collect();
    Some(Value::Array(items))
}

/// Écrit dans le nom d'une requête (renommage, duplication).
pub fn set_name(value: &mut Value, name: &str) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    if let Some(root) = value.as_object_mut() {
        with_object(root, "info", |info| {
            info.insert("name".to_owned(), Value::from(name));
            info.entry("type").or_insert_with(|| Value::from("http"));
        });
    }
}

/// Fichier OpenCollection d'une requête : `existing` (le fichier actuel) dont
/// seuls les champs gérés par Arnod sont remplacés (R3).
pub fn into_value(request: &Request, existing: Option<&Value>) -> Value {
    let mut root = existing
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let Some(map) = root.as_object_mut() else {
        return root;
    };

    with_object(map, "info", |info| {
        info.insert("name".to_owned(), Value::from(request.name.clone()));
        info.insert("type".to_owned(), Value::from("http"));
        let mut tags: Vec<Value> = info
            .get("tags")
            .and_then(Value::as_array)
            .map(|tags| {
                tags.iter()
                    .filter(|t| !t.as_str().is_some_and(|t| t.starts_with(CONTRACT_TAG)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if let Some(link) = &request.contract {
            tags.push(Value::from(contract_tag(link)));
        }
        set(
            info,
            "tags",
            (!tags.is_empty()).then_some(Value::Array(tags)),
        );
    });

    with_object(map, "http", |http| {
        let previous = http.clone();
        http.insert("method".to_owned(), Value::from(request.method.as_str()));
        http.insert("url".to_owned(), Value::from(request.url.clone()));
        set(
            http,
            "params",
            pair_values(&request.params, previous.get("params"), true),
        );
        set(
            http,
            "headers",
            pair_values(&request.headers, previous.get("headers"), false),
        );
        set(
            http,
            "body",
            body_value(&request.body, previous.get("body")),
        );
        set(
            http,
            "auth",
            auth_value(&request.auth, previous.get("auth")),
        );
    });

    with_object(map, "runtime", |runtime| {
        let previous = runtime.get("assertions").cloned();
        set(
            runtime,
            "assertions",
            assertion_values(&request.assertions, previous.as_ref()),
        );
    });
    prune(map, "runtime");

    with_object(map, "settings", |settings| {
        if request.timeout_ms == default_timeout() {
            if settings.get("timeout").is_some_and(Value::is_number) {
                settings.remove("timeout");
            }
        } else {
            settings.insert("timeout".to_owned(), Value::from(request.timeout_ms));
        }
    });
    prune(map, "settings");
    root
}
