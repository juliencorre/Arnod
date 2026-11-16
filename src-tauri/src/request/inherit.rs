//! Héritage d'une requête (plan de configuration d'un endpoint, R3, R4) :
//! authentification, en-têtes et réglages repris des `folder.yml` du dossier
//! de la requête et de ses parents, puis de `opencollection.yml`.
//!
//! Lecture seule : un fichier illisible ou non conforme de la chaîne est ignoré.

use std::path::Path;

use serde_json::Value;

use crate::opencollection::request::auth_from;
use crate::opencollection::schema::{self, Part};
use crate::opencollection::{FOLDER_FILE, collection, yaml};
use crate::request::model::{
    AuthKind, Inherited, InheritedAuth, InheritedHeader, ResolvedSettings,
};

/// Réglages par défaut d'une requête de la chaîne : `request` d'un dossier ou
/// de la collection, avec sa source.
struct Level {
    source: String,
    request: Value,
}

fn folder_level(dir: &Path) -> Option<Level> {
    let value = yaml::read(&dir.join(FOLDER_FILE)).ok()?;
    schema::check(Part::Folder, &value).ok()?;
    let name = value
        .pointer("/info/name")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))?;
    Some(Level {
        source: format!("dossier « {name} »"),
        request: value.get("request")?.clone(),
    })
}

fn collection_level(root: &Path) -> Option<Level> {
    let value = collection::read(root).ok()?;
    let name = collection::name(&value).unwrap_or_else(|| "projet".to_owned());
    Some(Level {
        source: format!("collection « {name} »"),
        request: value.get("request")?.clone(),
    })
}

/// Niveaux de la chaîne, du plus proche au plus lointain.
fn chain(root: &Path, request_path: &str) -> Vec<Level> {
    let mut levels = Vec::new();
    let parts: Vec<&str> = request_path.split('/').filter(|p| !p.is_empty()).collect();
    // Dossiers de la requête, du plus profond à la racine (exclue).
    for depth in (1..parts.len()).rev() {
        let dir = parts[..depth]
            .iter()
            .fold(root.to_path_buf(), |path, part| path.join(part));
        if let Some(level) = folder_level(&dir) {
            levels.push(level);
        }
    }
    if let Some(level) = collection_level(root) {
        levels.push(level);
    }
    levels
}

/// Réglage `key` de `request.settings.http` d'un niveau, s'il est fixé.
fn setting<'a>(level: &'a Level, key: &str) -> Option<&'a Value> {
    level
        .request
        .pointer(&format!("/settings/http/{key}"))
        .filter(|v| v.as_str() != Some("inherit"))
}

/// Ce que la requête `request_path` (relative à `root`) hérite (R3, R4).
pub fn resolve(root: &Path, request_path: &str) -> Inherited {
    let levels = chain(root, request_path);
    let auth = levels.iter().find_map(|level| {
        let value = level.request.get("auth")?;
        let auth = auth_from(Some(value));
        (auth.kind != AuthKind::Inherit).then(|| InheritedAuth {
            auth,
            source: level.source.clone(),
        })
    });
    let mut headers: Vec<InheritedHeader> = Vec::new();
    for level in &levels {
        for item in level
            .request
            .get("headers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = item.get("name").and_then(Value::as_str).unwrap_or("");
            let disabled = item.get("disabled").and_then(Value::as_bool) == Some(true);
            let known = headers.iter().any(|h| h.name.eq_ignore_ascii_case(name));
            if name.trim().is_empty() || disabled || known {
                continue;
            }
            headers.push(InheritedHeader {
                name: name.to_owned(),
                value: item
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                source: level.source.clone(),
            });
        }
    }
    let first = |key: &str| levels.iter().find_map(|level| setting(level, key));
    let defaults = ResolvedSettings::default();
    let number = |key: &str| {
        first(key)
            .and_then(Value::as_f64)
            .filter(|n| *n >= 0.0)
            .map(|n| n as u64)
    };
    let settings = ResolvedSettings {
        timeout_ms: number("timeout").unwrap_or(defaults.timeout_ms),
        follow_redirects: first("followRedirects")
            .and_then(Value::as_bool)
            .unwrap_or(defaults.follow_redirects),
        max_redirects: number("maxRedirects")
            .map(|n| u32::try_from(n).unwrap_or(u32::MAX))
            .unwrap_or(defaults.max_redirects),
        encode_url: first("encodeUrl")
            .and_then(Value::as_bool)
            .unwrap_or(defaults.encode_url),
    };
    Inherited {
        auth,
        headers,
        settings,
    }
}

/// Noms des variables (`request.variables`) des `folder.yml` de la requête et
/// de `opencollection.yml`, actives, du plus proche au plus lointain
/// (`docs/plan-mvp-b-saisie-curl.md`, R1).
pub fn variable_names(root: &Path, request_path: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for level in chain(root, request_path) {
        for item in level
            .request
            .get("variables")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let disabled = item.get("disabled").and_then(Value::as_bool) == Some(true);
            if !name.is_empty() && !disabled && !names.iter().any(|n| n == name) {
                names.push(name.to_owned());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::request::model::{Auth, BodyKind, Method, Pair, Request, Settings};
    use crate::request::resolve::{Variables, prepare};

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("opencollection.yml"),
            "opencollection: 1.0.0\ninfo:\n  name: Boutique\nrequest:\n  headers:\n    - name: Accept\n      value: application/json\n    - name: X-Client\n      value: arnod\n  auth:\n    type: bearer\n    token: \"{{token}}\"\n  settings:\n    http:\n      timeout: 20000\n      maxRedirects: 2\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("Commandes/v2")).unwrap();
        fs::write(
            dir.path().join("Commandes/folder.yml"),
            "info:\n  name: Commandes\n  type: folder\nrequest:\n  headers:\n    - name: x-client\n      value: commandes\n  settings:\n    http:\n      timeout: 5000\n      followRedirects: false\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn r3_reglages_herites_du_dossier_puis_de_la_collection() {
        let dir = project();
        let inherited = resolve(dir.path(), "Commandes/v2/creer.yml");
        assert_eq!(
            inherited.settings,
            ResolvedSettings {
                timeout_ms: 5000,
                follow_redirects: false,
                max_redirects: 2,
                encode_url: true,
            }
        );
        // Hors du dossier : la collection, puis le défaut.
        let top = resolve(dir.path(), "sante.yml");
        assert_eq!(top.settings.timeout_ms, 20_000);
        assert!(top.settings.follow_redirects);
    }

    #[test]
    fn r3_sans_chaine_les_defauts() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve(dir.path(), "a/b.yml"), Inherited::default());
    }

    #[test]
    fn r4_authentification_et_en_tetes_herites() {
        let dir = project();
        let inherited = resolve(dir.path(), "Commandes/v2/creer.yml");
        let auth = inherited.auth.clone().unwrap();
        assert_eq!(auth.auth.kind, AuthKind::Bearer);
        assert_eq!(auth.source, "collection « Boutique »");
        let headers: Vec<(&str, &str, &str)> = inherited
            .headers
            .iter()
            .map(|h| (h.name.as_str(), h.value.as_str(), h.source.as_str()))
            .collect();
        assert_eq!(
            headers,
            [
                ("x-client", "commandes", "dossier « Commandes »"),
                ("Accept", "application/json", "collection « Boutique »"),
            ]
        );

        // À l'envoi : l'authentification héritée et les en-têtes non redéfinis.
        let mut request = Request::new("Créer");
        request.method = Method::Post;
        request.url = "https://api.test/orders".to_owned();
        request.auth = Auth {
            kind: AuthKind::Inherit,
            ..Auth::default()
        };
        request.headers.push(Pair::new("Accept", "text/plain"));
        request.inherited = inherited;
        request.settings = Settings {
            timeout_ms: Some(0),
            ..Settings::default()
        };
        request.body = crate::request::model::Body::raw(BodyKind::Json, "{}");
        let mut variables = Variables::default();
        variables.values.insert("token".into(), "t0k".into());
        let prepared = prepare(&request, &variables).unwrap();
        assert!(
            prepared
                .headers
                .contains(&("x-client".into(), "commandes".into()))
        );
        assert!(
            prepared
                .headers
                .contains(&("Accept".into(), "text/plain".into()))
        );
        assert!(
            !prepared
                .headers
                .contains(&("Accept".into(), "application/json".into()))
        );
        assert!(
            prepared
                .headers
                .contains(&("Authorization".into(), "Bearer t0k".into()))
        );
        // R5 : délai 0, aucun délai ; redirections non suivies héritées du dossier.
        assert_eq!(prepared.timeout_ms, None);
        assert_eq!(prepared.max_redirects, 0);
    }

    #[test]
    fn r4_heritee_sans_source_rien_n_est_envoye() {
        let mut request = Request::new("x");
        request.url = "https://api.test/".to_owned();
        request.auth = Auth {
            kind: AuthKind::Inherit,
            ..Auth::default()
        };
        let prepared = prepare(&request, &Variables::default()).unwrap();
        assert!(!prepared.headers.iter().any(|(n, _)| n == "Authorization"));
    }

    #[test]
    fn un_dossier_non_conforme_est_ignore() {
        let dir = project();
        fs::write(dir.path().join("Commandes/folder.yml"), "info: [").unwrap();
        let inherited = resolve(dir.path(), "Commandes/creer.yml");
        assert_eq!(inherited.settings.timeout_ms, 20_000);
    }
}
