//! Contrat déduit des échanges réels avec un serveur
//! (`docs/plan-accueil-non-classes.md`, R6 à R10). Module pur : un document
//! OpenAPI 3.1 en `serde_json::Value`, enrichi échange après échange.

use serde_json::{Map, Value, json};

/// Extension qui marque un contrat écrit par Arnod (R9).
pub const DEDUCED_MARK: &str = "x-arnod-deduced";

/// Profondeur maximale d'un schéma déduit (R8).
const MAX_DEPTH: usize = 8;

/// Taille maximale d'un corps gardé comme exemple.
pub const MAX_EXAMPLE_BYTES: usize = 64 * 1024;

/// Échange observé : la requête envoyée et la réponse reçue.
#[derive(Debug, Clone)]
pub struct Exchange<'a> {
    pub method: &'a str,
    pub url: &'a reqwest::Url,
    pub status: u16,
    pub status_text: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a str,
}

/// Origine d'une adresse : schéma, hôte et port (R6).
pub fn origin(url: &reqwest::Url) -> String {
    url.origin().ascii_serialization()
}

/// Titre du contrat d'une origine : l'hôte, avec son port s'il n'est pas implicite.
pub fn title(url: &reqwest::Url) -> String {
    let host = url.host_str().unwrap_or("serveur");
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    }
}

/// Contrat déduit vide pour l'origine de `url` (R6).
pub fn new_document(url: &reqwest::Url) -> Value {
    let origin = origin(url);
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": title(url),
            "version": "0.0.0",
            "description": format!("Contrat déduit des échanges réels avec {origin}."),
        },
        DEDUCED_MARK: true,
        "servers": [{ "url": origin }],
        "paths": {},
    })
}

/// Contrat écrit par Arnod, donc qu'il peut enrichir (R9).
pub fn is_deduced(doc: &Value) -> bool {
    doc.get(DEDUCED_MARK).and_then(Value::as_bool) == Some(true)
}

/// Le contrat sert-il cette origine ? (premier serveur).
pub fn serves(doc: &Value, url: &reqwest::Url) -> bool {
    doc.pointer("/servers/0/url")
        .and_then(Value::as_str)
        .is_some_and(|server| server.trim_end_matches('/') == origin(url))
}

/// Chemin de l'opération : celui de l'adresse, sans paramètres ni fragment (R7).
pub fn path_of(url: &reqwest::Url) -> String {
    let path = url.path();
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

/// Clé d'opération, comme `contract::parse` : `GET /products`.
pub fn operation_key(method: &str, url: &reqwest::Url) -> String {
    format!("{} {}", method.to_uppercase(), path_of(url))
}

fn operation_id(method: &str, path: &str) -> String {
    let mut words = vec![method.to_lowercase()];
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        let word: String = segment
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        words.push(word.trim_matches('_').to_owned());
    }
    if words.len() == 1 {
        words.push("root".to_owned());
    }
    words.retain(|w| !w.is_empty());
    words.join("_")
}

/// Schéma JSON d'une valeur (R8) : types, propriétés toutes requises, éléments
/// d'un tableau d'après le premier.
pub fn schema_of(value: &Value) -> Value {
    schema_at(value, 0)
}

fn schema_at(value: &Value, depth: usize) -> Value {
    if depth >= MAX_DEPTH {
        return json!({});
    }
    match value {
        Value::Null => json!({ "type": "null" }),
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Number(n) if n.is_i64() || n.is_u64() => json!({ "type": "integer" }),
        Value::Number(_) => json!({ "type": "number" }),
        Value::String(_) => json!({ "type": "string" }),
        Value::Array(items) => match items.first() {
            Some(first) => json!({ "type": "array", "items": schema_at(first, depth + 1) }),
            None => json!({ "type": "array" }),
        },
        Value::Object(map) if map.is_empty() => json!({ "type": "object" }),
        Value::Object(map) => {
            let properties: Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), schema_at(v, depth + 1)))
                .collect();
            let required: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
            json!({ "type": "object", "properties": properties, "required": required })
        }
    }
}

fn media_type(headers: &[(String, String)]) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_lowercase()
        })
        .filter(|media| !media.is_empty())
}

/// Réponse OpenAPI d'un échange (R7) : exemple et schéma d'un JSON, texte sinon,
/// rien pour un corps vide.
fn response_of(exchange: &Exchange<'_>) -> Value {
    let description = if exchange.status_text.trim().is_empty() {
        "Réponse reçue".to_owned()
    } else {
        exchange.status_text.trim().to_owned()
    };
    let mut response = Map::new();
    response.insert("description".to_owned(), Value::String(description));
    let body = exchange.body.trim();
    if !body.is_empty() {
        let parsed = serde_json::from_str::<Value>(body).ok();
        let media = media_type(exchange.headers).unwrap_or_else(|| {
            if parsed.is_some() {
                "application/json".to_owned()
            } else {
                "text/plain".to_owned()
            }
        });
        let mut content = Map::new();
        match parsed {
            Some(value) if media.contains("json") || !media.starts_with("text/") => {
                content.insert("schema".to_owned(), schema_of(&value));
                if body.len() <= MAX_EXAMPLE_BYTES {
                    content.insert("example".to_owned(), value);
                }
            }
            _ => {
                content.insert("schema".to_owned(), json!({ "type": "string" }));
                if body.len() <= MAX_EXAMPLE_BYTES {
                    content.insert("example".to_owned(), Value::String(body.to_owned()));
                }
            }
        }
        response.insert(
            "content".to_owned(),
            json!({ media: Value::Object(content) }),
        );
    }
    Value::Object(response)
}

/// Paramètres de requête de l'adresse, avec leur valeur d'exemple (R7).
fn query_parameters(url: &reqwest::Url) -> Vec<Value> {
    let mut seen = Vec::new();
    url.query_pairs()
        .filter(|(name, _)| {
            let new = !seen.contains(&name.to_string());
            seen.push(name.to_string());
            new
        })
        .map(|(name, value)| {
            json!({
                "name": name,
                "in": "query",
                "required": false,
                "schema": { "type": "string" },
                "example": value,
            })
        })
        .collect()
}

/// Ajoute l'échange au contrat déduit (R7, R9) et renvoie la clé de son
/// opération. Opération absente : ajoutée ; présente : un statut nouveau est
/// ajouté, un statut connu n'est pas modifié.
pub fn add_exchange(doc: &mut Value, exchange: &Exchange<'_>) -> String {
    let method = exchange.method.to_lowercase();
    let path = path_of(exchange.url);
    let key = operation_key(exchange.method, exchange.url);
    let Some(root) = doc.as_object_mut() else {
        return key;
    };
    let paths = root
        .entry("paths")
        .or_insert_with(|| Value::Object(Map::new()));
    if !paths.is_object() {
        *paths = Value::Object(Map::new());
    }
    let Some(paths) = paths.as_object_mut() else {
        return key;
    };
    let item = paths
        .entry(path.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(item) = item.as_object_mut() else {
        return key;
    };
    let status = exchange.status.to_string();
    match item.get_mut(&method).and_then(Value::as_object_mut) {
        Some(operation) => {
            let responses = operation
                .entry("responses")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(responses) = responses.as_object_mut()
                && !responses.contains_key(&status)
            {
                responses.insert(status, response_of(exchange));
            }
        }
        None => {
            let mut operation = Map::new();
            operation.insert(
                "operationId".to_owned(),
                Value::String(operation_id(&method, &path)),
            );
            operation.insert("summary".to_owned(), Value::String(key.clone()));
            let parameters = query_parameters(exchange.url);
            if !parameters.is_empty() {
                operation.insert("parameters".to_owned(), Value::Array(parameters));
            }
            operation.insert(
                "responses".to_owned(),
                json!({ status: response_of(exchange) }),
            );
            item.insert(method, Value::Object(operation));
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{conformance, parse};
    use crate::request::assertions::ResponseData;

    fn url(text: &str) -> reqwest::Url {
        reqwest::Url::parse(text).unwrap()
    }

    fn json_headers() -> Vec<(String, String)> {
        vec![(
            "Content-Type".to_owned(),
            "application/json; charset=utf-8".to_owned(),
        )]
    }

    fn exchange<'a>(
        method: &'a str,
        url: &'a reqwest::Url,
        status: u16,
        headers: &'a [(String, String)],
        body: &'a str,
    ) -> Exchange<'a> {
        Exchange {
            method,
            url,
            status,
            status_text: "OK",
            headers,
            body,
        }
    }

    const PRODUCTS: &str = r#"[{"id":"prd_101","name":"T-shirt bleu","price":29.9,"stock":3}]"#;

    #[test]
    fn r6_contrat_vide_d_une_origine() {
        let doc = new_document(&url("https://api.boutique.dev:8443/products?x=1"));
        assert_eq!(doc["info"]["title"], "api.boutique.dev:8443");
        assert_eq!(doc["info"]["version"], "0.0.0");
        assert_eq!(doc["servers"][0]["url"], "https://api.boutique.dev:8443");
        assert!(is_deduced(&doc));
        assert!(serves(&doc, &url("https://api.boutique.dev:8443/autre")));
        assert!(!serves(&doc, &url("http://api.boutique.dev:8443/")));
        let contract = parse::load(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(contract.version, "0.0.0");
        assert!(contract.operations.is_empty());
        assert_eq!(title(&url("https://api.boutique.dev/")), "api.boutique.dev");
    }

    #[test]
    fn r7_operation_d_un_get_json() {
        let target = url("https://api.boutique.dev/products?page=2&page=3");
        let headers = json_headers();
        let mut doc = new_document(&target);
        let key = add_exchange(&mut doc, &exchange("GET", &target, 200, &headers, PRODUCTS));
        assert_eq!(key, "GET /products");
        let operation = &doc["paths"]["/products"]["get"];
        assert_eq!(operation["operationId"], "get_products");
        assert_eq!(operation["summary"], "GET /products");
        assert_eq!(operation["parameters"].as_array().unwrap().len(), 1);
        assert_eq!(operation["parameters"][0]["example"], "2");
        let content = &operation["responses"]["200"]["content"]["application/json"];
        assert_eq!(content["schema"]["type"], "array");
        assert_eq!(
            content["schema"]["items"]["properties"]["price"]["type"],
            "number"
        );
        assert_eq!(
            content["schema"]["items"]["properties"]["stock"]["type"],
            "integer"
        );
        assert_eq!(content["schema"]["items"]["required"][0], "id");
        assert_eq!(content["example"][0]["name"], "T-shirt bleu");

        // La clé est celle que lit `contract::parse`, et la réponse y est conforme.
        let contract = parse::load(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(contract.operations[0].key, key);
        let response = ResponseData {
            status: 200,
            duration_ms: 10,
            headers,
            body: PRODUCTS.to_owned(),
        };
        assert_eq!(conformance::check(&contract, &key, &response), Ok(()));
    }

    #[test]
    fn r7_corps_texte_vide_et_racine() {
        let target = url("http://localhost:8080");
        let mut doc = new_document(&target);
        let text = vec![("content-type".to_owned(), "text/plain".to_owned())];
        let key = add_exchange(&mut doc, &exchange("GET", &target, 200, &text, "pong"));
        assert_eq!(key, "GET /");
        let content = &doc["paths"]["/"]["get"]["responses"]["200"]["content"]["text/plain"];
        assert_eq!(content["schema"]["type"], "string");
        assert_eq!(content["example"], "pong");
        assert_eq!(doc["paths"]["/"]["get"]["operationId"], "get_root");

        add_exchange(&mut doc, &exchange("DELETE", &target, 204, &[], ""));
        let deleted = &doc["paths"]["/"]["delete"]["responses"]["204"];
        assert!(deleted.get("content").is_none(), "{deleted}");
    }

    #[test]
    fn r7_corps_json_sans_type_et_trop_gros() {
        let target = url("https://api.local/big");
        let mut doc = new_document(&target);
        let big = format!("[{}]", vec!["1"; MAX_EXAMPLE_BYTES].join(","));
        add_exchange(&mut doc, &exchange("GET", &target, 200, &[], &big));
        let content =
            &doc["paths"]["/big"]["get"]["responses"]["200"]["content"]["application/json"];
        assert_eq!(content["schema"]["items"]["type"], "integer");
        assert!(content.get("example").is_none());
    }

    #[test]
    fn r8_schema_borne_et_vides() {
        assert_eq!(schema_of(&json!([])), json!({ "type": "array" }));
        assert_eq!(schema_of(&json!({})), json!({ "type": "object" }));
        assert_eq!(schema_of(&json!(null)), json!({ "type": "null" }));
        assert_eq!(schema_of(&json!(true)), json!({ "type": "boolean" }));
        let mut deep = json!(1);
        for _ in 0..12 {
            deep = json!({ "a": deep });
        }
        let mut schema = schema_of(&deep);
        for _ in 0..MAX_DEPTH {
            schema = schema["properties"]["a"].clone();
        }
        assert_eq!(schema, json!({}));
    }

    #[test]
    fn r9_enrichissement_sans_ecraser() {
        let target = url("https://api.boutique.dev/products");
        let headers = json_headers();
        let mut doc = new_document(&target);
        add_exchange(&mut doc, &exchange("GET", &target, 200, &headers, PRODUCTS));
        // Même statut : rien ne change, même avec un autre corps.
        add_exchange(&mut doc, &exchange("GET", &target, 200, &headers, "[]"));
        let responses = &doc["paths"]["/products"]["get"]["responses"];
        assert_eq!(
            responses["200"]["content"]["application/json"]["example"][0]["id"],
            "prd_101"
        );
        // Statut nouveau : ajouté.
        add_exchange(
            &mut doc,
            &exchange("GET", &target, 404, &headers, r#"{"error":"absent"}"#),
        );
        let responses = &doc["paths"]["/products"]["get"]["responses"];
        assert_eq!(responses.as_object().map(Map::len), Some(2));
        // Autre opération, autre chemin : ajoutées.
        let orders = url("https://api.boutique.dev/orders/42/tracking");
        let key = add_exchange(&mut doc, &exchange("POST", &orders, 201, &headers, "{}"));
        assert_eq!(key, "POST /orders/42/tracking");
        assert_eq!(
            doc["paths"]["/orders/42/tracking"]["post"]["operationId"],
            "post_orders_42_tracking"
        );
        let contract = parse::load(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(contract.operations.len(), 2);
    }

    #[test]
    fn r9_contrat_ecrit_a_la_main_non_deduit() {
        let doc =
            json!({ "openapi": "3.1.0", "info": { "title": "x", "version": "1" }, "paths": {} });
        assert!(!is_deduced(&doc));
    }
}
