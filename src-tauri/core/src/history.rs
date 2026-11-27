//! Historique des envois d'une requête (écran 5.1-c bis, `docs/plan-v1.md`, H1 à H6).
//!
//! Les 20 derniers envois réussis de chaque requête restent sur le poste, jamais dans le projet :
//! `projects/<id>/history/<chemin encodé>.json` dans le dossier de données (un fichier par
//! requête, nom réversible). Chaque envoi se compare au précédent (`compare`, résumé en
//! français) et en donne les différences ligne à ligne (`diff`). Règles pures, testées dans
//! `history/tests.rs` ; `record`, `view`, `clear`, `rename` et `remove` touchent au disque.

use std::fs;
use std::path::{Path, PathBuf};

use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::local;
use crate::request::assertions::AssertionResult;
use crate::workbench::SendOutcome;

/// Envois gardés par requête (H1).
pub const MAX_ENTRIES: usize = 20;
/// Corps de réponse gardé, au plus (H1) : au-delà, il est coupé et marqué tronqué.
pub const MAX_BODY: usize = 256 * 1024;
/// Version du format des fichiers d'historique.
pub const VERSION: u32 = 1;
/// Lignes de différences montrées, au plus (H4).
pub const MAX_DIFF_LINES: usize = 300;

/// Ce qui est parti, masqué comme dans l'onglet « Envoyé ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistorySent {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Un envoi gardé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: Uuid,
    /// Réception de la réponse, en millisecondes depuis l'époque Unix.
    pub at: u64,
    /// Environnement de l'envoi.
    pub environment: Option<String>,
    pub status: u16,
    pub status_text: String,
    pub total_ms: u64,
    pub size_bytes: usize,
    /// Verdict de l'envoi.
    pub passed: bool,
    pub assertions: Vec<AssertionResult>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// Corps coupé : par l'envoi (5 Mo) ou par l'historique ([`MAX_BODY`]).
    pub truncated: bool,
    pub sent: Option<HistorySent>,
}

/// Coupe un texte à `max` octets au plus, sur une frontière de caractère.
fn cut(text: &str, max: usize) -> (String, bool) {
    if text.len() <= max {
        return (text.to_owned(), false);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

/// Envoi à garder, d'après sa réponse (H1).
pub fn entry_of(outcome: &SendOutcome, at: u64) -> HistoryEntry {
    let response = &outcome.response;
    let (body, cut_here) = cut(&response.body, MAX_BODY);
    HistoryEntry {
        id: Uuid::new_v4(),
        at,
        environment: outcome.sent.as_ref().map(|sent| sent.environment.clone()),
        status: response.status,
        status_text: response.status_text.clone(),
        total_ms: response.total_ms,
        size_bytes: response.size_bytes,
        passed: outcome.passed,
        assertions: outcome.assertions.clone(),
        headers: response.headers.clone(),
        body,
        truncated: response.truncated || cut_here,
        sent: outcome.sent.as_ref().map(|sent| HistorySent {
            method: sent.method.clone(),
            url: sent.url.clone(),
            headers: sent.headers.clone(),
            body: sent.body.as_deref().map(|body| cut(body, MAX_BODY).0),
        }),
    }
}

// ------------------------------------------------------------------------- comparaison

/// En-têtes dont la valeur change à chaque envoi : seule leur présence compte (H3).
const VOLATILE: [&str; 4] = ["date", "age", "expires", "x-request-id"];

/// Corps comparable : un JSON est remis en forme, sinon le texte tel quel.
fn normalized_body(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| body.to_owned())
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Noms d'en-têtes, sans doublon, dans l'ordre de la réponse.
fn names(headers: &[(String, String)]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for (name, _) in headers {
        if !out.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            out.push(name);
        }
    }
    out
}

fn passed_count(entry: &HistoryEntry) -> usize {
    entry.assertions.iter().filter(|a| a.passed).count()
}

/// « l'en-tête Location », « les en-têtes Location et ETag ».
fn header_list(list: &[&str]) -> String {
    match list {
        [one] => format!("l'en-tête {one}"),
        [rest @ .., last] => format!("les en-têtes {} et {last}", rest.join(", ")),
        [] => String::new(),
    }
}

/// Ce qui a changé depuis l'envoi précédent, en français (H3) ; vide : rien de notable.
pub fn compare(entry: &HistoryEntry, previous: &HistoryEntry) -> Vec<String> {
    let mut out = Vec::new();
    if entry.status != previous.status {
        out.push(format!(
            "statut {} au lieu de {}",
            entry.status, previous.status
        ));
    }
    if entry.environment != previous.environment
        && let (Some(now), Some(before)) = (&entry.environment, &previous.environment)
    {
        out.push(format!("environnement {now} au lieu de {before}"));
    }
    let now = names(&entry.headers);
    let before = names(&previous.headers);
    let gone: Vec<&str> = before
        .iter()
        .copied()
        .filter(|n| header(&entry.headers, n).is_none())
        .collect();
    let new: Vec<&str> = now
        .iter()
        .copied()
        .filter(|n| header(&previous.headers, n).is_none())
        .collect();
    let changed: Vec<&str> = now
        .iter()
        .copied()
        .filter(|n| !VOLATILE.contains(&n.to_ascii_lowercase().as_str()))
        .filter(|n| {
            header(&previous.headers, n).is_some_and(|v| Some(v) != header(&entry.headers, n))
        })
        .collect();
    match gone.len() {
        0 => {}
        1 => out.push(format!("{} a disparu", header_list(&gone))),
        _ => out.push(format!("{} ont disparu", header_list(&gone))),
    }
    match new.len() {
        0 => {}
        1 => out.push(format!("{} est apparu", header_list(&new))),
        _ => out.push(format!("{} sont apparus", header_list(&new))),
    }
    match changed.len() {
        0 => {}
        1 => out.push(format!("{} a changé", header_list(&changed))),
        _ => out.push(format!("{} ont changé", header_list(&changed))),
    }
    if normalized_body(&entry.body) != normalized_body(&previous.body) {
        out.push("le corps a changé".to_owned());
    }
    let (now, before) = (passed_count(entry), passed_count(previous));
    if now != before {
        let n = now.abs_diff(before);
        let more = if now > before { "de plus" } else { "de moins" };
        let word = if n > 1 {
            "assertions réussies"
        } else {
            "assertion réussie"
        };
        out.push(format!("{n} {word} {more}"));
    }
    let delta = i128::from(entry.total_ms) - i128::from(previous.total_ms);
    if delta > 0 {
        out.push(format!("{delta} ms de plus"));
    } else if delta < 0 {
        out.push(format!("{} ms de moins", -delta));
    }
    out
}

/// Partie de l'envoi où se trouve une différence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum DiffPart {
    Status,
    Header,
    Body,
    Assertion,
}

/// Ligne ajoutée (dans l'envoi) ou retirée (présente dans le précédent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub part: DiffPart,
    pub added: bool,
    pub text: String,
}

/// Différences d'un envoi avec le précédent (« Voir les différences », H4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistoryDiff {
    pub previous_id: Uuid,
    pub lines: Vec<DiffLine>,
    /// Trop de lignes : seules les [`MAX_DIFF_LINES`] premières sont montrées.
    pub truncated: bool,
}

/// Lignes retirées puis ajoutées entre deux textes, par la plus longue sous-suite commune ;
/// au-delà de 2 000 lignes de part et d'autre, tout l'ancien est retiré, tout le nouveau ajouté.
fn line_diff(before: &str, after: &str) -> Vec<(bool, String)> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    if a.len() > 2000 || b.len() > 2000 {
        return a
            .iter()
            .map(|l| (false, (*l).to_owned()))
            .chain(b.iter().map(|l| (true, (*l).to_owned())))
            .collect();
    }
    // lcs[i][j] : plus longue sous-suite commune de a[i..] et b[j..].
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push((false, a[i].to_owned()));
            i += 1;
        } else {
            out.push((true, b[j].to_owned()));
            j += 1;
        }
    }
    out
}

/// Différences ligne à ligne : statut, en-têtes, corps (JSON remis en forme), assertions (H4).
pub fn diff(entry: &HistoryEntry, previous: &HistoryEntry) -> HistoryDiff {
    let mut lines = Vec::new();
    let mut push = |part, added, text: String| lines.push(DiffLine { part, added, text });
    if entry.status != previous.status {
        push(
            DiffPart::Status,
            false,
            format!("{} {}", previous.status, previous.status_text),
        );
        push(
            DiffPart::Status,
            true,
            format!("{} {}", entry.status, entry.status_text),
        );
    }
    let line = |(name, value): &(String, String)| format!("{name}: {value}");
    let volatile = |name: &str| VOLATILE.contains(&name.to_ascii_lowercase().as_str());
    for pair in &previous.headers {
        let kept = header(&entry.headers, &pair.0);
        if kept.is_none() || (kept != Some(pair.1.as_str()) && !volatile(&pair.0)) {
            push(DiffPart::Header, false, line(pair));
        }
    }
    for pair in &entry.headers {
        let before = header(&previous.headers, &pair.0);
        if before.is_none() || (before != Some(pair.1.as_str()) && !volatile(&pair.0)) {
            push(DiffPart::Header, true, line(pair));
        }
    }
    for (added, text) in line_diff(
        &normalized_body(&previous.body),
        &normalized_body(&entry.body),
    ) {
        push(DiffPart::Body, added, text);
    }
    let label = |a: &AssertionResult| {
        format!(
            "{} {}",
            if a.passed { "✓" } else { "✕" },
            if a.target.is_empty() {
                &a.id
            } else {
                &a.target
            }
        )
    };
    for before in &previous.assertions {
        let now = entry.assertions.iter().find(|a| a.id == before.id);
        if now.is_none_or(|now| now.passed != before.passed) {
            push(DiffPart::Assertion, false, label(before));
        }
    }
    for now in &entry.assertions {
        let before = previous.assertions.iter().find(|a| a.id == now.id);
        if before.is_none_or(|before| before.passed != now.passed) {
            push(DiffPart::Assertion, true, label(now));
        }
    }
    let truncated = lines.len() > MAX_DIFF_LINES;
    lines.truncate(MAX_DIFF_LINES);
    HistoryDiff {
        previous_id: previous.id,
        lines,
        truncated,
    }
}

// ------------------------------------------------------------------------- vue

/// Un envoi, comparé au précédent s'il y en a un.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub entry: HistoryEntry,
    /// Envoi précédent : son heure, et ce qui a changé depuis (vide : rien de notable).
    pub previous_at: Option<u64>,
    pub changes: Vec<String>,
}

/// Historique d'une requête, du plus récent au plus ancien.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    pub path: String,
    pub items: Vec<HistoryItem>,
    pub max: usize,
}

/// Vue d'une liste d'envois, chacun comparé au suivant (plus ancien).
pub fn view_of(path: &str, entries: Vec<HistoryEntry>) -> HistoryView {
    let items = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let previous = entries.get(i + 1);
            HistoryItem {
                entry: entry.clone(),
                previous_at: previous.map(|p| p.at),
                changes: previous.map(|p| compare(entry, p)).unwrap_or_default(),
            }
        })
        .collect();
    HistoryView {
        path: path.to_owned(),
        items,
        max: MAX_ENTRIES,
    }
}

// ------------------------------------------------------------------------- disque

#[derive(Debug, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    entries: Vec<HistoryEntry>,
}

fn dir(data_dir: &Path, project: Uuid) -> PathBuf {
    local::project_dir(data_dir, project).join("history")
}

fn file(data_dir: &Path, project: Uuid, path: &str) -> PathBuf {
    let name = utf8_percent_encode(path, NON_ALPHANUMERIC).to_string();
    dir(data_dir, project).join(format!("{name}.json"))
}

/// Chemin de requête d'un fichier d'historique, `None` pour un autre fichier.
fn path_of(file_name: &str) -> Option<String> {
    let name = file_name.strip_suffix(".json")?;
    percent_decode_str(name)
        .decode_utf8()
        .ok()
        .map(|p| p.into_owned())
}

/// Le chemin `path` est `base` ou se trouve sous le dossier `base`.
fn under(path: &str, base: &str) -> bool {
    path == base
        || path
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Lecture : `None` pour un fichier d'une version plus récente, jamais réécrit (H6).
fn read(path: &Path) -> Option<Vec<HistoryEntry>> {
    let Ok(text) = fs::read_to_string(path) else {
        return Some(Vec::new());
    };
    match serde_json::from_str::<HistoryFile>(&text) {
        Ok(file) if file.version > VERSION => None,
        Ok(file) => Some(file.entries),
        Err(_) => Some(Vec::new()),
    }
}

fn io(action: &str, path: &Path, error: &std::io::Error) -> CommandError {
    CommandError::io(action, path, error)
}

fn write(path: &Path, entries: Vec<HistoryEntry>) -> Result<(), CommandError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| io("Création du dossier de l'historique impossible", parent, &e))?;
    }
    let text = serde_json::to_string(&HistoryFile {
        version: VERSION,
        entries,
    })
    .map_err(|e| CommandError::new(CommandErrorKind::Io, format!("Écriture impossible : {e}")))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| io("Écriture impossible", &tmp, &e))?;
    fs::rename(&tmp, path).map_err(|e| io("Écriture impossible", path, &e))
}

/// Garde un envoi en tête de l'historique de la requête, 20 au plus (H1).
pub fn record(
    data_dir: &Path,
    project: Uuid,
    path: &str,
    entry: HistoryEntry,
) -> Result<(), CommandError> {
    let target = file(data_dir, project, path);
    let Some(mut entries) = read(&target) else {
        return Ok(());
    };
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
    write(&target, entries)
}

/// Envois gardés d'une requête, du plus récent au plus ancien.
pub fn entries(data_dir: &Path, project: Uuid, path: &str) -> Vec<HistoryEntry> {
    read(&file(data_dir, project, path)).unwrap_or_default()
}

/// Historique d'une requête, chaque envoi comparé au précédent (H2, H3).
pub fn view(data_dir: &Path, project: Uuid, path: &str) -> HistoryView {
    view_of(path, entries(data_dir, project, path))
}

/// Différences d'un envoi avec le précédent ; introuvable s'il est le plus ancien (H4).
pub fn diff_of(
    data_dir: &Path,
    project: Uuid,
    path: &str,
    id: Uuid,
) -> Result<HistoryDiff, CommandError> {
    let entries = entries(data_dir, project, path);
    let index = entries.iter().position(|e| e.id == id);
    match index.and_then(|i| Some((entries.get(i)?, entries.get(i + 1)?))) {
        Some((entry, previous)) => Ok(diff(entry, previous)),
        None => Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Cet envoi n'a pas d'envoi précédent dans l'historique.",
        )),
    }
}

/// « Effacer » : l'historique de la requête est oublié (H5).
pub fn clear(data_dir: &Path, project: Uuid, path: &str) -> Result<(), CommandError> {
    let target = file(data_dir, project, path);
    match fs::remove_file(&target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io("Effacement impossible", &target, &error)),
    }
}

/// « Annuler » après « Effacer » : les envois effacés reviennent, après ceux reçus depuis, 20 au
/// plus (H5).
pub fn restore(
    data_dir: &Path,
    project: Uuid,
    path: &str,
    restored: Vec<HistoryEntry>,
) -> Result<(), CommandError> {
    let target = file(data_dir, project, path);
    let Some(mut entries) = read(&target) else {
        return Ok(());
    };
    for entry in restored {
        if !entries.iter().any(|e| e.id == entry.id) {
            entries.push(entry);
        }
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.at));
    entries.truncate(MAX_ENTRIES);
    write(&target, entries)
}

/// Chemins de requête qui ont un historique, sous `base`.
fn files_under(data_dir: &Path, project: Uuid, base: &str) -> Vec<(String, PathBuf)> {
    let Ok(read) = fs::read_dir(dir(data_dir, project)) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|item| {
            let name = item.file_name().into_string().ok()?;
            let path = path_of(&name)?;
            under(&path, base).then(|| (path, item.path()))
        })
        .collect()
}

/// Une requête ou un dossier renommé ou déplacé garde son historique (H6).
pub fn rename(data_dir: &Path, project: Uuid, from: &str, to: &str) {
    for (path, source) in files_under(data_dir, project, from) {
        let target = file(data_dir, project, &format!("{to}{}", &path[from.len()..]));
        let _ = fs::rename(&source, &target);
    }
}

/// Une requête ou un dossier supprimé perd son historique (H6).
pub fn remove(data_dir: &Path, project: Uuid, path: &str) {
    for (_, source) in files_under(data_dir, project, path) {
        let _ = fs::remove_file(source);
    }
}

#[cfg(test)]
mod tests;
