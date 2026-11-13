//! Jetons OAuth 2 par identifiants client (`docs/plan-mvp-h-reseau-auth.md`, R8 à R11).
//!
//! Module pur : la réponse du serveur de jetons, le cache et le placement du jeton. La demande
//! elle-même part par le client de `network`, avec le proxy et les autorités du poste.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::request::model::TokenPlacement;
use crate::request::resolve::{OAuth2Grant, Prepared};

/// Marge de renouvellement : un jeton qui expire dans moins d'une minute est redemandé (R9).
pub const REFRESH_MARGIN_MS: u64 = 60_000;

/// Jeton obtenu, gardé en mémoire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub access_token: String,
    pub obtained_at: u64,
    /// Expiration, en ms depuis l'époque Unix ; `None` si le serveur ne la donne pas.
    pub expires_at: Option<u64>,
}

impl Token {
    /// Encore utilisable à `now` (R9).
    pub fn is_fresh(&self, now: u64) -> bool {
        self.expires_at
            .is_none_or(|expires| now.saturating_add(REFRESH_MARGIN_MS) < expires)
    }
}

/// État du jeton d'un environnement, pour l'interface (3.4-a) : jamais sa valeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStatus {
    pub obtained_at: u64,
    pub expires_at: Option<u64>,
}

/// Clé d'un jeton : projet, environnement, serveur, client et portée.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenKey {
    pub project: Uuid,
    pub environment: String,
    pub token_url: String,
    pub client_id: String,
    pub scope: String,
}

impl TokenKey {
    pub fn of(project: Uuid, environment: &str, grant: &OAuth2Grant) -> Self {
        Self {
            project,
            environment: environment.to_owned(),
            token_url: grant.token_url.clone(),
            client_id: grant.client_id.clone(),
            scope: grant.scope.clone(),
        }
    }
}

/// Jetons de la session.
#[derive(Debug, Default)]
pub struct TokenCache {
    tokens: HashMap<TokenKey, Token>,
}

impl TokenCache {
    /// Jeton encore frais pour `key` (R9).
    pub fn fresh(&self, key: &TokenKey, now: u64) -> Option<&Token> {
        self.tokens.get(key).filter(|t| t.is_fresh(now))
    }

    pub fn put(&mut self, key: TokenKey, token: Token) {
        self.tokens.insert(key, token);
    }

    /// Oublie les jetons d'un environnement du projet (« Oublier le jeton »).
    pub fn forget(&mut self, project: Uuid, environment: &str) {
        self.tokens
            .retain(|k, _| !(k.project == project && k.environment == environment));
    }

    /// Dernier jeton obtenu pour un environnement du projet.
    pub fn status(&self, project: Uuid, environment: &str) -> Option<TokenStatus> {
        self.tokens
            .iter()
            .filter(|(k, _)| k.project == project && k.environment == environment)
            .map(|(_, t)| TokenStatus {
                obtained_at: t.obtained_at,
                expires_at: t.expires_at,
            })
            .max_by_key(|s| s.obtained_at)
    }
}

/// Lit la réponse du serveur de jetons (RFC 6749, § 5.1 et 5.2).
pub fn parse_response(status: u16, body: &str, now: u64) -> Result<Token, String> {
    let json: Option<Value> = serde_json::from_str(body).ok();
    if !(200..300).contains(&status) {
        let reason = json
            .as_ref()
            .and_then(|j| {
                let error = j.get("error").and_then(Value::as_str)?;
                Some(
                    j.get("error_description")
                        .and_then(Value::as_str)
                        .map_or_else(|| error.to_owned(), |d| format!("{error} : {d}")),
                )
            })
            .unwrap_or_else(|| format!("statut {status}"));
        return Err(reason);
    }
    let json = json.ok_or_else(|| "la réponse n'est pas du JSON".to_owned())?;
    let access_token = json
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "la réponse ne contient pas de champ access_token".to_owned())?;
    let expires_in = json.get("expires_in").and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    });
    Ok(Token {
        access_token: access_token.to_owned(),
        obtained_at: now,
        expires_at: expires_in.map(|s| now.saturating_add(s.saturating_mul(1000))),
    })
}

/// Formulaire de la demande de jeton, identifiants dans le corps ou non.
pub fn form(grant: &OAuth2Grant, credentials_in_body: bool) -> Vec<(String, String)> {
    let mut form = vec![("grant_type".to_owned(), "client_credentials".to_owned())];
    if !grant.scope.is_empty() {
        form.push(("scope".to_owned(), grant.scope.clone()));
    }
    if credentials_in_body {
        form.push(("client_id".to_owned(), grant.client_id.clone()));
        form.push(("client_secret".to_owned(), grant.client_secret.clone()));
    }
    form
}

/// Place le jeton dans la requête (R11) et le note comme valeur à masquer.
pub fn apply(prepared: &mut Prepared, grant: &OAuth2Grant, token: &str) {
    match grant.placement {
        TokenPlacement::Header => {
            let name = if grant.token_name.is_empty() {
                "Authorization"
            } else {
                grant.token_name.as_str()
            };
            let value = if name.eq_ignore_ascii_case("authorization") {
                format!("Bearer {token}")
            } else {
                token.to_owned()
            };
            prepared
                .headers
                .retain(|(n, _)| !n.eq_ignore_ascii_case(name));
            prepared.headers.push((name.to_owned(), value));
        }
        TokenPlacement::Query => {
            let name = if grant.token_name.is_empty() {
                "access_token"
            } else {
                grant.token_name.as_str()
            };
            let encoded: String = url_encode(token);
            let separator = if prepared.url.contains('?') { '&' } else { '?' };
            prepared
                .url
                .push_str(&format!("{separator}{}={encoded}", url_encode(name)));
        }
    }
    prepared.hidden.push(token.to_owned());
}

/// Caractères gardés tels quels dans un paramètre d'adresse (RFC 3986, non réservés).
const UNRESERVED: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub fn url_encode(text: &str) -> String {
    percent_encoding::utf8_percent_encode(text, UNRESERVED).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::model::{CredentialsPlacement, Method};

    fn grant(placement: TokenPlacement, name: &str) -> OAuth2Grant {
        OAuth2Grant {
            token_url: "https://auth.test/token".into(),
            client_id: "boutique".into(),
            client_secret: "s3cr3t".into(),
            scope: "orders:read".into(),
            credentials: CredentialsPlacement::BasicAuthHeader,
            placement,
            token_name: name.into(),
        }
    }

    fn prepared() -> Prepared {
        Prepared {
            method: Method::Get,
            url: "https://api.test/orders".into(),
            headers: vec![("Accept".into(), "application/json".into())],
            body: None,
            timeout_ms: None,
            max_redirects: 0,
            drawn: Vec::new(),
            used: Vec::new(),
            oauth2: None,
            hidden: Vec::new(),
        }
    }

    #[test]
    fn r9_reponse_lue_et_expiration() {
        let token = parse_response(
            200,
            r#"{"access_token":"abc","token_type":"Bearer","expires_in":3600}"#,
            1_000,
        )
        .unwrap();
        assert_eq!(token.access_token, "abc");
        assert_eq!(token.expires_at, Some(3_601_000));
        assert!(token.is_fresh(1_000));
        // Une minute avant l'expiration, il est redemandé.
        assert!(!token.is_fresh(3_601_000 - REFRESH_MARGIN_MS));
        // Sans expiration : gardé.
        let forever = parse_response(200, r#"{"access_token":"x"}"#, 0).unwrap();
        assert!(forever.is_fresh(u64::MAX - 1));
    }

    #[test]
    fn r9_refus_du_serveur_explique() {
        let error = parse_response(
            401,
            r#"{"error":"invalid_client","error_description":"secret inconnu"}"#,
            0,
        )
        .unwrap_err();
        assert_eq!(error, "invalid_client : secret inconnu");
        assert_eq!(parse_response(500, "oups", 0).unwrap_err(), "statut 500");
        assert!(
            parse_response(200, "{}", 0)
                .unwrap_err()
                .contains("access_token")
        );
    }

    #[test]
    fn r9_cache_par_environnement() {
        let project = Uuid::new_v4();
        let g = grant(TokenPlacement::Header, "Authorization");
        let mut cache = TokenCache::default();
        let recette = TokenKey::of(project, "Recette", &g);
        cache.put(
            recette.clone(),
            Token {
                access_token: "a".into(),
                obtained_at: 10,
                expires_at: Some(10_000_000),
            },
        );
        assert!(cache.fresh(&recette, 20).is_some());
        assert!(
            cache
                .fresh(&TokenKey::of(project, "Local", &g), 20)
                .is_none()
        );
        assert_eq!(cache.status(project, "Recette").unwrap().obtained_at, 10);
        cache.forget(project, "Recette");
        assert!(cache.status(project, "Recette").is_none());
    }

    #[test]
    fn r11_placement_du_jeton() {
        let mut p = prepared();
        apply(
            &mut p,
            &grant(TokenPlacement::Header, "Authorization"),
            "t0k",
        );
        assert!(
            p.headers
                .contains(&("Authorization".into(), "Bearer t0k".into()))
        );
        assert_eq!(p.hidden, ["t0k"]);

        let mut p = prepared();
        apply(&mut p, &grant(TokenPlacement::Header, "X-Token"), "t0k");
        assert!(p.headers.contains(&("X-Token".into(), "t0k".into())));

        let mut p = prepared();
        apply(&mut p, &grant(TokenPlacement::Query, "access_token"), "a b");
        assert_eq!(p.url, "https://api.test/orders?access_token=a%20b");
    }

    #[test]
    fn formulaire_identifiants_dans_le_corps_ou_non() {
        let g = grant(TokenPlacement::Header, "");
        assert_eq!(
            form(&g, false),
            [
                ("grant_type".to_owned(), "client_credentials".to_owned()),
                ("scope".to_owned(), "orders:read".to_owned()),
            ]
        );
        assert_eq!(form(&g, true).len(), 4);
    }
}
