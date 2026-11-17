//! Mises à jour (écran 1.6-g) : Arnod cherche une version plus récente parmi les releases
//! publiées du dépôt, sur le canal choisi (stable ou préversions), au démarrage puis au plus une
//! fois par jour, ou à la demande (lot L allégé). Lot L (`docs/plan-mvp-l-mises-a-jour.md`) : une
//! release qui porte `latest.json` s'installe depuis Arnod ; sinon sa page s'ouvre dans le
//! navigateur du système. Les préférences sont propres au poste (`updates.json` dans le dossier de
//! données, écriture atomique).
//!
//! Le choix de la version (`pick`), l'échéance (`due`), la lecture des notes (`highlights`), le
//! contrôle du manifeste (`check_manifest`), la progression (`progress_due`) et les messages du
//! plugin (`install_message`) sont purs ; les requêtes passent par `network::Network` (proxy et
//! autorités du poste).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};

/// Adresse des releases du dépôt (API GitHub).
pub const RELEASES_API: &str =
    "https://api.github.com/repos/juliencorre/Arnod/releases?per_page=30";

/// Pages des releases : seules adresses ouvertes par « Voir la version ».
pub const RELEASES_PAGE: &str = "https://github.com/juliencorre/Arnod/releases";

/// Intervalle entre deux vérifications automatiques.
pub const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Nombre de points repris des notes de version.
const MAX_HIGHLIGHTS: usize = 5;

/// Manifeste du plugin joint à chaque release installable (R1).
pub const MANIFEST_NAME: &str = "latest.json";

/// Taille maximale d'un paquet téléchargé (R4).
pub const MAX_PACKAGE_BYTES: u64 = 200 * 1024 * 1024;

/// Taille maximale du manifeste.
pub const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

/// Un téléchargement sans donnée reçue pendant ce délai est arrêté.
pub const STALL_SECS: u64 = 60;

/// Sans taille annoncée, la progression est diffusée à chaque nouveau mégaoctet (R8).
const PROGRESS_STEP_BYTES: u64 = 1024 * 1024;

/// Canal suivi.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename = "UpdateChannel"))]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    #[default]
    Stable,
    Prerelease,
}

/// Préférences du poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrefs {
    /// Chercher au démarrage, puis chaque jour.
    #[serde(default = "yes")]
    pub automatic: bool,
    #[serde(default)]
    pub channel: Channel,
    /// Dernière vérification réussie (ms depuis l'époque).
    #[serde(default)]
    pub last_check: Option<u64>,
}

fn yes() -> bool {
    true
}

impl Default for UpdatePrefs {
    fn default() -> Self {
        Self {
            automatic: true,
            channel: Channel::Stable,
            last_check: None,
        }
    }
}

/// Release telle que l'API GitHub la décrit (champs lus seulement).
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct GhRelease {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    /// Fichiers joints (paquets, signatures, `latest.json`).
    #[serde(default)]
    pub assets: Vec<GhAsset>,
}

/// Fichier joint à une release (champs lus seulement).
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct GhAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Version plus récente que celle installée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// `0.10.0`, sans le `v`.
    pub version: String,
    pub prerelease: bool,
    /// Premiers points des notes de version.
    pub highlights: Vec<String>,
    /// Page de la release.
    pub url: String,
    /// Date de publication (`2026-11-14`).
    pub published: Option<String>,
    /// Adresse de son `latest.json`, absente si la release n'en porte pas (R1). Gardée par le
    /// cœur, jamais envoyée à l'interface.
    #[serde(skip)]
    pub manifest: Option<String>,
}

/// Installation de la version trouvée (R8).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum InstallState {
    /// Rien n'est téléchargé.
    #[default]
    Idle,
    /// Téléchargement en cours : octets reçus, taille annoncée.
    Downloading { received: u64, total: Option<u64> },
    /// Paquet téléchargé et vérifié ; `on_exit` : il s'installera à la fermeture (R9).
    #[serde(rename_all = "camelCase")]
    Ready { version: String, on_exit: bool },
    /// Échec du dernier téléchargement ou de l'installation, message prêt à afficher.
    Failed { message: String },
}

/// Ce que montre la section « Mises à jour ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UpdatesView {
    /// Version installée.
    pub current: String,
    pub prefs: UpdatePrefs,
    /// Version plus récente trouvée à la dernière vérification de la session.
    pub latest: Option<Release>,
    /// La dernière vérification de la session a eu lieu (sinon `latest` ne dit rien).
    pub checked: bool,
    /// `latest` s'installe depuis Arnod (R1) ; sinon « Voir la version ».
    pub installable: bool,
    /// Installation de `latest`.
    pub install: InstallState,
}

/// Version `x.y.z[-pré]` lue d'un tag (`v0.10.0`, `0.10.0-beta.2`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    core: (u64, u64, u64),
    /// Une version sans suffixe passe avant ses préversions (`None` > `Some`).
    pre: Pre,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Pre {
    Some(Vec<PreId>),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PreId {
    Number(u64),
    Text(String),
}

fn parse(tag: &str) -> Option<Version> {
    let text = tag.trim().trim_start_matches(['v', 'V']);
    let text = text.split('+').next()?;
    let (core, pre) = match text.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (text, None),
    };
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() {
        return None;
    }
    let pre = match pre {
        None => Pre::None,
        Some(pre) => Pre::Some(
            pre.split('.')
                .map(|id| {
                    id.parse::<u64>()
                        .map_or_else(|_| PreId::Text(id.to_owned()), PreId::Number)
                })
                .collect(),
        ),
    };
    Some(Version { core: version, pre })
}

/// Points des notes de version : lignes de liste (`- `, `* `), sans le marqueur ni le gras.
pub fn highlights(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .map(|item| item.replace("**", "").trim().to_owned())
        })
        .filter(|item| !item.is_empty())
        .take(MAX_HIGHLIGHTS)
        .collect()
}

/// Version la plus récente du canal, si elle dépasse `current` ; brouillons et tags illisibles
/// ignorés. Le canal « préversions » propose aussi les versions stables.
pub fn pick(releases: &[GhRelease], channel: Channel, current: &str) -> Option<Release> {
    let installed = parse(current)?;
    releases
        .iter()
        .filter(|r| !r.draft && (channel == Channel::Prerelease || !r.prerelease))
        .filter_map(|r| parse(&r.tag_name).map(|v| (v, r)))
        .filter(|(v, _)| *v > installed)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, r)| Release {
            version: r.tag_name.trim_start_matches(['v', 'V']).to_owned(),
            prerelease: r.prerelease,
            highlights: highlights(r.body.as_deref().unwrap_or("")),
            url: r.html_url.clone(),
            published: r
                .published_at
                .as_deref()
                .and_then(|d| d.get(..10))
                .map(str::to_owned),
            manifest: r
                .assets
                .iter()
                .find(|a| a.name == MANIFEST_NAME)
                .map(|a| a.browser_download_url.clone()),
        })
}

/// Lit le manifeste d'une release (R2) : JSON, et version annoncée égale à `version`, celle de
/// la release trouvée. Rendu tel quel pour le plugin (`Updater::restore_update`).
pub fn check_manifest(body: &[u8], version: &str) -> Result<serde_json::Value, CommandError> {
    let unreadable = || {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "Le fichier de mise à jour d'Arnod {version} est illisible : rien n'a été téléchargé. Réessayez plus tard ou passez par la page de la version."
            ),
        )
    };
    let manifest: serde_json::Value = serde_json::from_slice(body).map_err(|_| unreadable())?;
    let announced = manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(unreadable)?;
    match (parse(announced), parse(version)) {
        (Some(a), Some(b)) if a == b => Ok(manifest),
        _ => Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "Le fichier de mise à jour annonce la version {announced} au lieu de {version} : rien n'a été téléchargé."
            ),
        )),
    }
}

/// La version trouvée s'installe depuis Arnod (R1) : elle porte un manifeste, Arnod embarque une
/// clé publique et ce poste est installé par un paquet que le plugin remplace (`platform`).
pub fn installable(release: &Release, pubkey: &str, platform: bool) -> bool {
    release.manifest.is_some() && !pubkey.trim().is_empty() && platform
}

/// La progression mérite d'être diffusée (R8) : au moins un pour cent de plus que la dernière
/// diffusion (`shown` octets), ou un mégaoctet sans taille annoncée, ou la fin.
pub fn progress_due(shown: u64, received: u64, total: Option<u64>) -> bool {
    match total {
        Some(total) if total > 0 => {
            received >= total
                || received.saturating_mul(100) / total > shown.saturating_mul(100) / total
        }
        _ => received / PROGRESS_STEP_BYTES > shown / PROGRESS_STEP_BYTES,
    }
}

/// Message d'un échec du plugin, prêt à afficher (R3, R7). Une signature refusée le dit : rien
/// n'a été installé.
pub fn install_message(error: &tauri_plugin_updater::Error, version: &str) -> String {
    use tauri_plugin_updater::Error as E;
    match error {
        E::Minisign(_) | E::Base64(_) | E::SignatureUtf8(_) => format!(
            "La signature d'Arnod {version} ne correspond pas à la clé d'Arnod : le fichier téléchargé est refusé, rien n'a été installé."
        ),
        E::SignedVersionMismatch { signed, .. } => format!(
            "Le fichier téléchargé est signé pour la version {signed}, pas pour {version} : il est refusé, rien n'a été installé."
        ),
        E::MissingSignedVersion => format!(
            "La signature d'Arnod {version} ne dit pas sa version : le fichier téléchargé est refusé, rien n'a été installé."
        ),
        E::TargetNotFound(_) | E::TargetsNotFound(_) => format!(
            "Arnod {version} n'est pas publiée pour ce système : passez par la page de la version."
        ),
        E::InvalidUpdaterFormat | E::BinaryNotFoundInArchive => format!(
            "Le fichier d'Arnod {version} n'est pas un installeur reconnu : rien n'a été installé."
        ),
        E::PreviousAppNotRestored(path) => format!(
            "L'installation d'Arnod {version} a échoué et l'ancienne version n'a pas pu être remise en place : elle est gardée dans {}.",
            path.display()
        ),
        E::Io(io) => format!(
            "L'installation d'Arnod {version} a échoué ({io}) : la version installée reste en place."
        ),
        other => format!(
            "L'installation d'Arnod {version} a échoué ({other}) : la version installée reste en place."
        ),
    }
}

/// Une vérification automatique est due : activée, et jamais faite ou faite il y a un jour.
pub fn due(prefs: &UpdatePrefs, now: u64) -> bool {
    prefs.automatic
        && prefs
            .last_check
            .is_none_or(|last| now.saturating_sub(last) >= DAY_MS)
}

/// Lit la réponse de l'API des releases.
pub fn read_releases(status: u16, body: &str) -> Result<Vec<GhRelease>, CommandError> {
    if status != 200 {
        return Err(CommandError::new(
            CommandErrorKind::Network,
            format!(
                "Les versions publiées n'ont pas pu être lues (réponse {status} de GitHub). Réessayez plus tard."
            ),
        ));
    }
    serde_json::from_str(body).map_err(|_| {
        CommandError::new(
            CommandErrorKind::Network,
            "La liste des versions publiées est illisible : réessayez plus tard.",
        )
    })
}

/// Seules les pages des releases du dépôt s'ouvrent.
pub fn openable(url: &str) -> bool {
    url == RELEASES_PAGE || url.starts_with(&format!("{RELEASES_PAGE}/"))
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("updates.json")
}

/// Préférences du poste ; un fichier absent ou illisible donne les valeurs par défaut.
pub fn load(data_dir: &Path) -> UpdatePrefs {
    fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Écrit les préférences de façon atomique.
pub fn save(data_dir: &Path, prefs: &UpdatePrefs) -> Result<(), CommandError> {
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

    fn release(tag: &str, prerelease: bool) -> GhRelease {
        GhRelease {
            tag_name: tag.to_owned(),
            name: None,
            body: Some("Nouveautés\n\n- **Copier et déplacer** une requête\n* Filtre\n".to_owned()),
            html_url: format!("{RELEASES_PAGE}/tag/{tag}"),
            draft: false,
            prerelease,
            published_at: Some("2026-11-14T09:12:00Z".to_owned()),
            assets: Vec::new(),
        }
    }

    fn with_manifest(mut release: GhRelease) -> GhRelease {
        release.assets = vec![
            GhAsset {
                name: "Arnod_0.10.0_x64-setup.exe".to_owned(),
                browser_download_url: "https://exemple.fr/setup.exe".to_owned(),
            },
            GhAsset {
                name: MANIFEST_NAME.to_owned(),
                browser_download_url: "https://exemple.fr/latest.json".to_owned(),
            },
        ];
        release
    }

    #[test]
    fn l1_la_version_la_plus_recente_du_canal() {
        let all = [
            release("v0.9.0", false),
            release("v0.10.0", false),
            release("v0.11.0-beta.2", true),
            release("v0.11.0-beta.10", true),
            release("pas-une-version", false),
        ];
        let stable = pick(&all, Channel::Stable, "0.9.0").unwrap();
        let pre = pick(&all, Channel::Prerelease, "0.9.0").unwrap();
        assert_eq!(
            (
                stable.version.as_str(),
                pre.version.as_str(),
                pre.prerelease
            ),
            ("0.10.0", "0.11.0-beta.10", true)
        );
        assert_eq!(
            stable.highlights,
            ["Copier et déplacer une requête", "Filtre"]
        );
        assert_eq!(stable.published.as_deref(), Some("2026-11-14"));
    }

    #[test]
    fn l2_rien_quand_a_jour_et_brouillons_ignores() {
        let mut draft = release("v1.0.0", false);
        draft.draft = true;
        assert_eq!(
            pick(&[draft, release("v0.9.0", false)], Channel::Stable, "0.9.0"),
            None
        );
        // Une version stable passe avant ses préversions.
        assert_eq!(
            pick(
                &[release("v0.10.0-rc.1", true)],
                Channel::Prerelease,
                "0.10.0"
            ),
            None
        );
    }

    #[test]
    fn l3_echeance_d_une_verification_automatique() {
        let mut prefs = UpdatePrefs::default();
        assert!(due(&prefs, 5));
        prefs.last_check = Some(1_000);
        assert!(!due(&prefs, 1_000 + DAY_MS - 1));
        assert!(due(&prefs, 1_000 + DAY_MS));
        prefs.automatic = false;
        assert!(!due(&prefs, 1_000 + 2 * DAY_MS));
    }

    #[test]
    fn l4_preferences_ecrites_et_relues() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()), UpdatePrefs::default());
        let prefs = UpdatePrefs {
            automatic: false,
            channel: Channel::Prerelease,
            last_check: Some(42),
        };
        save(dir.path(), &prefs).unwrap();
        assert_eq!(load(dir.path()), prefs);
        assert_eq!(
            serde_json::to_value(&prefs).unwrap(),
            serde_json::json!({ "automatic": false, "channel": "prerelease", "lastCheck": 42 })
        );
    }

    #[test]
    fn l5_reponse_de_github_et_adresses_ouvertes() {
        assert!(read_releases(403, "").is_err());
        assert!(read_releases(200, "pas du json").is_err());
        assert_eq!(
            read_releases(200, r#"[{"tag_name":"v1.0.0","html_url":"x"}]"#)
                .unwrap()
                .len(),
            1
        );
        assert!(openable(&format!("{RELEASES_PAGE}/tag/v1.0.0")));
        assert!(!openable("https://exemple.fr/releases"));
        assert!(!openable("https://github.com/juliencorre/Arnod/releasesx"));
    }

    #[test]
    fn r1_manifeste_de_la_release_garde_par_le_coeur() {
        let found = pick(
            &[with_manifest(release("v0.10.0", false))],
            Channel::Stable,
            "0.9.0",
        )
        .unwrap();
        assert_eq!(
            found.manifest.as_deref(),
            Some("https://exemple.fr/latest.json")
        );
        // Jamais envoyé à l'interface.
        assert!(
            serde_json::to_value(&found)
                .unwrap()
                .get("manifest")
                .is_none()
        );
        assert!(installable(&found, "clé", true));
        assert!(!installable(&found, "  ", true));
        assert!(!installable(&found, "clé", false));

        let without = pick(&[release("v0.10.0", false)], Channel::Stable, "0.9.0").unwrap();
        assert_eq!(without.manifest, None);
        assert!(!installable(&without, "clé", true));
    }

    #[test]
    fn r2_manifeste_de_la_version_trouvee_seulement() {
        let body = br#"{"version":"v0.10.0","platforms":{}}"#;
        assert!(check_manifest(body, "0.10.0").is_ok());
        let other = check_manifest(br#"{"version":"0.9.5"}"#, "0.10.0").unwrap_err();
        assert_eq!(other.kind, CommandErrorKind::InvalidRequest);
        assert!(
            other
                .message
                .contains("annonce la version 0.9.5 au lieu de 0.10.0")
        );
        assert!(
            check_manifest(b"<html>", "0.10.0")
                .unwrap_err()
                .message
                .contains("illisible")
        );
        assert!(check_manifest(br#"{"platforms":{}}"#, "0.10.0").is_err());
    }

    #[test]
    fn r8_progression_diffusee_par_pour_cent_ou_par_megaoctet() {
        let total = Some(1_000);
        assert!(!progress_due(0, 9, total));
        assert!(progress_due(0, 10, total));
        assert!(!progress_due(10, 19, total));
        assert!(progress_due(990, 1_000, total));
        let mo = 1024 * 1024;
        assert!(!progress_due(0, mo - 1, None));
        assert!(progress_due(mo - 1, mo, None));
        assert!(!progress_due(mo, 2 * mo - 1, None));
    }

    #[test]
    fn r8_etat_de_l_installation_en_json() {
        assert_eq!(
            serde_json::to_value(InstallState::Idle).unwrap(),
            serde_json::json!({ "state": "idle" })
        );
        assert_eq!(
            serde_json::to_value(InstallState::Downloading {
                received: 3,
                total: None
            })
            .unwrap(),
            serde_json::json!({ "state": "downloading", "received": 3, "total": null })
        );
        assert_eq!(
            serde_json::to_value(InstallState::Ready {
                version: "0.10.0".to_owned(),
                on_exit: true
            })
            .unwrap(),
            serde_json::json!({ "state": "ready", "version": "0.10.0", "onExit": true })
        );
        assert_eq!(
            serde_json::to_value(InstallState::Failed {
                message: "Refusé".to_owned()
            })
            .unwrap(),
            serde_json::json!({ "state": "failed", "message": "Refusé" })
        );
    }
}
