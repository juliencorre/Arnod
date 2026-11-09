//! Lecture d'un contrat OpenAPI 3.x ou Swagger 2.0 (R1, R3).

use serde::Serialize;
use serde_json::Value;

/// Méthodes d'une opération, dans l'ordre de la spécification.
const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// Nombre maximal de références suivies d'affilée (chaînes de `$ref`).
const MAX_HOPS: usize = 32;

/// Famille de spécification d'un contrat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpecKind {
    OpenApi3,
    Swagger2,
}

/// Contrat lu : en-tête, serveurs, opérations, et document complet pour les références.
#[derive(Debug, Clone)]
pub struct Contract {
    pub title: String,
    /// `info.version` : version du contrat d'échange.
    pub version: String,
    pub kind: SpecKind,
    /// Valeur de `openapi` ou `swagger` (`3.0.3`, `2.0`).
    pub spec_version: String,
    pub description: Option<String>,
    /// Adresses des serveurs, variables remplacées par leur valeur par défaut.
    pub servers: Vec<String>,
    /// Opérations groupées par tag, dans l'ordre du document.
    pub operations: Vec<Operation>,
    pub doc: Value,
}

/// Opération : une méthode sur un chemin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// `GET /pets/{petId}`.
    pub key: String,
    /// Méthode en majuscules.
    pub method: String,
    pub path: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    /// Premier tag de l'opération.
    pub tag: Option<String>,
    pub deprecated: bool,
    /// Référence vers un autre fichier ou une adresse (R3) : non résolue.
    pub external_refs: bool,
}

/// Paramètre d'une opération (communs au chemin compris).
#[derive(Debug, Clone)]
pub struct Parameter<'a> {
    pub name: String,
    /// `path`, `query`, `header`, `cookie`, `body`, `formData`.
    pub location: String,
    pub required: bool,
    /// Objet du paramètre, références résolues.
    pub value: &'a Value,
}

/// Interprète un texte JSON ou YAML (R1). L'erreur est un message en français.
pub fn load(text: &str) -> Result<Contract, String> {
    let doc = read_document(text)?;
    let Some(root) = doc.as_object() else {
        return Err("le texte n'est pas un objet OpenAPI.".to_owned());
    };
    let (kind, spec_version) = if let Some(version) = root.get("openapi") {
        let version = scalar_text(version);
        if !version.starts_with("3.") {
            return Err(format!(
                "la version OpenAPI « {version} » n'est pas prise en charge (3.0, 3.1 ou Swagger 2.0)."
            ));
        }
        (SpecKind::OpenApi3, version)
    } else if let Some(version) = root.get("swagger") {
        let version = scalar_text(version);
        if version != "2.0" {
            return Err(format!(
                "la version Swagger « {version} » n'est pas prise en charge (2.0)."
            ));
        }
        (SpecKind::Swagger2, version)
    } else {
        return Err(
            "ce n'est pas un contrat OpenAPI : champ « openapi » ou « swagger » absent.".to_owned(),
        );
    };
    let paths = root.get("paths");
    if !paths.is_some_and(Value::is_object) && !(spec_version.starts_with("3.1") && paths.is_none())
    {
        return Err("le contrat n'a pas d'objet « paths ».".to_owned());
    }
    let info = root.get("info");
    let text_of = |key: &str| {
        info.and_then(|info| info.get(key))
            .map(scalar_text)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    };
    let title = text_of("title").unwrap_or_else(|| "Contrat sans titre".to_owned());
    let version = text_of("version").unwrap_or_default();
    let description = text_of("description");
    let servers = match kind {
        SpecKind::OpenApi3 => servers_v3(&doc),
        SpecKind::Swagger2 => servers_v2(&doc),
    };
    let operations = operations(&doc);
    Ok(Contract {
        title,
        version,
        kind,
        spec_version,
        description,
        servers,
        operations,
        doc,
    })
}

fn read_document(text: &str) -> Result<Value, String> {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.is_empty() {
        return Err("le texte est vide.".to_owned());
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|error| format!("JSON illisible : {error}."))
    } else {
        serde_saphyr::from_str::<Value>(trimmed)
            .map_err(|error| format!("YAML illisible : {error}."))
    }
}

/// Texte d'une valeur scalaire (`3.0` écrit sans guillemets est un nombre en YAML).
pub fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn servers_v3(doc: &Value) -> Vec<String> {
    let Some(servers) = doc.get("servers").and_then(Value::as_array) else {
        return Vec::new();
    };
    servers
        .iter()
        .filter_map(|server| {
            let mut url = server.get("url")?.as_str()?.to_owned();
            if let Some(variables) = server.get("variables").and_then(Value::as_object) {
                for (name, variable) in variables {
                    let default = variable.get("default").map(scalar_text).unwrap_or_default();
                    url = url.replace(&format!("{{{name}}}"), &default);
                }
            }
            Some(url.trim_end_matches('/').to_owned())
        })
        .filter(|url| !url.is_empty())
        .collect()
}

fn servers_v2(doc: &Value) -> Vec<String> {
    let base_path = doc
        .get("basePath")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_end_matches('/');
    let Some(host) = doc.get("host").and_then(Value::as_str) else {
        return if base_path.is_empty() {
            Vec::new()
        } else {
            vec![base_path.to_owned()]
        };
    };
    let schemes: Vec<&str> = doc
        .get("schemes")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let schemes = if schemes.is_empty() {
        vec!["https"]
    } else {
        schemes
    };
    schemes
        .iter()
        .map(|scheme| format!("{scheme}://{host}{base_path}"))
        .collect()
}

/// Suit les références locales (`#/…`) d'une valeur (R3). Une référence externe
/// ou introuvable laisse la valeur telle quelle.
pub fn resolve<'a>(doc: &'a Value, value: &'a Value) -> &'a Value {
    let mut current = value;
    for _ in 0..MAX_HOPS {
        let Some(reference) = current.get("$ref").and_then(Value::as_str) else {
            return current;
        };
        let Some(target) = pointer(doc, reference) else {
            return current;
        };
        current = target;
    }
    current
}

/// Cible d'une référence locale `#/a/b`, ou `None`.
pub fn pointer<'a>(doc: &'a Value, reference: &str) -> Option<&'a Value> {
    let fragment = reference.strip_prefix('#')?;
    if fragment.is_empty() {
        return Some(doc);
    }
    doc.pointer(&fragment.replace("%7B", "{").replace("%7D", "}"))
}

fn has_external_ref(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            (key == "$ref" && value.as_str().is_some_and(|r| !r.starts_with('#')))
                || has_external_ref(value)
        }),
        Value::Array(items) => items.iter().any(has_external_ref),
        _ => false,
    }
}

fn operations(doc: &Value) -> Vec<Operation> {
    let Some(paths) = doc.get("paths").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for (path, item) in paths {
        let item = resolve(doc, item);
        for method in METHODS {
            let Some(operation) = item.get(method).filter(|o| o.is_object()) else {
                continue;
            };
            let text = |key: &str| {
                operation
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
            };
            let upper = method.to_uppercase();
            list.push(Operation {
                key: format!("{upper} {path}"),
                method: upper,
                path: path.clone(),
                operation_id: text("operationId"),
                summary: text("summary"),
                tag: operation
                    .get("tags")
                    .and_then(Value::as_array)
                    .and_then(|tags| tags.first())
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                deprecated: operation
                    .get("deprecated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                external_refs: has_external_ref(operation)
                    || item.get("parameters").is_some_and(has_external_ref),
            });
        }
    }
    // Groupes par tag, dans l'ordre de la liste `tags` du document, puis
    // d'apparition ; l'ordre du document est gardé dans chaque groupe.
    let mut order: Vec<Option<String>> = doc
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("name").and_then(Value::as_str))
                .map(|name| Some(name.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    for operation in &list {
        if !order.contains(&operation.tag) {
            order.push(operation.tag.clone());
        }
    }
    let rank = |tag: &Option<String>| order.iter().position(|t| t == tag).unwrap_or(usize::MAX);
    list.sort_by_key(|operation| rank(&operation.tag));
    list
}

impl Contract {
    /// Opération par sa clé.
    pub fn operation(&self, key: &str) -> Option<&Operation> {
        self.operations
            .iter()
            .find(|operation| operation.key == key)
    }

    /// Objet de l'opération dans le document.
    pub fn operation_value(&self, operation: &Operation) -> Option<&Value> {
        let item = self.doc.get("paths")?.get(&operation.path)?;
        resolve(&self.doc, item).get(operation.method.to_lowercase())
    }

    /// Paramètres de l'opération : communs au chemin, remplacés par ceux de
    /// l'opération de même nom et emplacement.
    pub fn parameters(&self, operation: &Operation) -> Vec<Parameter<'_>> {
        let Some(item) = self
            .doc
            .get("paths")
            .and_then(|paths| paths.get(&operation.path))
        else {
            return Vec::new();
        };
        let item = resolve(&self.doc, item);
        let own = self.operation_value(operation);
        let mut list: Vec<Parameter<'_>> = Vec::new();
        for source in [
            item.get("parameters"),
            own.and_then(|o| o.get("parameters")),
        ] {
            let Some(parameters) = source.and_then(Value::as_array) else {
                continue;
            };
            for raw in parameters {
                let value = resolve(&self.doc, raw);
                let Some(name) = value.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let location = value
                    .get("in")
                    .and_then(Value::as_str)
                    .unwrap_or("query")
                    .to_owned();
                let required = location == "path"
                    || value
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                let parameter = Parameter {
                    name: name.to_owned(),
                    location,
                    required,
                    value,
                };
                if let Some(existing) = list
                    .iter_mut()
                    .find(|p| p.name == parameter.name && p.location == parameter.location)
                {
                    *existing = parameter;
                } else {
                    list.push(parameter);
                }
            }
        }
        list
    }

    /// Réponses déclarées de l'opération.
    pub fn responses(&self, operation: &Operation) -> Option<&serde_json::Map<String, Value>> {
        self.operation_value(operation)?
            .get("responses")
            .and_then(Value::as_object)
    }

    /// Plus petit statut `2xx` déclaré (R7).
    pub fn success_status(&self, operation: &Operation) -> Option<u16> {
        self.responses(operation)?
            .keys()
            .filter_map(|code| code.parse::<u16>().ok())
            .filter(|code| (200..300).contains(code))
            .min()
    }
}
