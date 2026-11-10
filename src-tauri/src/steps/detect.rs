//! Reconnaissance de ce que l'on colle dans l'entrée de l'accueil (R2) : règles
//! déclaratives des définitions, évaluées ici, sans code venu du catalogue.

use std::collections::BTreeMap;

use serde::Serialize;

use super::definition::{Definition, Rule};

/// Nature reconnue, avec le champ principal et les valeurs lues au passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub nature: String,
    pub subject: String,
    pub values: BTreeMap<String, String>,
}

/// Une règle satisfaite et sa précision : la plus précise l'emporte.
struct Hit {
    rank: u8,
    subject: String,
    values: BTreeMap<String, String>,
}

/// Adresse `scheme://hôte…` avec un hôte.
pub fn is_url(text: &str, schemes: &[String]) -> bool {
    let Ok(url) = reqwest::Url::parse(text) else {
        return false;
    };
    schemes.iter().any(|s| s.eq_ignore_ascii_case(url.scheme()))
        && url.host_str().is_some_and(|host| !host.is_empty())
        && text
            .split_once("://")
            .is_some_and(|(_, rest)| !rest.is_empty() && !rest.starts_with('/'))
}

fn evaluate(definition: &Definition, rule: &Rule, input: &str) -> Option<Hit> {
    match rule {
        Rule::Url {
            schemes,
            method_field,
        } => {
            let mut values = BTreeMap::new();
            let mut subject = input;
            if let Some(key) = method_field
                && let Some((first, rest)) = input.split_once(char::is_whitespace)
            {
                let options = definition.field(key).map(|f| &f.options);
                if let Some(option) = options
                    .into_iter()
                    .flatten()
                    .find(|o| o.eq_ignore_ascii_case(first))
                {
                    values.insert(key.clone(), option.clone());
                    subject = rest.trim();
                }
            }
            is_url(subject, schemes).then(|| Hit {
                rank: 1,
                subject: subject.to_owned(),
                values,
            })
        }
        Rule::Keywords { words } => {
            let first = input.split_whitespace().next()?;
            words
                .iter()
                .any(|w| w.eq_ignore_ascii_case(first))
                .then(|| Hit {
                    rank: 3,
                    subject: input.to_owned(),
                    values: BTreeMap::new(),
                })
        }
        Rule::Prefix { text } => {
            input
                .to_lowercase()
                .starts_with(&text.to_lowercase())
                .then(|| Hit {
                    rank: 2,
                    subject: input.to_owned(),
                    values: BTreeMap::new(),
                })
        }
        Rule::Suffix { text } => input
            .to_lowercase()
            .ends_with(&text.to_lowercase())
            .then(|| Hit {
                rank: 2,
                subject: input.to_owned(),
                values: BTreeMap::new(),
            }),
    }
}

/// Nature de `input` parmi `definitions` (dans l'ordre du catalogue : intégrées
/// d'abord). À précision égale, la première l'emporte.
pub fn detect(definitions: &[Definition], input: &str) -> Option<Detection> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let mut best: Option<(u8, Detection)> = None;
    for definition in definitions {
        for rule in &definition.detect {
            let Some(hit) = evaluate(definition, rule, input) else {
                continue;
            };
            if best.as_ref().is_none_or(|(rank, _)| hit.rank > *rank) {
                best = Some((
                    hit.rank,
                    Detection {
                        nature: definition.id.clone(),
                        subject: hit.subject,
                        values: hit.values,
                    },
                ));
            }
        }
    }
    best.map(|(_, detection)| detection)
}
