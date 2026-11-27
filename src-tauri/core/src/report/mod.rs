//! Rapports d'un run (`docs/plan-v2.md`, lot D, R8) : JUnit ([`junit`]), Allure ([`allure`]) et
//! HTML ([`html`]), écrits par `arnod run --rapport`.
//!
//! Tout part du run tel que le cœur l'a calculé ([`RunView`]) : un **cas** est un test (ses
//! étapes, dans l'ordre) ou une requête jouée seule. Les valeurs y sont déjà masquées (réponses,
//! erreurs, valeurs extraites : comme l'onglet « Envoyé ») : aucun secret n'arrive dans un
//! rapport. Module pur : rien n'est lu ni écrit ici.

pub mod allure;
pub mod html;
pub mod junit;

#[cfg(test)]
mod tests;

use crate::request::assertions::AssertionResult;
use crate::workbench::{CONTRACT_CHECK, ItemStatus, RunItem, RunView};

/// Ce que le rapport décrit, en plus du run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    /// Version d'Arnod qui a joué le run.
    pub version: String,
    /// Nom du projet.
    pub project: String,
    /// Ce qui a été lancé, en clair (« Dossier commandes/v2 », « Étiquette smoke »).
    pub target: String,
}

/// Verdict d'un cas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    /// Une assertion n'est pas vérifiée.
    Failed,
    /// L'envoi n'a pas abouti (réseau, variable, fichier illisible).
    Error,
    /// Rien n'a été joué.
    Skipped,
    Cancelled,
}

impl Verdict {
    /// En échec au sens du code de sortie (R7) : échec ou erreur.
    pub fn failing(self) -> bool {
        matches!(self, Self::Failed | Self::Error)
    }
}

/// Un test (ses étapes) ou une requête jouée seule.
#[derive(Debug, Clone)]
pub struct Case<'a> {
    /// Chemin du test (dossier) ou de la requête, relatif au projet.
    pub path: String,
    pub name: String,
    /// Dossier qui le contient (`""` : racine du projet).
    pub folder: String,
    /// `true` pour un test à étapes.
    pub test: bool,
    pub verdict: Verdict,
    /// Somme des durées des étapes jouées, en millisecondes.
    pub duration_ms: u64,
    pub steps: Vec<&'a RunItem>,
    /// Ce qui a arrêté le cas : « étape 2 · headers.Location — absent ».
    pub failure: Option<String>,
}

/// Libellé d'une assertion dans un message : sa cible, ou la conformité au contrat.
pub fn check_label(check: &AssertionResult) -> String {
    if check.id == CONTRACT_CHECK || check.target == CONTRACT_CHECK {
        "conformité au contrat".to_owned()
    } else if check.target.is_empty() {
        "assertion".to_owned()
    } else {
        check.target.clone()
    }
}

/// Ce qui a fait échouer une ligne du run : son erreur, sinon sa première assertion en échec
/// (non ignorée).
pub fn item_failure(item: &RunItem) -> Option<String> {
    match item.status {
        ItemStatus::Error => Some(
            item.error
                .clone()
                .unwrap_or_else(|| "l'envoi n'a pas abouti".to_owned()),
        ),
        ItemStatus::Failed => Some(
            item.assertions
                .iter()
                .find(|c| !c.passed && !c.ignored)
                .map_or_else(
                    || "en échec".to_owned(),
                    |c| {
                        format!(
                            "{} — {}",
                            check_label(c),
                            c.message.as_deref().unwrap_or("en échec")
                        )
                    },
                ),
        ),
        _ => None,
    }
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default()
}

fn verdict_of(steps: &[&RunItem]) -> Verdict {
    let any = |status: ItemStatus| steps.iter().any(|s| s.status == status);
    if any(ItemStatus::Error) {
        Verdict::Error
    } else if any(ItemStatus::Failed) {
        Verdict::Failed
    } else if any(ItemStatus::Cancelled) {
        Verdict::Cancelled
    } else if !steps.is_empty() && steps.iter().all(|s| s.status == ItemStatus::Passed) {
        Verdict::Passed
    } else {
        Verdict::Skipped
    }
}

/// Cas d'un run, dans l'ordre de leur première ligne.
pub fn cases(view: &RunView) -> Vec<Case<'_>> {
    let mut cases: Vec<Case<'_>> = Vec::new();
    for item in &view.items {
        let existing = item
            .test
            .as_ref()
            .and_then(|test| cases.iter().position(|c| c.test && &c.path == test));
        if let Some(index) = existing {
            if let Some(case) = cases.get_mut(index) {
                case.steps.push(item);
            }
            continue;
        }
        let (path, name, test) = match &item.test {
            Some(test) => (
                test.clone(),
                test.rsplit('/').next().unwrap_or(test).to_owned(),
                true,
            ),
            None => (item.path.clone(), item.name.clone(), false),
        };
        cases.push(Case {
            folder: parent(&path),
            path,
            name,
            test,
            verdict: Verdict::Skipped,
            duration_ms: 0,
            steps: vec![item],
            failure: None,
        });
    }
    for case in &mut cases {
        case.verdict = verdict_of(&case.steps);
        // Un test joué par son schéma (V4, R5) prend le verdict de sa fin : une étape non
        // empruntée (branche, boucle) ne le rend pas « non joué ».
        if case.test
            && let Some(end) = view.ends.iter().find(|e| e.test == case.path)
            && !matches!(case.verdict, Verdict::Error | Verdict::Cancelled)
        {
            case.verdict = if end.passed {
                Verdict::Passed
            } else {
                Verdict::Failed
            };
        }
        case.duration_ms = case.steps.iter().filter_map(|s| s.duration_ms).sum();
        case.failure = case
            .steps
            .iter()
            .find_map(|step| {
                item_failure(step).map(|text| match (case.test, step.step) {
                    (true, Some(number)) => format!("étape {number} · {text}"),
                    _ => text,
                })
            })
            .or_else(|| {
                view.ends
                    .iter()
                    .find(|e| case.test && e.test == case.path && !e.passed)
                    .and_then(|e| e.reason.clone())
            });
    }
    cases
}

/// Décompte d'un run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub skipped: usize,
    pub cancelled: usize,
    /// Durée du run, du début à la fin, en millisecondes.
    pub duration_ms: u64,
}

impl Summary {
    pub fn of(view: &RunView, cases: &[Case<'_>]) -> Self {
        let count = |v: Verdict| cases.iter().filter(|c| c.verdict == v).count();
        Self {
            total: cases.len(),
            passed: count(Verdict::Passed),
            failed: count(Verdict::Failed),
            errors: count(Verdict::Error),
            skipped: count(Verdict::Skipped),
            cancelled: count(Verdict::Cancelled),
            duration_ms: view
                .finished_at
                .map_or(0, |end| end.saturating_sub(view.started_at)),
        }
    }

    /// Au moins un cas en échec ou en erreur (R7).
    pub fn failing(&self) -> bool {
        self.failed + self.errors > 0
    }
}

/// Nombre de dossiers distincts qui contiennent les cas (« 6 tests dans 2 dossiers »).
pub fn folder_count(cases: &[Case<'_>]) -> usize {
    let mut folders: Vec<&str> = cases.iter().map(|c| c.folder.as_str()).collect();
    folders.sort_unstable();
    folders.dedup();
    folders.len()
}

/// Durée en secondes, avec trois décimales (`0.042`) : JUnit.
pub fn seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// Durée lisible en français : « 42 ms », « 3,4 s », « 2 min 05 s ».
pub fn duration_fr(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 60_000 {
        let tenths = (ms + 50) / 100;
        format!("{},{} s", tenths / 10, tenths % 10)
    } else {
        let secs = (ms + 500) / 1000;
        format!("{} min {:02} s", secs / 60, secs % 60)
    }
}

/// Date et heure UTC au format ISO 8601, sans fuseau (`2026-11-27T09:21:11`), comme l'attend
/// JUnit.
pub fn iso8601(ms: u64) -> String {
    let secs = ms / 1000;
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rest = secs % 86_400;
    // Jours depuis 1970 vers une date civile (algorithme de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Texte sûr dans un document XML ou HTML : caractères spéciaux remplacés, caractères de
/// contrôle (interdits en XML 1.0) retirés.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' | '\t' | '\r' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Statut d'une étape, en clair.
pub fn step_status(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Passed => "réussie",
        ItemStatus::Failed => "en échec",
        ItemStatus::Error => "en erreur",
        ItemStatus::Skipped => "non jouée",
        ItemStatus::Cancelled => "annulée",
        ItemStatus::Pending | ItemStatus::Running => "non terminée",
    }
}

/// Une ligne par étape : « 1. POST Créer une commande · 201 · 64 ms · en échec ».
pub fn step_lines(case: &Case<'_>) -> Vec<String> {
    case.steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let mut line = format!(
                "{}. {} {}",
                step.step.unwrap_or(index + 1),
                step.method.as_str(),
                step.name
            );
            if let Some(status) = step.http_status {
                line.push_str(&format!(" · {status}"));
            }
            if let Some(ms) = step.duration_ms {
                line.push_str(&format!(" · {}", duration_fr(ms)));
            }
            line.push_str(&format!(" · {}", step_status(step.status)));
            if let Some(failure) = item_failure(step) {
                line.push_str(&format!(" : {failure}"));
            }
            line
        })
        .collect()
}
