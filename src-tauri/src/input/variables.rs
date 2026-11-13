//! Variables connues et inconnues d'une requête (5.1-b ter, R1, R2).

use std::collections::BTreeSet;

use serde::Serialize;

use crate::environment::Environment;
use crate::local::LocalVar;
use crate::request::model::{AuthKind, BodyKind, Pair, Request};
use crate::request::resolve::variable_names;

/// Environnement où une variable inconnue de l'environnement actif est définie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableSource {
    pub environment: String,
    /// Valeur d'une variable partagée ; `None` pour une variable locale.
    pub value: Option<String>,
}

/// Variable utilisée par la requête et absente de l'environnement actif (R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownVariable {
    pub name: String,
    /// Autres environnements qui la définissent, dans l'ordre de la chaîne.
    pub defined_in: Vec<VariableSource>,
}

/// Valeur dynamique (`$guid`, `$timestamp`…) : tirée à l'envoi, jamais inconnue
/// (R1 du plan de finalisation).
pub fn is_dynamic(name: &str) -> bool {
    name.starts_with('$')
}

/// Variables connues dans l'environnement `env` (R1) : partagées, locales
/// renseignées sur ce poste (un secret est au coffre), puis `inherited`
/// (dossiers et collection).
pub fn known_names(env: &Environment, locals: &[LocalVar], inherited: &[String]) -> Vec<String> {
    let mut known: BTreeSet<String> = env.variables.iter().map(|v| v.name.clone()).collect();
    for name in &env.locals {
        let set = locals
            .iter()
            .find(|var| var.name == *name)
            .is_some_and(|var| var.secret || var.value.is_some());
        if set {
            known.insert(name.clone());
        }
    }
    known.extend(inherited.iter().cloned());
    known.into_iter().collect()
}

fn active(pairs: &[Pair]) -> impl Iterator<Item = &Pair> {
    pairs.iter().filter(|pair| pair.enabled)
}

/// Textes d'une requête qui partent à l'envoi : adresse, paramètres et en-têtes
/// actifs, hérités compris, authentification, corps sélectionné.
fn sent_texts(request: &Request) -> Vec<&str> {
    let mut texts: Vec<&str> = vec![&request.url];
    for pair in active(&request.params).chain(active(&request.headers)) {
        texts.push(&pair.name);
        texts.push(&pair.value);
    }
    for header in &request.inherited.headers {
        texts.push(&header.name);
        texts.push(&header.value);
    }
    let auth = match request.auth.kind {
        AuthKind::Inherit => request
            .inherited
            .auth
            .as_ref()
            .map_or(&request.auth, |inherited| &inherited.auth),
        _ => &request.auth,
    };
    match auth.kind {
        AuthKind::Basic => texts.extend([auth.username.as_str(), auth.password.as_str()]),
        AuthKind::Bearer => texts.push(&auth.token),
        AuthKind::ApiKey => texts.extend([auth.name.as_str(), auth.value.as_str()]),
        // Lot H : OAuth 2 (docs/plan-mvp-h-reseau-auth.md).
        AuthKind::OAuth2 => texts.extend([
            auth.oauth2.access_token_url.as_str(),
            auth.oauth2.client_id.as_str(),
            auth.oauth2.client_secret.as_str(),
            auth.oauth2.scope.as_str(),
        ]),
        AuthKind::None | AuthKind::Inherit | AuthKind::Other => {}
    }
    match request.body.kind {
        BodyKind::Json | BodyKind::Xml | BodyKind::Text => texts.push(request.body.content()),
        BodyKind::Form => {
            for pair in active(request.body.fields()) {
                texts.push(&pair.name);
                texts.push(&pair.value);
            }
        }
        BodyKind::None | BodyKind::Other => {}
    }
    texts
}

/// Variables `{{…}}` de ce qui part à l'envoi, dans l'ordre de leur première
/// apparition, valeurs dynamiques comprises.
pub fn written_names(request: &Request) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for text in sent_texts(request) {
        for name in variable_names(text) {
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// Variables utilisées par ce qui part à l'envoi, sans les valeurs dynamiques (R1).
pub fn used_names(request: &Request) -> Vec<String> {
    written_names(request)
        .into_iter()
        .filter(|name| !is_dynamic(name))
        .collect()
}

/// Variables utilisées et inconnues (R2), avec les autres environnements de la
/// chaîne qui les définissent.
pub fn unknown(
    used: &[String],
    known: &[String],
    all: &[Environment],
    active: &str,
) -> Vec<UnknownVariable> {
    used.iter()
        .filter(|name| !known.contains(name))
        .map(|name| UnknownVariable {
            name: name.clone(),
            defined_in: all
                .iter()
                .filter(|env| env.name != active)
                .filter_map(|env| {
                    if let Some(var) = env.variables.iter().find(|v| v.name == *name) {
                        Some(VariableSource {
                            environment: env.name.clone(),
                            value: Some(var.value.clone()),
                        })
                    } else {
                        env.locals.contains(name).then(|| VariableSource {
                            environment: env.name.clone(),
                            value: None,
                        })
                    }
                })
                .collect(),
        })
        .collect()
}
