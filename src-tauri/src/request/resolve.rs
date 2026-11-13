//! Résolution des `{{variables}}` et préparation de l'envoi (R6, R9).

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CommandError, CommandErrorKind};
use crate::request::dynamic::{self, Dice};
use crate::request::model::{
    ApiKeyLocation, Auth, AuthKind, BodyKind, Method, Pair, ParamKind, Request,
};

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
    /// Valeurs secrètes non vides, à masquer partout (R9), les plus longues
    /// d'abord : un secret contenu dans un autre ne le découpe pas, ce qui en
    /// laisserait la fin lisible.
    fn secret_values(&self) -> Vec<&str> {
        let mut values: Vec<&str> = self
            .secrets
            .iter()
            .filter_map(|name| self.values.get(name))
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .collect();
        values.sort_by_key(|value| std::cmp::Reverse(value.len()));
        values
    }

    /// Remplace chaque valeur secrète de `text` par `••••` (R9).
    pub fn mask(&self, text: &str) -> String {
        self.mask_owned(text.to_owned())
    }

    /// Comme [`Self::mask`], sans copier un texte où rien n'est à masquer
    /// (corps d'une réponse, jusqu'à 5 Mo).
    pub fn mask_owned(&self, mut text: String) -> String {
        for secret in self.secret_values() {
            if text.contains(secret) {
                text = text.replace(secret, MASK);
            }
        }
        text
    }
}

/// Requête prête à partir : variables résolues, authentification appliquée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Délai ; `None` : aucun (R5).
    pub timeout_ms: Option<u64>,
    /// Redirections suivies au plus ; 0 : aucune (R3).
    pub max_redirects: u32,
    /// Valeurs dynamiques tirées, dans l'ordre (`docs/plan-mvp-e-tests-etapes.md`, R9).
    pub drawn: Vec<DrawnValue>,
    /// Variables lues, sans doublon, dans l'ordre de première lecture.
    pub used: Vec<String>,
}

/// Valeur dynamique tirée à un envoi (R9, R10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawnValue {
    /// Nom, avec son `$` (`$randomUUID`).
    pub name: String,
    pub value: String,
    /// Où la valeur est partie, si elle y forme seule une valeur
    /// (`request.body.ref`, `request.headers.X-Ref`) : de quoi la garder (R10).
    pub source: Option<String>,
}

/// Résolution en cours : variables connues, valeurs tirées, noms lus ou inconnus.
struct Resolver<'a> {
    variables: &'a Variables,
    dice: &'a mut Dice,
    unknown: BTreeSet<String>,
    drawn: Vec<DrawnValue>,
    used: Vec<String>,
}

impl Resolver<'_> {
    /// Remplace `{{nom}}` ; une valeur dynamique est tirée à chaque occurrence
    /// (R9), un nom inconnu est ajouté à `unknown`.
    fn sub(&mut self, text: &str) -> String {
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
            let raw = &rest[start..start + 2 + end + 2];
            // Une valeur dynamique donnée par l'appelant (copie cURL : laissée telle quelle) prime.
            if dynamic::is_dynamic(name) && !self.variables.values.contains_key(name) {
                // Inconnue, elle part telle quelle : jamais signalée (R1 du plan d'ensemble).
                match dynamic::draw(name, self.dice) {
                    Some(value) => {
                        out.push_str(&value);
                        self.drawn.push(DrawnValue {
                            name: name.to_owned(),
                            value,
                            source: None,
                        });
                    }
                    None => out.push_str(raw),
                }
            } else {
                match self.variables.values.get(name) {
                    Some(value) => {
                        out.push_str(value);
                        if !self.used.iter().any(|n| n == name) {
                            self.used.push(name.to_owned());
                        }
                    }
                    None => {
                        self.unknown.insert(name.to_owned());
                        out.push_str(raw);
                    }
                }
            }
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        out
    }
}

/// `{{$nom}}` seul dans un texte, espaces compris.
fn lone_placeholder(text: &str) -> Option<&str> {
    let inner = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    (dynamic::is_dynamic(inner) && !inner.contains("{{")).then_some(inner)
}

/// Met entre guillemets un `{{…}}` écrit hors d'une chaîne JSON (`"n": {{$randomInt}}`),
/// pour lire le modèle du corps comme du JSON.
fn quote_bare(template: &str) -> String {
    let mut out = String::with_capacity(template.len() + 8);
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let before = &rest[..start];
        let Some(end) = rest[start..].find("}}").map(|e| start + e + 2) else {
            break;
        };
        out.push_str(before);
        let previous = out.trim_end().chars().last();
        let placeholder = &rest[start..end];
        if matches!(previous, Some(':' | ',' | '[')) {
            out.push('"');
            out.push_str(placeholder);
            out.push('"');
        } else {
            out.push_str(placeholder);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Feuilles d'un document JSON : chemin (`body.a[0].b`) et valeur.
fn leaves(value: &Value, path: &str, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                leaves(child, &format!("{path}.{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                leaves(child, &format!("{path}[{index}]"), out);
            }
        }
        other => out.push((path.to_owned(), other.clone())),
    }
}

fn leaf_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Sources des valeurs tirées (R10) : une feuille du corps JSON envoyé, ou un
/// en-tête, écrite `{{$nom}}` seule dans la requête.
fn locate_drawn(
    drawn: &mut [DrawnValue],
    template: &str,
    body: Option<&str>,
    templates: &[(String, String)],
    headers: &[(String, String)],
) {
    let mut taken: Vec<String> = Vec::new();
    let sent = body.and_then(|b| serde_json::from_str::<Value>(b).ok());
    let mut sent_leaves = Vec::new();
    if let Some(sent) = &sent {
        leaves(sent, "body", &mut sent_leaves);
    }
    // Feuilles du corps écrites `{{$nom}}` seules, si le modèle est du JSON.
    let lone: Vec<(String, String)> = serde_json::from_str::<Value>(&quote_bare(template))
        .ok()
        .map(|value| {
            let mut list = Vec::new();
            leaves(&value, "body", &mut list);
            list.into_iter()
                .filter_map(|(path, v)| {
                    let name = lone_placeholder(v.as_str()?)?.to_owned();
                    Some((path, name))
                })
                .collect()
        })
        .unwrap_or_default();
    for draw in drawn.iter_mut() {
        let in_body = sent_leaves.iter().find(|(path, value)| {
            !taken.contains(path)
                && leaf_text(value) == draw.value
                && lone.iter().any(|(p, name)| p == path && *name == draw.name)
        });
        if let Some((path, _)) = in_body {
            taken.push(path.clone());
            draw.source = Some(format!("request.{path}"));
            continue;
        }
        let header = headers
            .iter()
            .zip(templates)
            .find(|((name, value), (_, raw))| {
                *value == draw.value
                    && lone_placeholder(raw) == Some(draw.name.as_str())
                    && !taken.contains(name)
            });
        if let Some(((name, _), _)) = header {
            taken.push(name.clone());
            draw.source = Some(format!("request.headers.{name}"));
        }
    }
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

/// Remplace le segment `:name` ou `{name}` d'une adresse (suivi de `/`, `?`,
/// `#` ou de la fin), R1.
fn replace_path_param(url: &str, name: &str, value: &str) -> String {
    let url = replace_marker(url, &format!(":{name}"), value);
    replace_marker(&url, &format!("{{{name}}}"), value)
}

fn replace_marker(url: &str, marker: &str, value: &str) -> String {
    let mut out = String::new();
    let mut rest = url;
    while let Some(index) = rest.find(marker) {
        let after = &rest[index + marker.len()..];
        let boundary = after.is_empty() || after.starts_with(['/', '?', '#']);
        let segment = rest[..index].ends_with('/');
        out.push_str(&rest[..index]);
        if boundary && segment {
            out.push_str(value);
        } else {
            out.push_str(marker);
        }
        rest = after;
    }
    out.push_str(rest);
    out
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
    prepare_with(request, variables, &mut Dice::new())
}

/// [`prepare`] avec un hasard donné (tests des valeurs dynamiques).
pub fn prepare_with(
    request: &Request,
    variables: &Variables,
    dice: &mut Dice,
) -> Result<Prepared, CommandError> {
    let mut resolver = Resolver {
        variables,
        dice,
        unknown: BTreeSet::new(),
        drawn: Vec::new(),
        used: Vec::new(),
    };
    let mut sub = |text: &str| resolver.sub(text);

    let settings = request.settings.resolve(&request.inherited.settings);
    // Adresse non encodée : les valeurs partent telles quelles (R5).
    let enc = |text: &str| {
        if settings.encode_url {
            encode(text)
        } else {
            text.to_owned()
        }
    };

    let mut url = sub(&request.url);
    // Paramètres de chemin (S8, R1) : `:nom` ou `{nom}` remplacé par la valeur.
    for pair in active(&request.params).filter(|p| p.kind == ParamKind::Path) {
        url = replace_path_param(&url, &sub(&pair.name), &enc(&sub(&pair.value)));
    }
    let mut query: Vec<String> = active(&request.params)
        .filter(|pair| pair.kind == ParamKind::Query)
        .map(|pair| format!("{}={}", enc(&sub(&pair.name)), enc(&sub(&pair.value))))
        .collect();

    let own: Vec<(String, String)> = active(&request.headers)
        .map(|pair| (sub(&pair.name), sub(&pair.value)))
        .collect();
    // Valeurs écrites des en-têtes propres, pour situer une valeur tirée (R10).
    let own_templates: Vec<(String, String)> = active(&request.headers)
        .map(|pair| (pair.name.clone(), pair.value.clone()))
        .collect();
    // En-têtes hérités d'abord, sauf ceux que la requête redéfinit (R4).
    let mut headers: Vec<(String, String)> = request
        .inherited
        .headers
        .iter()
        .filter(|h| !h.name.trim().is_empty() && !has_header(&own, &h.name))
        .map(|h| (sub(&h.name), sub(&h.value)))
        .collect();
    let own_sent = own.clone();
    headers.extend(own);

    // Authentification héritée : celle du dossier ou de la collection (R4).
    let auth: &Auth = match request.auth.kind {
        AuthKind::Inherit => request
            .inherited
            .auth
            .as_ref()
            .map_or(&request.auth, |inherited| &inherited.auth),
        _ => &request.auth,
    };
    match auth.kind {
        // Aucune, héritée sans source, ou autre (non gérée) : rien n'est envoyé.
        AuthKind::None | AuthKind::Inherit | AuthKind::Other => {}
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
                    enc(&sub(&auth.name)),
                    enc(&sub(&auth.value))
                ));
            }
        },
        AuthKind::ApiKey => {}
    }

    // Seule la variante sélectionnée part (R2).
    let body = &request.body;
    let (body, content_type) = match body.kind {
        BodyKind::None | BodyKind::Other => (None, None),
        BodyKind::Xml => (Some(sub(body.content())), Some("application/xml")),
        BodyKind::Json => (Some(sub(body.content())), Some("application/json")),
        BodyKind::Text => (Some(sub(body.content())), Some("text/plain; charset=utf-8")),
        BodyKind::Form => {
            let fields: Vec<String> = active(body.fields())
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

    let Resolver {
        unknown,
        mut drawn,
        used,
        ..
    } = resolver;
    if !drawn.is_empty() {
        let template = matches!(request.body.kind, BodyKind::Json)
            .then(|| request.body.content())
            .unwrap_or_default();
        let sent_json = matches!(request.body.kind, BodyKind::Json)
            .then_some(body.as_deref())
            .flatten();
        locate_drawn(&mut drawn, template, sent_json, &own_templates, &own_sent);
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
        timeout_ms: (settings.timeout_ms > 0).then_some(settings.timeout_ms),
        max_redirects: if settings.follow_redirects {
            settings.max_redirects
        } else {
            0
        },
        drawn,
        used,
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
            Pair::new("q", "a b&c"),
            Pair {
                enabled: false,
                ..Pair::new("off", "x")
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
        r.body = Body::raw(BodyKind::Json, "{}");
        let p = prepare(&r, &v).unwrap();
        assert_eq!(p.body.as_deref(), Some("{}"));
        assert!(
            p.headers
                .contains(&("Content-Type".into(), "application/json".into()))
        );
        r.headers
            .push(Pair::new("content-type", "application/vnd.x+json"));
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
        assert_eq!(v.mask_owned("Bearer s3cret".to_owned()), "Bearer ••••");
    }

    #[test]
    fn liste_les_noms_de_variables() {
        assert_eq!(variable_names("{{a}}/x/{{ b }}"), ["a", "b"]);
    }

    #[test]
    fn r9_valeurs_dynamiques_tirees_a_chaque_occurrence() {
        let v = vars(&[("baseUrl", "https://api.test")], &[]);
        let mut r = request("{{baseUrl}}/o/{{$randomInt}}?t={{$timestamp}}");
        r.method = Method::Post;
        r.headers.push(Pair::new("X-Ref", "{{ $guid }}"));
        r.body = Body::raw(
            BodyKind::Json,
            r#"{"ref": "{{$randomUUID}}", "copie": "{{$randomUUID}}", "n": {{$randomInt}}, "x": "{{$hasard}}"}"#,
        );
        let mut dice = Dice::seeded(7, 1_794_495_731_000);
        let p = prepare_with(&r, &v, &mut dice).unwrap();
        let names: Vec<&str> = p.drawn.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "$randomInt",
                "$timestamp",
                "$guid",
                "$randomUUID",
                "$randomUUID",
                "$randomInt"
            ]
        );
        assert!(p.url.ends_with("?t=1794495731"), "{}", p.url);
        // Une valeur par occurrence ; un nom inconnu part tel quel, sans erreur.
        assert_ne!(p.drawn[3].value, p.drawn[4].value);
        let body = p.body.unwrap();
        assert!(body.contains(&p.drawn[3].value) && body.contains("{{$hasard}}"));
        // Sources (R10) : feuilles du corps et en-tête écrits seuls.
        let sources: Vec<Option<&str>> = p.drawn.iter().map(|d| d.source.as_deref()).collect();
        assert_eq!(
            sources,
            [
                None,
                None,
                Some("request.headers.X-Ref"),
                Some("request.body.ref"),
                Some("request.body.copie"),
                Some("request.body.n"),
            ]
        );
        assert_eq!(p.used, ["baseUrl"]);
    }

    #[test]
    fn un_secret_contenu_dans_un_autre_ne_le_devoile_pas() {
        let v = vars(
            &[("court", "abc"), ("long", "abcdef-123")],
            &["court", "long"],
        );
        assert_eq!(
            v.mask("jeton abcdef-123 puis abc"),
            format!("jeton {MASK} puis {MASK}")
        );
    }
}
