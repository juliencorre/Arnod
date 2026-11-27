//! `arnod run` (LC1 à LC4) : même moteur que l'application (l'établi du cœur), en lecture seule
//! (R5). Les variables viennent d'`ARNOD_VAR_<nom>`, puis du poste (R6) ; le code de sortie dit
//! le résultat (R7) ; les rapports décrivent le run calculé par le cœur (R8).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use arnod_core::environment::{self, Environment};
use arnod_core::error::{CommandError, CommandErrorKind};
use arnod_core::report::{self, Meta, Summary};
use arnod_core::secrets::{SecretStore, SystemSecretStore};
use arnod_core::statuses::Statuses;
use arnod_core::tree;
use arnod_core::workbench::{ItemStatus, Notifier, ProjectCtx, RunView, Workbench};
use uuid::Uuid;

use crate::args::{ReportKind, RunArgs};
use crate::output;
use crate::project::{self, Project};
use crate::select::{self, Kind, Plan};
use crate::suggest::closest;

/// Préfixe des variables fournies par l'appelant (R6).
pub const VAR_PREFIX: &str = "ARNOD_VAR_";

/// Codes de sortie (R7).
pub const OK: i32 = 0;
pub const FAILED: i32 = 1;
pub const INVALID: i32 = 2;

/// Coffre du poste lu sans exiger qu'il soit là : un coffre indisponible (agent sans session,
/// exécutable sans coffre) ne donne simplement aucun secret (R6).
struct PosteSecrets(SystemSecretStore);

impl SecretStore for PosteSecrets {
    fn get(&self, key: &str) -> Result<Option<String>, CommandError> {
        Ok(self.0.get(key).ok().flatten())
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), CommandError> {
        Err(CommandError::new(
            CommandErrorKind::SecretStore,
            "La ligne de commande n'écrit rien dans le coffre.",
        ))
    }

    fn delete(&self, _key: &str) -> Result<(), CommandError> {
        Ok(())
    }
}

/// Valeurs `ARNOD_VAR_<nom>` de l'environnement du processus.
pub fn overrides(vars: impl Iterator<Item = (String, String)>) -> BTreeMap<String, String> {
    vars.filter_map(|(key, value)| {
        key.strip_prefix(VAR_PREFIX)
            .filter(|name| !name.is_empty())
            .map(|name| (name.to_owned(), value))
    })
    .collect()
}

/// `{{nom}}` (ou `{{ nom }}`) apparaît dans le texte.
fn uses(text: &str, name: &str) -> bool {
    text.match_indices("{{").any(|(start, _)| {
        text[start + 2..]
            .split_once("}}")
            .is_some_and(|(inside, _)| inside.trim() == name)
    })
}

/// Variables locales de l'environnement sans valeur, utilisées par une requête du run, ses
/// dossiers ou la collection (R6).
fn missing_locals(
    root: &Path,
    environment: &Environment,
    known: &BTreeSet<String>,
    paths: &[String],
) -> Vec<String> {
    let mut texts: Vec<String> = Vec::new();
    let mut folders: BTreeSet<PathBuf> = BTreeSet::new();
    for path in paths {
        if let Ok(full) = tree::resolve(root, path) {
            texts.push(fs::read_to_string(&full).unwrap_or_default());
            let mut dir = full.parent();
            while let Some(current) = dir {
                if !current.starts_with(root) {
                    break;
                }
                folders.insert(current.to_path_buf());
                dir = current.parent();
            }
        }
    }
    for folder in folders {
        for file in ["folder.yml", "opencollection.yml"] {
            if let Ok(text) = fs::read_to_string(folder.join(file)) {
                texts.push(text);
            }
        }
    }
    let mut missing: Vec<String> = environment
        .locals
        .iter()
        .filter(|name| !known.contains(*name))
        .filter(|name| texts.iter().any(|text| uses(text, name)))
        .cloned()
        .collect();
    missing.dedup();
    missing
}

/// Écrit l'avancement à mesure que les tests se terminent, puis rend le run terminé.
struct Progress {
    /// Test lancé seul : une ligne par étape.
    by_step: bool,
    printed: Mutex<HashSet<String>>,
    done: Mutex<Option<Sender<RunView>>>,
    out: Arc<Mutex<Box<dyn Write + Send>>>,
}

fn terminal(status: ItemStatus) -> bool {
    !matches!(status, ItemStatus::Pending | ItemStatus::Running)
}

impl Notifier for Progress {
    fn run_changed(&self, _project: Uuid, run: &RunView) {
        let mut lines = Vec::new();
        if let Ok(mut printed) = self.printed.lock() {
            if self.by_step {
                for (index, item) in run.items.iter().enumerate() {
                    if terminal(item.status) && printed.insert(item.path.clone()) {
                        lines.push(output::step_line(item, index));
                    }
                }
            } else {
                for case in report::cases(run) {
                    if case.steps.iter().all(|s| terminal(s.status))
                        && printed.insert(case.path.clone())
                    {
                        lines.push(output::case_line(&case));
                    }
                }
            }
        }
        if let Ok(mut out) = self.out.lock() {
            for line in lines {
                let _ = writeln!(out, "{line}");
            }
            let _ = out.flush();
        }
        if run.finished
            && let Ok(mut done) = self.done.lock()
            && let Some(sender) = done.take()
        {
            let _ = sender.send(run.clone());
        }
    }

    fn statuses_changed(&self, _project: Uuid, _statuses: &Statuses) {}
}

/// Ce qu'il faut pour lancer : projet, cible, environnement.
struct Prepared {
    project: Project,
    ctx: ProjectCtx,
    plan: Plan,
    environment: Environment,
    paths: Vec<String>,
}

fn prepare(args: &RunArgs, cwd: &Path, data_dir: Option<PathBuf>) -> Result<Prepared, String> {
    let start = args.project.as_deref().map_or_else(
        || cwd.to_path_buf(),
        |p| {
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            }
        },
    );
    let project = project::find(&start)?;
    let nodes = tree::read(&project.root).map_err(|e| e.message)?;
    let plan = select::resolve(&nodes, args.target.as_deref(), &args.tags)?;
    let environments = environment::list(&project.root);
    let environment = match &args.env {
        Some(wanted) => environments
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(wanted.trim()))
            .cloned()
            .ok_or_else(|| {
                let mut message = format!("L'environnement « {wanted} » n'existe pas.");
                if let Some(near) = closest(wanted, environments.iter().map(|e| e.name.as_str())) {
                    message.push_str(&format!(" Vouliez-vous dire « {near} » ?"));
                } else if !environments.is_empty() {
                    let names: Vec<String> = environments
                        .iter()
                        .map(|e| format!("« {} »", e.name))
                        .collect();
                    message.push_str(&format!(" Environnements : {}.", names.join(", ")));
                }
                message
            })?,
        None => environments.first().cloned().ok_or_else(|| {
            "Ce projet n'a aucun environnement : créez-en un dans Arnod.".to_owned()
        })?,
    };
    let mut paths = tree::requests_in(&project.root, &plan.folder).map_err(|e| e.message)?;
    if let Some(only) = &plan.only {
        paths.retain(|p| only.contains(p));
    }
    if paths.is_empty() {
        return Err(format!(
            "{} ne contient aucune requête à jouer.",
            plan.title
        ));
    }
    let ctx = ProjectCtx {
        id: project.id.unwrap_or(Uuid::nil()),
        root: project.root.clone(),
        // Sans dossier de données, un chemin absent : rien à lire, rien n'y est écrit (R5).
        data_dir: data_dir
            .unwrap_or_else(|| std::env::temp_dir().join(format!("arnod-{}", Uuid::new_v4()))),
    };
    Ok(Prepared {
        project,
        ctx,
        plan,
        environment,
        paths,
    })
}

fn shown(dir: &Path, name: &str, folder: bool) -> String {
    let mut text = dir.join(name).display().to_string();
    if folder {
        text.push(MAIN_SEPARATOR);
    }
    text
}

/// Écrit les rapports demandés ; rend leurs chemins, tels qu'affichés.
fn write_reports(
    kinds: &[ReportKind],
    dir: &Path,
    shown_dir: &Path,
    view: &RunView,
    meta: &Meta,
) -> Result<Vec<String>, String> {
    let failed = |path: &Path, error: std::io::Error| {
        format!(
            "Le rapport {} n'a pas pu être écrit : {error}.",
            path.display()
        )
    };
    fs::create_dir_all(dir).map_err(|e| failed(dir, e))?;
    let mut paths = Vec::new();
    for kind in kinds {
        match kind {
            ReportKind::Junit => {
                let path = dir.join("junit.xml");
                fs::write(&path, report::junit::render(view, meta))
                    .map_err(|e| failed(&path, e))?;
                paths.push(shown(shown_dir, "junit.xml", false));
            }
            ReportKind::Allure => {
                let results = dir.join("allure-results");
                fs::create_dir_all(&results).map_err(|e| failed(&results, e))?;
                // Les résultats d'un run précédent ne s'ajoutent pas à celui-ci.
                if let Ok(entries) = fs::read_dir(&results) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.ends_with("-result.json") || name == "environment.properties" {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
                for (name, text) in report::allure::render(view, meta) {
                    let path = results.join(name);
                    fs::write(&path, text).map_err(|e| failed(&path, e))?;
                }
                paths.push(shown(shown_dir, "allure-results", true));
            }
            ReportKind::Html => {
                let path = dir.join("rapport.html");
                fs::write(&path, report::html::render(view, meta)).map_err(|e| failed(&path, e))?;
                paths.push(shown(shown_dir, "rapport.html", false));
            }
        }
    }
    Ok(paths)
}

/// Lance `arnod run` ; rend le code de sortie (R7). `env` lit les variables du processus.
pub fn run(
    args: &RunArgs,
    cwd: &Path,
    vars: &[(String, String)],
    out: Box<dyn Write + Send>,
    err: &mut dyn Write,
) -> i32 {
    let lookup = |name: &str| {
        vars.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    let prepared = match prepare(args, cwd, project::data_dir(lookup)) {
        Ok(prepared) => prepared,
        Err(message) => {
            let _ = writeln!(err, "{message}");
            return INVALID;
        }
    };
    let Prepared {
        project,
        ctx,
        plan,
        environment,
        paths,
    } = prepared;

    let bench = Arc::new(Workbench::new(Arc::new(PosteSecrets(SystemSecretStore))));
    bench.set_read_only();
    bench.network().read_prefs(&ctx.data_dir);
    if let Some(parallel) = args.parallel {
        bench.set_run_concurrency(parallel);
    }
    bench.set_overrides(overrides(vars.iter().cloned()));

    // Une variable locale sans valeur arrête tout avant le premier envoi (R6).
    let known: BTreeSet<String> = match bench.variables(&ctx, &environment.name) {
        Ok((variables, _)) => variables.values.keys().cloned().collect(),
        Err(error) => {
            let _ = writeln!(err, "{}", error.message);
            return INVALID;
        }
    };
    let missing = missing_locals(&ctx.root, &environment, &known, &paths);
    if !missing.is_empty() {
        let names: Vec<String> = missing.iter().map(|n| format!("« {n} »")).collect();
        let _ = writeln!(
            err,
            "{} sans valeur sur ce poste pour l'environnement « {} » : {}. Donnez-{} par {}.",
            if missing.len() > 1 {
                "Variables locales"
            } else {
                "Variable locale"
            },
            environment.name,
            names.join(", "),
            if missing.len() > 1 { "les" } else { "la" },
            missing
                .iter()
                .map(|n| format!("{VAR_PREFIX}{n}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return INVALID;
    }

    let out = Arc::new(Mutex::new(out));
    let say = |line: &str| {
        if let Ok(mut out) = out.lock() {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    };
    say(&output::header(
        env!("CARGO_PKG_VERSION"),
        &project.name,
        &environment.name,
    ));

    let (sender, receiver) = mpsc::channel();
    let progress = Arc::new(Progress {
        by_step: plan.kind == Kind::Test,
        printed: Mutex::new(HashSet::new()),
        done: Mutex::new(Some(sender)),
        out: out.clone(),
    });
    bench.set_notifier(progress);
    let started = match bench.start_run(
        &ctx,
        &plan.folder,
        &environment.name,
        plan.only.clone(),
        args.confirm,
    ) {
        Ok(view) => view,
        Err(error) => {
            let mut message = error.message.clone();
            if error.kind == CommandErrorKind::ProtectedEnvironment {
                message.push_str(" Ajoutez --confirmer pour envoyer quand même.");
            }
            let _ = writeln!(err, "{message}");
            return INVALID;
        }
    };
    let initial = report::cases(&started);
    say(&output::scope(
        &plan,
        initial.len(),
        started.items.len(),
        report::folder_count(&initial),
        started.concurrency,
    ));
    let view = if started.finished {
        started
    } else {
        match receiver.recv() {
            Ok(view) => view,
            Err(_) => {
                let _ = writeln!(err, "Le run s'est interrompu avant la fin.");
                return INVALID;
            }
        }
    };

    let cases = report::cases(&view);
    let summary = Summary::of(&view, &cases);
    say(&output::summary(&summary));
    let mut code = if summary.failing() || view.cancelled {
        FAILED
    } else {
        OK
    };
    if !args.reports.is_empty() {
        let shown_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("rapports"));
        let dir = if shown_dir.is_absolute() {
            shown_dir.clone()
        } else {
            cwd.join(&shown_dir)
        };
        let meta = Meta {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            project: project.name.clone(),
            target: plan.title.clone(),
        };
        match write_reports(&args.reports, &dir, &shown_dir, &view, &meta) {
            Ok(paths) => say(&output::reports(&paths)),
            Err(message) => {
                let _ = writeln!(err, "{message}");
                code = INVALID;
            }
        }
    }
    say(&format!("Code de sortie : {code}"));
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r6_variables_de_l_appelant() {
        let vars = vec![
            ("ARNOD_VAR_token".to_owned(), "abc".to_owned()),
            ("ARNOD_VAR_".to_owned(), "x".to_owned()),
            ("PATH".to_owned(), "/bin".to_owned()),
        ];
        assert_eq!(
            overrides(vars.into_iter()),
            BTreeMap::from([("token".to_owned(), "abc".to_owned())])
        );
        assert!(uses("Bearer {{token}}", "token"));
        assert!(uses("{{ token }}", "token"));
        assert!(!uses("{{tokens}}", "token"));
        assert!(!uses("token", "token"));
    }
}
