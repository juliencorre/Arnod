//! Rapport HTML (`rapports/rapport.html`, R8) : un seul fichier autonome, sans script ni
//! ressource distante, lisible hors d'Arnod (clair ou sombre selon le système). Le verdict de
//! chaque test se lit à son signe (✓, ✕, –), jamais à la couleur seule.

use super::{
    Case, Meta, Summary, Verdict, cases, check_label, duration_fr, escape, folder_count, iso8601,
    step_status,
};
use crate::workbench::{ItemStatus, RunItem, RunView};

/// Couleurs reprises du design system d'Arnod (`src/styles.css`), pour les deux thèmes.
const STYLE: &str = "
:root { color-scheme: light dark; --bg: #fbfaf8; --surface: #ffffff; --text: #1d1b18;
  --muted: #6b665e; --line: #e6e2db; --ok: #2f7a4b; --ko: #b42318; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #1a1917; --surface: #23211e; --text: #ede9e3; --muted: #b0a99e;
    --line: #36322d; --ok: #6cc28b; --ko: #ff8a80; }
}
* { box-sizing: border-box; }
body { margin: 0; padding: 24px 16px; background: var(--bg); color: var(--text);
  font: 13px/1.5 system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif; }
main { max-width: 960px; margin: 0 auto; }
h1 { font-size: 20px; font-weight: 600; margin: 0 0 4px; }
h2 { font-size: 15px; font-weight: 600; margin: 24px 0 8px; }
p.meta { color: var(--muted); margin: 0 0 16px; }
.summary { display: flex; flex-wrap: wrap; gap: 8px; margin: 0 0 8px; padding: 0; list-style: none; }
.summary li { background: var(--surface); border: 1px solid var(--line); border-radius: 6px;
  padding: 8px 12px; }
.summary strong { font-size: 18px; font-weight: 600; display: block; }
details { background: var(--surface); border: 1px solid var(--line); border-radius: 6px;
  margin: 0 0 6px; }
summary { cursor: pointer; padding: 8px 12px; display: flex; gap: 8px; align-items: baseline; }
summary .name { flex: 1; font-weight: 600; }
summary .time, .muted { color: var(--muted); }
.mark { font-weight: 600; width: 1.2em; text-align: center; }
.passed .mark { color: var(--ok); }
.failed .mark, .error .mark { color: var(--ko); }
.failure { color: var(--ko); margin: 0 12px 8px 36px; }
ol { margin: 0 12px 12px 36px; padding: 0 0 0 16px; }
li.step { margin: 4px 0; }
code { font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: 12px; }
ul.checks { margin: 2px 0 0; padding-left: 16px; }
footer { color: var(--muted); margin-top: 24px; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
";

fn mark(verdict: Verdict) -> (&'static str, &'static str, &'static str) {
    match verdict {
        Verdict::Passed => ("✓", "passed", "réussi"),
        Verdict::Failed => ("✕", "failed", "en échec"),
        Verdict::Error => ("✕", "error", "en erreur"),
        Verdict::Skipped => ("–", "skipped", "non joué"),
        Verdict::Cancelled => ("–", "skipped", "annulé"),
    }
}

fn step(out: &mut String, item: &RunItem) {
    out.push_str(&format!(
        "<li class=\"step\"><code>{}</code> {}",
        item.method.as_str(),
        escape(&item.name)
    ));
    let mut facts = Vec::new();
    if let Some(status) = item.http_status {
        facts.push(status.to_string());
    }
    if let Some(ms) = item.duration_ms {
        facts.push(duration_fr(ms));
    }
    facts.push(step_status(item.status).to_owned());
    out.push_str(&format!(
        " <span class=\"muted\">· {}</span>",
        escape(&facts.join(" · "))
    ));
    if item.status == ItemStatus::Error
        && let Some(error) = &item.error
    {
        out.push_str(&format!("<div class=\"failure\">{}</div>", escape(error)));
    }
    let failed: Vec<_> = item
        .assertions
        .iter()
        .filter(|c| !c.passed && !c.ignored)
        .collect();
    if !failed.is_empty() {
        out.push_str("<ul class=\"checks\">");
        for check in failed {
            out.push_str(&format!(
                "<li>✕ <code>{}</code> — {}",
                escape(&check_label(check)),
                escape(check.message.as_deref().unwrap_or("en échec"))
            ));
            if let Some(actual) = &check.actual {
                out.push_str(&format!(
                    " <span class=\"muted\">(reçu <code>{}</code>)</span>",
                    escape(actual)
                ));
            }
            out.push_str("</li>");
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>");
}

fn case(out: &mut String, case: &Case<'_>) {
    let (sign, class, label) = mark(case.verdict);
    let open = if case.verdict.failing() { " open" } else { "" };
    out.push_str(&format!(
        "<details class=\"{class}\"{open}><summary><span class=\"mark\" aria-hidden=\"true\">\
         {sign}</span><span class=\"name\">{}</span><span class=\"sr\">{label}</span>\
         <span class=\"time\">{}</span></summary>",
        escape(&case.name),
        duration_fr(case.duration_ms)
    ));
    if let Some(failure) = &case.failure {
        out.push_str(&format!("<p class=\"failure\">{}</p>", escape(failure)));
    }
    out.push_str("<ol>");
    for item in &case.steps {
        step(out, item);
    }
    out.push_str("</ol></details>");
}

/// Document `rapport.html` d'un run terminé.
pub fn render(view: &RunView, meta: &Meta) -> String {
    let all = cases(view);
    let summary = Summary::of(view, &all);
    let mut out = String::from(
        "<!doctype html>\n<html lang=\"fr\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
    );
    out.push_str(&format!(
        "<title>Rapport Arnod · {}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<main>\n",
        escape(&meta.project)
    ));
    out.push_str(&format!(
        "<h1>{} · {}</h1>\n<p class=\"meta\">Environnement {} · {} · {} test{} dans {} \
         dossier{} · {} UTC</p>\n",
        escape(&meta.project),
        escape(&meta.target),
        escape(&view.environment),
        if summary.failing() {
            "en échec"
        } else {
            "réussi"
        },
        summary.total,
        if summary.total > 1 { "s" } else { "" },
        folder_count(&all),
        if folder_count(&all) > 1 { "s" } else { "" },
        iso8601(view.started_at).replace('T', " ")
    ));
    out.push_str("<ul class=\"summary\">");
    for (count, label) in [
        (summary.passed, "réussis"),
        (summary.failed, "en échec"),
        (summary.errors, "en erreur"),
        (summary.skipped + summary.cancelled, "non joués"),
    ] {
        if count > 0 || label == "réussis" {
            out.push_str(&format!("<li><strong>{count}</strong>{label}</li>"));
        }
    }
    out.push_str(&format!(
        "<li><strong>{}</strong>durée</li></ul>\n",
        duration_fr(summary.duration_ms)
    ));
    let failing: Vec<&Case<'_>> = all.iter().filter(|c| c.verdict.failing()).collect();
    if !failing.is_empty() {
        out.push_str("<h2>En échec</h2>\n");
        for item in failing {
            case(&mut out, item);
        }
    }
    let others: Vec<&Case<'_>> = all.iter().filter(|c| !c.verdict.failing()).collect();
    if !others.is_empty() {
        out.push_str("<h2>Autres tests</h2>\n");
        for item in others {
            case(&mut out, item);
        }
    }
    out.push_str(&format!(
        "<footer>Arnod {} · les valeurs secrètes sont masquées.</footer>\n</main>\n</body>\n\
         </html>\n",
        escape(&meta.version)
    ));
    out
}
