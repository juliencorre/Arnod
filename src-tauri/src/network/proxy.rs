//! Proxy effectif d'un envoi (`docs/plan-mvp-h-reseau-auth.md`, R2, R3).
//!
//! Module pur, sauf [`OsProxySource`] qui lit l'environnement et, sous Windows, le registre :
//! derrière le trait [`SystemProxySource`], remplacé dans les tests.

use serde::{Deserialize, Serialize};

/// Mode de proxy choisi dans les préférences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxyMode {
    /// Celui du système (variables d'environnement, registre Windows).
    #[default]
    System,
    /// Aucun : connexion directe.
    None,
    /// Saisi à la main.
    Manual,
}

/// D'où vient le proxy du système.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxySource {
    /// Variables `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`.
    Environment,
    /// `Internet Settings` du registre Windows.
    Windows,
}

/// Proxy du système détecté.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProxy {
    /// Proxy des adresses http.
    pub http: Option<String>,
    /// Proxy des adresses https.
    pub https: Option<String>,
    /// Exceptions (séparées par `,` ou `;`).
    pub no_proxy: String,
    pub source: Option<ProxySource>,
    /// Script de configuration automatique déclaré (PAC), non pris en charge.
    pub auto_config: Option<String>,
}

/// Lecture du proxy du système (R2).
pub trait SystemProxySource: Send + Sync {
    fn detect(&self) -> SystemProxy;
}

/// Proxy du système : environnement, puis registre Windows.
#[derive(Debug, Default)]
pub struct OsProxySource;

impl SystemProxySource for OsProxySource {
    fn detect(&self) -> SystemProxy {
        let from_env = from_env(|name| std::env::var(name).ok());
        if from_env.http.is_some() || from_env.https.is_some() {
            return from_env;
        }
        windows_proxy().unwrap_or_default()
    }
}

/// Proxy du système fixe, pour les tests.
#[derive(Debug, Default)]
pub struct FixedProxySource(pub SystemProxy);

impl SystemProxySource for FixedProxySource {
    fn detect(&self) -> SystemProxy {
        self.0.clone()
    }
}

#[cfg(windows)]
fn windows_proxy() -> Option<SystemProxy> {
    let key = windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;
    let enabled = key.get_u32("ProxyEnable").unwrap_or(0) != 0;
    let server = key.get_string("ProxyServer").unwrap_or_default();
    let exceptions = key.get_string("ProxyOverride").unwrap_or_default();
    let auto = key.get_string("AutoConfigURL").ok();
    Some(from_windows(enabled, &server, &exceptions, auto))
}

#[cfg(not(windows))]
fn windows_proxy() -> Option<SystemProxy> {
    None
}

/// Proxy des variables d'environnement (casse indifférente, minuscules d'abord comme curl).
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> SystemProxy {
    let var = |name: &str| {
        get(&name.to_lowercase())
            .or_else(|| get(name))
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())
    };
    let all = var("ALL_PROXY");
    let http = var("HTTP_PROXY").or_else(|| all.clone());
    let https = var("HTTPS_PROXY").or(all);
    let source = (http.is_some() || https.is_some()).then_some(ProxySource::Environment);
    SystemProxy {
        http,
        https,
        no_proxy: var("NO_PROXY").unwrap_or_default(),
        source,
        auto_config: None,
    }
}

/// Proxy du registre Windows : `ProxyServer` vaut `hôte:port` ou
/// `http=hôte:port;https=hôte:port` ; `ProxyOverride` liste les exceptions.
pub fn from_windows(
    enabled: bool,
    server: &str,
    exceptions: &str,
    auto_config: Option<String>,
) -> SystemProxy {
    let auto_config = auto_config.filter(|a| !a.trim().is_empty());
    if !enabled || server.trim().is_empty() {
        return SystemProxy {
            auto_config,
            ..SystemProxy::default()
        };
    }
    let (mut http, mut https) = (None, None);
    if server.contains('=') {
        for part in server.split(';') {
            if let Some((scheme, address)) = part.split_once('=') {
                let address = address.trim().to_owned();
                match scheme.trim().to_ascii_lowercase().as_str() {
                    "http" => http = Some(address),
                    "https" => https = Some(address),
                    _ => {}
                }
            }
        }
    } else {
        http = Some(server.trim().to_owned());
        https = http.clone();
    }
    SystemProxy {
        http,
        https,
        no_proxy: exceptions.to_owned(),
        source: Some(ProxySource::Windows),
        auto_config,
    }
}

/// Adresse de proxy complète : `http://` ajouté à `hôte:port`.
pub fn normalize(address: &str) -> String {
    let address = address.trim();
    if address.contains("://") {
        address.to_owned()
    } else {
        format!("http://{address}")
    }
}

/// L'hôte `host` est-il une exception de `no_proxy` (R3) ?
pub fn bypass(no_proxy: &str, host: &str) -> bool {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    no_proxy
        .split([',', ';'])
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .any(|entry| {
            if entry == "*" {
                return true;
            }
            if entry == "<local>" {
                return !host.contains('.') && !host.contains(':');
            }
            // Port éventuel retiré (`hôte:8080`), sauf pour une adresse IPv6.
            let entry = match entry.rsplit_once(':') {
                Some((name, port)) if !name.contains(':') && port.parse::<u16>().is_ok() => {
                    name.to_owned()
                }
                _ => entry,
            };
            let domain = entry.trim_start_matches('*').trim_start_matches('.');
            if domain.is_empty() {
                return false;
            }
            host == domain || host.ends_with(&format!(".{domain}"))
        })
}

/// Proxy réglé pour les envois : par schéma, avec ses exceptions et son identifiant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveProxy {
    pub http: Option<String>,
    pub https: Option<String>,
    pub no_proxy: String,
    pub username: String,
    pub password: Option<String>,
}

impl EffectiveProxy {
    /// Proxy utilisé pour une adresse, ou `None` (connexion directe).
    pub fn for_url(&self, scheme: &str, host: &str) -> Option<String> {
        let proxy = match scheme {
            "https" => self.https.as_ref(),
            "http" => self.http.as_ref(),
            _ => None,
        }?;
        (!bypass(&self.no_proxy, host)).then(|| normalize(proxy))
    }

    pub fn is_direct(&self) -> bool {
        self.http.is_none() && self.https.is_none()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn r2_variables_d_environnement() {
        let vars: HashMap<&str, &str> = [
            ("https_proxy", "proxy.corp:8443"),
            ("HTTP_PROXY", "http://proxy.corp:8080"),
            ("NO_PROXY", "localhost,.interne"),
        ]
        .into();
        let found = from_env(|name| vars.get(name).map(|v| (*v).to_owned()));
        assert_eq!(found.https.as_deref(), Some("proxy.corp:8443"));
        assert_eq!(found.http.as_deref(), Some("http://proxy.corp:8080"));
        assert_eq!(found.no_proxy, "localhost,.interne");
        assert_eq!(found.source, Some(ProxySource::Environment));
        // Sans variable : rien de détecté.
        let none = from_env(|_| None);
        assert_eq!(none, SystemProxy::default());
    }

    #[test]
    fn r2_registre_windows() {
        let one = from_windows(true, "proxy.entreprise.local:8080", "<local>;*.corp", None);
        assert_eq!(one.http.as_deref(), Some("proxy.entreprise.local:8080"));
        assert_eq!(one.https, one.http);
        assert_eq!(one.source, Some(ProxySource::Windows));
        let split = from_windows(true, "http=p1:80;https=p2:443;ftp=p3:21", "", None);
        assert_eq!(split.http.as_deref(), Some("p1:80"));
        assert_eq!(split.https.as_deref(), Some("p2:443"));
        // Désactivé : seul le script de configuration automatique est signalé.
        let off = from_windows(false, "p:1", "", Some("http://wpad/proxy.pac".into()));
        assert!(off.http.is_none() && off.https.is_none());
        assert_eq!(off.auto_config.as_deref(), Some("http://wpad/proxy.pac"));
    }

    #[test]
    fn r3_exceptions() {
        assert!(bypass("*", "api.test"));
        assert!(bypass("<local>", "intranet"));
        assert!(!bypass("<local>", "intranet.corp"));
        assert!(bypass(".corp", "api.corp"));
        assert!(bypass("*.corp", "a.b.corp"));
        assert!(bypass("corp", "corp"));
        assert!(!bypass(".corp", "corporate.fr"));
        assert!(bypass("localhost, 127.0.0.1", "127.0.0.1"));
        assert!(bypass("API.Test:8443", "api.test"));
        assert!(!bypass("", "api.test"));
    }

    #[test]
    fn proxy_par_schema_et_exception() {
        let proxy = EffectiveProxy {
            http: Some("p:8080".into()),
            https: None,
            no_proxy: ".interne".into(),
            ..EffectiveProxy::default()
        };
        assert_eq!(
            proxy.for_url("http", "api.test").as_deref(),
            Some("http://p:8080")
        );
        assert_eq!(proxy.for_url("https", "api.test"), None);
        assert_eq!(proxy.for_url("http", "a.interne"), None);
    }
}
