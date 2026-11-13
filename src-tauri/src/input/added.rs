//! En-têtes ajoutés par Arnod à l'envoi (5.1-b ter, R3) : ceux de
//! l'authentification et du corps (`request::resolve::prepare`), puis ceux du
//! client HTTP. Un en-tête du même nom saisi dans la requête ou hérité remplace
//! celui du corps ou du client ; celui de l'authentification s'y ajoute.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::http::USER_AGENT;
use crate::request::model::{ApiKeyLocation, AuthKind, BodyKind, Request, TokenPlacement};
use crate::request::resolve::{MASK, variable_names};

/// D'où vient un en-tête ajouté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AddedOrigin {
    /// Onglet Auth, ou authentification héritée.
    Auth,
    /// Type du corps.
    Body,
    /// Client HTTP.
    Client,
}

/// En-tête ajouté à l'envoi, tel que l'onglet En-têtes le montre, grisé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedHeader {
    pub name: String,
    /// Valeur, secret masqué, ou ce qui la donne (« calculé à l'envoi »).
    pub value: String,
    pub origin: AddedOrigin,
    /// Dossier ou collection d'une authentification héritée.
    pub source: Option<String>,
}

/// Remplace les variables connues de `values`, laisse les autres.
pub fn fill(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = text.to_owned();
    for name in variable_names(text) {
        if let Some(value) = values.get(&name) {
            out = out.replace(&format!("{{{{{name}}}}}"), value);
            out = out.replace(&format!("{{{{ {name} }}}}"), value);
        }
    }
    out
}

/// Hôte (et port s'il n'est pas celui du schéma) de l'adresse résolue.
fn host(url: &str, values: &BTreeMap<String, String>) -> String {
    reqwest::Url::parse(fill(url, values).trim())
        .ok()
        .and_then(|url| {
            let host = url.host_str()?.to_owned();
            Some(match url.port() {
                Some(port) => format!("{host}:{port}"),
                None => host,
            })
        })
        .unwrap_or_else(|| "d'après l'adresse".to_owned())
}

/// En-têtes ajoutés à l'envoi de `request` (R3). `values` : variables non
/// secrètes de l'environnement actif, pour nommer l'hôte.
pub fn added_headers(request: &Request, values: &BTreeMap<String, String>) -> Vec<AddedHeader> {
    let written: BTreeSet<String> = request
        .headers
        .iter()
        .filter(|h| h.enabled && !h.name.trim().is_empty())
        .map(|h| h.name.trim().to_ascii_lowercase())
        .chain(
            request
                .inherited
                .headers
                .iter()
                .map(|h| h.name.trim().to_ascii_lowercase()),
        )
        .collect();
    let mut added = Vec::new();

    let (auth, source) = match request.auth.kind {
        AuthKind::Inherit => request
            .inherited
            .auth
            .as_ref()
            .map_or((&request.auth, None), |inherited| {
                (&inherited.auth, Some(inherited.source.clone()))
            }),
        _ => (&request.auth, None),
    };
    let header = match auth.kind {
        AuthKind::Basic => Some(("Authorization".to_owned(), format!("Basic {MASK}"))),
        AuthKind::Bearer => Some(("Authorization".to_owned(), format!("Bearer {MASK}"))),
        // Lot H : jeton OAuth 2 obtenu à l'envoi (R11).
        AuthKind::OAuth2 if auth.oauth2.token_placement == TokenPlacement::Header => {
            let name = auth.oauth2.token_name.trim();
            Some(
                if name.is_empty() || name.eq_ignore_ascii_case("authorization") {
                    ("Authorization".to_owned(), format!("Bearer {MASK}"))
                } else {
                    (name.to_owned(), MASK.to_owned())
                },
            )
        }
        AuthKind::ApiKey
            if auth.location == ApiKeyLocation::Header && !auth.name.trim().is_empty() =>
        {
            Some((fill(&auth.name, values), MASK.to_owned()))
        }
        _ => None,
    };
    if let Some((name, value)) = header {
        added.push(AddedHeader {
            name,
            value,
            origin: AddedOrigin::Auth,
            source,
        });
    }

    let content_type = match request.body.kind {
        BodyKind::Json => Some("application/json"),
        BodyKind::Xml => Some("application/xml"),
        BodyKind::Text => Some("text/plain; charset=utf-8"),
        BodyKind::Form => Some("application/x-www-form-urlencoded"),
        BodyKind::None | BodyKind::Other => None,
    };
    let mut push = |name: &str, value: &str, origin: AddedOrigin| {
        if !written.contains(&name.to_ascii_lowercase()) {
            added.push(AddedHeader {
                name: name.to_owned(),
                value: value.to_owned(),
                origin,
                source: None,
            });
        }
    };
    if let Some(content_type) = content_type {
        push("Content-Type", content_type, AddedOrigin::Body);
    }
    push("User-Agent", USER_AGENT, AddedOrigin::Client);
    push("Accept", "*/*", AddedOrigin::Client);
    if content_type.is_some() {
        push("Content-Length", "calculé à l'envoi", AddedOrigin::Client);
    }
    push("Host", &host(&request.url, values), AddedOrigin::Client);
    added
}
