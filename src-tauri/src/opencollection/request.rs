//! Requête Arnod ↔ `HttpRequest` d'OpenCollection 1.0.0 (S2, S3, S7, R3, R4).
//!
//! La lecture traduit les champs gérés par Arnod ; l'écriture part du fichier
//! existant et ne remplace que ces champs : scripts, variables, actions, la
//! partie `request` des exemples et les autres authentifications ou corps
//! restent tels quels (R3 ; plan de configuration d'un endpoint, R6, R9).

use serde_json::{Map, Value, json};

use crate::opencollection::schema::{self, Part};
use crate::request::model::{
    ApiKeyLocation, Assertion, Auth, AuthKind, Body, BodyKind, BodyVariant, ContractLink,
    DEFAULT_MAX_DURATION_MS, Example, ExampleBodyKind, Extraction, Method, Operator, Origin, Pair,
    ParamKind, Request, ResponseHeader, Scalar, Settings,
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

/// Texte d'une description OpenCollection : texte, ou objet `{ content, type }`.
fn description_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Object(object)) => text(object.get("content")),
        other => text(other),
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
                    description: description_text(item.get("description")),
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

pub(crate) fn auth_from(value: Option<&Value>) -> Auth {
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

/// Type et contenu d'un corps OpenCollection simple.
fn single_body(value: &Value) -> Result<(BodyKind, BodyVariant), String> {
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    let data = value.get("data");
    let raw = |kind| {
        Ok((
            kind,
            BodyVariant {
                content: text(data),
                ..BodyVariant::default()
            },
        ))
    };
    match kind {
        "json" => raw(BodyKind::Json),
        "text" => raw(BodyKind::Text),
        "xml" => raw(BodyKind::Xml),
        "form-urlencoded" => Ok((
            BodyKind::Form,
            BodyVariant {
                form: pairs(data),
                ..BodyVariant::default()
            },
        )),
        other => Err(other.to_owned()),
    }
}

fn other_body(label: &str) -> Body {
    Body {
        kind: BodyKind::Other,
        other: label.to_owned(),
        ..Body::default()
    }
}

/// Corps simple ou variantes (R2) ; un corps qu'Arnod ne sait pas éditer
/// (multipart, fichier, variantes de types différents) devient `other`.
fn body_from(value: Option<&Value>) -> Body {
    let Some(value) = value else {
        return Body::default();
    };
    let Some(list) = value.as_array() else {
        return match single_body(value) {
            Ok((kind, variant)) => Body {
                kind,
                variants: vec![variant],
                ..Body::default()
            },
            Err(other) => other_body(&other),
        };
    };
    let mut kind = None;
    let mut variants = Vec::new();
    let mut selected = 0;
    for (index, item) in list.iter().enumerate() {
        let Some(body) = item.get("body") else {
            return other_body("variantes");
        };
        match single_body(body) {
            Ok((k, mut variant)) if kind.is_none_or(|known| known == k) => {
                kind = Some(k);
                variant.title = text(item.get("title"));
                variants.push(variant);
            }
            _ => return other_body("variantes"),
        }
        if item.get("selected").and_then(Value::as_bool) == Some(true) {
            selected = index;
        }
    }
    match kind {
        Some(kind) => Body {
            kind,
            variants,
            selected,
            ..Body::default()
        },
        None => Body::default(),
    }
}

fn settings_from(value: Option<&Value>) -> Settings {
    let get = |key: &str| value.and_then(|s| s.get(key));
    let number = |key: &str| {
        get(key)
            .and_then(Value::as_f64)
            .filter(|n| *n >= 0.0)
            .map(|n| n as u64)
    };
    Settings {
        timeout_ms: number("timeout"),
        follow_redirects: get("followRedirects").and_then(Value::as_bool),
        max_redirects: number("maxRedirects").map(|n| u32::try_from(n).unwrap_or(u32::MAX)),
        encode_url: get("encodeUrl").and_then(Value::as_bool),
    }
}

fn example_body_kind(name: &str) -> ExampleBodyKind {
    match name {
        "json" => ExampleBodyKind::Json,
        "text" => ExampleBodyKind::Text,
        "xml" => ExampleBodyKind::Xml,
        "html" => ExampleBodyKind::Html,
        "binary" => ExampleBodyKind::Binary,
        _ => ExampleBodyKind::None,
    }
}

fn example_body_name(kind: ExampleBodyKind) -> Option<&'static str> {
    Some(match kind {
        ExampleBodyKind::None => return None,
        ExampleBodyKind::Json => "json",
        ExampleBodyKind::Text => "text",
        ExampleBodyKind::Xml => "xml",
        ExampleBodyKind::Html => "html",
        ExampleBodyKind::Binary => "binary",
    })
}

/// Réponses attendues (R6).
fn examples_from(value: Option<&Value>) -> Vec<Example> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            let response = item.get("response");
            let get = |key: &str| response.and_then(|r| r.get(key));
            let body = get("body");
            Example {
                name: text(item.get("name")),
                description: description_text(item.get("description")),
                status: get("status")
                    .and_then(Value::as_f64)
                    .and_then(|s| u16::try_from(s as i64).ok()),
                status_text: text(get("statusText")),
                headers: get("headers")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|h| ResponseHeader {
                        name: text(h.get("name")),
                        value: text(h.get("value")),
                    })
                    .collect(),
                body_kind: body
                    .and_then(|b| b.get("type"))
                    .and_then(Value::as_str)
                    .map_or(ExampleBodyKind::None, example_body_kind),
                body: text(body.and_then(|b| b.get("data"))),
            }
        })
        .collect()
}

/// Étiquettes de l'utilisateur, sans celle du lien au contrat (R9).
fn tags_from(tags: Option<&Value>) -> Vec<String> {
    tags.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|tag| !tag.starts_with(CONTRACT_TAG))
        .map(str::to_owned)
        .collect()
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

/// Parties du fichier conservées sans être gérées par Arnod ; les actions ne
/// le sont que si l'une n'est pas une extraction (`docs/plan-mvp-e-tests-etapes.md`, R8).
fn preserved(value: &Value) -> Vec<String> {
    let present = |pointer: &str| {
        value
            .pointer(pointer)
            .is_some_and(|v| !(v.is_null() || v.as_array().is_some_and(Vec::is_empty)))
    };
    let other_actions = value
        .pointer("/runtime/actions")
        .and_then(Value::as_array)
        .is_some_and(|list| list.iter().any(|item| extraction_from(item).is_none()));
    [
        ("/runtime/scripts", "scripts"),
        ("/runtime/variables", "variables de requête"),
    ]
    .into_iter()
    .filter(|(pointer, _)| present(pointer))
    .map(|(_, label)| label.to_owned())
    .chain(other_actions.then(|| "actions".to_owned()))
    .collect()
}

/// Préfixe OpenCollection d'une valeur lue dans la requête envoyée.
const REQUEST_EXPRESSION: &str = "req.";

/// Expression OpenCollection de la source d'une extraction (R7) : `res.…`
/// pour la réponse, `req.…` pour la requête envoyée.
pub fn extraction_expression(source: &str) -> String {
    match source.strip_prefix(crate::request::extract::REQUEST_PREFIX) {
        Some(rest) => {
            let response = expression(rest);
            match response.strip_prefix("res.") {
                Some(tail) => format!("{REQUEST_EXPRESSION}{tail}"),
                None => response,
            }
        }
        None => expression(source),
    }
}

/// Source Arnod d'une expression d'extraction.
pub fn extraction_source(expression: &str) -> String {
    let expression = expression.trim();
    // Raccourci de Bruno : `res("data.id")` lit le corps (import du lot C).
    if let Some(path) = expression
        .strip_prefix("res(")
        .and_then(|rest| rest.strip_suffix(')'))
        .map(|inner| inner.trim().trim_matches(|c| c == '"' || c == '\''))
    {
        return if path.is_empty() || path.starts_with('[') {
            format!("body{path}")
        } else {
            format!("body.{path}")
        };
    }
    match expression.strip_prefix(REQUEST_EXPRESSION) {
        Some(rest) => format!(
            "{}{}",
            crate::request::extract::REQUEST_PREFIX,
            target_from(&format!("res.{rest}"))
        ),
        None => target_from(expression),
    }
}

/// Extraction d'une action : `set-variable` hors de la phase `before-request` (R8).
fn extraction_from(item: &Value) -> Option<Extraction> {
    if item.get("type").and_then(Value::as_str) != Some("set-variable")
        || item.get("phase").and_then(Value::as_str) == Some("before-request")
    {
        return None;
    }
    let name = item.pointer("/variable/name").and_then(Value::as_str)?;
    let expression = item
        .pointer("/selector/expression")
        .and_then(Value::as_str)?;
    Some(Extraction {
        name: name.to_owned(),
        source: extraction_source(expression),
        enabled: !disabled(item),
    })
}

/// Actions écrites : celles qu'Arnod ne gère pas d'abord, telles quelles, puis
/// les extractions ; description et portée d'une extraction de même nom gardées (R8).
fn action_values(list: &[Extraction], previous: Option<&Value>) -> Option<Value> {
    let previous: &[Value] = previous
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let mut items: Vec<Value> = previous
        .iter()
        .filter(|item| extraction_from(item).is_none())
        .cloned()
        .collect();
    for extraction in list {
        let name = extraction.name.trim();
        if name.is_empty() {
            continue;
        }
        let old = previous.iter().find(|item| {
            extraction_from(item).is_some()
                && item.pointer("/variable/name").and_then(Value::as_str) == Some(name)
        });
        let scope = old
            .and_then(|o| o.pointer("/variable/scope"))
            .and_then(Value::as_str)
            .unwrap_or("runtime");
        let mut item = Map::new();
        item.insert("type".to_owned(), Value::from("set-variable"));
        if let Some(description) = old.and_then(|o| o.get("description")) {
            item.insert("description".to_owned(), description.clone());
        }
        item.insert("phase".to_owned(), Value::from("after-response"));
        item.insert(
            "selector".to_owned(),
            json!({ "expression": extraction_expression(&extraction.source), "method": "jsonq" }),
        );
        item.insert(
            "variable".to_owned(),
            json!({ "name": name, "scope": scope }),
        );
        if !extraction.enabled {
            item.insert("disabled".to_owned(), Value::Bool(true));
        }
        items.push(Value::Object(item));
    }
    (!items.is_empty()).then_some(Value::Array(items))
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
            let target = target_from(&text(item.get("expression")));
            // L'origine n'est pas écrite : seule la durée par défaut se reconnaît (V5).
            let origin = if target == "responseTime"
                && operator == Operator::LessThan
                && value == Some(Scalar::Int(DEFAULT_MAX_DURATION_MS))
            {
                Origin::Default
            } else {
                Origin::Manual
            };
            assertions.push(Assertion {
                id: format!("a{index}"),
                target,
                operator,
                value,
                origin,
                enabled: !disabled(item),
            });
        }
    }
    Ok(Request {
        name: name_of(value).unwrap_or_else(|| fallback.to_owned()),
        description: description_text(value.pointer("/info/description")),
        tags: tags_from(value.pointer("/info/tags")),
        method,
        url: text(http.and_then(|h| h.get("url"))),
        params: pairs(http.and_then(|h| h.get("params"))),
        headers: pairs(http.and_then(|h| h.get("headers"))),
        auth: auth_from(http.and_then(|h| h.get("auth"))),
        body: body_from(http.and_then(|h| h.get("body"))),
        settings: settings_from(value.get("settings")),
        docs: description_text(value.get("docs")),
        assertions,
        extractions: value
            .pointer("/runtime/actions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(extraction_from)
            .collect(),
        seq: value.pointer("/info/seq").and_then(Value::as_f64),
        examples: examples_from(value.get("examples")),
        contract: contract_from(value.pointer("/info/tags")),
        preserved: preserved(value),
        inherited: crate::request::model::Inherited::default(),
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

/// Description écrite : texte, ou objet `{ content, type }` si le fichier en
/// avait un (son type est gardé).
fn description_value(text: &str, previous: Option<&Value>) -> Option<Value> {
    if text.is_empty() {
        return None;
    }
    if let Some(Value::Object(object)) = previous
        && object.contains_key("type")
    {
        let mut object = object.clone();
        object.insert("content".to_owned(), Value::from(text));
        return Some(Value::Object(object));
    }
    Some(Value::from(text))
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
            let old = description(previous, |p| {
                p.get("name").and_then(Value::as_str) == Some(pair.name.as_str())
            });
            if let Some(text) = description_value(&pair.description, old.as_ref()) {
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

fn variant_body(kind: BodyKind, variant: &BodyVariant, previous: Option<&Value>) -> Value {
    let raw = |name: &str| json!({ "type": name, "data": variant.content });
    match kind {
        BodyKind::Json => raw("json"),
        BodyKind::Text => raw("text"),
        BodyKind::Xml => raw("xml"),
        _ => json!({
            "type": "form-urlencoded",
            "data": pair_values(&variant.form, previous.and_then(|p| p.get("data")), false)
                .unwrap_or_else(|| Value::Array(Vec::new())),
        }),
    }
}

/// Corps écrit (R2) : simple sans variante nommée, sinon tableau de variantes.
fn body_value(body: &Body, previous: Option<&Value>) -> Option<Value> {
    match body.kind {
        BodyKind::None => return None,
        BodyKind::Other => return previous.cloned(),
        _ => {}
    }
    let empty = BodyVariant::default();
    let simple = body.variants.len() <= 1 && body.variants.iter().all(|v| v.title.is_empty());
    if simple {
        let variant = body.variants.first().unwrap_or(&empty);
        let previous = previous.filter(|p| p.is_object());
        return Some(variant_body(body.kind, variant, previous));
    }
    let selected = body.selected.min(body.variants.len().saturating_sub(1));
    let items = body
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let old = previous
                .and_then(Value::as_array)
                .and_then(|list| {
                    list.iter().find(|item| {
                        item.get("title").and_then(Value::as_str) == Some(variant.title.as_str())
                    })
                })
                .and_then(|item| item.get("body"));
            let mut item = Map::new();
            item.insert("title".to_owned(), Value::from(variant.title.clone()));
            if index == selected {
                item.insert("selected".to_owned(), Value::Bool(true));
            }
            item.insert("body".to_owned(), variant_body(body.kind, variant, old));
            Value::Object(item)
        })
        .collect();
    Some(Value::Array(items))
}

/// Réglages propres écrits ; un réglage hérité est absent (R3).
fn settings_value(settings: &Settings, previous: Option<&Value>) -> Option<Value> {
    let mut map = previous
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    set(&mut map, "timeout", settings.timeout_ms.map(Value::from));
    set(
        &mut map,
        "followRedirects",
        settings.follow_redirects.map(Value::Bool),
    );
    set(
        &mut map,
        "maxRedirects",
        settings.max_redirects.map(Value::from),
    );
    set(&mut map, "encodeUrl", settings.encode_url.map(Value::Bool));
    (!map.is_empty()).then_some(Value::Object(map))
}

/// Réponses attendues écrites ; la partie `request` d'un exemple de même nom
/// est conservée (R6).
fn example_values(list: &[Example], previous: Option<&Value>) -> Option<Value> {
    if list.is_empty() {
        return None;
    }
    let items = list
        .iter()
        .map(|example| {
            let old = previous.and_then(Value::as_array).and_then(|items| {
                items.iter().find(|item| {
                    item.get("name").and_then(Value::as_str) == Some(example.name.as_str())
                })
            });
            let mut item = Map::new();
            item.insert("name".to_owned(), Value::from(example.name.clone()));
            if let Some(text) =
                description_value(&example.description, old.and_then(|o| o.get("description")))
            {
                item.insert("description".to_owned(), text);
            }
            if let Some(request) = old.and_then(|o| o.get("request")) {
                item.insert("request".to_owned(), request.clone());
            }
            let mut response = Map::new();
            if let Some(status) = example.status {
                response.insert("status".to_owned(), Value::from(status));
            }
            if !example.status_text.is_empty() {
                response.insert(
                    "statusText".to_owned(),
                    Value::from(example.status_text.clone()),
                );
            }
            if !example.headers.is_empty() {
                let headers = example
                    .headers
                    .iter()
                    .map(|h| json!({ "name": h.name, "value": h.value }))
                    .collect();
                response.insert("headers".to_owned(), Value::Array(headers));
            }
            if let Some(kind) = example_body_name(example.body_kind) {
                response.insert(
                    "body".to_owned(),
                    json!({ "type": kind, "data": example.body }),
                );
            }
            if !response.is_empty() {
                item.insert("response".to_owned(), Value::Object(response));
            }
            Value::Object(item)
        })
        .collect();
    Some(Value::Array(items))
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
    // Paramètres de chemin alignés sur l'adresse (R1 de la configuration d'un endpoint).
    let mut normalized = request.clone();
    crate::request::model::normalize_path_params(&mut normalized);
    let request = &normalized;
    let mut root = existing
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let Some(map) = root.as_object_mut() else {
        return root;
    };

    with_object(map, "info", |info| {
        // Un nom effacé dans l'éditeur garde celui du fichier.
        let name = request.name.trim();
        if !name.is_empty() || info.get("name").and_then(Value::as_str).is_none() {
            info.insert("name".to_owned(), Value::from(name));
        }
        let old = info.get("description").cloned();
        set(
            info,
            "description",
            description_value(&request.description, old.as_ref()),
        );
        info.insert("type".to_owned(), Value::from("http"));
        // Rang de l'étape : écrit s'il est connu, sinon celui du fichier reste.
        if let Some(seq) = request.seq {
            let value = if seq.fract() == 0.0 && seq.abs() < 1e15 {
                Value::from(seq as i64)
            } else {
                json!(seq)
            };
            info.insert("seq".to_owned(), value);
        }
        let mut tags: Vec<Value> = request
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty() && !tag.starts_with(CONTRACT_TAG))
            .map(Value::from)
            .collect();
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
        let previous = runtime.get("actions").cloned();
        set(
            runtime,
            "actions",
            action_values(&request.extractions, previous.as_ref()),
        );
    });
    prune(map, "runtime");

    let old = map.get("settings").cloned();
    set(
        map,
        "settings",
        settings_value(&request.settings, old.as_ref()),
    );
    set(
        map,
        "docs",
        (!request.docs.is_empty()).then(|| Value::from(request.docs.clone())),
    );
    let old = map.get("examples").cloned();
    set(
        map,
        "examples",
        example_values(&request.examples, old.as_ref()),
    );
    root
}
