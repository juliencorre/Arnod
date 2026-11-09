//! Résolution des `{{variables}}` et préparation de l'envoi (R6, R9).

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::error::{CommandError, CommandErrorKind};
use crate::request::model::{ApiKeyLocation, AuthKind, BodyKind, Method, Pair, Request};

/// Texte affiché à la place d'un secret (R9).
pub const MASK: &str = "••••";

/// Caractères conservés tels quels dans un paramètre d'adresse.
const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Variables de l'environnement actif, locales comprises (R6).
#[derive(Debug, Clone, Default)]
pub struct Variables {
    /// Valeur de chaque variable.
    pub values: BTreeMap<String, String>,
    /// Noms des variables secrètes (R9).
    pub secrets: BTreeSet<String>,
    /// Noms de l'environnement, ou du projet, utilisés dans les messages.
    pub environment: String,
}

impl Variables {
    /// Valeurs secrètes non vides, à masquer partout (R9).
    fn secret_values(&self) -> Vec<&str> {
        self.secrets
            .iter()
            .filter_map(|name| self.values.get(name))
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .collect()
    }

    /// Remplace chaque valeur secrète de `text` par `••••` (R9).
    pub fn mask(&self, text: &str) -> String {
        self.secret_values()
            .into_iter()
            .fold(text.to_owned(), |text, secret| text.replace(secret, MASK))
    }
}

/// Requête prête à partir : variables résolues, authentification appliquée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

/// Remplace `{{nom}}` ; les noms inconnus sont ajoutés à `unknown`.
fn substitute(text: &str, variables: &Variables, unknown: &mut BTreeSet<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = after[..end].trim();
        match variables.values.get(name) {
            Some(value) => out.push_str(value),
            None => {
                unknown.insert(name.to_owned());
                out.push_str(&rest[start..start + 2 + end + 2]);
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// Noms des variables `{{…}}` d'un texte (B6).
pub fn variable_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            break;
        };
        names.push(after[..end].trim().to_owned());
        rest = &after[end + 2..];
    }
    names
}

fn encode(text: &str) -> String {
    utf8_percent_encode(text, UNRESERVED).to_string()
}

fn active(pairs: &[Pair]) -> impl Iterator<Item = &Pair> {
    pairs
        .iter()
        .filter(|pair| pair.enabled && !pair.name.trim().is_empty())
}

fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(header, _)| header.eq_ignore_ascii_case(name))
}

/// Prépare l'envoi (R6) : variables, paramètres, authentification, corps.
///
/// Une variable inconnue bloque l'envoi et est nommée dans le message.
pub fn prepare(request: &Request, variables: &Variables) -> Result<Prepared, CommandError> {
    let mut unknown = BTreeSet::new();
    let mut sub = |text: &str| substitute(text, variables, &mut unknown);

    let mut url = sub(&request.url);
    let mut query: Vec<String> = active(&request.params)
        .map(|pair| format!("{}={}", encode(&sub(&pair.name)), encode(&sub(&pair.value))))
        .collect();

    let mut headers: Vec<(String, String)> = active(&request.headers)
        .map(|pair| (sub(&pair.name), sub(&pair.value)))
        .collect();

    let auth = &request.auth;
    match auth.kind {
        AuthKind::None => {}
        AuthKind::Basic => {
            let credentials = format!("{}:{}", sub(&auth.username), sub(&auth.password));
            headers.push((
                "Authorization".to_owned(),
                format!("Basic {}", STANDARD.encode(credentials)),
            ));
        }
        AuthKind::Bearer => {
            headers.push((
                "Authorization".to_owned(),
                format!("Bearer {}", sub(&auth.token)),
            ));
        }
        AuthKind::ApiKey if !auth.name.trim().is_empty() => match auth.location {
            ApiKeyLocation::Header => headers.push((sub(&auth.name), sub(&auth.value))),
            ApiKeyLocation::Query => {
                query.push(format!(
                    "{}={}",
                    encode(&sub(&auth.name)),
                    encode(&sub(&auth.value))
                ));
            }
        },
        AuthKind::ApiKey => {}
    }

    let (body, content_type) = match request.body.kind {
        BodyKind::None => (None, None),
        BodyKind::Json => (Some(sub(&request.body.content)), Some("application/json")),
        BodyKind::Text => (
            Some(sub(&request.body.content)),
            Some("text/plain; charset=utf-8"),
        ),
        BodyKind::Form => {
            let fields: Vec<String> = active(&request.body.form)
                .map(|pair| format!("{}={}", encode(&sub(&pair.name)), encode(&sub(&pair.value))))
                .collect();
            (
                Some(fields.join("&")),
                Some("application/x-www-form-urlencoded"),
            )
        }
    };
    if let Some(content_type) = content_type
        && !has_header(&headers, "Content-Type")
    {
        headers.push(("Content-Type".to_owned(), content_type.to_owned()));
    }

    if !unknown.is_empty() {
        let names: Vec<String> = unknown.iter().map(|name| format!("« {name} »")).collect();
        return Err(CommandError::new(
            CommandErrorKind::UnknownVariable,
            format!(
                "{} absente de l'environnement {}.",
                if names.len() == 1 {
                    format!("Variable {}", names[0])
                } else {
                    format!("Variables {}", names.join(", "))
                },
                variables.environment
            ),
        ));
    }

    if !query.is_empty() {
        let separator = if url.contains('?') { '&' } else { '?' };
        url.push(separator);
        url.push_str(&query.join("&"));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "L'adresse « {} » doit commencer par http:// ou https://. Renseignez la variable baseUrl de l'environnement ou écrivez l'adresse complète.",
                variables.mask(&url)
            ),
        ));
    }

    Ok(Prepared {
        method: request.method,
        url,
        headers,
        body,
        timeout_ms: request.timeout_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::model::*;

    fn vars(pairs: &[(&str, &str)], secrets: &[&str]) -> Variables {
        Variables {
            values: pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            secrets: secrets.iter().map(|s| (*s).to_owned()).collect(),
            environment: "Recette".to_owned(),
        }
    }

    fn request(url: &str) -> Request {
        let mut request = Request::new("t");
        request.url = url.to_owned();
        request
    }

    #[test]
    fn remplace_les_variables_avec_ou_sans_espaces() {
        let v = vars(&[("baseUrl", "https://api.test"), ("id", "42")], &[]);
        let prepared = prepare(&request("{{ baseUrl }}/orders/{{id}}"), &v).unwrap();
        assert_eq!(prepared.url, "https://api.test/orders/42");
    }

    #[test]
    fn variable_inconnue_bloque_et_est_nommee() {
        let v = vars(&[("baseUrl", "https://api.test")], &[]);
        let mut r = request("{{baseUrl}}/x");
        r.auth = Auth {
            kind: AuthKind::Bearer,
            token: "{{token}}".to_owned(),
            ..Auth::default()
        };
        let error = prepare(&r, &v).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::UnknownVariable);
        assert!(error.message.contains("« token »"), "{}", error.message);
        assert!(error.message.contains("Recette"));
    }

    #[test]
    fn parametres_actifs_encodes() {
        let v = vars(&[("baseUrl", "https://api.test")], &[]);
        let mut r = request("{{baseUrl}}/s");
        r.params = vec![
            Pair {
                name: "q".into(),
                value: "a b&c".into(),
                enabled: true,
            },
            Pair {
                name: "off".into(),
                value: "x".into(),
                enabled: false,
            },
        ];
        assert_eq!(
            prepare(&r, &v).unwrap().url,
            "https://api.test/s?q=a%20b%26c"
        );
        r.url = "{{baseUrl}}/s?x=1".into();
        assert_eq!(
            prepare(&r, &v).unwrap().url,
            "https://api.test/s?x=1&q=a%20b%26c"
        );
    }

    #[test]
    fn authentification_basic_bearer_et_cle_dapi() {
        let v = vars(&[("baseUrl", "https://api.test")], &[]);
        let mut r = request("{{baseUrl}}/x");
        r.auth = Auth {
            kind: AuthKind::Basic,
            username: "a".into(),
            password: "b".into(),
            ..Auth::default()
        };
        let p = prepare(&r, &v).unwrap();
        assert!(
            p.headers
                .contains(&("Authorization".into(), "Basic YTpi".into()))
        );
        r.auth = Auth {
            kind: AuthKind::Bearer,
            token: "tok".into(),
            ..Auth::default()
        };
        assert!(
            prepare(&r, &v)
                .unwrap()
                .headers
                .contains(&("Authorization".into(), "Bearer tok".into()))
        );
        r.auth = Auth {
            kind: AuthKind::ApiKey,
            name: "X-Key".into(),
            value: "k".into(),
            ..Auth::default()
        };
        assert!(
            prepare(&r, &v)
                .unwrap()
                .headers
                .contains(&("X-Key".into(), "k".into()))
        );
        r.auth.location = ApiKeyLocation::Query;
        assert_eq!(prepare(&r, &v).unwrap().url, "https://api.test/x?X-Key=k");
    }

    #[test]
    fn corps_et_content_type() {
        let v = vars(&[("baseUrl", "https://api.test")], &[]);
        let mut r = request("{{baseUrl}}/x");
        r.method = Method::Post;
        r.body = Body {
            kind: BodyKind::Json,
            content: "{}".into(),
            form: vec![],
        };
        let p = prepare(&r, &v).unwrap();
        assert_eq!(p.body.as_deref(), Some("{}"));
        assert!(
            p.headers
                .contains(&("Content-Type".into(), "application/json".into()))
        );
        r.headers.push(Pair {
            name: "content-type".into(),
            value: "application/vnd.x+json".into(),
            enabled: true,
        });
        let p = prepare(&r, &v).unwrap();
        assert_eq!(
            p.headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .count(),
            1
        );
    }

    #[test]
    fn adresse_sans_schema_refusee() {
        let v = vars(&[("baseUrl", "")], &[]);
        let error = prepare(&request("{{baseUrl}}/x"), &v).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    }

    #[test]
    fn les_secrets_sont_masques() {
        let v = vars(&[("token", "s3cret")], &["token"]);
        assert_eq!(v.mask("Bearer s3cret et s3cret"), "Bearer •••• et ••••");
        assert_eq!(v.mask("rien"), "rien");
    }

    #[test]
    fn liste_les_noms_de_variables() {
        assert_eq!(variable_names("{{a}}/x/{{ b }}"), ["a", "b"]);
    }
}
