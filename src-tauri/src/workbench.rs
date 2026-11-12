//! Établi du projet ouvert : envoi d'une requête, runs, environnements actifs.
//!
//! Relie les modules purs (requête, environnement, HTTP) au disque, au coffre
//! des secrets et aux exécutions en cours. Les commandes en sont la façade.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::contract::file as contract_file;
use crate::environment::{self, Environment};
use crate::error::{CommandError, CommandErrorKind};
use crate::http::{self, Cancel, HttpResponse};
use crate::local::{self, LocalData, LocalVar};
use crate::request::assertions::{self, AssertionResult, DEFAULT_LIMIT};
use crate::request::model::{Method, Request};
use crate::request::resolve::{self, Variables};
use crate::request::{file, inherit};
use crate::secrets::{self, SecretStore};
use crate::statuses::{self, StatusBook, Statuses};
use crate::tree;

/// Identifiant du résultat de conformité au contrat d'une requête liée.
pub const CONTRACT_CHECK: &str = "contrat";

/// Requêtes envoyées en parallèle pendant un run.
pub const RUN_CONCURRENCY: usize = 8;

/// Runs terminés gardés en mémoire pour être relus ou relancés (R15) ; les
/// plus anciens sont oubliés, un run en cours ne l'est jamais.
pub const MAX_FINISHED_RUNS: usize = 20;

/// Où se trouve un projet : identifiant, dossier et données locales.
#[derive(Debug, Clone)]
pub struct ProjectCtx {
    pub id: Uuid,
    pub root: PathBuf,
    pub data_dir: PathBuf,
}

/// Variable locale vue par l'interface : un secret n'expose jamais sa valeur (R9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVarView {
    pub name: String,
    pub secret: bool,
    /// Valeur d'une variable non secrète ; `null` pour un secret.
    pub value: Option<String>,
    pub has_value: bool,
}

/// Environnement vu par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentView {
    #[serde(flatten)]
    pub environment: Environment,
    pub local_vars: Vec<LocalVarView>,
}

/// Environnements du projet et choix courant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentsView {
    pub environments: Vec<EnvironmentView>,
    pub active: String,
}

/// Résultat d'un envoi.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendOutcome {
    pub response: HttpResponse,
    pub assertions: Vec<AssertionResult>,
    pub passed: bool,
    pub slow: bool,
}

/// État d'une requête dans un run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Error,
    Cancelled,
}

/// Ligne d'un run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunItem {
    pub path: String,
    pub name: String,
    pub method: Method,
    pub status: ItemStatus,
    pub http_status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub slow: bool,
    pub assertions: Vec<AssertionResult>,
    pub error: Option<String>,
}

/// Résultat d'un groupe de requêtes d'un run (`docs/plan-fin-premier-usage.md`, R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunGroup {
    /// Nom affiché : le dossier, ou « Projet » pour la racine.
    pub name: String,
    /// Chemin du dossier, relatif au projet (vide pour la racine).
    pub path: String,
    pub passed: usize,
    /// Requêtes en échec ou en erreur.
    pub failed: usize,
    pub total: usize,
}

/// Run vu par l'interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub id: Uuid,
    /// Numéro du run dans le projet (R1) ; 0 pour un run gardé par une version précédente.
    #[serde(default)]
    pub number: u32,
    pub folder: String,
    pub environment: String,
    /// Requêtes envoyées en même temps (R1).
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    pub started_at: u64,
    /// Fin du run, une fois terminé (R1).
    #[serde(default)]
    pub finished_at: Option<u64>,
    pub finished: bool,
    pub cancelled: bool,
    pub items: Vec<RunItem>,
    /// Nom du contrat dont les requêtes sont exécutées (C10), sinon `None`.
    #[serde(default)]
    pub contract: Option<String>,
    /// Révision, croissante à chaque changement (R3).
    #[serde(default)]
    pub revision: u64,
    /// Résultat par dossier (R2), recalculé à chaque changement.
    #[serde(default)]
    pub groups: Vec<RunGroup>,
}

fn default_concurrency() -> usize {
    RUN_CONCURRENCY
}

/// Groupe d'une requête du run : premier segment de son chemin sous le
/// dossier exécuté, ou ce dossier lui-même (R2).
fn group_of(folder: &str, path: &str) -> (String, String) {
    let relative = if folder.is_empty() {
        path
    } else {
        path.strip_prefix(folder)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(path)
    };
    match relative.split_once('/') {
        Some((first, _)) => {
            let path = if folder.is_empty() {
                first.to_owned()
            } else {
                format!("{folder}/{first}")
            };
            (first.to_owned(), path)
        }
        None if folder.is_empty() => ("Projet".to_owned(), String::new()),
        None => (
            folder.rsplit('/').next().unwrap_or(folder).to_owned(),
            folder.to_owned(),
        ),
    }
}

/// Résultat par dossier des requêtes d'un run, dans l'ordre de l'arborescence (R2).
pub fn run_groups(folder: &str, items: &[RunItem]) -> Vec<RunGroup> {
    let mut groups: Vec<RunGroup> = Vec::new();
    for item in items {
        let (name, path) = group_of(folder, &item.path);
        let index = match groups.iter().position(|g| g.path == path) {
            Some(index) => index,
            None => {
                groups.push(RunGroup {
                    name,
                    path,
                    passed: 0,
                    failed: 0,
                    total: 0,
                });
                groups.len() - 1
            }
        };
        if let Some(group) = groups.get_mut(index) {
            group.total += 1;
            match item.status {
                ItemStatus::Passed => group.passed += 1,
                ItemStatus::Failed | ItemStatus::Error => group.failed += 1,
                _ => {}
            }
        }
    }
    groups
}

impl RunView {
    /// Prend en compte un changement : révision suivante, groupes recalculés (R2, R3).
    pub fn touch(&mut self) {
        self.revision += 1;
        self.groups = run_groups(&self.folder, &self.items);
    }
}

/// Reçoit les changements de l'établi, pour les diffuser aux fenêtres (R3, R4).
pub trait Notifier: Send + Sync {
    /// Un run a changé (avancement, fin, vérification ignorée).
    fn run_changed(&self, project: Uuid, run: &RunView);
    /// Les statuts des requêtes d'un projet ont changé.
    fn statuses_changed(&self, project: Uuid, statuses: &Statuses);
}

/// Résumé du dernier run d'un projet, pour « Tous les projets » (O12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLastRun {
    pub project_id: Uuid,
    pub started_at: u64,
    pub passed: usize,
    pub total: usize,
}

impl ProjectLastRun {
    pub fn of(project_id: Uuid, run: &RunView) -> Self {
        Self {
            project_id,
            started_at: run.started_at,
            passed: run
                .items
                .iter()
                .filter(|i| i.status == ItemStatus::Passed)
                .count(),
            total: run.items.len(),
        }
    }
}

struct RunHandle {
    project: Uuid,
    view: Mutex<RunView>,
    cancel: Cancel,
}

/// Exécutions en cours, statuts des requêtes et coffre des secrets.
pub struct Workbench {
    secrets: Arc<dyn SecretStore>,
    executions: Mutex<HashMap<Uuid, Arc<Cancel>>>,
    runs: Mutex<HashMap<Uuid, Arc<RunHandle>>>,
    /// Statuts des projets déjà lus (R5), un seul verrou pour toutes les écritures.
    statuses: Mutex<HashMap<Uuid, StatusBook>>,
    notifier: Mutex<Option<Arc<dyn Notifier>>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}

fn busy() -> CommandError {
    CommandError::new(
        CommandErrorKind::Io,
        "L'état des exécutions est indisponible : redémarrez l'application.",
    )
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, message)
}

fn secret_key(ctx: &ProjectCtx, environment: &str, name: &str) -> String {
    secrets::key(ctx.id, environment, name)
}

impl Workbench {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        Self {
            secrets,
            executions: Mutex::new(HashMap::new()),
            runs: Mutex::new(HashMap::new()),
            statuses: Mutex::new(HashMap::new()),
            notifier: Mutex::new(None),
        }
    }

    /// Branche la diffusion des changements (événements des fenêtres).
    pub fn set_notifier(&self, notifier: Arc<dyn Notifier>) {
        if let Ok(mut current) = self.notifier.lock() {
            *current = Some(notifier);
        }
    }

    fn notifier(&self) -> Option<Arc<dyn Notifier>> {
        self.notifier.lock().ok().and_then(|n| n.clone())
    }

    // ---------------------------------------------------------------- statuts

    /// Modifie les statuts d'un projet ; s'ils ont changé, les écrit et les diffuse (R4, R5).
    fn with_statuses(&self, ctx: &ProjectCtx, change: impl FnOnce(&mut StatusBook) -> bool) {
        let changed = {
            let Ok(mut all) = self.statuses.lock() else {
                return;
            };
            let book = all
                .entry(ctx.id)
                .or_insert_with(|| statuses::load(&ctx.data_dir, ctx.id));
            if change(book) {
                let _ = statuses::save(&ctx.data_dir, ctx.id, book);
                Some(book.requests.clone())
            } else {
                None
            }
        };
        if let (Some(statuses), Some(notifier)) = (changed, self.notifier()) {
            notifier.statuses_changed(ctx.id, &statuses);
        }
    }

    /// Statut de la dernière exécution de chaque requête du projet (R5).
    pub fn statuses(&self, ctx: &ProjectCtx) -> Statuses {
        let mut found = Statuses::new();
        self.with_statuses(ctx, |book| {
            found = book.requests.clone();
            false
        });
        found
    }

    /// Fin de l'exécution d'une requête (R4).
    pub fn record_status(&self, ctx: &ProjectCtx, path: &str, passed: bool) {
        self.with_statuses(ctx, |book| {
            book.record(path, passed, now_ms());
            true
        });
    }

    /// Requête ou dossier renommé ou déplacé (R5).
    pub fn rename_status(&self, ctx: &ProjectCtx, from: &str, to: &str) {
        self.with_statuses(ctx, |book| book.rename(from, to));
    }

    /// Requête ou dossier supprimé (R5).
    pub fn remove_status(&self, ctx: &ProjectCtx, path: &str) {
        self.with_statuses(ctx, |book| book.remove(path));
    }

    // ---------------------------------------------------------------- environnements

    fn find_environment(ctx: &ProjectCtx, name: &str) -> Result<Environment, CommandError> {
        environment::list(&ctx.root)
            .into_iter()
            .find(|env| env.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| not_found(&format!("L'environnement « {name} » n'existe pas.")))
    }

    /// Environnement actif : dernier utilisé (R7) s'il existe encore, sinon le premier.
    fn active_name(ctx: &ProjectCtx, all: &[Environment]) -> String {
        let local = local::load(&ctx.data_dir, ctx.id);
        local
            .last_environment
            .filter(|name| all.iter().any(|env| env.name == *name))
            .or_else(|| all.first().map(|env| env.name.clone()))
            .unwrap_or_default()
    }

    fn view(&self, ctx: &ProjectCtx, env: Environment, local: &LocalData) -> EnvironmentView {
        let stored = local.locals.get(&env.name).cloned().unwrap_or_default();
        let mut local_vars: Vec<LocalVarView> = Vec::new();
        for name in &env.locals {
            let known = stored.iter().find(|var| var.name == *name);
            let secret = known.is_some_and(|var| var.secret);
            let has_value = if secret {
                self.secrets
                    .get(&secret_key(ctx, &env.name, name))
                    .ok()
                    .flatten()
                    .is_some()
            } else {
                known.is_some_and(|var| var.value.as_deref().is_some_and(|v| !v.is_empty()))
            };
            local_vars.push(LocalVarView {
                name: name.clone(),
                secret,
                value: if secret {
                    None
                } else {
                    known.and_then(|v| v.value.clone())
                },
                has_value,
            });
        }
        EnvironmentView {
            environment: env,
            local_vars,
        }
    }

    pub fn environments(&self, ctx: &ProjectCtx) -> EnvironmentsView {
        // Un projet importé sans environnement reçoit « Local » (N1).
        let _ = environment::init(&ctx.root);
        let all = environment::list(&ctx.root);
        let active = Self::active_name(ctx, &all);
        let local = local::load(&ctx.data_dir, ctx.id);
        EnvironmentsView {
            environments: all
                .into_iter()
                .map(|env| self.view(ctx, env, &local))
                .collect(),
            active,
        }
    }

    pub fn select_environment(&self, ctx: &ProjectCtx, name: &str) -> Result<(), CommandError> {
        let env = Self::find_environment(ctx, name)?;
        let mut data = local::load(&ctx.data_dir, ctx.id);
        data.last_environment = Some(env.name);
        local::save(&ctx.data_dir, ctx.id, &data)
    }

    pub fn save_environment(
        &self,
        ctx: &ProjectCtx,
        env: &Environment,
        previous: Option<&str>,
    ) -> Result<(), CommandError> {
        environment::save(&ctx.root, env, previous)?;
        let mut data = local::load(&ctx.data_dir, ctx.id);
        if let Some(previous) = previous.filter(|p| *p != env.name) {
            if let Some(vars) = data.locals.remove(previous) {
                for var in vars.iter().filter(|v| v.secret) {
                    if let Some(value) = self.secrets.get(&secret_key(ctx, previous, &var.name))? {
                        self.secrets
                            .set(&secret_key(ctx, &env.name, &var.name), &value)?;
                        self.secrets.delete(&secret_key(ctx, previous, &var.name))?;
                    }
                }
                data.locals.insert(env.name.clone(), vars);
            }
            if data.last_environment.as_deref() == Some(previous) {
                data.last_environment = Some(env.name.clone());
            }
        }
        local::save(&ctx.data_dir, ctx.id, &data)
    }

    pub fn delete_environment(&self, ctx: &ProjectCtx, name: &str) -> Result<(), CommandError> {
        environment::delete(&ctx.root, name)?;
        let mut data = local::load(&ctx.data_dir, ctx.id);
        for var in data.locals.remove(name).unwrap_or_default() {
            if var.secret {
                self.secrets.delete(&secret_key(ctx, name, &var.name))?;
            }
        }
        if data.last_environment.as_deref() == Some(name) {
            data.last_environment = None;
        }
        local::save(&ctx.data_dir, ctx.id, &data)
    }

    /// Renseigne la valeur locale d'une variable ; un secret va dans le coffre (R9).
    pub fn set_local(
        &self,
        ctx: &ProjectCtx,
        env: &str,
        name: &str,
        value: &str,
        secret: bool,
    ) -> Result<(), CommandError> {
        let environment = Self::find_environment(ctx, env)?;
        if !environment.locals.iter().any(|n| n == name) {
            return Err(not_found(&format!(
                "La variable locale « {name} » n'est pas déclarée."
            )));
        }
        let mut data = local::load(&ctx.data_dir, ctx.id);
        let vars = data.locals.entry(environment.name.clone()).or_default();
        vars.retain(|var| var.name != name);
        let key = secret_key(ctx, &environment.name, name);
        if secret {
            self.secrets.set(&key, value)?;
            vars.push(LocalVar {
                name: name.to_owned(),
                value: None,
                secret: true,
            });
        } else {
            self.secrets.delete(&key)?;
            vars.push(LocalVar {
                name: name.to_owned(),
                value: Some(value.to_owned()),
                secret: false,
            });
        }
        local::save(&ctx.data_dir, ctx.id, &data)
    }

    /// Variables résolues de l'environnement `env` : partagées, puis locales (R6).
    pub fn variables(
        &self,
        ctx: &ProjectCtx,
        env: &str,
    ) -> Result<(Variables, Environment), CommandError> {
        let environment = Self::find_environment(ctx, env)?;
        let mut values: BTreeMap<String, String> = environment
            .variables
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect();
        let mut secret_names = BTreeSet::new();
        let data = local::load(&ctx.data_dir, ctx.id);
        for var in data.locals.get(&environment.name).into_iter().flatten() {
            if var.secret {
                if let Some(value) =
                    self.secrets
                        .get(&secret_key(ctx, &environment.name, &var.name))?
                {
                    values.insert(var.name.clone(), value);
                }
                secret_names.insert(var.name.clone());
            } else if let Some(value) = &var.value {
                values.insert(var.name.clone(), value.clone());
            }
        }
        Ok((
            Variables {
                values,
                secrets: secret_names,
                environment: environment.name.clone(),
            },
            environment,
        ))
    }

    // ---------------------------------------------------------------- envoi

    fn check_protected(
        env: &Environment,
        methods: &[Method],
        confirmed: bool,
    ) -> Result<(), CommandError> {
        if env.protected && !confirmed && methods.iter().any(|m| !m.is_safe()) {
            return Err(CommandError::new(
                CommandErrorKind::ProtectedEnvironment,
                format!(
                    "L'environnement « {} » est protégé : confirmez pour envoyer une requête qui modifie des données.",
                    env.name
                ),
            ));
        }
        Ok(())
    }

    fn mask_response(variables: &Variables, mut response: HttpResponse) -> HttpResponse {
        response.body = variables.mask_owned(std::mem::take(&mut response.body));
        for (_, value) in &mut response.headers {
            *value = variables.mask_owned(std::mem::take(value));
        }
        response
    }

    /// Envoie la requête `request` (ou celle enregistrée) dans l'environnement `env` (E1).
    /// Refuse d'avance un envoi qui demande confirmation (R8), sans rien envoyer :
    /// les autres fenêtres ne voient pas d'envoi qui n'a pas lieu.
    pub fn precheck(
        &self,
        ctx: &ProjectCtx,
        env: &str,
        request: &Request,
        confirmed: bool,
    ) -> Result<(), CommandError> {
        let (_, environment) = self.variables(ctx, env)?;
        Self::check_protected(&environment, &[request.method], confirmed)
    }

    pub async fn send(
        &self,
        ctx: &ProjectCtx,
        exec_id: Uuid,
        env: &str,
        request: Request,
        confirmed: bool,
    ) -> Result<SendOutcome, CommandError> {
        let (variables, environment) = self.variables(ctx, env)?;
        Self::check_protected(&environment, &[request.method], confirmed)?;
        let prepared = resolve::prepare(&request, &variables)?;
        let cancel = Arc::new(Cancel::new());
        self.executions
            .lock()
            .map_err(|_| busy())?
            .insert(exec_id, cancel.clone());
        let result = http::send(&prepared, &cancel).await;
        self.executions.lock().map_err(|_| busy())?.remove(&exec_id);
        let response = Self::mask_response(
            &variables,
            result.map_err(|mut e| {
                e.message = variables.mask(&e.message);
                e
            })?,
        );
        Ok(Self::outcome(&ctx.root, &request, response))
    }

    /// Envoie un essai hors de tout projet, sans variables
    /// (`docs/plan-accueil-non-classes.md`, R12 : rien n'est créé avant la réponse).
    pub async fn send_detached(
        &self,
        exec_id: Uuid,
        request: &Request,
    ) -> Result<HttpResponse, CommandError> {
        let variables = Variables {
            environment: environment::LOCAL.to_owned(),
            ..Variables::default()
        };
        let prepared = resolve::prepare(request, &variables)?;
        let cancel = Arc::new(Cancel::new());
        self.executions
            .lock()
            .map_err(|_| busy())?
            .insert(exec_id, cancel.clone());
        let result = http::send(&prepared, &cancel).await;
        self.executions.lock().map_err(|_| busy())?.remove(&exec_id);
        result
    }

    /// Évalue les vérifications ; celle de conformité lit le contrat lié (C8).
    pub fn outcome(root: &Path, request: &Request, response: HttpResponse) -> SendOutcome {
        let list = &request.assertions;
        let data = response.data();
        let mut results = assertions::evaluate_all(list, &data);
        // Requête liée à un contrat : sa conformité est vérifiée (S7, R8).
        if request.contract.is_some() {
            results.push(contract_file::conformance_result(
                root,
                request.contract.as_ref(),
                CONTRACT_CHECK,
                &data,
            ));
        }
        let passed = assertions::request_passed(&results, response.status);
        let limit = assertions::duration_limit(list).unwrap_or(DEFAULT_LIMIT);
        let slow = i64::try_from(response.total_ms).is_ok_and(|ms| ms > limit);
        SendOutcome {
            response,
            assertions: results,
            passed,
            slow,
        }
    }

    pub fn cancel(&self, exec_id: Uuid) {
        if let Some(cancel) = self
            .executions
            .lock()
            .ok()
            .and_then(|m| m.get(&exec_id).cloned())
        {
            cancel.cancel();
        }
    }

    // ---------------------------------------------------------------- runs

    fn last_run_file(ctx: &ProjectCtx) -> PathBuf {
        local::project_dir(&ctx.data_dir, ctx.id).join("last-run.json")
    }

    /// Dernier run terminé du projet (R14) ; un run gardé par une version
    /// précédente retrouve ses groupes (R2).
    pub fn last_run(ctx: &ProjectCtx) -> Option<RunView> {
        std::fs::read_to_string(Self::last_run_file(ctx))
            .ok()
            .and_then(|text| serde_json::from_str::<RunView>(&text).ok())
            .map(|mut run| {
                run.groups = run_groups(&run.folder, &run.items);
                run
            })
    }

    fn save_last_run(ctx: &ProjectCtx, view: &RunView) {
        let dir = local::project_dir(&ctx.data_dir, ctx.id);
        if std::fs::create_dir_all(&dir).is_ok()
            && let Ok(text) = serde_json::to_string(view)
        {
            let _ = std::fs::write(Self::last_run_file(ctx), text);
        }
    }

    /// Lance l'exécution des requêtes d'un dossier (`only` : sous-ensemble, pour « Relancer les échecs »).
    pub fn start_run(
        self: &Arc<Self>,
        ctx: &ProjectCtx,
        folder: &str,
        env: &str,
        only: Option<Vec<String>>,
        confirmed: bool,
    ) -> Result<RunView, CommandError> {
        self.launch(ctx, folder, env, only, None, confirmed)
    }

    /// Run des requêtes liées au contrat `path` (C10, R12).
    pub fn start_contract_run(
        self: &Arc<Self>,
        ctx: &ProjectCtx,
        path: &str,
        env: &str,
        confirmed: bool,
    ) -> Result<RunView, CommandError> {
        let name = contract_file::read(&ctx.root, path)
            .map(|contract| contract.title)
            .unwrap_or_else(|_| path.to_owned());
        let paths = contract_file::linked_paths(&ctx.root, path);
        self.launch(ctx, "", env, Some(paths), Some(name), confirmed)
    }

    fn launch(
        self: &Arc<Self>,
        ctx: &ProjectCtx,
        folder: &str,
        env: &str,
        only: Option<Vec<String>>,
        contract: Option<String>,
        confirmed: bool,
    ) -> Result<RunView, CommandError> {
        let (variables, environment) = self.variables(ctx, env)?;
        let mut paths = tree::requests_in(&ctx.root, folder)?;
        if let Some(only) = &only {
            paths.retain(|p| only.contains(p));
        }
        let mut items = Vec::new();
        let mut methods = Vec::new();
        for path in &paths {
            let name = Path::new(path)
                .file_stem()
                .map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
            let request = tree::resolve(&ctx.root, path).and_then(|p| file::read(&p));
            let (name, method) = request.map_or((name, Method::Get), |r| (r.name, r.method));
            methods.push(method);
            items.push(RunItem {
                path: path.clone(),
                name,
                method,
                status: ItemStatus::Pending,
                http_status: None,
                duration_ms: None,
                slow: false,
                assertions: Vec::new(),
                error: None,
            });
        }
        Self::check_protected(&environment, &methods, confirmed)?;
        let started_at = now_ms();
        let mut view = RunView {
            id: Uuid::new_v4(),
            number: 0,
            folder: folder.to_owned(),
            environment: environment.name.clone(),
            concurrency: RUN_CONCURRENCY,
            started_at,
            finished_at: items.is_empty().then_some(started_at),
            finished: items.is_empty(),
            cancelled: false,
            items,
            contract,
            revision: 0,
            groups: Vec::new(),
        };
        view.groups = run_groups(&view.folder, &view.items);
        let handle = {
            let mut runs = self.runs.lock().map_err(|_| busy())?;
            // Numéro suivant celui du dernier run gardé ou en mémoire (R1).
            let latest = runs
                .values()
                .filter(|h| h.project == ctx.id)
                .filter_map(|h| h.view.lock().ok().map(|v| v.number))
                .chain(Self::last_run(ctx).map(|run| run.number))
                .max()
                .unwrap_or(0);
            view.number = latest + 1;
            let handle = Arc::new(RunHandle {
                project: ctx.id,
                view: Mutex::new(view.clone()),
                cancel: Cancel::new(),
            });
            Self::forget_old_runs(&mut runs);
            runs.insert(view.id, handle.clone());
            handle
        };
        if view.finished {
            Self::save_last_run(ctx, &view);
            return Ok(view);
        }
        // Runtime de Tauri : la commande qui lance le run n'est pas forcément
        // exécutée dans un contexte tokio (commande synchrone, fil principal).
        tauri::async_runtime::spawn(self.clone().execute(ctx.clone(), handle, variables));
        Ok(view)
    }

    /// Ne garde que les [`MAX_FINISHED_RUNS`] runs terminés les plus récents :
    /// sans cela, chaque run resterait en mémoire jusqu'à la fermeture d'Arnod.
    fn forget_old_runs(runs: &mut HashMap<Uuid, Arc<RunHandle>>) {
        let mut finished: Vec<(u64, Uuid)> = runs
            .iter()
            .filter_map(|(id, handle)| {
                let view = handle.view.lock().ok()?;
                view.finished.then_some((view.started_at, *id))
            })
            .collect();
        // Place pour le run qui démarre.
        let excess = (finished.len() + 1).saturating_sub(MAX_FINISHED_RUNS);
        finished.sort_unstable();
        for (_, id) in finished.into_iter().take(excess) {
            runs.remove(&id);
        }
    }

    async fn execute(
        self: Arc<Self>,
        ctx: ProjectCtx,
        handle: Arc<RunHandle>,
        variables: Variables,
    ) {
        let semaphore = Arc::new(Semaphore::new(RUN_CONCURRENCY));
        let count = handle.view.lock().map_or(0, |v| v.items.len());
        let mut tasks = Vec::new();
        for index in 0..count {
            let (bench, ctx, handle, variables, semaphore) = (
                self.clone(),
                ctx.clone(),
                handle.clone(),
                variables.clone(),
                semaphore.clone(),
            );
            tasks.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire_owned().await else {
                    return;
                };
                bench.run_item(&ctx, &handle, &variables, index).await;
            }));
        }
        for task in tasks {
            let _ = task.await;
        }
        let finished = handle.view.lock().ok().map(|mut view| {
            view.finished = true;
            view.finished_at = Some(now_ms());
            view.cancelled = handle.cancel.is_cancelled();
            view.touch();
            Self::save_last_run(&ctx, &view);
            view.clone()
        });
        if let (Some(view), Some(notifier)) = (finished, self.notifier()) {
            notifier.run_changed(ctx.id, &view);
        }
    }

    /// Change une ligne du run et le diffuse (R3). Une ligne terminée met à jour
    /// le statut de sa requête (R4).
    fn update(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        index: usize,
        change: impl FnOnce(&mut RunItem),
    ) {
        let changed = handle.view.lock().ok().and_then(|mut view| {
            let item = view.items.get_mut(index)?;
            change(item);
            let done = match item.status {
                ItemStatus::Passed => Some(true),
                ItemStatus::Failed | ItemStatus::Error => Some(false),
                _ => None,
            }
            .map(|passed| (item.path.clone(), passed));
            view.touch();
            Some((view.clone(), done))
        });
        let Some((view, done)) = changed else { return };
        if let Some((path, passed)) = done {
            self.record_status(ctx, &path, passed);
        }
        if let Some(notifier) = self.notifier() {
            notifier.run_changed(ctx.id, &view);
        }
    }

    async fn run_item(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        variables: &Variables,
        index: usize,
    ) {
        let path = handle
            .view
            .lock()
            .ok()
            .and_then(|v| v.items.get(index).map(|i| i.path.clone()))
            .unwrap_or_default();
        if handle.cancel.is_cancelled() {
            self.update(ctx, handle, index, |i| i.status = ItemStatus::Cancelled);
            return;
        }
        self.update(ctx, handle, index, |i| i.status = ItemStatus::Running);
        let prepared = tree::resolve(&ctx.root, &path)
            .and_then(|p| file::read(&p))
            .and_then(|mut request| {
                // Héritage des dossiers et de la collection (R3, R4).
                request.inherited = inherit::resolve(&ctx.root, &path);
                let prepared = resolve::prepare(&request, variables)?;
                Ok((request, prepared))
            });
        let (request, prepared) = match prepared {
            Ok(ok) => ok,
            Err(error) => {
                let message = variables.mask(&error.message);
                self.update(ctx, handle, index, |i| {
                    i.status = ItemStatus::Error;
                    i.error = Some(message);
                });
                return;
            }
        };
        let result = http::send(&prepared, &handle.cancel).await;
        match result {
            Ok(response) => {
                let outcome = Self::outcome(
                    &ctx.root,
                    &request,
                    Self::mask_response(variables, response),
                );
                self.update(ctx, handle, index, |i| {
                    i.status = if outcome.passed {
                        ItemStatus::Passed
                    } else {
                        ItemStatus::Failed
                    };
                    i.http_status = Some(outcome.response.status);
                    i.duration_ms = Some(outcome.response.total_ms);
                    i.slow = outcome.slow;
                    i.assertions = outcome.assertions;
                });
            }
            Err(error) => {
                let cancelled = error.kind == CommandErrorKind::Cancelled;
                let message = variables.mask(&error.message);
                self.update(ctx, handle, index, |i| {
                    i.status = if cancelled {
                        ItemStatus::Cancelled
                    } else {
                        ItemStatus::Error
                    };
                    i.error = (!cancelled).then_some(message);
                });
            }
        }
    }

    fn handle(&self, run_id: Uuid) -> Result<Arc<RunHandle>, CommandError> {
        self.runs
            .lock()
            .map_err(|_| busy())?
            .get(&run_id)
            .cloned()
            .ok_or_else(|| not_found("Ce run n'existe plus."))
    }

    pub fn run_status(&self, run_id: Uuid) -> Result<RunView, CommandError> {
        let handle = self.handle(run_id)?;
        let view = handle.view.lock().map_err(|_| busy())?.clone();
        Ok(view)
    }

    /// Ignore l'échec d'une vérification dans une ligne d'un run
    /// (`docs/plan-fin-premier-usage.md`, R7) ; la ligne réussit si plus rien n'échoue.
    pub fn ignore_in_run(
        &self,
        ctx: &ProjectCtx,
        run_id: Uuid,
        path: &str,
        check: &str,
    ) -> Result<RunView, CommandError> {
        let missing = || not_found("Cette vérification n'est plus dans le run.");
        let apply = |view: &mut RunView| -> Result<Option<bool>, CommandError> {
            let item = view
                .items
                .iter_mut()
                .find(|i| i.path == path)
                .ok_or_else(missing)?;
            if !assertions::ignore(&mut item.assertions, check) {
                return Err(missing());
            }
            if item.status == ItemStatus::Failed
                && assertions::request_passed(&item.assertions, item.http_status.unwrap_or(0))
            {
                item.status = ItemStatus::Passed;
            }
            let passed = match item.status {
                ItemStatus::Passed => Some(true),
                ItemStatus::Failed | ItemStatus::Error => Some(false),
                _ => None,
            };
            view.touch();
            Ok(passed)
        };
        let (view, passed) = match self.handle(run_id) {
            Ok(handle) => {
                let mut view = handle.view.lock().map_err(|_| busy())?;
                let passed = apply(&mut view)?;
                if view.finished && Self::last_run(ctx).is_some_and(|last| last.id == run_id) {
                    Self::save_last_run(ctx, &view);
                }
                (view.clone(), passed)
            }
            // Run oublié : seul le dernier run du projet, gardé sur disque, se relit.
            Err(error) => {
                let mut view = Self::last_run(ctx)
                    .filter(|run| run.id == run_id)
                    .ok_or(error)?;
                let passed = apply(&mut view)?;
                Self::save_last_run(ctx, &view);
                (view, passed)
            }
        };
        if let Some(passed) = passed {
            self.record_status(ctx, path, passed);
        }
        if let Some(notifier) = self.notifier() {
            notifier.run_changed(ctx.id, &view);
        }
        Ok(view)
    }

    pub fn cancel_run(&self, run_id: Uuid) -> Result<(), CommandError> {
        // Mémorisée : une requête dont l'envoi commence juste après n'est pas envoyée.
        self.handle(run_id)?.cancel.cancel();
        Ok(())
    }

    /// Relance les requêtes en échec ou en erreur d'un run terminé (R15).
    pub fn retry_failures(
        self: &Arc<Self>,
        ctx: &ProjectCtx,
        run_id: Uuid,
        confirmed: bool,
    ) -> Result<RunView, CommandError> {
        // Run oublié (redémarrage d'Arnod, runs plus récents) : le dernier run
        // du projet, gardé sur disque, se relance aussi.
        let previous = match self.run_status(run_id) {
            Ok(view) => view,
            Err(error) => Self::last_run(ctx)
                .filter(|run| run.id == run_id)
                .ok_or(error)?,
        };
        let failed: Vec<String> = previous
            .items
            .iter()
            .filter(|i| matches!(i.status, ItemStatus::Failed | ItemStatus::Error))
            .map(|i| i.path.clone())
            .collect();
        self.launch(
            ctx,
            &previous.folder,
            &previous.environment,
            Some(failed),
            previous.contract,
            confirmed,
        )
    }
}

#[cfg(test)]
mod tests;
