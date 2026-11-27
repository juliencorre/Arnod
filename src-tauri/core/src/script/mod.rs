//! Scripts (`docs/plan-v4.md`, lot C) : l'étape script d'un test (4.3-g, R9) et les scripts Bruno
//! gardés à l'import (R10).
//!
//! JavaScript exécuté par QuickJS (`rquickjs`), compilé avec Arnod : aucun module n'est chargé,
//! rien ne touche au disque ni au réseau ; l'exécution est interrompue après [`TIMEOUT`] et la
//! mémoire bornée à [`MEMORY_LIMIT`]. Les échanges avec le script passent par du JSON : variables
//! et réponse à l'entrée, variables posées, tests et requête modifiée à la sortie.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, Context, Runtime};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::request::assertions::ResponseData;

#[cfg(test)]
mod tests;

/// Durée d'un script au plus (R9).
pub const TIMEOUT: Duration = Duration::from_secs(2);
/// Mémoire d'un script au plus (R9).
pub const MEMORY_LIMIT: usize = 32 * 1024 * 1024;

/// Fonctions communes : `vars`, `response`, `assert`, `console`.
const PRELUDE: &str = include_str!("prelude.js");
/// API de Bruno : `bru`, `req`, `res`, `test`, `expect`.
const BRUNO: &str = include_str!("bruno.js");

/// Résultat d'une étape script (R9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptOutcome {
    pub passed: bool,
    /// Message d'un `assert` faux ou d'une exception, en français.
    pub error: Option<String>,
    /// Variables posées par `vars.set`, en texte.
    pub set: BTreeMap<String, String>,
    /// Lignes écrites par `console.log`.
    pub logs: Vec<String>,
    pub duration_ms: u64,
}

/// Phase d'un script Bruno (R10) : `type` d'un script OpenCollection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    BeforeRequest,
    AfterResponse,
    Tests,
}

impl Phase {
    /// Phase d'un `type` de script ; `hooks` et les autres ne s'exécutent pas.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "before-request" => Some(Self::BeforeRequest),
            "after-response" => Some(Self::AfterResponse),
            "tests" => Some(Self::Tests),
            _ => None,
        }
    }
}

/// Requête vue par un script Bruno avant l'envoi (`req`), qu'il peut modifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Test d'un script Bruno (`test("…", fn)`) : il devient une assertion (R10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptTest {
    pub name: String,
    pub passed: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// Résultat d'un script Bruno.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrunoOutcome {
    /// Variables posées (`bru.setVar`, `bru.setEnvVar`).
    pub set: BTreeMap<String, String>,
    /// Requête modifiée par un script d'avant l'envoi.
    pub request: Option<ScriptRequest>,
    pub tests: Vec<ScriptTest>,
    /// Exception du script, en français.
    pub error: Option<String>,
}

/// Réponse vue par un script : statut, en-têtes, corps lu en JSON s'il en est.
fn response_json(response: Option<&ResponseData>) -> Value {
    let Some(data) = response else {
        return Value::Null;
    };
    let body = serde_json::from_str::<Value>(&data.body)
        .unwrap_or_else(|_| Value::String(data.body.clone()));
    let headers: serde_json::Map<String, Value> = data
        .headers
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), Value::String(value.clone())))
        .collect();
    json!({
        "status": data.status,
        "headers": headers,
        "body": body,
        "text": data.body,
        "duration": data.duration_ms,
    })
}

/// Message d'une erreur du moteur, en français.
fn engine_error(error: impl std::fmt::Display) -> String {
    format!("Le moteur JavaScript n'a pas pu démarrer : {error}")
}

/// Exécute `body` (prélude et code de l'utilisateur) avec `input` (JSON) ; rend le JSON de
/// sortie (`__output()`), ou le message d'une exception.
fn evaluate(preludes: &[&str], body: &str, input: &Value) -> Result<Value, String> {
    let runtime = Runtime::new().map_err(engine_error)?;
    runtime.set_memory_limit(MEMORY_LIMIT);
    runtime.set_max_stack_size(1024 * 1024);
    let started = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || started.elapsed() > TIMEOUT)));
    let context = Context::full(&runtime).map_err(engine_error)?;
    context.with(|ctx| {
        let globals = ctx.globals();
        globals
            .set("__input", input.to_string())
            .map_err(engine_error)?;
        for prelude in preludes {
            ctx.eval::<(), _>(*prelude)
                .catch(&ctx)
                .map_err(|e| engine_error(e.to_string()))?;
        }
        let run = ctx.eval::<(), _>(body).catch(&ctx);
        let output: String = ctx
            .eval::<String, _>("__output()")
            .catch(&ctx)
            .map_err(|e| engine_error(e.to_string()))?;
        let mut output: Value = serde_json::from_str(&output).unwrap_or(Value::Null);
        if let Err(error) = run {
            let message = if started.elapsed() > TIMEOUT {
                format!(
                    "Script interrompu : plus de {} s d'exécution.",
                    TIMEOUT.as_secs()
                )
            } else {
                clean_error(&error.to_string())
            };
            if let Some(map) = output.as_object_mut() {
                map.insert("error".to_owned(), Value::String(message));
            } else {
                return Err(message);
            }
        }
        Ok(output)
    })
}

/// Message d'une exception sans la pile ni le préfixe du moteur.
fn clean_error(text: &str) -> String {
    let first = text.lines().next().unwrap_or(text).trim();
    let first = first
        .strip_prefix("Error: ")
        .or_else(|| first.strip_prefix("Uncaught "))
        .unwrap_or(first);
    if first.is_empty() {
        "Le script a levé une exception.".to_owned()
    } else {
        first.to_owned()
    }
}

/// Le code de l'utilisateur, dans une fonction : `return` y est permis.
fn wrap(code: &str) -> String {
    format!("(function () {{\n{code}\n}})();")
}

fn strings(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(k, v)| {
                    let text = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), text)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Étape script (4.3-g, R9) : `vars` (variables du test, `vars.set`), `response` (réponse de
/// l'étape précédente) et `assert()`.
pub fn run_step(
    code: &str,
    vars: &BTreeMap<String, String>,
    response: Option<&ResponseData>,
) -> ScriptOutcome {
    let started = Instant::now();
    let input = json!({ "vars": vars, "response": response_json(response) });
    let result = evaluate(&[PRELUDE], &wrap(code), &input);
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(output) => {
            let error = output
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned);
            ScriptOutcome {
                passed: error.is_none(),
                error,
                set: strings(output.get("set")),
                logs: output
                    .get("logs")
                    .and_then(Value::as_array)
                    .map(|l| {
                        l.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
                duration_ms,
            }
        }
        Err(error) => ScriptOutcome {
            passed: false,
            error: Some(error),
            duration_ms,
            ..ScriptOutcome::default()
        },
    }
}

/// Script Bruno (R10) : `bru`, `req` (avant l'envoi), `res` (après la réponse), `test` et
/// `expect` (tests).
pub fn run_bruno(
    phase: Phase,
    code: &str,
    vars: &BTreeMap<String, String>,
    request: Option<&ScriptRequest>,
    response: Option<&ResponseData>,
) -> BrunoOutcome {
    let input = json!({
        "vars": vars,
        "phase": phase,
        "request": request,
        "response": response_json(response),
    });
    match evaluate(&[PRELUDE, BRUNO], &wrap(code), &input) {
        Ok(output) => BrunoOutcome {
            set: strings(output.get("set")),
            request: output
                .get("request")
                .filter(|r| !r.is_null())
                .and_then(|r| serde_json::from_value(r.clone()).ok()),
            tests: output
                .get("tests")
                .and_then(|t| serde_json::from_value(t.clone()).ok())
                .unwrap_or_default(),
            error: output
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned),
        },
        Err(error) => BrunoOutcome {
            error: Some(error),
            ..BrunoOutcome::default()
        },
    }
}
