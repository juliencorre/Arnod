//! Mises à jour (lot L allégé, écran 1.6-g) : Arnod cherche une version plus récente parmi les
//! releases publiées du dépôt, sur le canal choisi (stable ou préversions), au démarrage puis au
//! plus une fois par jour, ou à la demande. Rien n'est téléchargé ni installé : la page de la
//! release s'ouvre dans le navigateur du système. Les préférences sont propres au poste
//! (`updates.json` dans le dossier de données, écriture atomique).
//!
//! Le choix de la version (`pick`), l'échéance (`due`) et la lecture des notes (`highlights`)
//! sont purs ; la requête passe par `network::Network` (proxy et autorités du poste).

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

/// Canal suivi.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    #[default]
    Stable,
    Prerelease,
}

/// Préférences du poste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Version plus récente que celle installée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

/// Ce que montre la section « Mises à jour ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesView {
    /// Version installée.
    pub current: String,
    pub prefs: UpdatePrefs,
    /// Version plus récente trouvée à la dernière vérification de la session.
    pub latest: Option<Release>,
    /// La dernière vérification de la session a eu lieu (sinon `latest` ne dit rien).
    pub checked: bool,
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
        })
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
        }
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
}
