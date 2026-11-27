//! Conversion d'un projet au format précédent d'Arnod (`arnod.yml`) en
//! collection OpenCollection (S6, R6). Faite une fois, à l'ouverture.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use uuid::Uuid;

use crate::environment::{self, Environment, Variable};
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::ROOT_FILE;
use crate::opencollection::collection::{self, ArnodMeta};
use crate::request::file as request_file;
use crate::request::model::{
    ApiKeyLocation, Assertion, Auth, AuthKind, Body, BodyKind, ContractLink, DEFAULT_TIMEOUT_MS,
    Method, Operator, Origin, Pair, Request, Scalar, Settings,
};
use crate::tree::ENVIRONMENTS_DIR;

/// Manifeste du format précédent.
pub const LEGACY_MANIFEST: &str = "arnod.yml";
/// Ordre des environnements du format précédent.
const LEGACY_ORDER: &str = "_order.yml";
/// Dossier des contrats, jamais converti.
const CONTRACTS_DIR: &str = "contracts";

#[derive(Deserialize)]
struct LegacyManifest {
    version: u32,
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct LegacyOrder {
    #[serde(default)]
    order: Vec<String>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct LegacyEnvironment {
    name: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    protected: bool,
    #[serde(default)]
    variables: Vec<Variable>,
    #[serde(default)]
    locals: Vec<String>,
}

#[derive(Deserialize)]
struct LegacyPair {
    name: String,
    #[serde(default)]
    value: String,
    #[serde(default = "yes")]
    enabled: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyAuth {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    location: String,
}

#[derive(Deserialize, Default)]
struct LegacyBody {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    form: Vec<LegacyPair>,
}

#[derive(Deserialize)]
struct LegacyAssertion {
    target: String,
    op: String,
    #[serde(default)]
    value: Option<Scalar>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRequest {
    #[allow(dead_code)]
    version: u32,
    name: String,
    method: Method,
    url: String,
    #[serde(default)]
    params: Vec<LegacyPair>,
    #[serde(default)]
    headers: Vec<LegacyPair>,
    #[serde(default)]
    auth: LegacyAuth,
    #[serde(default)]
    body: LegacyBody,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    assertions: Vec<LegacyAssertion>,
    #[serde(default)]
    contract: Option<ContractLink>,
}

fn pairs(list: Vec<LegacyPair>) -> Vec<Pair> {
    list.into_iter()
        .map(|pair| Pair {
            enabled: pair.enabled,
            ..Pair::new(pair.name, pair.value)
        })
        .collect()
}

fn operator(name: &str) -> Option<Operator> {
    Some(match name {
        "equals" => Operator::Equals,
        "notEquals" => Operator::NotEquals,
        "exists" => Operator::Exists,
        "contains" => Operator::Contains,
        "lessThan" => Operator::LessThan,
        "greaterThan" => Operator::GreaterThan,
        "isNumber" => Operator::IsNumber,
        "isText" => Operator::IsText,
        // `matchesContract` : la conformité est désormais déduite du lien (S7).
        _ => return None,
    })
}

/// Requête au format précédent, traduite dans le modèle d'Arnod.
fn convert(legacy: LegacyRequest) -> Request {
    let auth = match legacy.auth.kind.as_str() {
        "basic" => Auth {
            kind: AuthKind::Basic,
            username: legacy.auth.username,
            password: legacy.auth.password,
            ..Auth::default()
        },
        "bearer" => Auth {
            kind: AuthKind::Bearer,
            token: legacy.auth.token,
            ..Auth::default()
        },
        "apiKey" => Auth {
            kind: AuthKind::ApiKey,
            name: legacy.auth.name,
            value: legacy.auth.value,
            location: if legacy.auth.location == "query" {
                ApiKeyLocation::Query
            } else {
                ApiKeyLocation::Header
            },
            ..Auth::default()
        },
        _ => Auth::default(),
    };
    let body = match legacy.body.kind.as_str() {
        "json" => Body::raw(BodyKind::Json, legacy.body.content),
        "text" => Body::raw(BodyKind::Text, legacy.body.content),
        "form" => Body::form(pairs(legacy.body.form)),
        _ => Body::default(),
    };
    let assertions = legacy
        .assertions
        .into_iter()
        .filter_map(|a| {
            Some(Assertion {
                id: String::new(),
                operator: operator(&a.op)?,
                target: a.target,
                value: a.value,
                origin: Origin::Manual,
                enabled: true,
            })
        })
        .collect();
    let mut request = Request::new(legacy.name);
    request.method = legacy.method;
    request.url = legacy.url;
    request.params = pairs(legacy.params);
    request.headers = pairs(legacy.headers);
    request.auth = auth;
    request.body = body;
    // Le délai par défaut du format précédent est celui d'Arnod : hérité.
    request.settings = Settings {
        timeout_ms: legacy.timeout_ms.filter(|ms| *ms != DEFAULT_TIMEOUT_MS),
        ..Settings::default()
    };
    request.assertions = assertions;
    request.contract = legacy.contract;
    request
}

fn convert_requests(dir: &Path, top: bool) -> Result<(), CommandError> {
    let entries =
        fs::read_dir(dir).map_err(|error| CommandError::io("Lecture impossible", dir, &error))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if top && (name == ENVIRONMENTS_DIR || name == CONTRACTS_DIR) {
                continue;
            }
            convert_requests(&path, false)?;
            continue;
        }
        let is_yaml = path
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml");
        if !is_yaml || (top && (name == LEGACY_MANIFEST || name == ROOT_FILE)) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        // Un fichier qui n'est pas une requête du format précédent est laissé tel quel.
        let Ok(legacy) = serde_saphyr::from_str::<LegacyRequest>(&text) else {
            continue;
        };
        // Écrit de zéro : le fichier précédent n'est pas une base OpenCollection.
        let _ = fs::remove_file(&path);
        request_file::write(&path, &convert(legacy))?;
    }
    Ok(())
}

/// Convertit le projet de `folder` s'il est au format précédent ; renvoie
/// `true` s'il l'a été. Un projet déjà OpenCollection n'est pas touché.
pub fn run(folder: &Path) -> Result<bool, CommandError> {
    let legacy_path = folder.join(LEGACY_MANIFEST);
    if !legacy_path.is_file() || collection::exists(folder) {
        return Ok(false);
    }
    let text = fs::read_to_string(&legacy_path).map_err(|error| {
        CommandError::io("Lecture du manifeste impossible", &legacy_path, &error)
    })?;
    let manifest: LegacyManifest = serde_saphyr::from_str(&text).map_err(|error| {
        CommandError::new(
            CommandErrorKind::NotAProject,
            format!(
                "Le manifeste {} est illisible : {error}",
                legacy_path.display()
            ),
        )
    })?;
    if manifest.version > 1 {
        return Err(CommandError::new(
            CommandErrorKind::NewerVersion,
            format!(
                "Le projet {} a été créé par une version plus récente d'Arnod.",
                folder.display()
            ),
        ));
    }

    let env_dir = folder.join(ENVIRONMENTS_DIR);
    let order = fs::read_to_string(env_dir.join(LEGACY_ORDER))
        .ok()
        .and_then(|text| serde_saphyr::from_str::<LegacyOrder>(&text).ok())
        .map(|o| o.order)
        .unwrap_or_default();
    let mut environments = Vec::new();
    for entry in fs::read_dir(&env_dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if entry.file_name() == LEGACY_ORDER || !path.extension().is_some_and(|ext| ext == "yml") {
            continue;
        }
        if let Some(legacy) = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_saphyr::from_str::<LegacyEnvironment>(&text).ok())
        {
            let _ = fs::remove_file(&path);
            environments.push(Environment {
                name: legacy.name,
                color: legacy.color.unwrap_or_else(|| "gray".to_owned()),
                protected: legacy.protected,
                variables: legacy.variables,
                locals: legacy.locals,
            });
        }
    }

    let mut root = collection::new_root(&manifest.name, manifest.id);
    collection::set_meta(
        &mut root,
        &ArnodMeta {
            id: Some(manifest.id),
            environment_order: order.clone(),
            protected: environments
                .iter()
                .filter(|e| e.protected)
                .map(|e| e.name.clone())
                .collect(),
        },
    );
    collection::write(folder, &root)?;
    for env in &environments {
        environment::save(folder, env, None)?;
    }
    if !order.is_empty() {
        environment::reorder(folder, &order)?;
    }
    convert_requests(folder, true)?;
    let _ = fs::remove_file(env_dir.join(LEGACY_ORDER));
    let _ = fs::remove_file(&legacy_path);
    Ok(true)
}
