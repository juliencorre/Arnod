//! Parcours d'un test en schéma (`docs/plan-v4.md`, R5 à R8).
//!
//! Le parcours ne connaît ni le disque ni le réseau : il suit les renvois et demande à un
//! [`Runner`] de jouer une requête, un script, un test appelé ou un template, et de rendre compte
//! de chaque étape. L'établi fournit le sien (`workbench`), les tests un faux.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use super::check::{self, CheckOutcome};
use super::{Action, END, FAIL, Flow, MAX_ATTEMPTS, MAX_DEPTH, MAX_LOOP, MAX_STEPS, OnFailure};
use crate::request::assertions::ResponseData;
use crate::request::extract::ExtractedValue;
use crate::script::ScriptOutcome;

/// Tâche asynchrone rendue par un [`Runner`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Requête jouée par une étape.
#[derive(Debug, Clone, Default)]
pub struct RequestDone {
    pub passed: bool,
    /// Réponse reçue ; `None` pour une erreur (réseau, variable inconnue…).
    pub response: Option<ResponseData>,
    /// Valeurs extraites, non masquées : elles passent aux étapes suivantes.
    pub extracted: BTreeMap<String, String>,
}

/// Test ou template joué par une étape.
#[derive(Debug, Clone, Default)]
pub struct CallDone {
    pub passed: bool,
    /// Étapes réussies et jouées (« 9/12 »).
    pub passed_steps: usize,
    pub total_steps: usize,
    /// Raison d'un échec, en français.
    pub message: Option<String>,
}

/// Ce qu'une étape appelle : un test du référentiel ou un template.
#[derive(Debug, Clone, Copy)]
pub enum Call<'a> {
    Test(&'a str),
    Template {
        template: &'a str,
        values: &'a BTreeMap<String, String>,
    },
}

/// État d'une étape, rendu au [`Runner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Running,
    Passed,
    Failed,
    Error,
}

/// Compte rendu d'une étape (R8).
#[derive(Debug, Clone)]
pub struct Report {
    pub state: StepState,
    /// Résumé court : « oui », « 3 tours », « 9/12 », « après 3 essais ».
    pub detail: Option<String>,
    pub error: Option<String>,
    /// Branche prise par une condition.
    pub branch: Option<bool>,
    /// Valeurs posées par l'étape (variable, script), non masquées : le [`Runner`] les masque.
    pub values: Vec<ExtractedValue>,
    pub duration_ms: Option<u64>,
}

impl Report {
    pub fn running() -> Self {
        Self {
            state: StepState::Running,
            detail: None,
            error: None,
            branch: None,
            values: Vec::new(),
            duration_ms: None,
        }
    }

    fn done(passed: bool) -> Self {
        Self {
            state: if passed {
                StepState::Passed
            } else {
                StepState::Failed
            },
            ..Self::running()
        }
    }
}

/// Ce que l'établi fournit au parcours.
pub trait Runner: Send + Sync {
    /// Joue la requête `file` de l'étape `step` avec les variables d'exécution `vars` ; rend
    /// compte de l'étape lui-même (statut HTTP, assertions). `attempt` compte les essais d'une
    /// attente (1 pour une requête).
    fn request<'a>(
        &'a self,
        step: &'a str,
        file: &'a str,
        vars: &'a BTreeMap<String, String>,
        attempt: u32,
    ) -> BoxFuture<'a, RequestDone>;

    /// Joue un test appelé ou un template avec `vars`, qu'il complète de ce que le test extrait.
    fn call<'a>(
        &'a self,
        step: &'a str,
        call: Call<'a>,
        vars: &'a mut BTreeMap<String, String>,
        depth: usize,
    ) -> BoxFuture<'a, CallDone>;

    /// Exécute un script (R9).
    fn script<'a>(
        &'a self,
        code: &'a str,
        vars: &'a BTreeMap<String, String>,
        response: Option<&'a ResponseData>,
    ) -> BoxFuture<'a, ScriptOutcome>;

    /// Remplace les `{{…}}` d'un texte ; rend la raison d'un échec (variable inconnue).
    fn substitute(&self, text: &str, vars: &BTreeMap<String, String>) -> Result<String, String>;

    /// Variables de l'environnement et des dossiers, sous les variables d'exécution, pour une
    /// condition ou un script qui lit `vars`.
    fn base(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Compte rendu d'une étape autre qu'une requête.
    fn report(&self, step: &str, report: Report);

    fn cancelled(&self) -> bool;

    fn sleep(&self, ms: u64) -> BoxFuture<'_, ()>;
}

/// Fin d'un parcours.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowDone {
    pub passed: bool,
    pub cancelled: bool,
    /// Première étape en échec.
    pub failed_step: Option<String>,
    /// Raison de l'échec du test (« Fin en échec », plafond atteint…), en français.
    pub reason: Option<String>,
    /// Étapes jouées, passages compris, et réussies.
    pub played: usize,
    pub passed_steps: usize,
}

/// Boucle en cours.
struct Looping {
    /// Position de l'étape boucle.
    at: usize,
    /// Position de la dernière étape répétée.
    last: usize,
    items: Vec<String>,
    /// Tour en cours (à partir de 0).
    turn: usize,
    item: String,
    id: String,
}

/// Résultat d'une étape pour la suite du parcours.
enum Outcome {
    Passed,
    Failed,
    /// Condition : oui ou non.
    Branch(bool),
    /// Boucle sans élément : ses étapes sont sautées.
    SkipTo(Option<usize>),
}

fn millis(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn value(name: &str, text: &str) -> ExtractedValue {
    ExtractedValue {
        name: name.to_owned(),
        source: String::new(),
        value: Some(text.to_owned()),
        message: None,
    }
}

/// Variables lues par une condition ou un script : celles de l'environnement et des dossiers,
/// sous les variables d'exécution.
fn readable(runner: &dyn Runner, vars: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut all = runner.base();
    all.extend(vars.iter().map(|(k, v)| (k.clone(), v.clone())));
    all
}

/// Joue le schéma `flow` (R5) avec les variables d'exécution `vars`, qu'il complète. `depth`
/// compte les tests appelés au-dessus (R7).
pub async fn run(
    flow: &Flow,
    runner: &dyn Runner,
    vars: &mut BTreeMap<String, String>,
    depth: usize,
) -> FlowDone {
    let mut done = FlowDone {
        passed: true,
        ..FlowDone::default()
    };
    if depth > MAX_DEPTH {
        done.passed = false;
        done.reason = Some(format!(
            "Plus de {MAX_DEPTH} tests appelés les uns dans les autres : un test s'appelle-t-il lui-même ?"
        ));
        return done;
    }
    let mut last: Option<ResponseData> = None;
    let mut loops: Vec<Looping> = Vec::new();
    let mut index = (!flow.steps.is_empty()).then_some(0);
    while let Some(at) = index {
        let Some(step) = flow.steps.get(at) else {
            break;
        };
        if runner.cancelled() {
            done.passed = false;
            done.cancelled = true;
            return done;
        }
        done.played += 1;
        if done.played > MAX_STEPS {
            done.passed = false;
            done.reason = Some(format!(
                "Plus de {MAX_STEPS} étapes jouées : le test est arrêté (renvois en boucle ?)."
            ));
            break;
        }
        let on_failure = step.on_failure.unwrap_or(flow.on_failure);
        // Étape enregistrée « à configurer » : elle échoue sans rien jouer.
        let missing = super::reconcile::incomplete(step);
        let outcome = match &step.action {
            _ if missing.is_some() => {
                let mut report = Report::done(false);
                report.state = StepState::Error;
                report.error = missing.map(|what| format!("Étape à configurer : {what}."));
                runner.report(&step.id, report);
                Outcome::Failed
            }
            Action::Request { file } => {
                let result = runner.request(&step.id, file, vars, 1).await;
                vars.extend(result.extracted);
                if result.response.is_some() {
                    last = result.response;
                }
                if result.passed {
                    Outcome::Passed
                } else {
                    Outcome::Failed
                }
            }
            Action::Condition { check, .. } => {
                let started = std::time::Instant::now();
                let readable = readable(runner, vars);
                let CheckOutcome { holds, actual, .. } =
                    check::evaluate(check, last.as_ref(), &readable);
                let mut report = Report::done(true);
                report.branch = Some(holds);
                report.detail = Some(if holds { "oui" } else { "non" }.to_owned());
                report.values = actual
                    .map(|a| vec![value(&check.target, &a)])
                    .unwrap_or_default();
                report.duration_ms = Some(millis(started));
                runner.report(&step.id, report);
                Outcome::Branch(holds)
            }
            Action::Variable {
                variable,
                value: text,
            } => {
                let readable = readable(runner, vars);
                match runner.substitute(text, &readable) {
                    Ok(resolved) => {
                        let name = variable.trim().to_owned();
                        vars.insert(name.clone(), resolved.clone());
                        let mut report = Report::done(true);
                        report.values = vec![value(&name, &resolved)];
                        runner.report(&step.id, report);
                        Outcome::Passed
                    }
                    Err(message) => {
                        let mut report = Report::done(false);
                        report.state = StepState::Error;
                        report.error = Some(message);
                        runner.report(&step.id, report);
                        Outcome::Failed
                    }
                }
            }
            Action::Script { code } => {
                runner.report(&step.id, Report::running());
                let readable = readable(runner, vars);
                let outcome = runner.script(code, &readable, last.as_ref()).await;
                vars.extend(outcome.set.clone());
                let mut report = Report::done(outcome.passed);
                report.error = outcome.error.clone();
                report.values = outcome.set.iter().map(|(k, v)| value(k, v)).collect();
                report.duration_ms = Some(outcome.duration_ms);
                report.detail = (!outcome.logs.is_empty()).then(|| outcome.logs.join("\n"));
                runner.report(&step.id, report);
                if outcome.passed {
                    Outcome::Passed
                } else {
                    Outcome::Failed
                }
            }
            Action::Wait {
                file,
                check,
                interval_ms,
                attempts,
            } => {
                let started = std::time::Instant::now();
                runner.report(&step.id, Report::running());
                let attempts = (*attempts).clamp(1, MAX_ATTEMPTS);
                let mut held = false;
                let mut tries = 0;
                let mut actual = None;
                for attempt in 1..=attempts {
                    tries = attempt;
                    if attempt > 1 {
                        runner.sleep(*interval_ms).await;
                        if runner.cancelled() {
                            break;
                        }
                    }
                    if let Some(file) = file {
                        let result = runner.request(&step.id, file, vars, attempt).await;
                        vars.extend(result.extracted);
                        if result.response.is_some() {
                            last = result.response;
                        }
                    }
                    let readable = readable(runner, vars);
                    let outcome = check::evaluate(check, last.as_ref(), &readable);
                    actual = outcome.actual;
                    if outcome.holds {
                        held = true;
                        break;
                    }
                }
                let mut report = Report::done(held);
                report.detail = Some(if tries <= 1 {
                    "1 essai".to_owned()
                } else {
                    format!("{tries} essais")
                });
                if !held {
                    report.error = Some(format!(
                        "Condition fausse après {tries} essai{} ({} reçu : {}).",
                        if tries > 1 { "s" } else { "" },
                        check.target,
                        actual.unwrap_or_else(|| "rien".to_owned())
                    ));
                }
                report.duration_ms = Some(millis(started));
                runner.report(&step.id, report);
                if held {
                    Outcome::Passed
                } else {
                    Outcome::Failed
                }
            }
            Action::Loop {
                count,
                list,
                item,
                until,
                ..
            } => {
                let readable = readable(runner, vars);
                let items: Result<Vec<String>, String> = match (count, list) {
                    (_, Some(list)) if !list.trim().is_empty() => {
                        check::list_items(list, last.as_ref(), &readable)
                    }
                    (Some(n), _) => Ok((1..=*n as usize).map(|i| i.to_string()).collect()),
                    _ => Ok(Vec::new()),
                };
                let last_index = until
                    .as_deref()
                    .and_then(|id| flow.position(id))
                    .filter(|&p| p > at)
                    .unwrap_or(at + 1)
                    .min(flow.steps.len().saturating_sub(1));
                match items {
                    Err(message) => {
                        let mut report = Report::done(false);
                        report.state = StepState::Error;
                        report.error = Some(message);
                        runner.report(&step.id, report);
                        Outcome::Failed
                    }
                    Ok(items) if items.is_empty() || last_index <= at => {
                        let mut report = Report::done(true);
                        report.detail = Some("0 tour".to_owned());
                        runner.report(&step.id, report);
                        Outcome::SkipTo(Some(last_index + 1))
                    }
                    Ok(mut items) => {
                        items.truncate(MAX_LOOP);
                        let name = if item.trim().is_empty() {
                            "item".to_owned()
                        } else {
                            item.trim().to_owned()
                        };
                        if let Some(first) = items.first() {
                            vars.insert(name.clone(), first.clone());
                        }
                        let mut report = Report::running();
                        report.detail = Some(format!("tour 1/{}", items.len()));
                        runner.report(&step.id, report);
                        loops.push(Looping {
                            at,
                            last: last_index,
                            items,
                            turn: 0,
                            item: name,
                            id: step.id.clone(),
                        });
                        Outcome::Passed
                    }
                }
            }
            Action::Test { test } => {
                runner.report(&step.id, Report::running());
                let result = runner
                    .call(&step.id, Call::Test(test), vars, depth + 1)
                    .await;
                report_call(runner, &step.id, &result);
                if result.passed {
                    Outcome::Passed
                } else {
                    Outcome::Failed
                }
            }
            Action::Template { template, values } => {
                runner.report(&step.id, Report::running());
                let result = runner
                    .call(
                        &step.id,
                        Call::Template { template, values },
                        vars,
                        depth + 1,
                    )
                    .await;
                report_call(runner, &step.id, &result);
                if result.passed {
                    Outcome::Passed
                } else {
                    Outcome::Failed
                }
            }
        };
        let failed = matches!(outcome, Outcome::Failed);
        if failed {
            done.passed = false;
            done.failed_step.get_or_insert_with(|| step.id.clone());
        } else if !matches!(outcome, Outcome::SkipTo(_)) {
            done.passed_steps += 1;
        }
        if failed && on_failure == OnFailure::Stop {
            break;
        }
        // Suite de l'étape (R5) : une condition choisit sa branche ; une boucle entre dans ses
        // étapes.
        let entered = matches!(&step.action, Action::Loop { .. })
            && loops
                .last()
                .is_some_and(|top| top.at == at && top.turn == 0)
            && matches!(outcome, Outcome::Passed);
        let mut target: Option<usize> = if entered {
            Some(at + 1)
        } else {
            match outcome {
                Outcome::SkipTo(next) => next.filter(|n| *n < flow.steps.len()),
                Outcome::Branch(false) => {
                    let otherwise = match &step.action {
                        Action::Condition { otherwise, .. } => otherwise.clone(),
                        _ => None,
                    };
                    let next = match otherwise {
                        Some(target) => resolve(flow, &target, &mut done),
                        None => follow(flow, at, &mut done),
                    };
                    next.unwrap_or_default()
                }
                _ => follow(flow, at, &mut done).unwrap_or_default(),
            }
        };
        // Fin des étapes d'une boucle : tour suivant, ou sortie (Q5) ; un renvoi qui sort d'une
        // boucle la termine.
        while let Some(top) = loops.last_mut() {
            if top.last == at && !entered {
                top.turn += 1;
                if let Some(item) = top.items.get(top.turn).cloned() {
                    vars.insert(top.item.clone(), item);
                    let mut report = Report::running();
                    report.detail = Some(format!("tour {}/{}", top.turn + 1, top.items.len()));
                    runner.report(&top.id, report);
                    target = Some(top.at + 1);
                    break;
                }
            } else if target.is_some_and(|t| t > top.at && t <= top.last) {
                break;
            }
            let turns = top.turn.min(top.items.len()).max(1);
            let mut report = Report::done(true);
            report.detail = Some(format!("{turns} tour{}", if turns > 1 { "s" } else { "" }));
            runner.report(&top.id, report);
            loops.pop();
        }
        if done.reason.is_some() {
            break;
        }
        index = target;
    }
    // Boucles interrompues (échec, fin) : leur compte rendu est clos.
    for top in loops.drain(..).rev() {
        let mut report = Report::done(true);
        let turns = (top.turn + 1).min(top.items.len());
        report.detail = Some(format!("{turns} tour{}", if turns > 1 { "s" } else { "" }));
        runner.report(&top.id, report);
    }
    done
}

fn report_call(runner: &dyn Runner, step: &str, result: &CallDone) {
    let mut report = Report::done(result.passed);
    report.detail = Some(format!("{}/{}", result.passed_steps, result.total_steps));
    report.error = result.message.clone();
    runner.report(step, report);
}

/// Position de la suite de l'étape `at` ; `None` : le parcours s'arrête (fin ou fin en échec,
/// notée dans `done`).
fn follow(flow: &Flow, at: usize, done: &mut FlowDone) -> Option<Option<usize>> {
    resolve(flow, &flow.following(at), done)
}

/// Position d'un renvoi ; `None` pour une fin (« Fin en échec » notée dans `done`). Un renvoi
/// vers une étape inconnue vaut l'étape suivante, déjà résolue par la réconciliation (R2).
fn resolve(flow: &Flow, target: &str, done: &mut FlowDone) -> Option<Option<usize>> {
    match target {
        END => None,
        FAIL => {
            done.passed = false;
            done.reason = Some("Fin en échec".to_owned());
            None
        }
        id => Some(flow.position(id)),
    }
}
