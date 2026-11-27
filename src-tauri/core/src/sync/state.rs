//! État de la synchronisation propre au poste (`projects/<id>/sync.json`, format 1, écriture
//! atomique) : compte relié, branche, dernière vérification, ce que les collègues ont changé à la
//! dernière récupération (pastilles, CF4), environnements dont les secrets attendent « plus
//! tard » (R15). Jamais dans le projet (R10).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::forge::Forge;
use crate::error::{CommandError, CommandErrorKind};
use crate::local;

pub const FILE: &str = "sync.json";
pub const VERSION: u32 = 1;

/// Compte relié au projet (Q11 : auteur des envois) ; son jeton est au coffre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncAccount"))]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub forge: Forge,
    /// Hôte de la forge (`github.com`, `gitlab.exemple.fr`).
    pub host: String,
    pub login: String,
    /// Nom affiché, sinon l'identifiant.
    pub name: String,
    /// Adresse des envois (adresse « noreply » de la forge si elle n'est pas publique).
    pub email: String,
}

/// Changement d'un collègue reçu à la dernière récupération (pastille, CF4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    pub author: String,
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub account: Option<Account>,
    /// Branche suivie (une seule par projet).
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub checked_at: Option<u64>,
    /// Dernier échec de vérification, en clair.
    #[serde(default)]
    pub error: Option<String>,
    /// Fichiers changés par un collègue à la dernière récupération, avec leur auteur.
    #[serde(default)]
    pub received: BTreeMap<String, Received>,
    /// Environnements dont l'utilisateur a choisi « Plus tard » pour les secrets manquants, avec
    /// les noms manquants à ce moment : un nouveau secret manquant les signale de nouveau.
    #[serde(default)]
    pub secrets_later: BTreeMap<String, Vec<String>>,
}

fn path(data_dir: &Path, project: Uuid) -> PathBuf {
    local::project_dir(data_dir, project).join(FILE)
}

pub fn load(data_dir: &Path, project: Uuid) -> SyncState {
    fs::read_to_string(path(data_dir, project))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, project: Uuid, state: &SyncState) -> Result<(), CommandError> {
    let file = path(data_dir, project);
    let io = |error: std::io::Error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("L'état de la synchronisation n'a pas pu être écrit : {error}."),
        )
    };
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(io)?;
    }
    let mut state = state.clone();
    state.version = VERSION;
    let text = serde_json::to_string_pretty(&state).map_err(|e| io(std::io::Error::other(e)))?;
    let temp = file.with_extension("json.tmp");
    fs::write(&temp, text).map_err(io)?;
    fs::rename(&temp, &file).map_err(io)
}

/// Clé du coffre d'un jeton (`arnod-git:<hôte>:<compte>`).
pub fn token_key(host: &str, login: &str) -> String {
    format!("arnod-git:{}:{login}", host.to_ascii_lowercase())
}
