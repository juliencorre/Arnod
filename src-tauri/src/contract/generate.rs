//! Requête prête à envoyer pour une opération du contrat (R5 à R7).

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde_json::Value;

use crate::catalog::names::MAX_NAME_CHARS;
use crate::contract::parse::{Contract, Operation, Parameter, SpecKind, resolve};
use crate::contract::sample::{as_param, declared_example, sample};
use crate::request::model::{
    ApiKeyLocation, Assertion, Auth, AuthKind, Body, BodyKind, ContractLink, Method, Operator,
    Origin, Pair, Request, Scalar,
};

/// Caractères encodés dans un segment de chemin.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Caractères interdits dans un nom de requête ou de dossier (R10 des projets).
const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Identifiants des vérifications ajoutées à la création (R7).
pub const STATUS_ASSERTION: &str = "statut";
pub const CONTRACT_ASSERTION: &str = "contrat";

/// Nom utilisable comme nom de requête ou de dossier : caractères interdits
/// remplacés, espaces réduits, 64 caractères au plus.
pub fn safe_name(raw: &str, fallback: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| {
            if FORBIDDEN.contains(&c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let mut name = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.chars().count() > MAX_NAME_CHARS {
        name = name.chars().take(MAX_NAME_CHARS).collect::<String>();
        name = name.trim_end().to_owned();
    }
    let name = name.trim_end_matches('.').trim().to_owned();
    if name.is_empty() {
        fallback.to_owned()
    } else {
        name
    }
}

/// Nom de la requête d'une opération (R5).
pub fn request_name(operation: &Operation) -> String {
    let raw = operation
        .summary
        .clone()
        .or_else(|| operation.operation_id.clone())
        .unwrap_or_else(|| operation.key.clone());
    safe_name(&raw, "Opération")
}

fn method_of(operation: &Operation) -> Option<Method> {
    Some(match operation.method.as_str() {
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

/// Une opération dont la méthode n'est pas envoyable (TRACE) n'a pas de requête.
pub fn supported(operation: &Operation) -> bool {
    method_of(operation).is_some()
}

/// Valeur d'exemple d'un paramètre.
fn parameter_value(contract: &Contract, parameter: &Parameter<'_>) -> String {
    let doc = &contract.doc;
    if let Some(example) = declared_example(doc, parameter.value) {
        return as_param(&example);
    }
    let value = match parameter.value.get("schema") {
        Some(schema) => sample(doc, schema),
        // Swagger 2 : le type est porté par le paramètre lui-même.
        None => sample(doc, parameter.value),
    };
    as_param(&value)
}

/// Requête de l'opération, liée au contrat `file` (R6, R7).
pub fn request_for(
    contract: &Contract,
    operation: &Operation,
    file: &str,
) -> Result<Request, String> {
    let method = method_of(operation)
        .ok_or_else(|| format!("la méthode {} n'est pas prise en charge", operation.method))?;
    let mut request = Request::new(request_name(operation));
    request.method = method;
    let parameters = contract.parameters(operation);

    let mut path = operation.path.clone();
    for parameter in parameters.iter().filter(|p| p.location == "path") {
        let value = parameter_value(contract, parameter);
        let encoded = utf8_percent_encode(&value, SEGMENT).to_string();
        path = path.replace(&format!("{{{}}}", parameter.name), &encoded);
    }
    // Paramètre de chemin non déclaré : variable, nommée à l'envoi si elle manque.
    let path = undeclared_to_variables(&path);
    request.url = format!("{{{{baseUrl}}}}{path}");

    for parameter in &parameters {
        let pair = Pair {
            name: parameter.name.clone(),
            value: parameter_value(contract, parameter),
            enabled: parameter.required,
        };
        match parameter.location.as_str() {
            "query" => request.params.push(pair),
            "header"
                if !["accept", "content-type", "authorization"]
                    .contains(&parameter.name.to_lowercase().as_str()) =>
            {
                request.headers.push(pair);
            }
            _ => {}
        }
    }

    let (body, media) = body_for(contract, operation, &parameters);
    if let Some(media) = media {
        request.headers.push(Pair {
            name: "Content-Type".to_owned(),
            value: media,
            enabled: true,
        });
    }
    request.body = body;
    if let Some(accept) = accept_for(contract, operation) {
        request.headers.push(Pair {
            name: "Accept".to_owned(),
            value: accept,
            enabled: true,
        });
    }
    request.auth = auth_for(contract, operation);

    if let Some(status) = contract.success_status(operation) {
        request.assertions.push(Assertion {
            id: STATUS_ASSERTION.to_owned(),
            target: "status".to_owned(),
            operator: Operator::Equals,
            value: Some(Scalar::Int(i64::from(status))),
            origin: Origin::Default,
        });
    }
    request.assertions.push(Assertion {
        id: CONTRACT_ASSERTION.to_owned(),
        target: "response".to_owned(),
        operator: Operator::MatchesContract,
        value: None,
        origin: Origin::Default,
    });
    request.contract = Some(ContractLink {
        file: file.to_owned(),
        operation: operation.key.clone(),
    });
    Ok(request)
}

fn undeclared_to_variables(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(&format!("{{{{{}}}}}", &rest[start + 1..start + end]));
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

fn is_json(media: &str) -> bool {
    let media = media.to_lowercase();
    media.starts_with("application/json") || media.contains("+json")
}

/// Type de contenu préféré : JSON, puis formulaire, puis texte, puis le premier.
fn preferred_media<'a>(types: impl Iterator<Item = &'a String> + Clone) -> Option<&'a String> {
    let mut list = types.clone();
    list.find(|t| is_json(t))
        .or_else(|| {
            types
                .clone()
                .find(|t| t.starts_with("application/x-www-form-urlencoded"))
        })
        .or_else(|| types.clone().find(|t| t.starts_with("multipart/form-data")))
        .or_else(|| types.clone().find(|t| t.starts_with("text/")))
        .or_else(|| types.clone().next())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn form_pairs(value: &Value) -> Vec<Pair> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(name, value)| Pair {
                    name: name.clone(),
                    value: match value {
                        Value::Object(_) => value.to_string(),
                        other => as_param(other),
                    },
                    enabled: true,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn body_from(media: &str, example: Value) -> Body {
    if is_json(media) {
        Body {
            kind: BodyKind::Json,
            content: pretty(&example),
            form: Vec::new(),
        }
    } else if media.starts_with("application/x-www-form-urlencoded")
        || media.starts_with("multipart/form-data")
    {
        Body {
            kind: BodyKind::Form,
            content: String::new(),
            form: form_pairs(&example),
        }
    } else {
        Body {
            kind: BodyKind::Text,
            content: match example {
                Value::String(text) => text,
                Value::Null => String::new(),
                other => pretty(&other),
            },
            form: Vec::new(),
        }
    }
}

/// Corps d'exemple et type de contenu de l'opération.
fn body_for(
    contract: &Contract,
    operation: &Operation,
    parameters: &[Parameter<'_>],
) -> (Body, Option<String>) {
    let doc = &contract.doc;
    match contract.kind {
        SpecKind::OpenApi3 => {
            let Some(body) = contract
                .operation_value(operation)
                .and_then(|o| o.get("requestBody"))
                .map(|b| resolve(doc, b))
            else {
                return (Body::default(), None);
            };
            let Some(content) = body.get("content").and_then(Value::as_object) else {
                return (Body::default(), None);
            };
            let Some(media) = preferred_media(content.keys()) else {
                return (Body::default(), None);
            };
            let entry = &content[media];
            let example = declared_example(doc, entry)
                .or_else(|| entry.get("schema").map(|schema| sample(doc, schema)))
                .unwrap_or(Value::Null);
            let content_type = if media.starts_with("multipart/form-data") {
                // Le formulaire est envoyé encodé ; le type multipart demanderait un séparateur.
                "application/x-www-form-urlencoded".to_owned()
            } else {
                media.clone()
            };
            (body_from(media, example), Some(content_type))
        }
        SpecKind::Swagger2 => {
            let consumes = consumes_v2(contract, operation);
            if let Some(parameter) = parameters.iter().find(|p| p.location == "body") {
                let example = declared_example(doc, parameter.value)
                    .or_else(|| parameter.value.get("schema").map(|s| sample(doc, s)))
                    .unwrap_or(Value::Null);
                let media = preferred_media(consumes.iter())
                    .cloned()
                    .unwrap_or_else(|| "application/json".to_owned());
                return (body_from(&media, example), Some(media));
            }
            let fields: Vec<Pair> = parameters
                .iter()
                .filter(|p| p.location == "formData")
                .map(|p| Pair {
                    name: p.name.clone(),
                    value: parameter_value(contract, p),
                    enabled: p.required,
                })
                .collect();
            if fields.is_empty() {
                return (Body::default(), None);
            }
            (
                Body {
                    kind: BodyKind::Form,
                    content: String::new(),
                    form: fields,
                },
                Some("application/x-www-form-urlencoded".to_owned()),
            )
        }
    }
}

fn consumes_v2(contract: &Contract, operation: &Operation) -> Vec<String> {
    let list = |value: Option<&Value>| -> Vec<String> {
        value
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let own = list(
        contract
            .operation_value(operation)
            .and_then(|o| o.get("consumes")),
    );
    if own.is_empty() {
        list(contract.doc.get("consumes"))
    } else {
        own
    }
}

/// `Accept` : type JSON de la réponse de succès, s'il y en a un.
fn accept_for(contract: &Contract, operation: &Operation) -> Option<String> {
    let responses = contract.responses(operation)?;
    let status = contract
        .success_status(operation)
        .map(|s| s.to_string())
        .or_else(|| responses.keys().next().cloned())?;
    let response = resolve(&contract.doc, responses.get(&status)?);
    match contract.kind {
        SpecKind::OpenApi3 => response
            .get("content")
            .and_then(Value::as_object)
            .and_then(|content| content.keys().find(|t| is_json(t)).cloned()),
        SpecKind::Swagger2 => {
            response.get("schema")?;
            let produces = contract
                .operation_value(operation)
                .and_then(|o| o.get("produces"))
                .or_else(|| contract.doc.get("produces"))
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            produces
                .into_iter()
                .find(|t| is_json(t))
                .or_else(|| Some("application/json".to_owned()))
        }
    }
}

/// Authentification de la première exigence de sécurité, valeurs en variables.
fn auth_for(contract: &Contract, operation: &Operation) -> Auth {
    let doc = &contract.doc;
    let requirement = contract
        .operation_value(operation)
        .and_then(|o| o.get("security"))
        .or_else(|| doc.get("security"))
        .and_then(Value::as_array)
        .and_then(|list| {
            list.iter()
                .find_map(|r| r.as_object()?.keys().next().cloned())
        });
    let Some(name) = requirement else {
        return Auth::default();
    };
    let schemes = match contract.kind {
        SpecKind::OpenApi3 => doc.pointer("/components/securitySchemes"),
        SpecKind::Swagger2 => doc.get("securityDefinitions"),
    };
    let Some(scheme) = schemes.and_then(|s| s.get(&name)).map(|s| resolve(doc, s)) else {
        return Auth::default();
    };
    let kind = scheme.get("type").and_then(Value::as_str).unwrap_or("");
    let http = scheme
        .get("scheme")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    match kind {
        "basic" => basic(),
        "http" if http == "basic" => basic(),
        "http" | "oauth2" | "openIdConnect" => Auth {
            kind: AuthKind::Bearer,
            token: "{{token}}".to_owned(),
            ..Auth::default()
        },
        "apiKey" => {
            let location = match scheme.get("in").and_then(Value::as_str) {
                Some("query") => ApiKeyLocation::Query,
                Some("header") => ApiKeyLocation::Header,
                _ => return Auth::default(),
            };
            Auth {
                kind: AuthKind::ApiKey,
                name: scheme
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("X-API-Key")
                    .to_owned(),
                value: "{{apiKey}}".to_owned(),
                location,
                ..Auth::default()
            }
        }
        _ => Auth::default(),
    }
}

fn basic() -> Auth {
    Auth {
        kind: AuthKind::Basic,
        username: "{{username}}".to_owned(),
        password: "{{password}}".to_owned(),
        ..Auth::default()
    }
}
