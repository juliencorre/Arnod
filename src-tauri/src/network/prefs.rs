//! Préférences réseau du poste (`docs/plan-mvp-h-reseau-auth.md`, R1) : `network.json` dans le
//! dossier de données de l'application, jamais dans un projet. Écriture atomique, comme
//! `storage.rs` ; un fichier illisible donne les valeurs par défaut.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};
use crate::network::proxy::ProxyMode;

/// Nom du fichier des préférences réseau.
pub const FILE: &str = "network.json";
/// Version du format écrit.
pub const FORMAT: u32 = 1;
/// Délai de connexion par défaut (10 s, maquette 1.6-e).
pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 10_000;
/// Clé du mot de passe du proxy au coffre du système.
pub const PROXY_SECRET_KEY: &str = "arnod/preferences/proxy";

/// Proxy saisi à la main.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualProxy {
    /// Adresse, `hôte:port` ou `http://hôte:port`.
    #[serde(default)]
    pub url: String,
    /// Exceptions, séparées par `,` ou `;` (R3).
    #[serde(default)]
    pub no_proxy: String,
    /// Identifiant ; le mot de passe est au coffre.
    #[serde(default)]
    pub username: String,
}

/// Réglage du proxy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyPrefs {
    #[serde(default)]
    pub mode: ProxyMode,
    #[serde(default)]
    pub manual: ManualProxy,
}

/// Autorité interne ajoutée (R4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAuthority {
    pub pem: String,
    /// Ajout, en millisecondes depuis l'époque Unix.
    #[serde(default)]
    pub added_at: u64,
}

/// Contenu de `network.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPrefs {
    #[serde(default = "format")]
    pub format: u32,
    #[serde(default)]
    pub proxy: ProxyPrefs,
    #[serde(default = "connect_timeout")]
    pub connect_timeout_ms: u64,
    #[serde(default)]
    pub authorities: Vec<StoredAuthority>,
}

fn format() -> u32 {
    FORMAT
}

fn connect_timeout() -> u64 {
    DEFAULT_CONNECT_TIMEOUT_MS
}

impl Default for NetworkPrefs {
    fn default() -> Self {
        Self {
            format: FORMAT,
            proxy: ProxyPrefs::default(),
            connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
            authorities: Vec::new(),
        }
    }
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

/// Lit les préférences ; absentes ou illisibles : valeurs par défaut (R1).
pub fn load(data_dir: &Path) -> NetworkPrefs {
    fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Écrit les préférences de façon atomique (R1).
pub fn save(data_dir: &Path, prefs: &NetworkPrefs) -> Result<(), CommandError> {
    fs::create_dir_all(data_dir).map_err(|error| {
        CommandError::io(
            "Création du dossier de données impossible",
            data_dir,
            &error,
        )
    })?;
    let path = file(data_dir);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(prefs).map_err(|error| {
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

    #[test]
    fn r1_aller_retour_et_defauts() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()), NetworkPrefs::default());
        let mut prefs = NetworkPrefs::default();
        prefs.proxy.mode = ProxyMode::Manual;
        prefs.proxy.manual.url = "proxy.corp:8080".into();
        prefs.authorities.push(StoredAuthority {
            pem: "pem".into(),
            added_at: 1,
        });
        save(dir.path(), &prefs).unwrap();
        assert_eq!(load(dir.path()), prefs);
        assert!(!dir.path().join("network.json.tmp").exists());
        // Forme JSON : camelCase, format 1.
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(FILE)).unwrap()).unwrap();
        assert_eq!(json["format"], 1);
        assert_eq!(json["proxy"]["mode"], "manual");
        assert_eq!(json["proxy"]["manual"]["noProxy"], "");
        assert_eq!(json["connectTimeoutMs"], 10_000);
    }

    #[test]
    fn r1_fichier_illisible_donne_les_defauts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE), "{").unwrap();
        assert_eq!(load(dir.path()), NetworkPrefs::default());
    }
}
