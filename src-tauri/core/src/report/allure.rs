//! Résultats Allure (`rapports/allure-results/`, R8) : un fichier `<uuid>-result.json` par cas,
//! au modèle « TestResult » d'Allure 2, et `environment.properties` (projet, environnement,
//! version). Une erreur d'envoi est « broken », comme le veut Allure ; une assertion en échec,
//! « failed ».

use serde_json::{Value, json};
use uuid::Uuid;

use super::{Case, Meta, Verdict, cases, check_label, item_failure, step_status};
use crate::workbench::{ItemStatus, RunItem, RunView};

/// Statut Allure d'un cas.
fn status(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Passed => "passed",
        Verdict::Failed => "failed",
        Verdict::Error => "broken",
        Verdict::Skipped | Verdict::Cancelled => "skipped",
    }
}

fn step_status_of(item: &RunItem) -> &'static str {
    match item.status {
        ItemStatus::Passed => "passed",
        ItemStatus::Failed => "failed",
        ItemStatus::Error => "broken",
        _ => "skipped",
    }
}

/// Empreinte stable d'un texte (FNV-1a 64 bits) : `historyId` d'un cas, le même d'un run à
/// l'autre pour suivre son historique dans Allure.
fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn details(message: Option<String>, trace: Option<String>) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(message) = message {
        out.insert("message".into(), Value::String(message));
    }
    if let Some(trace) = trace {
        out.insert("trace".into(), Value::String(trace));
    }
    Value::Object(out)
}

/// Une étape : la requête, puis ses assertions en sous-étapes.
fn step(item: &RunItem, index: usize, start: u64) -> Value {
    let stop = start + item.duration_ms.unwrap_or(0);
    let mut name = format!(
        "{}. {} {}",
        item.step.unwrap_or(index + 1),
        item.method.as_str(),
        item.name
    );
    if let Some(status) = item.http_status {
        name.push_str(&format!(" · {status}"));
    }
    let checks: Vec<Value> = item
        .assertions
        .iter()
        .map(|check| {
            let state = if check.passed || check.ignored {
                "passed"
            } else {
                "failed"
            };
            json!({
                "name": check_label(check),
                "status": state,
                "stage": "finished",
                "start": stop,
                "stop": stop,
                "statusDetails": details(
                    check.message.clone().filter(|_| !check.passed),
                    check.actual.clone().map(|a| format!("reçu : {a}")),
                ),
                "steps": [],
                "attachments": [],
                "parameters": [],
            })
        })
        .collect();
    json!({
        "name": name,
        "status": step_status_of(item),
        "stage": "finished",
        "start": start,
        "stop": stop,
        "statusDetails": details(item_failure(item), Some(step_status(item.status).to_owned())),
        "steps": checks,
        "attachments": [],
        "parameters": [],
    })
}

fn result(case: &Case<'_>, view: &RunView, meta: &Meta) -> Value {
    let start = view.started_at;
    let mut offset = start;
    let steps: Vec<Value> = case
        .steps
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let value = step(item, index, offset);
            offset += item.duration_ms.unwrap_or(0);
            value
        })
        .collect();
    let full_name = format!("{}/{}", meta.project, case.path);
    let suite = if case.folder.is_empty() {
        meta.project.clone()
    } else {
        case.folder.clone()
    };
    json!({
        "uuid": Uuid::new_v4().to_string(),
        "historyId": fingerprint(&full_name),
        "testCaseId": fingerprint(&case.path),
        "name": case.name,
        "fullName": full_name,
        "status": status(case.verdict),
        "statusDetails": details(case.failure.clone(), None),
        "stage": "finished",
        "start": start,
        "stop": start + case.duration_ms,
        "labels": [
            { "name": "parentSuite", "value": meta.project },
            { "name": "suite", "value": suite },
            { "name": "framework", "value": "arnod" },
            { "name": "host", "value": "arnod" },
        ],
        "parameters": [
            { "name": "environnement", "value": view.environment },
        ],
        "links": [],
        "attachments": [],
        "steps": steps,
    })
}

/// Fichiers du dossier `allure-results` : nom et contenu.
pub fn render(view: &RunView, meta: &Meta) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = cases(view)
        .iter()
        .map(|case| {
            let value = result(case, view, meta);
            let uuid = value["uuid"].as_str().unwrap_or_default().to_owned();
            let text = serde_json::to_string_pretty(&value).unwrap_or_default();
            (format!("{uuid}-result.json"), text)
        })
        .collect();
    files.push(("environment.properties".to_owned(), environment(view, meta)));
    files
}

/// Échappe une valeur de fichier `.properties` (ISO 8859-1 : le reste en `\uXXXX`).
fn property(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '=' | ':' | '#' | '!' => {
                out.push('\\');
                out.push(c);
            }
            c if u32::from(c) < 0x20 => {}
            c if u32::from(c) < 0x7f => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out
}

fn environment(view: &RunView, meta: &Meta) -> String {
    [
        ("Arnod", meta.version.as_str()),
        ("Projet", meta.project.as_str()),
        ("Environnement", view.environment.as_str()),
        ("Cible", meta.target.as_str()),
    ]
    .iter()
    .map(|(key, value)| format!("{key}={}\n", property(value)))
    .collect()
}
