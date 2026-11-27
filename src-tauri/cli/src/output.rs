//! Sortie de `arnod run` (écrans 5.3-a et 4.9-d) : en-tête, une ligne par test (ou par étape
//! pour un test lancé seul), résumé, rapports, code de sortie. Mise en forme pure.

use arnod_core::report::{Case, Summary, Verdict, duration_fr, item_failure};
use arnod_core::workbench::{ItemStatus, RunItem};

use crate::select::{Kind, Plan};

/// Largeur de la colonne des noms.
const NAME_WIDTH: usize = 40;

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count > 1 { many } else { one })
}

/// « Arnod 0.0.4 · projet boutique-tests · environnement Recette fonctionnelle ».
pub fn header(version: &str, project: &str, environment: &str) -> String {
    format!("Arnod {version} · projet {project} · environnement {environment}")
}

/// Ce qui est joué : « Dossier commandes/v2 · 28 tests · 4 en parallèle »,
/// « Étiquette smoke · 6 tests dans 2 dossiers · 4 en parallèle », « Test parcours · 5 étapes ».
pub fn scope(plan: &Plan, cases: usize, steps: usize, folders: usize, parallel: usize) -> String {
    match plan.kind {
        Kind::Test => format!("{} · {}", plan.title, plural(steps, "étape", "étapes")),
        Kind::Request => plan.title.clone(),
        Kind::Tags => format!(
            "{} · {} dans {} · {parallel} en parallèle",
            plan.title,
            plural(cases, "test", "tests"),
            plural(folders, "dossier", "dossiers")
        ),
        Kind::Folder | Kind::Project => format!(
            "{} · {} · {parallel} en parallèle",
            plan.title,
            plural(cases, "test", "tests")
        ),
    }
}

fn mark(verdict: Verdict) -> char {
    match verdict {
        Verdict::Passed => '✓',
        Verdict::Failed | Verdict::Error => '✗',
        Verdict::Skipped | Verdict::Cancelled => '–',
    }
}

fn padded(name: &str) -> String {
    let count = name.chars().count();
    if count >= NAME_WIDTH {
        format!("{name} ")
    } else {
        format!("{name}{}", " ".repeat(NAME_WIDTH - count))
    }
}

/// « ✓ Lister les commandes        42 ms », suivi de ce qui a échoué.
pub fn case_line(case: &Case<'_>) -> String {
    let mut line = format!(
        "  {} {}{:>8}",
        mark(case.verdict),
        padded(&case.name),
        duration_fr(case.duration_ms)
    );
    match case.verdict {
        Verdict::Skipped => line.push_str("   non joué"),
        Verdict::Cancelled => line.push_str("   annulé"),
        _ => {}
    }
    if let Some(failure) = &case.failure {
        line.push_str("   ");
        line.push_str(failure);
    }
    line
}

/// Une étape d'un test lancé seul : « ✓ 1  Créer un client        42 ms ».
pub fn step_line(item: &RunItem, index: usize) -> String {
    let verdict = match item.status {
        ItemStatus::Passed => Verdict::Passed,
        ItemStatus::Failed => Verdict::Failed,
        ItemStatus::Error => Verdict::Error,
        ItemStatus::Cancelled => Verdict::Cancelled,
        _ => Verdict::Skipped,
    };
    let number = item.step.unwrap_or(index + 1);
    let mut line = format!(
        "  {} {number:<2} {}{:>8}",
        mark(verdict),
        padded(&item.name),
        item.duration_ms.map(duration_fr).unwrap_or_default()
    );
    if item.status == ItemStatus::Skipped {
        line.push_str("   non jouée");
    }
    if let Some(failure) = item_failure(item) {
        line.push_str("   ");
        line.push_str(&failure);
    }
    line
}

/// « 25 réussis · 3 en échec · 3,4 s ».
pub fn summary(summary: &Summary) -> String {
    let mut parts = vec![plural(summary.passed, "réussi", "réussis")];
    parts.push(format!("{} en échec", summary.failed));
    if summary.errors > 0 {
        parts.push(format!("{} en erreur", summary.errors));
    }
    let skipped = summary.skipped + summary.cancelled;
    if skipped > 0 {
        parts.push(plural(skipped, "non joué", "non joués"));
    }
    parts.push(duration_fr(summary.duration_ms));
    parts.join(" · ")
}

/// « Rapports : rapports\junit.xml · rapports\allure-results\ ».
pub fn reports(paths: &[String]) -> String {
    format!("Rapports : {}", paths.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(kind: Kind, title: &str) -> Plan {
        Plan {
            kind,
            folder: String::new(),
            only: None,
            title: title.to_owned(),
        }
    }

    #[test]
    fn lc1_lc3_en_tete_et_portee() {
        assert_eq!(
            header("0.0.4", "boutique-tests", "Recette fonctionnelle"),
            "Arnod 0.0.4 · projet boutique-tests · environnement Recette fonctionnelle"
        );
        assert_eq!(
            scope(&plan(Kind::Folder, "Dossier commandes/v2"), 28, 40, 3, 4),
            "Dossier commandes/v2 · 28 tests · 4 en parallèle"
        );
        assert_eq!(
            scope(&plan(Kind::Tags, "Étiquette smoke"), 6, 9, 2, 4),
            "Étiquette smoke · 6 tests dans 2 dossiers · 4 en parallèle"
        );
        assert_eq!(
            scope(&plan(Kind::Test, "Test parcours-achat"), 1, 5, 1, 4),
            "Test parcours-achat · 5 étapes"
        );
        assert_eq!(
            scope(&plan(Kind::Tags, "Étiquette smoke"), 1, 1, 1, 8),
            "Étiquette smoke · 1 test dans 1 dossier · 8 en parallèle"
        );
    }

    #[test]
    fn lc1_resume_en_francais() {
        let mut s = Summary {
            total: 28,
            passed: 25,
            failed: 3,
            duration_ms: 3400,
            ..Summary::default()
        };
        assert_eq!(summary(&s), "25 réussis · 3 en échec · 3,4 s");
        s.passed = 1;
        s.failed = 0;
        s.errors = 1;
        s.skipped = 1;
        s.duration_ms = 900;
        assert_eq!(
            summary(&s),
            "1 réussi · 0 en échec · 1 en erreur · 1 non joué · 900 ms"
        );
        assert_eq!(
            reports(&[
                "rapports/junit.xml".into(),
                "rapports/allure-results/".into()
            ]),
            "Rapports : rapports/junit.xml · rapports/allure-results/"
        );
    }
}
