//! Démarrage : erreurs bloquantes expliquées (lot G, écrans 0.3-b, 0.3-b bis et 0.3-b ter).
//!
//! La lecture d'`arnod.json` ([`crate::storage`]) ou de `windows.json` ([`crate::windows`]) peut
//! échouer : fichier écrit par une version plus récente, illisible, verrouillé, ou dossier de
//! données inaccessible. L'échec est classé ([`Failure`]), puis décrit pour l'utilisateur
//! ([`describe`]) : la cause en clair, ce qui est intact, deux issues dont la recommandée en premier,
//! et les détails pour le support. Tout est pur ici, sauf [`set_aside`] (renommage, jamais de
//! suppression) et [`open_external`] (navigateur ou explorateur du système, sans plugin Tauri).
//!
//! Plan : `docs/plan-mvp-g-demarrage-erreurs.md`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;

use crate::error::{CommandError, CommandErrorKind};

/// Dépôt d'Arnod : page des versions et issues préremplies.
pub const REPOSITORY: &str = "https://github.com/juliencorre/Arnod";

/// Délais, en secondes, des essais automatiques d'un fichier verrouillé (R4).
pub const RETRY_DELAYS: [u64; 3] = [2, 4, 8];

/// Fichier d'état concerné.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateFile {
    /// `arnod.json` : projets, workspaces.
    Catalog,
    /// `windows.json` : fenêtres, zones et onglets.
    Windows,
}

/// Cause d'un échec de lecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// Format plus récent que ceux lus ici.
    Newer {
        format: u64,
        /// Version d'Arnod qui l'a écrit, si le fichier la donne (`writtenBy`).
        written_by: Option<String>,
        /// Dernière modification, en secondes depuis 1970.
        modified: Option<u64>,
    },
    /// Contenu illisible : JSON invalide, version absente, contenu incohérent.
    Corrupt { reason: String },
    /// Accès refusé, ou fichier tenu par un autre programme.
    Locked { code: Option<i32>, error: String },
    /// Autre erreur d'accès : dossier de données absent, disque, chemin.
    Inaccessible { code: Option<i32>, error: String },
}

/// Échec de lecture d'un fichier d'état au démarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub file: StateFile,
    /// Fichier concerné, ou dossier de données s'il ne peut pas être créé.
    pub path: PathBuf,
    pub cause: Cause,
}

/// Une erreur d'accès est-elle un verrou ou un refus d'accès ? Sous Windows, les codes 32
/// (`ERROR_SHARING_VIOLATION`) et 33 (`ERROR_LOCK_VIOLATION`) disent qu'un autre programme tient
/// le fichier ; le code 5 (`ERROR_ACCESS_DENIED`) donne `PermissionDenied`.
pub fn is_lock(kind: io::ErrorKind, code: Option<i32>, windows: bool) -> bool {
    kind == io::ErrorKind::PermissionDenied || (windows && matches!(code, Some(32 | 33)))
}

impl Failure {
    /// Échec d'entrée-sortie sur `path`, classé verrou ou accès impossible.
    pub fn io(file: StateFile, path: &Path, error: &io::Error) -> Self {
        let code = error.raw_os_error();
        let text = error.to_string();
        let cause = if is_lock(error.kind(), code, cfg!(windows)) {
            Cause::Locked { code, error: text }
        } else {
            Cause::Inaccessible { code, error: text }
        };
        Self {
            file,
            path: path.to_path_buf(),
            cause,
        }
    }

    /// Nature de l'échec, telle que l'interface la reçoit.
    pub fn nature(&self) -> ProblemNature {
        match self.cause {
            Cause::Newer { .. } => ProblemNature::NewerVersion,
            Cause::Corrupt { .. } => ProblemNature::CorruptState,
            Cause::Locked { .. } => ProblemNature::Locked,
            Cause::Inaccessible { .. } => ProblemNature::Inaccessible,
        }
    }

    /// Erreur renvoyée par les commandes tant que l'échec n'est pas réglé.
    pub fn to_command_error(&self) -> CommandError {
        let kind = match self.cause {
            Cause::Newer { .. } => CommandErrorKind::NewerVersion,
            Cause::Corrupt { .. } => CommandErrorKind::CorruptState,
            Cause::Locked { .. } | Cause::Inaccessible { .. } => CommandErrorKind::Io,
        };
        CommandError::new(
            kind,
            format!(
                "{} ({}) : {}",
                title(self),
                self.path.display(),
                technical(&self.cause)
            ),
        )
    }

    /// Suffixe du fichier mis de côté par « Repartir d'un état vide », s'il peut l'être (R3).
    pub fn aside_suffix(&self) -> Option<String> {
        match &self.cause {
            Cause::Newer { format, .. } => Some(format!("format-{format}")),
            Cause::Corrupt { .. } => Some("illisible".to_owned()),
            Cause::Locked { .. } | Cause::Inaccessible { .. } => None,
        }
    }
}

/// Nature d'une erreur bloquante (miroir `ProblemNature` de `src/types/ipc.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemNature {
    NewerVersion,
    CorruptState,
    Locked,
    Inaccessible,
}

/// Issue proposée (miroir `StartupAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupAction {
    /// Ouvre la page des versions dans le navigateur.
    OpenReleases,
    /// Met le fichier de côté (renommé) et repart d'un état vide.
    StartEmpty,
    /// Relit le fichier.
    Retry,
    /// Ouvre le dossier de données dans l'explorateur.
    OpenFolder,
}

/// Cible ouverte dans le système (miroir `StartupTarget`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupTarget {
    /// Dossier de données.
    Folder,
    /// Page des versions.
    Releases,
    /// Issue GitHub préremplie : rien n'est envoyé sans l'utilisateur.
    Issue,
}

/// Une issue : ce qu'elle fait et ses conséquences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupOption {
    pub action: StartupAction,
    pub title: String,
    /// Conséquences de l'issue.
    pub description: String,
    /// Libellé du bouton.
    pub button: String,
    /// La recommandée, toujours en premier (R2).
    pub recommended: bool,
    /// Ligne de plus (essais automatiques épuisés).
    pub note: Option<String>,
}

/// Prochain essai automatique (R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryPlan {
    /// Numéro de l'essai à venir, à partir de 1.
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_seconds: u64,
}

/// Ligne des détails pour le support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub label: String,
    pub value: String,
}

/// Erreur bloquante décrite pour l'écran de démarrage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupProblem {
    pub nature: ProblemNature,
    /// Nom du fichier concerné (`arnod.json`, `windows.json`).
    pub file: String,
    pub title: String,
    /// La cause en clair.
    pub message: String,
    /// Ce qui est intact : début en gras, puis la suite.
    pub intact_lead: String,
    pub intact_text: String,
    /// Deux issues, la recommandée en premier.
    pub options: Vec<StartupOption>,
    /// Libellé du chemin (« Fichier d'état »).
    pub path_label: String,
    pub path: String,
    /// Prochain essai automatique, pour un verrou ou un accès refusé.
    pub retry: Option<RetryPlan>,
    /// Détails pour le support, repliés par défaut.
    pub details: Vec<DetailRow>,
    /// Les mêmes détails en texte, pour « Copier le détail ».
    pub support_text: String,
    /// Adresse de l'issue préremplie.
    pub issue_url: String,
    /// Version et système, en pied d'écran.
    pub footer: String,
}

/// Poste et application, pour les détails (lus par la commande).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub version: String,
    /// `windows`, `linux`, `macos` (`std::env::consts::OS`).
    pub os: String,
    pub arch: String,
    pub webview: Option<String>,
    /// Maintenant, en secondes depuis 1970.
    pub now: u64,
}

/// Formats lus par cette version, pour un fichier.
fn read_formats(file: StateFile) -> String {
    let last = match file {
        StateFile::Catalog => u64::from(crate::catalog::FILE_VERSION),
        StateFile::Windows => u64::from(crate::layout::windows::WINDOWS_VERSION),
    };
    let all: Vec<String> = (1..=last).map(|v| v.to_string()).collect();
    match all.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} et {last}", rest.join(", ")),
        _ => all.join(""),
    }
}

fn file_name(file: StateFile) -> &'static str {
    match file {
        StateFile::Catalog => crate::storage::STATE_FILE_NAME,
        StateFile::Windows => crate::windows::WINDOWS_FILE,
    }
}

/// Nom affiché d'un système (`Windows`, `Linux`, `macOS`).
fn os_name(os: &str) -> &str {
    match os {
        "windows" => "Windows",
        "linux" => "Linux",
        "macos" => "macOS",
        other => other,
    }
}

fn title(failure: &Failure) -> String {
    let catalog = failure.file == StateFile::Catalog;
    match (&failure.cause, catalog) {
        (Cause::Newer { .. }, true) => "Cet état vient d'une version plus récente d'Arnod",
        (Cause::Newer { .. }, false) => {
            "Cette disposition vient d'une version plus récente d'Arnod"
        }
        (Cause::Corrupt { .. }, true) => "L'état d'Arnod est illisible",
        (Cause::Corrupt { .. }, false) => "La disposition des fenêtres est illisible",
        (Cause::Locked { .. }, true) => "Arnod n'arrive pas à lire son état",
        (Cause::Locked { .. }, false) => "Arnod n'arrive pas à lire la disposition de ses fenêtres",
        (Cause::Inaccessible { .. }, _) => "Arnod n'arrive pas à accéder à son dossier de données",
    }
    .to_owned()
}

fn technical(cause: &Cause) -> String {
    match cause {
        Cause::Newer { format, .. } => format!("format {format}"),
        Cause::Corrupt { reason } => reason.clone(),
        Cause::Locked { error, .. } | Cause::Inaccessible { error, .. } => error.clone(),
    }
}

/// « (erreur système 5) », si le système donne un code.
fn system_code(code: Option<i32>) -> String {
    code.map(|c| format!(" (erreur système {c})"))
        .unwrap_or_default()
}

/// Nom libre pour mettre `path` de côté : `<nom>.<suffixe>`, sinon `<nom>.<suffixe>-2`, `-3`…
pub fn aside_path(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut candidate = path.with_file_name(format!("{name}.{suffix}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = path.with_file_name(format!("{name}.{suffix}-{n}"));
        n += 1;
    }
    candidate
}

/// Met le fichier de côté sous un nom libre : renommé, jamais supprimé ni réécrit (R3).
pub fn set_aside(path: &Path, suffix: &str) -> Result<PathBuf, CommandError> {
    let target = aside_path(path, suffix);
    fs::rename(path, &target).map_err(|error| {
        CommandError::io(
            "Mise de côté du fichier impossible, rien n'a été modifié",
            path,
            &error,
        )
    })?;
    Ok(target)
}

/// Prochain essai automatique après `done` essais, pour un verrou ou un accès refusé (R4).
pub fn retry_plan(cause: &Cause, done: u32) -> Option<RetryPlan> {
    if !matches!(cause, Cause::Locked { .. } | Cause::Inaccessible { .. }) {
        return None;
    }
    let delay = RETRY_DELAYS.get(usize::try_from(done).ok()?)?;
    Some(RetryPlan {
        attempt: done + 1,
        max_attempts: u32::try_from(RETRY_DELAYS.len()).unwrap_or(u32::MAX),
        delay_seconds: *delay,
    })
}

/// Décrit un échec pour l'écran de démarrage, après `attempts` essais automatiques.
pub fn describe(failure: &Failure, env: &Env, attempts: u32) -> StartupProblem {
    let catalog = failure.file == StateFile::Catalog;
    let name = file_name(failure.file);
    let shown = aside_name(failure);
    let (message, intact_text, options) = match &failure.cause {
        Cause::Newer {
            format, written_by, ..
        } => newer(catalog, *format, written_by.as_deref(), env, &shown),
        Cause::Corrupt { reason } => corrupt(catalog, name, reason, &shown),
        Cause::Locked { code, .. } => locked(catalog, *code, attempts),
        Cause::Inaccessible { error, .. } => inaccessible(failure, error, attempts),
    };
    let path_label = match (&failure.cause, catalog) {
        (Cause::Inaccessible { .. }, _) if failure.path.extension().is_none() => {
            "Dossier de données"
        }
        (_, true) => "Fichier d'état",
        (_, false) => "Fichier des fenêtres",
    };
    let title = title(failure);
    let details = details(failure, env, attempts);
    let support_text = support_text(&title, &details);
    StartupProblem {
        nature: failure.nature(),
        file: name.to_owned(),
        issue_url: issue_url(&title, &message, &details),
        title,
        message,
        intact_lead: "Rien n'a été modifié.".to_owned(),
        intact_text,
        options,
        path_label: path_label.to_owned(),
        path: failure.path.display().to_string(),
        retry: retry_plan(&failure.cause, attempts),
        details,
        support_text,
        footer: format!("Arnod {} · {}", env.version, os_name(&env.os)),
    }
}

/// Nom sous lequel le fichier serait mis de côté, s'il peut l'être.
fn aside_name(failure: &Failure) -> String {
    failure
        .aside_suffix()
        .map(|suffix| {
            aside_path(&failure.path, &suffix)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

type Described = (String, String, Vec<StartupOption>);

fn option(
    action: StartupAction,
    title: impl Into<String>,
    description: impl Into<String>,
    button: impl Into<String>,
    recommended: bool,
) -> StartupOption {
    StartupOption {
        action,
        title: title.into(),
        description: description.into(),
        button: button.into(),
        recommended,
        note: None,
    }
}

fn start_empty(catalog: bool, shown: &str) -> StartupOption {
    if catalog {
        option(
            StartupAction::StartEmpty,
            "Repartir d'un état vide",
            format!(
                "L'état actuel est mis de côté sous {shown}, sans être modifié. Arnod démarre sans \
                 projet ; rouvrez les vôtres avec « Ouvrir un projet »."
            ),
            "Repartir d'un état vide",
            false,
        )
    } else {
        option(
            StartupAction::StartEmpty,
            "Repartir d'une disposition vide",
            format!(
                "La disposition actuelle est mise de côté sous {shown}, sans être modifiée. Vos \
                 projets et vos workspaces restent ; seuls les onglets sont à rouvrir."
            ),
            "Repartir d'une disposition vide",
            false,
        )
    }
}

fn intact_state(catalog: bool) -> String {
    if catalog {
        "Ce fichier est intact, et vos projets aussi : leurs dossiers et leurs fichiers ne \
         dépendent pas de cet état."
    } else {
        "Ce fichier est intact, vos projets et vos workspaces aussi : seule la disposition des \
         fenêtres et des onglets en dépend."
    }
    .to_owned()
}

fn newer(catalog: bool, format: u64, by: Option<&str>, env: &Env, shown: &str) -> Described {
    let file = if catalog {
        "Le fichier d'état"
    } else {
        "Le fichier des fenêtres"
    };
    let writer = by.map_or_else(
        || "une version plus récente d'Arnod".to_owned(),
        |v| format!("Arnod {v}"),
    );
    let formats = read_formats(if catalog {
        StateFile::Catalog
    } else {
        StateFile::Windows
    });
    let read = if formats.contains(" et ") {
        format!("les formats {formats}")
    } else {
        format!("le format {formats}")
    };
    let message = format!(
        "{file} a été écrit par {writer} (format {format}). Vous lancez Arnod {}, qui lit {read} : \
         il ne l'ouvre pas, pour ne rien abîmer.",
        env.version
    );
    let install = by.map_or_else(
        || "Installer la dernière version d'Arnod".to_owned(),
        |v| format!("Installer Arnod {v} ou plus récent"),
    );
    let mut empty = start_empty(catalog, shown);
    empty.recommended = false;
    let options = vec![
        option(
            StartupAction::OpenReleases,
            install,
            if catalog {
                "La nouvelle version relit cet état tel quel : projets, workspaces et onglets \
                 reviennent."
            } else {
                "La nouvelle version relit cette disposition telle quelle : fenêtres et onglets \
                 reviennent."
            },
            "Ouvrir la page des versions",
            true,
        ),
        empty,
    ];
    (message, intact_state(catalog), options)
}

fn corrupt(catalog: bool, name: &str, reason: &str, shown: &str) -> Described {
    let file = if catalog {
        "Le fichier d'état"
    } else {
        "Le fichier des fenêtres"
    };
    let message = format!(
        "{file} n'est pas lisible ({reason}). Il a peut-être été coupé par un arrêt brutal ou \
         modifié à la main : Arnod ne l'ouvre pas et ne l'écrase pas."
    );
    let mut empty = start_empty(catalog, shown);
    empty.recommended = true;
    let options = vec![
        empty,
        option(
            StartupAction::Retry,
            "Corriger le fichier",
            format!(
                "Ouvrez {name} dans un éditeur de texte et corrigez-le, puis réessayez : Arnod le \
                 relit tel quel."
            ),
            "Réessayer",
            false,
        ),
    ];
    (message, intact_state(catalog), options)
}

/// Issue « Réessayer », recommandée, avec la note des essais épuisés.
fn retry_option(description: &str, attempts: u32) -> StartupOption {
    let mut retry = option(
        StartupAction::Retry,
        "Réessayer",
        description,
        "Réessayer",
        true,
    );
    if attempts > 0 && usize::try_from(attempts).is_ok_and(|n| n >= RETRY_DELAYS.len()) {
        retry.note = Some(format!(
            "Arnod a réessayé seul {attempts} fois, sans succès."
        ));
    }
    retry
}

fn check_folder() -> StartupOption {
    option(
        StartupAction::OpenFolder,
        "Vérifier le dossier de données",
        "Excluez-le de la synchronisation, ou vérifiez que votre compte peut lire et écrire ce \
         fichier.",
        "Ouvrir le dossier",
        false,
    )
}

fn locked(catalog: bool, code: Option<i32>, attempts: u32) -> Described {
    let file = if catalog {
        "au fichier d'état"
    } else {
        "au fichier des fenêtres"
    };
    let message = format!(
        "Accès refusé {file}{}. Il est sans doute ouvert par un autre programme : un antivirus qui \
         l'analyse, ou une synchronisation (OneDrive, Dropbox) de ce dossier.",
        system_code(code)
    );
    let options = vec![
        retry_option(
            "Le plus souvent, le verrou se libère en quelques secondes.",
            attempts,
        ),
        check_folder(),
    ];
    (
        message,
        "Arnod n'a rien écrit ; vos projets ne dépendent pas de ce fichier.".to_owned(),
        options,
    )
}

fn inaccessible(failure: &Failure, error: &str, attempts: u32) -> Described {
    let message = format!(
        "{error}. Le dossier de données est peut-être sur un disque absent, ou votre compte n'a \
         pas le droit d'y écrire."
    );
    let mut folder = check_folder();
    folder.description = "Vérifiez que ce dossier existe et que votre compte peut y lire et y \
                          écrire."
        .to_owned();
    let options = vec![
        retry_option(
            "Une fois le disque branché ou les droits rétablis, Arnod repart.",
            attempts,
        ),
        folder,
    ];
    let intact = if failure.file == StateFile::Catalog {
        "Arnod n'a rien écrit ; vos projets ne dépendent pas de ce dossier."
    } else {
        "Arnod n'a rien écrit ; vos projets et vos workspaces sont intacts."
    };
    (message, intact.to_owned(), options)
}

fn details(failure: &Failure, env: &Env, attempts: u32) -> Vec<DetailRow> {
    let row = |label: &str, value: String| DetailRow {
        label: label.to_owned(),
        value,
    };
    let mut rows = vec![
        row("Nature", serialized(failure.nature())),
        row("Fichier", failure.path.display().to_string()),
    ];
    match &failure.cause {
        Cause::Newer {
            format,
            written_by,
            modified,
        } => {
            let mut origin = Vec::new();
            if let Some(by) = written_by {
                origin.push(format!("écrit par Arnod {by}"));
            }
            if let Some(at) = modified {
                origin.push(format!("le {}", format_day(*at)));
            }
            let value = if origin.is_empty() {
                format.to_string()
            } else {
                format!("{format} ({})", origin.join(", "))
            };
            rows.push(row("Format du fichier", value));
            rows.push(row("Formats lus ici", read_formats(failure.file)));
        }
        Cause::Corrupt { reason } => rows.push(row("Erreur technique", reason.clone())),
        Cause::Locked { code, error } | Cause::Inaccessible { code, error } => {
            let code = code.map(|c| format!(" (code {c})")).unwrap_or_default();
            rows.push(row("Erreur technique", format!("{error}{code}")));
        }
    }
    let mut system = format!("{} · {} {}", env.version, os_name(&env.os), env.arch);
    if let Some(webview) = &env.webview {
        system.push_str(&format!(" · WebView {webview}"));
    }
    rows.push(row("Arnod", system));
    if attempts > 0 {
        rows.push(row("Essais automatiques", attempts.to_string()));
    }
    rows.push(row("Date", format!("{} UTC", format_time(env.now))));
    rows
}

fn serialized(nature: ProblemNature) -> String {
    match serde_json::to_value(nature) {
        Ok(serde_json::Value::String(name)) => name,
        _ => String::new(),
    }
}

/// Détails en texte, prêts à coller.
pub fn support_text(title: &str, details: &[DetailRow]) -> String {
    let width = details
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0);
    let mut text = format!("Arnod ne démarre pas : {title}\n\n");
    for row in details {
        let pad = width - row.label.chars().count();
        text.push_str(&format!(
            "{}{}  {}\n",
            row.label,
            " ".repeat(pad),
            row.value
        ));
    }
    text
}

/// Adresse d'une issue préremplie : ouverte dans le navigateur, rien n'est envoyé sans
/// l'utilisateur (R6).
pub fn issue_url(title: &str, message: &str, details: &[DetailRow]) -> String {
    let rows: Vec<String> = details
        .iter()
        .map(|row| format!("{} : {}", row.label, row.value))
        .collect();
    let body = format!(
        "## Ce qui s'est passé\n\n{message}\n\n## Détails\n\n```\n{}\n```\n\n\
         Relisez ces détails avant d'envoyer : un chemin peut contenir votre nom d'utilisateur.\n",
        rows.join("\n")
    );
    let encode = |text: &str| utf8_percent_encode(text, NON_ALPHANUMERIC).to_string();
    format!(
        "{REPOSITORY}/issues/new?title={}&body={}",
        encode(&format!("Démarrage : {title}")),
        encode(&body)
    )
}

/// Date civile (année, mois, jour) d'un nombre de jours depuis 1970 (algorithme de H. Hinnant).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// « 08/11/2026 », en UTC.
pub fn format_day(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let (y, m, d) = civil(days);
    format!("{d:02}/{m:02}/{y}")
}

/// « 08/11/2026 09:12:04 », en UTC.
pub fn format_time(secs: u64) -> String {
    let rest = secs % 86_400;
    format!(
        "{} {:02}:{:02}:{:02}",
        format_day(secs),
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Ouvre une adresse ou un dossier avec le programme du système (navigateur, explorateur),
/// sans plugin Tauri ni permission WebView. Chemins absolus : pas de recherche dans `PATH`.
pub fn open_external(target: &str) -> io::Result<()> {
    let program = if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("explorer.exe")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/usr/bin/open")
    } else {
        PathBuf::from("/usr/bin/xdg-open")
    };
    Command::new(program).arg(target).spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ts_contract::{serialized_name, union_members};

    fn env() -> Env {
        Env {
            version: "0.9.0".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            webview: Some("131".into()),
            // 10/11/2026 09:12:04 UTC
            now: 1_794_301_924,
        }
    }

    fn failure(cause: Cause) -> Failure {
        Failure {
            file: StateFile::Catalog,
            path: PathBuf::from("/donnees/arnod.json"),
            cause,
        }
    }

    fn newer() -> Failure {
        failure(Cause::Newer {
            format: 3,
            written_by: Some("1.0.2".into()),
            // 08/11/2026
            modified: Some(1_794_100_000),
        })
    }

    fn locked() -> Failure {
        failure(Cause::Locked {
            code: Some(5),
            error: "Accès refusé".into(),
        })
    }

    fn actions(problem: &StartupProblem) -> Vec<(StartupAction, bool)> {
        problem
            .options
            .iter()
            .map(|o| (o.action, o.recommended))
            .collect()
    }

    fn detail<'a>(problem: &'a StartupProblem, label: &str) -> &'a str {
        &problem
            .details
            .iter()
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("ligne « {label} » absente"))
            .value
    }

    #[test]
    fn r1_verrou_ou_acces_refuse() {
        assert!(is_lock(io::ErrorKind::PermissionDenied, Some(5), true));
        assert!(is_lock(io::ErrorKind::Other, Some(32), true));
        assert!(is_lock(io::ErrorKind::Other, Some(33), true));
        assert!(!is_lock(io::ErrorKind::Other, Some(32), false));
        assert!(!is_lock(io::ErrorKind::NotADirectory, None, false));

        let refused = io::Error::from(io::ErrorKind::PermissionDenied);
        let failure = Failure::io(StateFile::Windows, Path::new("/d/windows.json"), &refused);
        assert_eq!(failure.nature(), ProblemNature::Locked);
        let other = io::Error::other("disque absent");
        let failure = Failure::io(StateFile::Catalog, Path::new("/d"), &other);
        assert_eq!(failure.nature(), ProblemNature::Inaccessible);
    }

    #[test]
    fn r2_version_plus_recente_deux_issues_la_recommandee_en_premier() {
        let problem = describe(&newer(), &env(), 0);

        assert_eq!(
            problem.title,
            "Cet état vient d'une version plus récente d'Arnod"
        );
        assert_eq!(
            problem.message,
            "Le fichier d'état a été écrit par Arnod 1.0.2 (format 3). Vous lancez Arnod 0.9.0, \
             qui lit les formats 1 et 2 : il ne l'ouvre pas, pour ne rien abîmer."
        );
        assert_eq!(
            actions(&problem),
            [
                (StartupAction::OpenReleases, true),
                (StartupAction::StartEmpty, false)
            ]
        );
        assert_eq!(
            problem.options[0].title,
            "Installer Arnod 1.0.2 ou plus récent"
        );
        assert!(
            problem.options[1]
                .description
                .contains("mis de côté sous arnod.json.format-3"),
            "{}",
            problem.options[1].description
        );
        assert_eq!(problem.intact_lead, "Rien n'a été modifié.");
        assert_eq!(problem.path_label, "Fichier d'état");
        assert_eq!(problem.retry, None);
        assert_eq!(problem.footer, "Arnod 0.9.0 · Windows");
    }

    #[test]
    fn r2_fichier_illisible_repartir_d_un_etat_vide_recommande() {
        let problem = describe(
            &failure(Cause::Corrupt {
                reason: "JSON invalide : EOF".into(),
            }),
            &env(),
            0,
        );

        assert_eq!(problem.nature, ProblemNature::CorruptState);
        assert!(problem.message.contains("(JSON invalide : EOF)"));
        assert_eq!(
            actions(&problem),
            [
                (StartupAction::StartEmpty, true),
                (StartupAction::Retry, false)
            ]
        );
        assert!(
            problem.options[0]
                .description
                .contains("arnod.json.illisible")
        );
    }

    #[test]
    fn r2_disposition_illisible_garde_projets_et_workspaces() {
        let mut windows = failure(Cause::Corrupt {
            reason: "JSON invalide".into(),
        });
        windows.file = StateFile::Windows;
        windows.path = PathBuf::from("/donnees/windows.json");

        let problem = describe(&windows, &env(), 0);

        assert_eq!(problem.title, "La disposition des fenêtres est illisible");
        assert_eq!(problem.file, "windows.json");
        assert_eq!(problem.path_label, "Fichier des fenêtres");
        assert_eq!(problem.options[0].title, "Repartir d'une disposition vide");
        assert!(problem.intact_text.contains("vos workspaces aussi"));
    }

    #[test]
    fn r3_mise_de_cote_sous_un_nom_libre_sans_rien_ecraser() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("arnod.json");
        fs::write(&path, "un").unwrap();
        fs::write(dir.path().join("arnod.json.illisible"), "ancien").unwrap();

        let aside = set_aside(&path, "illisible").unwrap();

        assert_eq!(aside, dir.path().join("arnod.json.illisible-2"));
        assert_eq!(fs::read_to_string(&aside).unwrap(), "un");
        assert_eq!(
            fs::read_to_string(dir.path().join("arnod.json.illisible")).unwrap(),
            "ancien"
        );
        assert!(!path.exists());
        let error = set_aside(&path, "illisible").unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Io);
    }

    #[test]
    fn r3_seuls_un_format_recent_ou_un_fichier_illisible_se_mettent_de_cote() {
        assert_eq!(newer().aside_suffix().as_deref(), Some("format-3"));
        assert_eq!(locked().aside_suffix(), None);
    }

    #[test]
    fn r4_verrou_trois_essais_automatiques_puis_la_main() {
        let plans: Vec<Option<RetryPlan>> = (0..4)
            .map(|done| retry_plan(&locked().cause, done))
            .collect();
        assert_eq!(
            plans,
            [
                Some(RetryPlan {
                    attempt: 1,
                    max_attempts: 3,
                    delay_seconds: 2
                }),
                Some(RetryPlan {
                    attempt: 2,
                    max_attempts: 3,
                    delay_seconds: 4
                }),
                Some(RetryPlan {
                    attempt: 3,
                    max_attempts: 3,
                    delay_seconds: 8
                }),
                None
            ]
        );
        assert_eq!(retry_plan(&newer().cause, 0), None);

        let problem = describe(&locked(), &env(), 3);
        assert_eq!(problem.title, "Arnod n'arrive pas à lire son état");
        assert!(
            problem
                .message
                .starts_with("Accès refusé au fichier d'état (erreur système 5).")
        );
        assert_eq!(
            actions(&problem),
            [
                (StartupAction::Retry, true),
                (StartupAction::OpenFolder, false)
            ]
        );
        assert_eq!(
            problem.options[0].note.as_deref(),
            Some("Arnod a réessayé seul 3 fois, sans succès.")
        );
        assert_eq!(detail(&problem, "Essais automatiques"), "3");
    }

    #[test]
    fn r4_dossier_de_donnees_inaccessible() {
        let mut failure = failure(Cause::Inaccessible {
            code: Some(2),
            error: "Création du dossier impossible".into(),
        });
        failure.path = PathBuf::from("/donnees");

        let problem = describe(&failure, &env(), 0);

        assert_eq!(problem.nature, ProblemNature::Inaccessible);
        assert_eq!(problem.path_label, "Dossier de données");
        assert!(problem.retry.is_some());
        assert_eq!(problem.options[0].note, None);
    }

    #[test]
    fn r5_details_pour_le_support() {
        let problem = describe(&newer(), &env(), 0);

        let labels: Vec<&str> = problem.details.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Nature",
                "Fichier",
                "Format du fichier",
                "Formats lus ici",
                "Arnod",
                "Date"
            ]
        );
        assert_eq!(detail(&problem, "Nature"), "newerVersion");
        assert_eq!(
            detail(&problem, "Format du fichier"),
            "3 (écrit par Arnod 1.0.2, le 08/11/2026)"
        );
        assert_eq!(detail(&problem, "Formats lus ici"), "1 et 2");
        assert_eq!(
            detail(&problem, "Arnod"),
            "0.9.0 · Windows x86_64 · WebView 131"
        );
        assert_eq!(detail(&problem, "Date"), "10/11/2026 09:12:04 UTC");
        assert!(
            problem
                .support_text
                .starts_with("Arnod ne démarre pas : Cet état vient")
        );
        assert!(problem.support_text.contains("Formats lus ici    1 et 2\n"));
    }

    #[test]
    fn r6_issue_preremplie_encodee() {
        let problem = describe(&newer(), &env(), 0);

        assert!(
            problem.issue_url.starts_with(
                "https://github.com/juliencorre/Arnod/issues/new?title=D%C3%A9marrage"
            )
        );
        let body = problem.issue_url.split("&body=").nth(1).unwrap();
        assert!(!body.contains(' ') && !body.contains('&') && !body.contains('\n'));
        assert!(body.contains("newerVersion"));
    }

    #[test]
    fn dates_civiles_en_utc() {
        assert_eq!(format_time(0), "01/01/1970 00:00:00");
        assert_eq!(format_day(951_782_400), "29/02/2000");
        assert_eq!(format_time(1_794_301_924), "10/11/2026 09:12:04");
    }

    #[test]
    fn forme_json_serialisee() {
        let problem = describe(&locked(), &env(), 0);
        let json = serde_json::to_value(&problem).unwrap();

        assert_eq!(json["nature"], "locked");
        assert_eq!(json["intactLead"], "Rien n'a été modifié.");
        assert_eq!(json["pathLabel"], "Fichier d'état");
        assert_eq!(
            json["retry"],
            serde_json::json!({ "attempt": 1, "maxAttempts": 3, "delaySeconds": 2 })
        );
        assert_eq!(json["options"][0]["action"], "retry");
        assert_eq!(json["options"][0]["note"], serde_json::Value::Null);
        assert!(json["supportText"].is_string());
        assert!(json["issueUrl"].is_string());
    }

    #[test]
    fn natures_actions_et_cibles_identiques_cote_typescript() {
        let natures: BTreeSet<String> = [
            ProblemNature::NewerVersion,
            ProblemNature::CorruptState,
            ProblemNature::Locked,
            ProblemNature::Inaccessible,
        ]
        .into_iter()
        .map(serialized_name)
        .collect();
        assert_eq!(natures, union_members("ProblemNature"));
        let actions: BTreeSet<String> = [
            StartupAction::OpenReleases,
            StartupAction::StartEmpty,
            StartupAction::Retry,
            StartupAction::OpenFolder,
        ]
        .into_iter()
        .map(serialized_name)
        .collect();
        assert_eq!(actions, union_members("StartupAction"));
        let targets: BTreeSet<String> = [
            StartupTarget::Folder,
            StartupTarget::Releases,
            StartupTarget::Issue,
        ]
        .into_iter()
        .map(serialized_name)
        .collect();
        assert_eq!(targets, union_members("StartupTarget"));
    }
}
