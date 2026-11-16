//! Cookies gardés par environnement (`docs/plan-mvp-h-reseau-auth.md`, R12 à R15, écran 3.4-g).
//!
//! Le magasin est pur ; `load` et `save` gardent les cookies persistants dans
//! `projects/<id>/cookies.json` (poste, jamais le projet). Les cookies de session restent en
//! mémoire.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::local;
use crate::request::resolve::MASK;

/// Cookie gardé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    /// Domaine, en minuscules, sans point initial.
    pub domain: String,
    /// Sans attribut `Domain` : seul cet hôte le reçoit.
    pub host_only: bool,
    pub path: String,
    /// Expiration, en ms depuis l'époque Unix ; `None` : cookie de session.
    pub expires_at: Option<u64>,
    pub secure: bool,
    pub http_only: bool,
    /// Requête qui l'a reçu (`POST /login`).
    pub received_from: String,
    pub received_at: u64,
}

/// Cookies d'un environnement.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jar {
    #[serde(default)]
    pub cookies: Vec<StoredCookie>,
    /// Domaines dont les cookies ne sont plus renvoyés (R15).
    #[serde(default)]
    pub disabled_domains: BTreeSet<String>,
}

/// Cookie montré à l'interface : valeur masquée (`HttpOnly`) ou abrégée (R14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieView {
    pub name: String,
    pub value: String,
    pub path: String,
    pub expires_at: Option<u64>,
    pub http_only: bool,
    pub secure: bool,
    pub received_from: String,
}

/// Cookies d'un domaine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieDomain {
    pub domain: String,
    /// Renvoyés aux requêtes suivantes (R15).
    pub sending: bool,
    pub cookies: Vec<CookieView>,
}

/// Cookies gardés d'un environnement, pour l'interface (3.4-g).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieJarView {
    pub environment: String,
    /// Hôte de la requête demandée (adresse résolue), s'il est lisible.
    pub host: Option<String>,
    pub domains: Vec<CookieDomain>,
}

/// Valeur montrée (R14) : masquée si `HttpOnly`, abrégée au-delà de 8 caractères.
pub fn display_value(cookie: &StoredCookie) -> String {
    if cookie.http_only {
        return MASK.repeat(2);
    }
    let chars: Vec<char> = cookie.value.chars().collect();
    if chars.len() <= 8 {
        cookie.value.clone()
    } else {
        let head: String = chars[..4].iter().collect();
        let tail: String = chars[chars.len() - 2..].iter().collect();
        format!("{head}…{tail}")
    }
}

/// Chemin par défaut d'un cookie (RFC 6265, § 5.1.4).
fn default_path(url: &Url) -> String {
    let path = url.path();
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(i) => path[..i].to_owned(),
    }
}

fn path_matches(cookie_path: &str, request_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/') || request_path[cookie_path.len()..].starts_with('/')))
}

fn domain_matches(cookie: &StoredCookie, host: &str) -> bool {
    if cookie.host_only {
        host == cookie.domain
    } else {
        host == cookie.domain || host.ends_with(&format!(".{}", cookie.domain))
    }
}

impl Jar {
    /// Oublie les cookies expirés (R13).
    pub fn purge(&mut self, now: u64) {
        self.cookies
            .retain(|c| c.expires_at.is_none_or(|expires| expires > now));
    }

    /// Garde les cookies de `set_cookies`, reçus de `url` (R12, R13). Renvoie les noms gardés.
    pub fn store(
        &mut self,
        url: &Url,
        set_cookies: &[&str],
        origin: &str,
        now: u64,
    ) -> Vec<String> {
        let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
            return Vec::new();
        };
        let mut kept = Vec::new();
        for header in set_cookies {
            let Ok(parsed) = cookie::Cookie::parse(header.to_string()) else {
                continue;
            };
            let (domain, host_only) = match parsed.domain() {
                Some(d) if !d.trim_start_matches('.').is_empty() => {
                    let d = d.trim_start_matches('.').to_ascii_lowercase();
                    // Un domaine qui ne couvre pas l'hôte est refusé (R13).
                    if host != d && !host.ends_with(&format!(".{d}")) {
                        continue;
                    }
                    (d, false)
                }
                _ => (host.clone(), true),
            };
            let path = parsed
                .path()
                .filter(|p| p.starts_with('/'))
                .map_or_else(|| default_path(url), str::to_owned);
            // `Max-Age` prime sur `Expires` (R13).
            let expires_at = if let Some(max_age) = parsed.max_age() {
                let secs = max_age.whole_seconds();
                Some(if secs <= 0 {
                    0
                } else {
                    now.saturating_add(u64::try_from(secs).unwrap_or(u64::MAX).saturating_mul(1000))
                })
            } else {
                parsed.expires_datetime().map(|at| {
                    let ms = at.unix_timestamp().saturating_mul(1000);
                    u64::try_from(ms).unwrap_or(0)
                })
            };
            let name = parsed.name().to_owned();
            self.cookies
                .retain(|c| !(c.name == name && c.domain == domain && c.path == path));
            if expires_at.is_some_and(|e| e <= now) {
                continue;
            }
            self.cookies.push(StoredCookie {
                name: name.clone(),
                value: parsed.value().to_owned(),
                domain,
                host_only,
                path,
                expires_at,
                secure: parsed.secure().unwrap_or(false),
                http_only: parsed.http_only().unwrap_or(false),
                received_from: origin.to_owned(),
                received_at: now,
            });
            kept.push(name);
        }
        kept
    }

    /// En-tête `Cookie` d'une requête vers `url` (R12, R15), et les valeurs `HttpOnly` à masquer.
    pub fn header_for(&self, url: &Url, now: u64) -> Option<(String, Vec<String>)> {
        let host = url.host_str()?.to_ascii_lowercase();
        let secure = url.scheme() == "https";
        let mut matching: Vec<&StoredCookie> = self
            .cookies
            .iter()
            .filter(|c| c.expires_at.is_none_or(|e| e > now))
            .filter(|c| !self.disabled_domains.contains(&c.domain))
            .filter(|c| domain_matches(c, &host))
            .filter(|c| path_matches(&c.path, url.path()))
            .filter(|c| secure || !c.secure)
            .collect();
        if matching.is_empty() {
            return None;
        }
        // Chemins les plus longs d'abord, puis les plus anciens (RFC 6265, § 5.4).
        matching.sort_by(|a, b| {
            b.path
                .len()
                .cmp(&a.path.len())
                .then(a.received_at.cmp(&b.received_at))
        });
        let header = matching
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");
        let hidden = matching
            .iter()
            .filter(|c| c.http_only && !c.value.is_empty())
            .map(|c| c.value.clone())
            .collect();
        Some((header, hidden))
    }

    /// Oublie les cookies d'un domaine, ou tous (« Vider », CK3).
    pub fn clear(&mut self, domain: Option<&str>) {
        match domain {
            Some(d) => self.cookies.retain(|c| c.domain != d),
            None => self.cookies.clear(),
        }
    }

    /// Renvoie ou non les cookies d'un domaine (R15).
    pub fn set_sending(&mut self, domain: &str, enabled: bool) {
        if enabled {
            self.disabled_domains.remove(domain);
        } else {
            self.disabled_domains.insert(domain.to_owned());
        }
    }

    /// Vue de l'interface : domaines qui couvrent `host` (tous si `None`).
    pub fn view(&self, environment: &str, host: Option<&str>, now: u64) -> CookieJarView {
        let mut domains: BTreeMap<String, Vec<CookieView>> = BTreeMap::new();
        for cookie in &self.cookies {
            if cookie.expires_at.is_some_and(|e| e <= now) {
                continue;
            }
            if let Some(host) = host
                && !domain_matches(cookie, host)
            {
                continue;
            }
            domains
                .entry(cookie.domain.clone())
                .or_default()
                .push(CookieView {
                    name: cookie.name.clone(),
                    value: display_value(cookie),
                    path: cookie.path.clone(),
                    expires_at: cookie.expires_at,
                    http_only: cookie.http_only,
                    secure: cookie.secure,
                    received_from: cookie.received_from.clone(),
                });
        }
        // Un domaine sans cookie mais dont l'envoi est coupé reste visible pour le rétablir.
        if let Some(host) = host {
            for domain in &self.disabled_domains {
                if host == domain || host.ends_with(&format!(".{domain}")) {
                    domains.entry(domain.clone()).or_default();
                }
            }
        }
        CookieJarView {
            environment: environment.to_owned(),
            host: host.map(str::to_owned),
            domains: domains
                .into_iter()
                .map(|(domain, cookies)| CookieDomain {
                    sending: !self.disabled_domains.contains(&domain),
                    domain,
                    cookies,
                })
                .collect(),
        }
    }

    /// Cookies persistants seulement, pour l'écriture sur le poste.
    fn persistent(&self) -> Self {
        Self {
            cookies: self
                .cookies
                .iter()
                .filter(|c| c.expires_at.is_some())
                .cloned()
                .collect(),
            disabled_domains: self.disabled_domains.clone(),
        }
    }
}

/// Cookies d'un projet, par environnement.
pub type ProjectJars = BTreeMap<String, Jar>;

fn file(data_dir: &Path, project: Uuid) -> PathBuf {
    local::project_dir(data_dir, project).join("cookies.json")
}

/// Cookies persistants d'un projet ; absents ou illisibles : aucun.
pub fn load(data_dir: &Path, project: Uuid, now: u64) -> ProjectJars {
    let mut jars: ProjectJars = fs::read_to_string(file(data_dir, project))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    for jar in jars.values_mut() {
        jar.purge(now);
    }
    jars
}

/// Écrit les cookies persistants d'un projet, de façon atomique.
pub fn save(data_dir: &Path, project: Uuid, jars: &ProjectJars) -> Result<(), CommandError> {
    let dir = local::project_dir(data_dir, project);
    fs::create_dir_all(&dir)
        .map_err(|error| CommandError::io("Création du dossier local impossible", &dir, &error))?;
    let kept: ProjectJars = jars
        .iter()
        .map(|(env, jar)| (env.clone(), jar.persistent()))
        .filter(|(_, jar)| !jar.cookies.is_empty() || !jar.disabled_domains.is_empty())
        .collect();
    let path = file(data_dir, project);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(&kept).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture impossible : {error}"),
        )
    })?;
    fs::write(&tmp, text).map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, &path).map_err(|error| CommandError::io("Écriture impossible", &path, &error))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000_000;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn r12_garde_et_renvoie_au_meme_domaine() {
        let mut jar = Jar::default();
        let kept = jar.store(
            &url("https://recette.boutique.dev/login"),
            &[
                "session=abc123; Path=/; HttpOnly; Max-Age=1680",
                "csrf=9f2c0000e1",
            ],
            "POST /login",
            NOW,
        );
        assert_eq!(kept, ["session", "csrf"]);
        let (header, hidden) = jar
            .header_for(&url("https://recette.boutique.dev/orders"), NOW + 1000)
            .unwrap();
        assert_eq!(header, "session=abc123; csrf=9f2c0000e1");
        assert_eq!(hidden, ["abc123"]);
        // Autre domaine : rien.
        assert!(jar.header_for(&url("https://autre.dev/"), NOW).is_none());
        // Hôte seul : un sous-domaine ne le reçoit pas.
        assert!(
            jar.header_for(&url("https://a.recette.boutique.dev/"), NOW)
                .is_none()
        );
    }

    #[test]
    fn r12_domaine_et_chemin() {
        let mut jar = Jar::default();
        jar.store(
            &url("https://api.boutique.dev/v2/login"),
            &[
                "a=1; Domain=.boutique.dev; Path=/",
                "b=2",
                "c=3; Domain=autre.dev",
                "d=4; Secure",
            ],
            "POST /v2/login",
            NOW,
        );
        // `c` refusé : son domaine ne couvre pas l'hôte (R13).
        assert_eq!(jar.cookies.len(), 3);
        let (header, _) = jar
            .header_for(&url("https://www.boutique.dev/v2/x"), NOW)
            .unwrap();
        assert_eq!(header, "a=1");
        let (header, _) = jar
            .header_for(&url("https://api.boutique.dev/v2/orders"), NOW)
            .unwrap();
        // `b` et `d` ont le chemin par défaut /v2, plus long : en premier.
        assert_eq!(header, "b=2; d=4; a=1");
        // En http, le cookie `Secure` ne part pas.
        let (header, _) = jar
            .header_for(&url("http://api.boutique.dev/v2/orders"), NOW)
            .unwrap();
        assert_eq!(header, "b=2; a=1");
    }

    #[test]
    fn r13_expiration_et_suppression() {
        let mut jar = Jar::default();
        let u = url("https://a.dev/");
        jar.store(
            &u,
            &[
                "x=1; Max-Age=10",
                "y=2; Expires=Wed, 21 Oct 2015 07:28:00 GMT",
            ],
            "GET /",
            NOW,
        );
        assert_eq!(jar.cookies.len(), 1, "y a déjà expiré");
        assert!(jar.header_for(&u, NOW + 11_000).is_none());
        jar.store(&u, &["x=1; Max-Age=0"], "GET /", NOW);
        assert!(jar.cookies.is_empty(), "Max-Age=0 supprime");
        jar.store(&u, &["z=3; Max-Age=10"], "GET /", NOW);
        jar.purge(NOW + 20_000);
        assert!(jar.cookies.is_empty());
    }

    #[test]
    fn r14_valeurs_masquees_ou_abregees() {
        let mut jar = Jar::default();
        jar.store(
            &url("https://a.dev/"),
            &["s=secret; HttpOnly", "csrf=9f2c4d5e6fe1", "court=abc"],
            "POST /login",
            NOW,
        );
        let view = jar.view("Recette", Some("a.dev"), NOW);
        let values: Vec<&str> = view.domains[0]
            .cookies
            .iter()
            .map(|c| c.value.as_str())
            .collect();
        assert_eq!(values, ["••••••••", "9f2c…e1", "abc"]);
        assert_eq!(view.domains[0].cookies[0].received_from, "POST /login");
    }

    #[test]
    fn r15_envoi_coupe_par_domaine_et_vider() {
        let mut jar = Jar::default();
        let u = url("https://a.dev/");
        jar.store(&u, &["x=1"], "GET /", NOW);
        jar.set_sending("a.dev", false);
        assert!(jar.header_for(&u, NOW).is_none());
        let view = jar.view("Local", Some("a.dev"), NOW);
        assert!(!view.domains[0].sending);
        jar.set_sending("a.dev", true);
        assert!(jar.header_for(&u, NOW).is_some());
        jar.clear(Some("a.dev"));
        assert!(jar.cookies.is_empty());
    }

    #[test]
    fn persistants_seulement_sur_le_poste() {
        let dir = tempfile::tempdir().unwrap();
        let project = Uuid::new_v4();
        let mut jar = Jar::default();
        jar.store(
            &url("https://a.dev/"),
            &["session=1", "long=2; Max-Age=3600"],
            "GET /",
            NOW,
        );
        let mut jars = ProjectJars::new();
        jars.insert("Recette".into(), jar);
        save(dir.path(), project, &jars).unwrap();
        let again = load(dir.path(), project, NOW);
        let names: Vec<&str> = again["Recette"]
            .cookies
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["long"]);
        // Relu après expiration : oublié.
        assert!(
            load(dir.path(), project, NOW + 4_000_000)["Recette"]
                .cookies
                .is_empty()
        );
    }
}
