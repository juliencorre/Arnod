//! Rapport JUnit (`rapports/junit.xml`, R8), au format du schéma JUnit d'Apache Ant repris par
//! Jenkins, GitLab et GitHub : une suite par dossier, un cas par test ou requête jouée seule.

use super::{Case, Meta, Summary, Verdict, cases, escape, iso8601, seconds, step_lines};
use crate::workbench::RunView;

/// Nom d'une suite : le dossier, sinon le projet (cas à la racine).
fn suite_name<'a>(folder: &'a str, meta: &'a Meta) -> &'a str {
    if folder.is_empty() {
        &meta.project
    } else {
        folder
    }
}

/// `classname` d'un cas : projet puis dossiers, séparés par des points.
fn class_name(case: &Case<'_>, meta: &Meta) -> String {
    let mut name = meta.project.replace('.', "_");
    for part in case.folder.split('/').filter(|p| !p.is_empty()) {
        name.push('.');
        name.push_str(&part.replace('.', "_"));
    }
    name
}

fn testcase(out: &mut String, case: &Case<'_>, meta: &Meta) {
    out.push_str(&format!(
        "    <testcase name=\"{}\" classname=\"{}\" time=\"{}\"",
        escape(&case.name),
        escape(&class_name(case, meta)),
        seconds(case.duration_ms)
    ));
    let details = escape(&step_lines(case).join("\n"));
    let message = escape(case.failure.as_deref().unwrap_or(""));
    match case.verdict {
        Verdict::Passed => {
            out.push_str(">\n");
        }
        Verdict::Failed => out.push_str(&format!(
            ">\n      <failure message=\"{message}\" type=\"assertion\">{details}</failure>\n"
        )),
        Verdict::Error => out.push_str(&format!(
            ">\n      <error message=\"{message}\" type=\"erreur\">{details}</error>\n"
        )),
        Verdict::Skipped | Verdict::Cancelled => {
            let why = if case.verdict == Verdict::Cancelled {
                "annulé"
            } else {
                "non joué"
            };
            out.push_str(&format!(">\n      <skipped message=\"{why}\"/>\n"));
        }
    }
    out.push_str(&format!(
        "      <system-out>{details}</system-out>\n    </testcase>\n"
    ));
}

/// Document `junit.xml` d'un run terminé.
pub fn render(view: &RunView, meta: &Meta) -> String {
    let all = cases(view);
    let summary = Summary::of(view, &all);
    let timestamp = iso8601(view.started_at);
    let mut folders: Vec<&str> = Vec::new();
    for case in &all {
        if !folders.contains(&case.folder.as_str()) {
            folders.push(&case.folder);
        }
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuites name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" \
         time=\"{}\" timestamp=\"{timestamp}\">\n",
        escape(&meta.project),
        summary.total,
        summary.failed,
        summary.errors,
        summary.skipped + summary.cancelled,
        seconds(summary.duration_ms)
    ));
    for (id, folder) in folders.iter().enumerate() {
        let suite: Vec<&Case<'_>> = all.iter().filter(|c| c.folder == *folder).collect();
        let count = |v: &[Verdict]| suite.iter().filter(|c| v.contains(&c.verdict)).count();
        let time: u64 = suite.iter().map(|c| c.duration_ms).sum();
        out.push_str(&format!(
            "  <testsuite name=\"{}\" package=\"{}\" id=\"{id}\" tests=\"{}\" failures=\"{}\" \
             errors=\"{}\" skipped=\"{}\" time=\"{}\" timestamp=\"{timestamp}\" \
             hostname=\"arnod\">\n",
            escape(suite_name(folder, meta)),
            escape(&meta.project),
            suite.len(),
            count(&[Verdict::Failed]),
            count(&[Verdict::Error]),
            count(&[Verdict::Skipped, Verdict::Cancelled]),
            seconds(time)
        ));
        out.push_str("    <properties>\n");
        for (name, value) in [
            ("arnod.version", meta.version.as_str()),
            ("arnod.projet", meta.project.as_str()),
            ("arnod.environnement", view.environment.as_str()),
            ("arnod.cible", meta.target.as_str()),
        ] {
            out.push_str(&format!(
                "      <property name=\"{name}\" value=\"{}\"/>\n",
                escape(value)
            ));
        }
        out.push_str("    </properties>\n");
        for case in suite {
            testcase(&mut out, case, meta);
        }
        out.push_str("    <system-out/>\n    <system-err/>\n  </testsuite>\n");
    }
    out.push_str("</testsuites>\n");
    out
}
