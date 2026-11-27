//! Conditions d'un schéma (`docs/plan-v4.md`, R6) : une cible lue dans la dernière réponse ou
//! dans une variable, comparée comme une assertion. Module pur.

use std::collections::BTreeMap;

use serde_json::Value;

use super::Check;
use crate::request::assertions::{self, ResponseData};
use crate::request::model::{Assertion, Origin};

/// Résultat d'une condition : vraie ou fausse, valeur lue, raison en français.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub holds: bool,
    pub actual: Option<String>,
    pub message: Option<String>,
}

/// Nom de variable d'une cible `vars.nom` ou `{{nom}}`.
pub fn variable_of(target: &str) -> Option<&str> {
    let target = target.trim();
    if let Some(name) = target.strip_prefix("vars.") {
        return Some(name.trim());
    }
    target
        .strip_prefix("{{")
        .and_then(|rest| rest.strip_suffix("}}"))
        .map(str::trim)
}

/// Une variable lue comme un corps JSON : du JSON tel quel, sinon une chaîne.
fn as_body(value: &str) -> String {
    if serde_json::from_str::<Value>(value).is_ok() {
        value.to_owned()
    } else {
        Value::String(value.to_owned()).to_string()
    }
}

/// Évalue `check` sur la dernière réponse (`response`) ou les variables (`vars`).
pub fn evaluate(
    check: &Check,
    response: Option<&ResponseData>,
    vars: &BTreeMap<String, String>,
) -> CheckOutcome {
    let (target, data) = match variable_of(&check.target) {
        Some(name) => {
            let body = vars.get(name).map(|v| as_body(v)).unwrap_or_default();
            (
                "body".to_owned(),
                ResponseData {
                    status: 0,
                    duration_ms: 0,
                    headers: Vec::new(),
                    body,
                },
            )
        }
        None => match response {
            Some(data) => (check.target.trim().to_owned(), data.clone()),
            None => {
                return CheckOutcome {
                    holds: false,
                    actual: None,
                    message: Some(
                        "Aucune réponse avant cette étape : la condition lit une variable (vars.nom) ou suit une requête."
                            .to_owned(),
                    ),
                };
            }
        },
    };
    let assertion = Assertion {
        id: "condition".to_owned(),
        target,
        operator: check.operator,
        value: check.value.clone(),
        origin: Origin::Manual,
        enabled: true,
    };
    let json = serde_json::from_str::<Value>(&data.body).ok();
    let result = assertions::evaluate(&assertion, &data, json.as_ref());
    CheckOutcome {
        holds: result.passed,
        actual: result.actual,
        message: result.message,
    }
}

/// Éléments d'une liste à parcourir (boucle « pour chaque élément ») : `{{nom}}` ou `vars.nom`
/// (variable dont la valeur est un tableau JSON), ou `body.…` de la dernière réponse. Chaque
/// élément est rendu en texte : une chaîne telle quelle, le reste en JSON.
pub fn list_items(
    list: &str,
    response: Option<&ResponseData>,
    vars: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let list = list.trim();
    let value: Option<Value> = match variable_of(list) {
        Some(name) => {
            let Some(text) = vars.get(name) else {
                return Err(format!("La variable « {name} » n'a pas de valeur."));
            };
            serde_json::from_str::<Value>(text).ok()
        }
        None => {
            let Some(data) = response else {
                return Err("Aucune réponse avant la boucle : la liste lit une variable (vars.nom) ou suit une requête.".to_owned());
            };
            assertions::value_at(list, data)
        }
    };
    match value {
        Some(Value::Array(items)) => Ok(items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect()),
        Some(_) => Err(format!("« {list} » n'est pas une liste.")),
        None => Err(format!("« {list} » est absent.")),
    }
}
