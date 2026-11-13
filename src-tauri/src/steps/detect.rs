//! Reconnaissance de ce que l'on colle dans l'entrée de l'accueil (R2) : règles
//! déclaratives des définitions, évaluées ici, sans code venu du catalogue.

use std::collections::BTreeMap;

use serde::Serialize;

use super::definition::{Definition, Rule};
use crate::curl::parse::{self as curl, Ignored, Secret};
use crate::input::{PairsFormat, to_text};
use crate::request::model::Pair;

/// Ce qu'une commande cURL collée a donné en plus des valeurs du formulaire
/// (`docs/plan-mvp-b-saisie-curl.md`, R5, R6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlImport {
    /// Secrets retirés des en-têtes, remplacés par `{{nom}}` : gardés au coffre
    /// à l'envoi de l'essai, jamais écrits dans le projet.
    pub secrets: Vec<Secret>,
    /// Options sans équivalent, signalées.
    pub ignored: Vec<Ignored>,
}

/// Nature reconnue, avec le champ principal et les valeurs lues au passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub nature: String,
    pub subject: String,
    pub values: BTreeMap<String, String>,
    /// Commande cURL reconnue (règle « curl »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curl: Option<CurlImport>,
}

/// Une règle satisfaite et sa précision : la plus précise l'emporte.
struct Hit {
    rank: u8,
    subject: String,
    values: BTreeMap<String, String>,
    curl: Option<CurlImport>,
}

/// Type du corps du formulaire REST d'après le `Content-Type`.
fn body_type(headers: &[(String, String)]) -> &'static str {
    let content_type = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Content-Type"))
        .map(|(_, value)| value.to_ascii_lowercase())
        .unwrap_or_default();
    if content_type.contains("json") {
        "JSON"
    } else if content_type.contains("xml") {
        "XML"
    } else {
        "Texte"
    }
}

/// Règle « curl » : la commande lue par le cœur, ses secrets retirés (R4, R5).
fn curl_hit(input: &str) -> Option<Hit> {
    if !curl::is_curl(input) {
        return None;
    }
    let mut request = curl::parse(input).ok()?;
    let secrets = curl::take_secrets(&mut request.headers);
    let pairs: Vec<Pair> = request
        .headers
        .iter()
        .map(|(name, value)| Pair::new(name.clone(), value.clone()))
        .collect();
    let mut values = BTreeMap::from([("method".to_owned(), request.method.clone())]);
    if !pairs.is_empty() {
        values.insert("headers".to_owned(), to_text(&pairs, PairsFormat::Header));
    }
    if let Some(body) = request.body {
        values.insert(
            "bodyType".to_owned(),
            body_type(&request.headers).to_owned(),
        );
        values.insert("body".to_owned(), body);
    }
    Some(Hit {
        rank: 4,
        subject: request.url,
        values,
        curl: Some(CurlImport {
            secrets,
            ignored: request.ignored,
        }),
    })
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
                curl: None,
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
                    curl: None,
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
                    curl: None,
                })
        }
        Rule::Suffix { text } => input
            .to_lowercase()
            .ends_with(&text.to_lowercase())
            .then(|| Hit {
                rank: 2,
                subject: input.to_owned(),
                values: BTreeMap::new(),
                curl: None,
            }),
        Rule::Curl => curl_hit(input),
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
                        curl: hit.curl,
                    },
                ));
            }
        }
    }
    best.map(|(_, detection)| detection)
}
