//! Collection Bruno → fichiers OpenCollection 1.0.0 et rapport d'import
//! (docs/plan-mvp-c-import-bruno.md, R1 à R10).
//!
//! Pur : la collection est déjà lue (`read`), rien n'est écrit ici. Chaque
//! fichier produit est validé contre le schéma ; rien n'est inventé.

use std::collections::{BTreeSet, HashSet};

use serde_json::{Map, Value, json};

use crate::bruno::parse::{Body, BruFile, Entry};
use crate::bruno::read::{Collection, DOTENV, Folder, Source};
use crate::bruno::{ConversionExample, ImportCounts, ItemKind, ItemStatus, ReportItem};
use crate::opencollection::request::{operator_from, target_from};
use crate::opencollection::schema::{self, Part};
use crate::request::model::Operator;
use crate::tree::{item_name, slug};

/// Fichier OpenCollection produit, relatif au dossier d'arrivée.
#[derive(Debug, Clone, PartialEq)]
pub struct OutFile {
    pub path: String,
    pub part: Part,
    pub value: Value,
}

/// Valeur connue d'une variable locale (`.env`), destinée au coffre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretValue {
    pub environment: String,
    pub name: String,
    pub value: String,
}

/// Résultat de la conversion.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Converted {
    pub name: String,
    /// Réglages de la collection (`RequestDefaults` : en-têtes, auth, variables, scripts).
    pub defaults: Map<String, Value>,
    pub docs: Option<String>,
    /// Dossiers à créer, parents d'abord.
    pub folders: Vec<String>,
    pub files: Vec<OutFile>,
    /// Environnements (`$defs/Environment`), dans l'ordre de la collection.
    pub environments: Vec<Value>,
    pub secrets: Vec<SecretValue>,
    pub bru_files: usize,
    pub counts: ImportCounts,
    pub items: Vec<ReportItem>,
    pub examples: Vec<ConversionExample>,
}

/// Méthodes HTTP de Bruno (blocs `get { … }`…).
const METHODS: [&str; 9] = [
    "get", "post", "put", "patch", "delete", "options", "head", "connect", "trace",
];

/// Blocs d'une requête repris par la conversion ; les autres sont signalés.
fn handled(name: &str) -> bool {
    METHODS.contains(&name)
        || name.starts_with("auth:")
        || name.starts_with("body:")
        || name.starts_with("script:")
        || matches!(
            name,
            "meta"
                | "params:query"
                | "params:path"
                | "headers"
                | "vars:pre-request"
                | "vars:post-response"
                | "assert"
                | "tests"
                | "docs"
                | "settings"
                | "auth"
        )
}

/// Libellé français d'un opérateur, pour les exemples du rapport.
fn operator_label(operator: Operator) -> &'static str {
    match operator {
        Operator::Equals => "égal à",
        Operator::NotEquals => "différent de",
        Operator::Exists => "existe",
        Operator::Contains => "contient",
        Operator::NotContains => "ne contient pas",
        Operator::LessThan => "inférieur à",
        Operator::LessOrEqual => "inférieur ou égal à",
        Operator::GreaterThan => "supérieur à",
        Operator::GreaterOrEqual => "supérieur ou égal à",
        Operator::In => "parmi",
        Operator::NotIn => "hors de",
        Operator::Between => "entre",
        Operator::StartsWith => "commence par",
        Operator::EndsWith => "finit par",
        Operator::Length => "de longueur",
        Operator::Matches => "correspond à",
        Operator::NotMatches => "ne correspond pas à",
        Operator::IsNull => "est nul",
        Operator::IsUndefined => "est absent",
        Operator::IsEmpty => "est vide",
        Operator::IsNotEmpty => "n'est pas vide",
        Operator::IsTruthy => "est vrai",
        Operator::IsFalsy => "est faux",
        Operator::IsJson => "est du JSON",
        Operator::IsNumber => "est un nombre",
        Operator::IsText => "est un texte",
        Operator::IsBoolean => "est un booléen",
        Operator::IsArray => "est une liste",
    }
}

/// Conversion en cours : rapport et exemples.
struct Ctx {
    counts: ImportCounts,
    items: Vec<ReportItem>,
    examples: Vec<ConversionExample>,
    /// Exemples déjà donnés, par nature.
    shown: HashSet<&'static str>,
    dynamic: BTreeSet<String>,
}

/// Où se trouve l'élément converti.
struct At<'a> {
    name: &'a str,
    place: &'a str,
    file: &'a str,
    /// Fichier créé dans le projet.
    request: Option<&'a str>,
}

impl Ctx {
    fn item(
        &mut self,
        at: &At<'_>,
        kind: ItemKind,
        status: ItemStatus,
        block: Option<&str>,
        detail: impl Into<String>,
        code: Option<String>,
    ) {
        self.items.push(ReportItem {
            kind,
            status,
            name: at.name.to_owned(),
            place: at.place.to_owned(),
            file: at.file.to_owned(),
            block: block.map(str::to_owned),
            request: at.request.map(str::to_owned),
            detail: detail.into(),
            code,
        });
    }

    fn example(&mut self, kind: &'static str, before: String, after: String) {
        if self.shown.insert(kind) {
            self.examples.push(ConversionExample { before, after });
        }
    }

    /// Relève les valeurs dynamiques (`{{$nom}}`, R1) d'un texte : gardées telles quelles.
    fn scan(&mut self, text: &str) {
        let mut rest = text;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else {
                break;
            };
            let name = after[..end].trim();
            if let Some(dynamic) = name.strip_prefix('$')
                && !dynamic.is_empty()
                && dynamic
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                self.counts.dynamic_values += 1;
                if self.dynamic.insert(name.to_owned()) {
                    self.example(
                        "dynamic",
                        format!("{{{{{name}}}}}"),
                        "valeur dynamique, même nom".to_owned(),
                    );
                }
            }
            rest = &after[end + 2..];
        }
    }
}

fn disabled(map: &mut Map<String, Value>, entry: &Entry) {
    if !entry.enabled {
        map.insert("disabled".to_owned(), Value::Bool(true));
    }
}

/// Paires nom / valeur (en-têtes, champs de formulaire).
fn pairs(entries: &[Entry], ctx: &mut Ctx) -> Vec<Value> {
    entries
        .iter()
        .map(|entry| {
            ctx.scan(&entry.value);
            let mut map = Map::new();
            map.insert("name".to_owned(), Value::from(entry.key.clone()));
            map.insert("value".to_owned(), Value::from(entry.value.clone()));
            disabled(&mut map, entry);
            Value::Object(map)
        })
        .collect()
}

/// Variables `vars:pre-request` (`$defs/Variable`).
fn variables(entries: &[Entry], ctx: &mut Ctx) -> Vec<Value> {
    pairs(entries, ctx)
}

fn bool_of(text: &str) -> Option<bool> {
    match text.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Authentification d'un mode Bruno (R5). `None` : aucune, ou non reprise (signalée).
fn auth(file: &BruFile, mode: &str, at: &At<'_>, ctx: &mut Ctx) -> Option<Value> {
    let get = |key: &str| {
        file.value(&format!("auth:{mode}"), key)
            .unwrap_or("")
            .to_owned()
    };
    let not_sent = |ctx: &mut Ctx, label: &str| {
        ctx.item(
            at,
            ItemKind::Auth,
            ItemStatus::ToCheck,
            Some(&format!("auth:{mode}")),
            format!(
                "Authentification {label} gardée dans le fichier : Arnod ne l'envoie pas encore."
            ),
            None,
        );
    };
    let value = match mode {
        "" | "none" => return None,
        "inherit" => Value::from("inherit"),
        "basic" => {
            json!({ "type": "basic", "username": get("username"), "password": get("password") })
        }
        "bearer" => json!({ "type": "bearer", "token": get("token") }),
        "apikey" => json!({
            "type": "apikey",
            "key": get("key"),
            "value": get("value"),
            "placement": if get("placement") == "queryparams" || get("placement") == "query" { "query" } else { "header" },
        }),
        "digest" => {
            not_sent(ctx, "digest");
            json!({ "type": "digest", "username": get("username"), "password": get("password") })
        }
        "ntlm" => {
            not_sent(ctx, "NTLM");
            json!({ "type": "ntlm", "username": get("username"), "password": get("password"), "domain": get("domain") })
        }
        "wsse" => {
            not_sent(ctx, "WSSE");
            json!({ "type": "wsse", "username": get("username"), "password": get("password") })
        }
        "awsv4" => {
            not_sent(ctx, "AWS Signature v4");
            json!({
                "type": "awsv4",
                "accessKeyId": get("accessKeyId"),
                "secretAccessKey": get("secretAccessKey"),
                "sessionToken": get("sessionToken"),
                "service": get("service"),
                "region": get("region"),
                "profileName": get("profileName"),
            })
        }
        other => {
            let label = if other == "oauth2" { "OAuth 2" } else { other };
            let code = file
                .block(&format!("auth:{other}"))
                .map(|block| match &block.body {
                    Body::Dict(entries) => entries
                        .iter()
                        .map(|e| {
                            format!("{}{}: {}", if e.enabled { "" } else { "~" }, e.key, e.value)
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    Body::Text(text) => text.clone(),
                    Body::List(_) => String::new(),
                });
            ctx.item(
                at,
                ItemKind::Auth,
                ItemStatus::Skipped,
                Some(&format!("auth:{other}")),
                format!("Authentification {label} non reprise : à régler dans l'onglet Auth."),
                code,
            );
            return None;
        }
    };
    if let Value::Object(map) = &value {
        for text in map.values().filter_map(Value::as_str) {
            ctx.scan(text);
        }
    }
    Some(value)
}

/// Scripts et tests gardés dans `runtime.scripts`, sans exécution (R6).
fn scripts(file: &BruFile, at: &At<'_>, ctx: &mut Ctx) -> Vec<Value> {
    let mut list = Vec::new();
    for (block, kind, item) in [
        ("script:pre-request", "before-request", ItemKind::PreRequest),
        (
            "script:post-response",
            "after-response",
            ItemKind::PostResponse,
        ),
        ("tests", "tests", ItemKind::Tests),
    ] {
        let Some(code) = file.text(block) else {
            continue;
        };
        list.push(json!({ "type": kind, "code": code }));
        ctx.counts.scripts += 1;
        ctx.item(
            at,
            item,
            ItemStatus::NotRun,
            Some(block),
            "Aucun JavaScript n'est exécuté par Arnod : ce script n'a pas d'équivalent sans code.",
            Some(code.to_owned()),
        );
    }
    list
}

/// Réglages d'un niveau de la collection (`collection.bru`, `folder.bru`) :
/// `RequestDefaults` d'OpenCollection.
fn defaults(file: &BruFile, at: &At<'_>, ctx: &mut Ctx) -> Map<String, Value> {
    let mut map = Map::new();
    let headers = pairs(file.dict("headers"), ctx);
    if !headers.is_empty() {
        map.insert("headers".to_owned(), Value::Array(headers));
    }
    let mode = file.value("auth", "mode").unwrap_or("");
    if let Some(auth) = auth(file, mode, at, ctx) {
        map.insert("auth".to_owned(), auth);
    }
    let vars = variables(file.dict("vars:pre-request"), ctx);
    if !vars.is_empty() {
        map.insert("variables".to_owned(), Value::Array(vars));
    }
    let scripts = scripts(file, at, ctx);
    if !scripts.is_empty() {
        map.insert("scripts".to_owned(), Value::Array(scripts));
    }
    if let Some(settings) = settings(file, at, ctx) {
        map.insert("settings".to_owned(), json!({ "http": settings }));
    }
    for (block, kind, label) in [
        ("vars:post-response", ItemKind::Extraction, "Extractions"),
        ("assert", ItemKind::Assertion, "Vérifications"),
    ] {
        let entries = file.dict(block);
        if !entries.is_empty() {
            ctx.item(
                at,
                kind,
                ItemStatus::Skipped,
                Some(block),
                format!(
                    "{label} d'un dossier ou de la collection : sans équivalent OpenCollection, non reprises."
                ),
                Some(
                    entries
                        .iter()
                        .map(|e| format!("{}: {}", e.key, e.value))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            );
        }
    }
    map
}

/// Réglages `settings { … }` → `HttpRequestSettings`.
fn settings(file: &BruFile, at: &At<'_>, ctx: &mut Ctx) -> Option<Value> {
    let mut map = Map::new();
    for entry in file.dict("settings").iter().filter(|e| e.enabled) {
        let value = match entry.key.as_str() {
            "encodeUrl" | "followRedirects" => bool_of(&entry.value).map(Value::Bool),
            // Un délai nul vaut « réglage par défaut » dans Bruno.
            "timeout" => entry
                .value
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|n| *n > 0)
                .map(Value::from),
            "maxRedirects" => entry.value.trim().parse::<u64>().ok().map(Value::from),
            _ => {
                ctx.item(
                    at,
                    ItemKind::Option,
                    ItemStatus::Skipped,
                    Some("settings"),
                    format!("Réglage « {} » sans équivalent, non repris.", entry.key),
                    Some(format!("{}: {}", entry.key, entry.value)),
                );
                continue;
            }
        };
        if let Some(value) = value {
            map.insert(entry.key.clone(), value);
        }
    }
    (!map.is_empty()).then_some(Value::Object(map))
}

/// Vérification `res.status: eq 200` → `Assertion` (R4).
fn assertion(entry: &Entry, at: &At<'_>, ctx: &mut Ctx) -> Value {
    let text = entry.value.trim();
    let (first, rest) = text
        .split_once(char::is_whitespace)
        .map_or((text, ""), |(a, b)| (a, b.trim()));
    // Sans opérateur connu en tête, Bruno compare par égalité à toute la valeur.
    let (operator, value) = match operator_from(first) {
        Some(operator) => (operator, rest),
        None => (Operator::Equals, text),
    };
    let name = crate::opencollection::request::operator_name(operator);
    let mut map = Map::new();
    map.insert("expression".to_owned(), Value::from(entry.key.clone()));
    map.insert("operator".to_owned(), Value::from(name));
    if !value.is_empty() {
        map.insert("value".to_owned(), Value::from(value));
    }
    disabled(&mut map, entry);
    ctx.scan(value);
    ctx.counts.assertions += 1;
    if matches!(operator, Operator::Matches | Operator::NotMatches) {
        ctx.item(
            at,
            ItemKind::Assertion,
            ItemStatus::ToCheck,
            Some("assert"),
            format!(
                "Vérification « {} {name} » reprise, mais pas encore évaluée par Arnod (expression régulière).",
                entry.key
            ),
            Some(format!("{}: {}", entry.key, entry.value)),
        );
    }
    let target = target_from(&entry.key);
    let label = operator_label(operator);
    ctx.example(
        "assertion",
        format!("assert {{ {}: {} }}", entry.key, entry.value),
        if value.is_empty() {
            format!("{target} {label}")
        } else {
            format!("{target} {label} {}", value.trim_matches('"'))
        },
    );
    Value::Object(map)
}

/// L'expression lit simplement la réponse (`res.status`, `res.body.a[0]`, `res.headers["x"]`).
fn is_response_path(expression: &str) -> bool {
    let rest = expression.trim();
    ["res.status", "res.body", "res.headers", "res.responseTime"]
        .iter()
        .any(|prefix| rest.starts_with(prefix))
        && !rest.contains(['(', ' ', '+', '?', '|', '&'])
}

/// Extraction `vars:post-response { id: res.body.id }` → action `set-variable`
/// en phase `after-response` (règle commune R2).
fn extraction(entry: &Entry, at: &At<'_>, ctx: &mut Ctx) -> Value {
    let mut map = Map::new();
    map.insert("type".to_owned(), Value::from("set-variable"));
    map.insert("phase".to_owned(), Value::from("after-response"));
    map.insert(
        "selector".to_owned(),
        json!({ "expression": entry.value.trim(), "method": "jsonq" }),
    );
    map.insert(
        "variable".to_owned(),
        json!({ "name": entry.key, "scope": "runtime" }),
    );
    disabled(&mut map, entry);
    ctx.counts.extractions += 1;
    if is_response_path(&entry.value) {
        ctx.example(
            "extraction",
            format!(
                "vars:post-response {{ {}: {} }}",
                entry.key,
                entry.value.trim()
            ),
            format!("extrait {} ← {}", entry.key, target_from(&entry.value)),
        );
    } else {
        ctx.item(
            at,
            ItemKind::Extraction,
            ItemStatus::ToCheck,
            Some("vars:post-response"),
            format!(
                "Extraction « {} » reprise telle quelle : son expression n'est pas un simple chemin de la réponse.",
                entry.key
            ),
            Some(format!("{}: {}", entry.key, entry.value)),
        );
    }
    Value::Object(map)
}

/// Corps de la requête, selon le mode choisi dans le bloc de la méthode.
fn body(file: &BruFile, mode: &str, at: &At<'_>, ctx: &mut Ctx) -> Option<Value> {
    let raw = |kind: &str, block: &str, ctx: &mut Ctx| {
        let data = file.text(block).unwrap_or("").to_owned();
        ctx.scan(&data);
        Some(json!({ "type": kind, "data": data }))
    };
    let not_sent = |ctx: &mut Ctx, block: &str, label: &str| {
        ctx.item(
            at,
            ItemKind::Body,
            ItemStatus::ToCheck,
            Some(block),
            format!("Corps {label} gardé dans le fichier : Arnod ne l'envoie pas encore."),
            None,
        );
    };
    match mode {
        "" | "none" => None,
        "json" => raw("json", "body:json", ctx),
        "text" => raw("text", "body:text", ctx),
        "xml" => raw("xml", "body:xml", ctx),
        "sparql" => {
            not_sent(ctx, "body:sparql", "SPARQL");
            raw("sparql", "body:sparql", ctx)
        }
        "formUrlEncoded" => Some(json!({
            "type": "form-urlencoded",
            "data": pairs(file.dict("body:form-urlencoded"), ctx),
        })),
        "multipartForm" => {
            not_sent(ctx, "body:multipart-form", "multipart");
            let data: Vec<Value> = file
                .dict("body:multipart-form")
                .iter()
                .map(|entry| {
                    ctx.scan(&entry.value);
                    let mut map = Map::new();
                    map.insert("name".to_owned(), Value::from(entry.key.clone()));
                    let (value, content_type) = split_content_type(&entry.value);
                    match value
                        .strip_prefix("@file(")
                        .and_then(|v| v.strip_suffix(')'))
                    {
                        Some(files) => {
                            map.insert("type".to_owned(), Value::from("file"));
                            map.insert(
                                "value".to_owned(),
                                Value::Array(files.split('|').map(Value::from).collect()),
                            );
                        }
                        None => {
                            map.insert("type".to_owned(), Value::from("text"));
                            map.insert("value".to_owned(), Value::from(value));
                        }
                    }
                    if let Some(content_type) = content_type {
                        map.insert("contentType".to_owned(), Value::from(content_type));
                    }
                    disabled(&mut map, entry);
                    Value::Object(map)
                })
                .collect();
            Some(json!({ "type": "multipart-form", "data": data }))
        }
        other => {
            let block = match other {
                "graphql" => "body:graphql".to_owned(),
                "file" => "body:file".to_owned(),
                _ => format!("body:{other}"),
            };
            ctx.item(
                at,
                ItemKind::Body,
                ItemStatus::Skipped,
                Some(&block),
                format!("Corps « {other} » sans équivalent, non repris."),
                file.text(&block).map(str::to_owned),
            );
            None
        }
    }
}

/// `valeur @contentType(type)` → valeur et type.
fn split_content_type(value: &str) -> (&str, Option<&str>) {
    match value.find("@contentType(") {
        Some(index) => {
            let content = value[index + "@contentType(".len()..].trim_end();
            (
                value[..index].trim_end(),
                content.strip_suffix(')').map(str::trim),
            )
        }
        None => (value.trim(), None),
    }
}

/// Requête `.bru` → `HttpRequest` d'OpenCollection ; `None` si elle n'est pas reprise.
fn request(source: &Source, place: &str, out_path: &str, ctx: &mut Ctx) -> Option<Value> {
    let file = &source.file;
    let stem = source
        .path
        .rsplit('/')
        .next()
        .unwrap_or(&source.path)
        .trim_end_matches(".bru");
    let name = file
        .value("meta", "name")
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(stem)
        .to_owned();
    let at = At {
        name: &name,
        place,
        file: &source.path,
        request: Some(out_path),
    };
    let kind = file.value("meta", "type").unwrap_or("http");
    if kind != "http" {
        let label = match kind {
            "graphql" => "GraphQL",
            "grpc" => "gRPC",
            "ws" => "WebSocket",
            other => other,
        };
        let skipped = At {
            request: None,
            ..at
        };
        ctx.item(
            &skipped,
            ItemKind::Request,
            ItemStatus::Skipped,
            None,
            format!("Requête {label} : non prise en charge par Arnod, non importée."),
            None,
        );
        return None;
    }
    let Some(method) = METHODS.iter().find(|m| file.block(m).is_some()) else {
        let skipped = At {
            request: None,
            ..at
        };
        ctx.item(
            &skipped,
            ItemKind::Request,
            ItemStatus::Skipped,
            None,
            "Requête sans méthode HTTP, non importée.",
            None,
        );
        return None;
    };
    let url = file.value(method, "url").unwrap_or("").to_owned();
    ctx.scan(&url);

    let mut info = Map::new();
    info.insert("name".to_owned(), Value::from(name.clone()));
    info.insert("type".to_owned(), Value::from("http"));
    if let Some(seq) = file
        .value("meta", "seq")
        .and_then(|s| s.trim().parse::<f64>().ok())
    {
        info.insert("seq".to_owned(), json!(seq));
    }
    let tags: Vec<Value> = file
        .value("meta", "tags")
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(Value::from)
        .collect();
    if !tags.is_empty() {
        info.insert("tags".to_owned(), Value::Array(tags));
    }

    let mut http = Map::new();
    http.insert("method".to_owned(), Value::from(method.to_uppercase()));
    http.insert("url".to_owned(), Value::from(url));
    let headers = pairs(file.dict("headers"), ctx);
    if !headers.is_empty() {
        http.insert("headers".to_owned(), Value::Array(headers));
    }
    let mut params = Vec::new();
    for (block, kind) in [("params:query", "query"), ("params:path", "path")] {
        for entry in file.dict(block) {
            ctx.scan(&entry.value);
            let mut map = Map::new();
            map.insert("name".to_owned(), Value::from(entry.key.clone()));
            map.insert("value".to_owned(), Value::from(entry.value.clone()));
            map.insert("type".to_owned(), Value::from(kind));
            disabled(&mut map, entry);
            params.push(Value::Object(map));
        }
    }
    if !params.is_empty() {
        http.insert("params".to_owned(), Value::Array(params));
    }
    if let Some(body) = body(file, file.value(method, "body").unwrap_or(""), &at, ctx) {
        http.insert("body".to_owned(), body);
    }
    if let Some(auth) = auth(file, file.value(method, "auth").unwrap_or(""), &at, ctx) {
        http.insert("auth".to_owned(), auth);
    }

    let mut runtime = Map::new();
    let vars = variables(file.dict("vars:pre-request"), ctx);
    if !vars.is_empty() {
        runtime.insert("variables".to_owned(), Value::Array(vars));
    }
    let scripts = scripts(file, &at, ctx);
    if !scripts.is_empty() {
        runtime.insert("scripts".to_owned(), Value::Array(scripts));
    }
    let assertions: Vec<Value> = file
        .dict("assert")
        .iter()
        .map(|entry| assertion(entry, &at, ctx))
        .collect();
    if !assertions.is_empty() {
        runtime.insert("assertions".to_owned(), Value::Array(assertions));
    }
    let actions: Vec<Value> = file
        .dict("vars:post-response")
        .iter()
        .map(|entry| extraction(entry, &at, ctx))
        .collect();
    if !actions.is_empty() {
        runtime.insert("actions".to_owned(), Value::Array(actions));
    }

    let mut value = Map::new();
    value.insert("info".to_owned(), Value::Object(info));
    value.insert("http".to_owned(), Value::Object(http));
    if !runtime.is_empty() {
        value.insert("runtime".to_owned(), Value::Object(runtime));
    }
    if let Some(settings) = settings(file, &at, ctx) {
        value.insert("settings".to_owned(), settings);
    }
    if let Some(docs) = file.text("docs") {
        value.insert("docs".to_owned(), Value::from(docs));
    }
    for block in file.blocks.iter().filter(|b| !handled(&b.name)) {
        ctx.item(
            &at,
            ItemKind::Option,
            ItemStatus::Skipped,
            Some(&block.name),
            format!("Bloc « {} » sans équivalent, non repris.", block.name),
            None,
        );
    }
    Some(Value::Object(value))
}

/// Nom de fichier libre parmi `taken` : `base.yml`, `base-2.yml`…
fn free(taken: &mut HashSet<String>, base: &str, ext: &str) -> String {
    let mut candidate = format!("{base}{ext}");
    let mut n = 2;
    while taken.contains(&candidate.to_lowercase()) {
        candidate = format!("{base}-{n}{ext}");
        n += 1;
    }
    taken.insert(candidate.to_lowercase());
    candidate
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// Convertit un dossier et son contenu ; `out` est son chemin d'arrivée.
fn folder(source: &Folder, out: &str, converted: &mut Converted, ctx: &mut Ctx) {
    let mut taken: HashSet<String> = HashSet::new();
    let place = source.path.as_str();
    for child in &source.folders {
        let dir = item_name(&child.dir).unwrap_or_else(|_| slug(&child.dir));
        let dir = free(&mut taken, &dir, "");
        let child_out = join(out, &dir);
        converted.folders.push(child_out.clone());
        ctx.counts.folders += 1;
        let mut info = Map::new();
        let display = child
            .settings
            .as_ref()
            .and_then(|s| s.file.value("meta", "name"))
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .unwrap_or(&child.dir)
            .to_owned();
        info.insert("name".to_owned(), Value::from(display.clone()));
        info.insert("type".to_owned(), Value::from("folder"));
        let mut value = Map::new();
        if let Some(settings) = &child.settings {
            if let Some(seq) = settings
                .file
                .value("meta", "seq")
                .and_then(|s| s.trim().parse::<f64>().ok())
            {
                info.insert("seq".to_owned(), json!(seq));
            }
            let at = At {
                name: &display,
                place: "réglages du dossier",
                file: &settings.path,
                request: None,
            };
            let request = defaults(&settings.file, &at, ctx);
            if !request.is_empty() {
                value.insert("request".to_owned(), Value::Object(request));
            }
            if let Some(docs) = settings.file.text("docs") {
                value.insert("docs".to_owned(), Value::from(docs));
            }
        }
        value.insert("info".to_owned(), Value::Object(info));
        push(
            converted,
            ctx,
            OutFile {
                path: join(&child_out, crate::opencollection::FOLDER_FILE),
                part: Part::Folder,
                value: Value::Object(value),
            },
            &display,
            child.settings.as_ref().map_or(&child.path, |s| &s.path),
        );
        folder(child, &child_out, converted, ctx);
    }
    taken.insert(crate::opencollection::FOLDER_FILE.to_owned());
    for source_request in &source.requests {
        let stem = source_request
            .path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".bru");
        let file_name = free(&mut taken, &slug(stem), ".yml");
        let out_path = join(out, &file_name);
        if let Some(value) = request(source_request, place, &out_path, ctx) {
            let name = value
                .pointer("/info/name")
                .and_then(Value::as_str)
                .unwrap_or(stem)
                .to_owned();
            if push(
                converted,
                ctx,
                OutFile {
                    path: out_path,
                    part: Part::HttpRequest,
                    value,
                },
                &name,
                &source_request.path,
            ) {
                ctx.counts.requests += 1;
            }
        }
    }
}

/// Ajoute un fichier valide pour le schéma ; sinon, le signale (R3).
fn push(converted: &mut Converted, ctx: &mut Ctx, file: OutFile, name: &str, origin: &str) -> bool {
    match schema::check(file.part, &file.value) {
        Ok(()) => {
            converted.files.push(file);
            true
        }
        Err(reason) => {
            ctx.item(
                &At {
                    name,
                    place: "",
                    file: origin,
                    request: None,
                },
                ItemKind::Request,
                ItemStatus::Skipped,
                None,
                format!("Non importé : {reason}."),
                None,
            );
            false
        }
    }
}

/// `{{process.env.NOM}}` seul dans une valeur → `NOM`.
fn process_env(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix("{{")?
        .strip_suffix("}}")?
        .trim()
        .strip_prefix("process.env.")
}

/// Environnement Bruno → `$defs/Environment` ; les secrets deviennent des
/// variables locales sans valeur (R7).
fn environment(
    name: &str,
    source: &Source,
    collection: &Collection,
    converted: &mut Converted,
    ctx: &mut Ctx,
) {
    let at = At {
        name,
        place: name,
        file: &source.path,
        request: None,
    };
    let mut variables = Vec::new();
    for entry in source.file.dict("vars") {
        let mut map = Map::new();
        map.insert("name".to_owned(), Value::from(entry.key.clone()));
        if let Some(env_name) = process_env(&entry.value) {
            // Valeur du fichier .env : variable locale, jamais écrite dans le projet.
            map.insert("secret".to_owned(), Value::Bool(true));
            disabled(&mut map, entry);
            variables.push(Value::Object(map));
            ctx.counts.secrets += 1;
            let known = collection.dotenv.get(env_name);
            if let Some(value) = known.filter(|_| entry.enabled) {
                converted.secrets.push(SecretValue {
                    environment: name.to_owned(),
                    name: entry.key.clone(),
                    value: value.clone(),
                });
            }
            ctx.items.push(ReportItem {
                kind: ItemKind::Env,
                status: if known.is_some() && entry.enabled {
                    ItemStatus::Stored
                } else {
                    ItemStatus::ToFill
                },
                name: entry.key.clone(),
                place: name.to_owned(),
                file: source.path.clone(),
                block: Some("vars".to_owned()),
                request: None,
                detail: if known.is_some() {
                    format!(
                        "Valeur de {DOTENV} (process.env.{env_name}) : variable locale, rangée dans le coffre du système, jamais dans le projet."
                    )
                } else {
                    format!(
                        "Lue dans {DOTENV} par Bruno (process.env.{env_name}) : variable locale à renseigner."
                    )
                },
                code: Some(format!("{}: {}", entry.key, entry.value)),
            });
            continue;
        }
        ctx.scan(&entry.value);
        map.insert("value".to_owned(), Value::from(entry.value.clone()));
        disabled(&mut map, entry);
        variables.push(Value::Object(map));
    }
    let mut secrets = Vec::new();
    if let Some(Body::List(items)) = source.file.block("vars:secret").map(|b| &b.body) {
        for item in items {
            let mut map = Map::new();
            map.insert("name".to_owned(), Value::from(item.name.clone()));
            map.insert("secret".to_owned(), Value::Bool(true));
            if !item.enabled {
                map.insert("disabled".to_owned(), Value::Bool(true));
            }
            variables.push(Value::Object(map));
            ctx.counts.secrets += 1;
            secrets.push(item.name.clone());
        }
    }
    if !secrets.is_empty() {
        ctx.example(
            "secret",
            format!("vars:secret [ {} ]", secrets.join(", ")),
            "variables locales, à renseigner (coffre du système)".to_owned(),
        );
        ctx.item(
            &At {
                name: &secrets.join(", "),
                ..at
            },
            ItemKind::Env,
            ItemStatus::ToFill,
            Some("vars:secret"),
            "Valeurs gardées par Bruno hors de la collection : variables locales à renseigner, jamais dans le projet.",
            None,
        );
    }
    let mut value = Map::new();
    value.insert("name".to_owned(), Value::from(name));
    value.insert("color".to_owned(), Value::from("gray"));
    if !variables.is_empty() {
        value.insert("variables".to_owned(), Value::Array(variables));
    }
    let value = Value::Object(value);
    match schema::check(Part::Environment, &value) {
        Ok(()) => {
            converted.environments.push(value);
            ctx.counts.environments += 1;
        }
        Err(reason) => ctx.item(
            &at,
            ItemKind::Env,
            ItemStatus::Skipped,
            None,
            format!("Environnement non importé : {reason}."),
            None,
        ),
    }
    for block in source
        .file
        .blocks
        .iter()
        .filter(|b| b.name != "vars" && b.name != "vars:secret")
    {
        ctx.item(
            &at,
            ItemKind::Option,
            ItemStatus::Skipped,
            Some(&block.name),
            format!("Bloc « {} » sans équivalent, non repris.", block.name),
            None,
        );
    }
}

/// Convertit une collection Bruno lue.
pub fn convert(collection: &Collection) -> Converted {
    let mut converted = Converted {
        name: collection.name.clone(),
        bru_files: collection.bru_files,
        ..Converted::default()
    };
    let mut ctx = Ctx {
        counts: ImportCounts::default(),
        items: Vec::new(),
        examples: Vec::new(),
        shown: HashSet::new(),
        dynamic: BTreeSet::new(),
    };
    for (path, reason) in &collection.unreadable {
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".bru");
        ctx.item(
            &At {
                name,
                place: path.rsplit_once('/').map_or("", |(dir, _)| dir),
                file: path,
                request: None,
            },
            ItemKind::Request,
            ItemStatus::Skipped,
            None,
            format!("Fichier illisible, non importé : {reason}."),
            None,
        );
    }
    if let Some(settings) = &collection.settings {
        let at = At {
            name: &collection.name,
            place: "réglages de la collection",
            file: &settings.path,
            request: None,
        };
        converted.defaults = defaults(&settings.file, &at, &mut ctx);
        converted.docs = settings.file.text("docs").map(str::to_owned);
    }
    for (name, source) in &collection.environments {
        environment(name, source, collection, &mut converted, &mut ctx);
    }
    folder(&collection.tree, "", &mut converted, &mut ctx);
    converted.counts = ctx.counts;
    converted.items = ctx.items;
    converted.examples = ctx.examples;
    converted
}
