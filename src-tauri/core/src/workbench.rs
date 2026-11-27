//! Établi du projet ouvert : envoi d'une requête, runs, environnements actifs.
//!
//! Relie les modules purs (requête, environnement, HTTP) au disque, au coffre
//! des secrets et aux exécutions en cours. Les commandes en sont la façade.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contract::file as contract_file;
use crate::environment::{self, Environment};
use crate::error::{CommandError, CommandErrorKind};
use crate::flow::{HookPhase, StepKind};
use crate::http::{Cancel, HttpResponse};
use crate::local::{self, LocalData, LocalVar};
use crate::network::Network;
use crate::network::oauth::TokenStatus;
use crate::request::assertions::{self, AssertionResult, DEFAULT_LIMIT, ResponseData};
use crate::request::extract::{self, ExtractedValue};
use crate::request::model::{Extraction, ExtractionScope, Method, Request};
use crate::request::resolve::{self, DrawnValue, MASK, Prepared, Variables};
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentView {
    #[serde(flatten)]
    pub environment: Environment,
    pub local_vars: Vec<LocalVarView>,
}

/// Environnements du projet et choix courant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentsView {
    pub environments: Vec<EnvironmentView>,
    pub active: String,
}

/// Résultat d'un envoi.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SendOutcome {
    pub response: HttpResponse,
    pub assertions: Vec<AssertionResult>,
    pub passed: bool,
    pub slow: bool,
    /// Ce qui est parti, pour l'onglet « Envoyé » (`docs/plan-mvp-e-tests-etapes.md`, VD2).
    pub sent: Option<Sent>,
}

/// Résultat de l'envoi d'un cas d'un jeu de données (4.7-b, R12).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CaseOutcome {
    /// Numéro du cas, à partir de 1.
    pub number: usize,
    /// Valeurs du cas, masquées.
    pub values: Vec<(String, String)>,
    pub outcome: Option<SendOutcome>,
    /// Erreur de l'envoi (réseau, variable inconnue…), en français.
    pub error: Option<String>,
}

/// Envoi de tous les cas d'un jeu de données (4.7-b) : réussi si chaque cas réussit (R12).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CasesOutcome {
    pub cases: Vec<CaseOutcome>,
    pub passed: bool,
}

/// D'où vient une variable lue par un envoi (5.1-h, AM29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum UsedSource {
    /// Variable partagée de l'environnement de l'envoi.
    Environment,
    /// Variable locale de cet environnement (valeur sur le poste, ou au coffre).
    Local,
    /// Variable d'exécution : valeur extraite par une étape ou un envoi précédent.
    Extracted,
}

/// Variable lue par un envoi (VD2) : un secret est masqué.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UsedVariable {
    pub name: String,
    pub value: String,
    pub source: UsedSource,
}

/// Jeton OAuth 2 d'un envoi (3.4-b, AM35), jamais sa valeur : serveur de jetons, et quand le
/// jeton gardé a été obtenu et expire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SentToken {
    pub token_url: String,
    pub status: Option<TokenStatus>,
}

/// Requête envoyée, secrets masqués, avec ses valeurs tirées, ses variables
/// et ses valeurs extraites (VD2, EX3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    /// Environnement de l'envoi, montré sur la ligne d'état de la réponse (5.1-b, AM28).
    pub environment: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub drawn: Vec<DrawnValue>,
    pub used: Vec<UsedVariable>,
    pub extracted: Vec<ExtractedValue>,
    /// Jeton OAuth 2 ajouté à l'envoi, s'il y en a un (AM35).
    pub oauth2: Option<SentToken>,
}

impl Sent {
    /// Ce qui est parti, masqué (R9 du premier usage).
    fn of(
        prepared: &Prepared,
        variables: &Variables,
        locals: &[String],
        runtime: &BTreeMap<String, String>,
        extracted: Vec<ExtractedValue>,
    ) -> Self {
        let mask = |text: &str| variables.mask(text);
        let source = |name: &str| {
            if runtime.contains_key(name) {
                UsedSource::Extracted
            } else if locals.iter().any(|local| local == name) {
                UsedSource::Local
            } else {
                UsedSource::Environment
            }
        };
        Self {
            environment: variables.environment.clone(),
            method: prepared.method.as_str().to_owned(),
            url: mask(&prepared.url),
            headers: prepared
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), mask(v)))
                .collect(),
            body: prepared.body.as_deref().map(mask),
            drawn: prepared.drawn.clone(),
            used: prepared
                .used
                .iter()
                .map(|name| UsedVariable {
                    name: name.clone(),
                    value: if variables.secrets.contains(name) {
                        MASK.to_owned()
                    } else {
                        mask(variables.values.get(name).map_or("", String::as_str))
                    },
                    source: source(name),
                })
                .collect(),
            extracted: extracted
                .into_iter()
                .map(|mut e| {
                    e.value = e.value.map(|v| mask(&v));
                    e
                })
                .collect(),
            oauth2: prepared.oauth2.as_ref().map(|grant| SentToken {
                token_url: mask(&grant.token_url),
                status: None,
            }),
        }
    }
}

/// Données de la requête envoyée, pour une extraction `request.…` (R7).
fn sent_data(prepared: &Prepared) -> ResponseData {
    ResponseData {
        status: 0,
        duration_ms: 0,
        headers: prepared.headers.clone(),
        body: prepared.body.clone().unwrap_or_default(),
    }
}

/// État d'une requête dans un run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "RunItemStatus"))]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Error,
    Cancelled,
    /// Non jouée : une étape précédente du test a échoué
    /// (`docs/plan-mvp-e-tests-etapes.md`, R3).
    Skipped,
}

/// Ligne d'un run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// Test dont la requête est une étape (R1), sinon `None`.
    #[serde(default)]
    pub test: Option<String>,
    /// Numéro de l'étape dans son test, à partir de 1.
    #[serde(default)]
    pub step: Option<usize>,
    /// Valeurs extraites par la ligne (R5), masquées.
    #[serde(default)]
    pub extracted: Vec<ExtractedValue>,
    /// Étape d'un schéma, Avant / Après, cas d'un jeu de données (`docs/plan-v4.md`).
    #[serde(flatten, default)]
    pub flow: FlowInfo,
}

/// Ce qu'une ligne de run porte des tests en schéma (`docs/plan-v4.md`, R8, R12, R13).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FlowInfo {
    /// Nature de l'étape (requête pour une ligne d'avant la V4).
    #[serde(default)]
    pub kind: StepKind,
    /// Identifiant de l'étape dans le schéma de son test.
    #[serde(default)]
    pub step_id: Option<String>,
    /// Résumé court : « oui », « 3 tours », « 9/12 », « 3 essais ».
    #[serde(default)]
    pub detail: Option<String>,
    /// Branche prise par une condition.
    #[serde(default)]
    pub branch: Option<bool>,
    /// Passages de l'étape (boucle, attente, Avant chaque test).
    #[serde(default)]
    pub passes: u32,
    /// Phase d'un Avant / Après, et son dossier.
    #[serde(default)]
    pub hook: Option<HookPhase>,
    #[serde(default)]
    pub folder: Option<String>,
    /// Résultat de chaque cas d'un jeu de données (R12).
    #[serde(default)]
    pub cases: Vec<CaseResult>,
}

/// Résultat d'un cas d'un jeu de données (4.7-b, R12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    /// Numéro du cas, à partir de 1.
    pub number: usize,
    /// Valeurs du cas, masquées.
    pub values: Vec<(String, String)>,
    pub status: ItemStatus,
    pub http_status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub assertions: Vec<AssertionResult>,
    pub error: Option<String>,
}

/// Fin d'un test joué par son schéma (R5) : verdict, et raison d'un échec qui n'est pas celui
/// d'une étape (« Fin en échec », « Avant en échec »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TestEnd {
    pub test: String,
    pub passed: bool,
    pub reason: Option<String>,
    /// Étape qui a arrêté le test (identifiant dans le schéma).
    pub failed_step: Option<String>,
}

/// Verdict d'un test dans un run (R4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RunTest {
    pub path: String,
    pub name: String,
    /// `passed`, `failed`, `running`, `pending` ou `cancelled`.
    pub status: ItemStatus,
    pub steps: usize,
    pub passed: usize,
    /// Étape qui a arrêté le test (R3).
    pub failed_step: Option<usize>,
}

/// Verdict de chaque test d'un run, dans l'ordre (R4).
pub fn run_tests(items: &[RunItem]) -> Vec<RunTest> {
    run_tests_with(items, &[])
}

/// Verdict de chaque test d'un run, fins des schémas comprises (`docs/plan-v4.md`, R5) : un test
/// fini prend son verdict ; ses étapes non empruntées ne le rendent pas « non joué ».
pub fn run_tests_with(items: &[RunItem], ends: &[TestEnd]) -> Vec<RunTest> {
    let mut tests: Vec<RunTest> = Vec::new();
    let mut states: Vec<Vec<ItemStatus>> = Vec::new();
    for item in items {
        let Some(path) = &item.test else { continue };
        let index = match tests.iter().position(|t| &t.path == path) {
            Some(index) => index,
            None => {
                tests.push(RunTest {
                    path: path.clone(),
                    name: path.rsplit('/').next().unwrap_or(path).to_owned(),
                    status: ItemStatus::Pending,
                    steps: 0,
                    passed: 0,
                    failed_step: None,
                });
                states.push(Vec::new());
                tests.len() - 1
            }
        };
        if let (Some(test), Some(state)) = (tests.get_mut(index), states.get_mut(index)) {
            test.steps += 1;
            if item.status == ItemStatus::Passed {
                test.passed += 1;
            }
            if matches!(item.status, ItemStatus::Failed | ItemStatus::Error)
                && test.failed_step.is_none()
            {
                test.failed_step = item.step.or(Some(test.steps));
            }
            state.push(item.status);
        }
    }
    for (test, state) in tests.iter_mut().zip(states) {
        let any = |status: ItemStatus| state.contains(&status);
        if let Some(end) = ends.iter().find(|e| e.test == test.path) {
            if let Some(failed) = &end.failed_step {
                test.failed_step = items
                    .iter()
                    .filter(|i| i.test.as_deref() == Some(test.path.as_str()))
                    .position(|i| i.flow.step_id.as_deref() == Some(failed.as_str()))
                    .map(|p| p + 1)
                    .or(test.failed_step);
            }
            test.status = if end.passed {
                ItemStatus::Passed
            } else {
                ItemStatus::Failed
            };
            continue;
        }
        test.status = if test.failed_step.is_some() {
            ItemStatus::Failed
        } else if any(ItemStatus::Running) {
            ItemStatus::Running
        } else if any(ItemStatus::Cancelled) {
            ItemStatus::Cancelled
        } else if test.passed == test.steps {
            ItemStatus::Passed
        } else if any(ItemStatus::Pending) && state.iter().any(|s| *s != ItemStatus::Pending) {
            ItemStatus::Running
        } else if any(ItemStatus::Skipped) {
            // Échec ignoré après coup : les étapes non jouées le restent (R3).
            ItemStatus::Skipped
        } else {
            ItemStatus::Pending
        };
    }
    tests
}

/// Résultat d'un groupe de requêtes d'un run (`docs/plan-fin-premier-usage.md`, R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// Verdict de chaque test (`docs/plan-mvp-e-tests-etapes.md`, R4).
    #[serde(default)]
    pub tests: Vec<RunTest>,
    /// Fin de chaque test joué par son schéma (`docs/plan-v4.md`, R5).
    #[serde(default)]
    pub ends: Vec<TestEnd>,
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
        self.tests = run_tests_with(&self.items, &self.ends);
    }
}

/// Reçoit les changements de l'établi, pour les diffuser aux fenêtres (R3, R4).
pub trait Notifier: Send + Sync {
    /// Un run a changé (avancement, fin, vérification ignorée).
    fn run_changed(&self, project: Uuid, run: &RunView);
    /// Les statuts des requêtes d'un projet ont changé.
    fn statuses_changed(&self, project: Uuid, statuses: &Statuses);
    /// Les environnements d'un projet ont changé hors d'une commande : une valeur extraite de
    /// portée « l'environnement » (AM44) a été gardée.
    fn environments_changed(&self, _project: Uuid) {}
}

/// Tâche de fond d'un run, lancée par un [`Spawner`].
pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Lance les tâches de fond de l'établi (un run). L'application passe par le runtime de Tauri,
/// la ligne de commande et les tests par [`TokioSpawner`] : la commande qui lance un run n'est
/// pas forcément exécutée dans un contexte tokio (commande synchrone, fil principal).
pub trait Spawner: Send + Sync {
    fn spawn(&self, task: Task);
}

/// Runtime tokio propre au cœur, créé au premier run (comme celui de Tauri).
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSpawner;

/// Runtime tokio propre au cœur, créé au premier besoin : runs hors de l'application, échanges
/// du transport Git (appelés depuis un fil sans runtime, V2 lot E).
pub fn core_runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RUNTIME: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .ok()
        })
        .as_ref()
}

impl Spawner for TokioSpawner {
    fn spawn(&self, task: Task) {
        match core_runtime() {
            Some(runtime) => {
                runtime.spawn(task);
            }
            // Sans runtime propre (création refusée par le système), celui de l'appelant.
            None => {
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(task);
                }
            }
        }
    }
}

/// Utilisation d'un environnement (3.3-a, AM43, Q8) : le dernier run fait dessus parmi ceux que
/// le cœur garde (en mémoire et le dernier du projet sur le poste).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentUsage {
    pub environment: String,
    /// Numéro du run dans le projet (0 : run gardé par une version précédente).
    pub number: u32,
    /// Dossier exécuté (`""` : tout le projet).
    pub folder: String,
    /// Contrat exécuté, sinon `None`.
    pub contract: Option<String>,
    /// Début du run, en millisecondes depuis l'époque Unix.
    pub started_at: u64,
}

/// Résumé du dernier run d'un projet, pour « Tous les projets » (O12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// Lance les runs ([`Spawner`]) : runtime de Tauri dans l'application.
    spawner: Mutex<Arc<dyn Spawner>>,
    /// Lignes de commande (V2, lot D) : rien n'est écrit sur le poste ni dans le projet (R5) ;
    /// statuts, dernier run et valeurs gardées dans l'environnement restent en mémoire.
    read_only: AtomicBool,
    /// Requêtes envoyées en même temps pendant un run (`--parallele`).
    concurrency: AtomicUsize,
    /// Valeurs fournies par l'appelant (`ARNOD_VAR_<nom>`, R6) : elles priment sur
    /// l'environnement, le coffre et `local.json` ; masquées comme des secrets.
    overrides: Mutex<BTreeMap<String, String>>,
    /// Variables d'exécution de chaque projet pour la session (R6) : jamais écrites.
    runtime: Mutex<HashMap<Uuid, BTreeMap<String, String>>>,
    /// Lot H : client HTTP du poste (proxy, autorités), jetons OAuth 2 et cookies.
    network: Arc<Network>,
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

/// De quoi remettre un environnement supprimé (« Annuler », ER1), gardé en mémoire par le cœur.
#[derive(Debug, Clone)]
pub struct EnvironmentRestore {
    removed: environment::Removed,
    locals: Option<Vec<LocalVar>>,
    last: bool,
    /// Clé du coffre et valeur de chaque secret retiré.
    secrets: Vec<(String, String)>,
}

fn secret_key(ctx: &ProjectCtx, environment: &str, name: &str) -> String {
    secrets::key(ctx.id, environment, name)
}

impl Workbench {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        let network = Arc::new(Network::new(
            secrets.clone(),
            Box::new(crate::network::proxy::OsProxySource),
        ));
        Self {
            secrets,
            executions: Mutex::new(HashMap::new()),
            runs: Mutex::new(HashMap::new()),
            statuses: Mutex::new(HashMap::new()),
            notifier: Mutex::new(None),
            spawner: Mutex::new(Arc::new(TokioSpawner)),
            read_only: AtomicBool::new(false),
            concurrency: AtomicUsize::new(RUN_CONCURRENCY),
            overrides: Mutex::new(BTreeMap::new()),
            runtime: Mutex::new(HashMap::new()),
            network,
        }
    }

    /// Lot H : réseau du poste, partagé avec les commandes des préférences.
    pub fn network(&self) -> &Arc<Network> {
        &self.network
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

    /// Change le lanceur des runs (l'application branche le runtime de Tauri).
    pub fn set_spawner(&self, spawner: Arc<dyn Spawner>) {
        if let Ok(mut current) = self.spawner.lock() {
            *current = spawner;
        }
    }

    /// Ligne de commande (R5) : plus aucune écriture, ni sur le poste ni dans le projet.
    pub fn set_read_only(&self) {
        self.read_only.store(true, Ordering::Relaxed);
    }

    fn read_only(&self) -> bool {
        self.read_only.load(Ordering::Relaxed)
    }

    /// Requêtes envoyées en même temps pendant un run (au moins une).
    pub fn set_run_concurrency(&self, concurrency: usize) {
        self.concurrency
            .store(concurrency.max(1), Ordering::Relaxed);
    }

    pub fn run_concurrency(&self) -> usize {
        self.concurrency.load(Ordering::Relaxed).max(1)
    }

    /// Valeurs de variables fournies par l'appelant (`ARNOD_VAR_<nom>`, R6).
    pub fn set_overrides(&self, values: BTreeMap<String, String>) {
        if let Ok(mut current) = self.overrides.lock() {
            *current = values;
        }
    }

    fn spawner(&self) -> Arc<dyn Spawner> {
        self.spawner.lock().map_or_else(
            |_| Arc::new(TokioSpawner) as Arc<dyn Spawner>,
            |s| s.clone(),
        )
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
                if !self.read_only() {
                    let _ = statuses::save(&ctx.data_dir, ctx.id, book);
                }
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
        // L'historique des envois suit aussi (5.1-c bis, H6).
        crate::history::rename(&ctx.data_dir, ctx.id, from, to);
        self.with_statuses(ctx, |book| book.rename(from, to));
    }

    /// Requête ou dossier supprimé (R5).
    pub fn remove_status(&self, ctx: &ProjectCtx, path: &str) {
        crate::history::remove(&ctx.data_dir, ctx.id, path);
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

    /// Supprime un environnement, ses valeurs locales et ses secrets ; rend de quoi tout remettre
    /// (« Annuler », ER1). Les secrets sont lus avant d'être retirés du coffre et ne sont gardés
    /// qu'en mémoire, jamais écrits.
    pub fn delete_environment(
        &self,
        ctx: &ProjectCtx,
        name: &str,
    ) -> Result<EnvironmentRestore, CommandError> {
        let mut data = local::load(&ctx.data_dir, ctx.id);
        let locals = data.locals.get(name).cloned();
        let mut secrets = Vec::new();
        for var in locals.iter().flatten().filter(|var| var.secret) {
            let key = secret_key(ctx, name, &var.name);
            if let Some(value) = self.secrets.get(&key)? {
                secrets.push((key, value));
            }
        }
        let removed = environment::delete(&ctx.root, name)?;
        for (key, _) in &secrets {
            self.secrets.delete(key)?;
        }
        data.locals.remove(name);
        let last = data.last_environment.as_deref() == Some(name);
        if last {
            data.last_environment = None;
        }
        local::save(&ctx.data_dir, ctx.id, &data)?;
        Ok(EnvironmentRestore {
            removed,
            locals,
            last,
            secrets,
        })
    }

    /// Remet un environnement supprimé : fichiers, valeurs locales, dernier choix et secrets.
    pub fn restore_environment(
        &self,
        ctx: &ProjectCtx,
        restore: &EnvironmentRestore,
    ) -> Result<(), CommandError> {
        environment::restore(&ctx.root, &restore.removed)?;
        let name = restore.removed.name();
        let mut data = local::load(&ctx.data_dir, ctx.id);
        if let Some(locals) = &restore.locals {
            data.locals.insert(name.to_owned(), locals.clone());
        }
        if restore.last {
            data.last_environment = Some(name.to_owned());
        }
        local::save(&ctx.data_dir, ctx.id, &data)?;
        for (key, value) in &restore.secrets {
            self.secrets.set(key, value)?;
        }
        Ok(())
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
        // Valeurs de l'appelant (R6) : elles priment, masquées sauf pour une variable partagée.
        if let Ok(overrides) = self.overrides.lock() {
            for (name, value) in overrides.iter() {
                if !environment.variables.iter().any(|v| &v.name == name) {
                    secret_names.insert(name.clone());
                }
                values.insert(name.clone(), value.clone());
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

    // ---------------------------------------------------------------- variables d'exécution

    /// Variables d'exécution du projet pour la session (R6).
    pub fn runtime_variables(&self, project: Uuid) -> BTreeMap<String, String> {
        self.runtime
            .lock()
            .ok()
            .and_then(|all| all.get(&project).cloned())
            .unwrap_or_default()
    }

    /// Valeurs extraites de portée « l'environnement » (4.2-b bis, AM44, Q9) : gardées aussi en
    /// variables locales (poste) de l'environnement `env`, déclarées au besoin, jamais dans ses
    /// variables partagées. Une valeur qui ne peut pas être gardée reste variable d'exécution :
    /// l'envoi n'échoue pas pour autant.
    fn keep_in_environment(
        &self,
        ctx: &ProjectCtx,
        env: &str,
        extractions: &[Extraction],
        values: &[ExtractedValue],
    ) {
        let wanted: Vec<(&str, &str)> = values
            .iter()
            .filter(|value| {
                extractions.iter().any(|e| {
                    e.enabled
                        && e.scope == ExtractionScope::Environment
                        && e.name.trim() == value.name
                })
            })
            .filter_map(|value| value.value.as_deref().map(|v| (value.name.as_str(), v)))
            .collect();
        // Ligne de commande (R5) : la valeur reste une variable d'exécution.
        if wanted.is_empty() || self.read_only() {
            return;
        }
        let Ok(mut environment) = Self::find_environment(ctx, env) else {
            return;
        };
        let shared = |name: &str| environment.variables.iter().any(|v| v.name == name);
        let missing: Vec<String> = wanted
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .filter(|name| !shared(name) && !environment.locals.contains(name))
            .collect();
        if !missing.is_empty() {
            environment.locals.extend(missing);
            let previous = environment.name.clone();
            if environment::save(&ctx.root, &environment, Some(&previous)).is_err() {
                return;
            }
        }
        let mut kept = false;
        for (name, value) in wanted {
            kept |= self
                .set_local(ctx, &environment.name, name, value, false)
                .is_ok();
        }
        if let (true, Some(notifier)) = (kept, self.notifier()) {
            notifier.environments_changed(ctx.id);
        }
    }

    /// Dernier run de chaque environnement du projet (3.3-a, AM43) ; un environnement sans run
    /// gardé n'apparaît pas.
    pub fn environment_usage(&self, ctx: &ProjectCtx) -> Vec<EnvironmentUsage> {
        let mut views: Vec<RunView> = self
            .runs
            .lock()
            .map(|runs| {
                runs.values()
                    .filter(|h| h.project == ctx.id)
                    .filter_map(|h| h.view.lock().ok().map(|v| v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(last) = Self::last_run(ctx)
            && !views.iter().any(|v| v.id == last.id)
        {
            views.push(last);
        }
        views.sort_by_key(|view| std::cmp::Reverse(view.started_at));
        let mut usage: Vec<EnvironmentUsage> = Vec::new();
        for view in views {
            if usage.iter().any(|u| u.environment == view.environment) {
                continue;
            }
            usage.push(EnvironmentUsage {
                environment: view.environment,
                number: view.number,
                folder: view.folder,
                contract: view.contract,
                started_at: view.started_at,
            });
        }
        usage
    }

    /// Garde des valeurs extraites pour la session (R6).
    fn keep_runtime(&self, project: Uuid, values: &BTreeMap<String, String>) {
        if values.is_empty() {
            return;
        }
        if let Ok(mut all) = self.runtime.lock() {
            all.entry(project)
                .or_default()
                .extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }

    /// Variables de l'environnement, puis variables d'exécution, qui priment (R5).
    fn with_runtime(variables: &Variables, runtime: &BTreeMap<String, String>) -> Variables {
        let mut merged = variables.clone();
        for (name, value) in runtime {
            merged.values.insert(name.clone(), value.clone());
        }
        merged
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
        self.send_with(ctx, exec_id, env, request, confirmed, &BTreeMap::new())
            .await
    }

    /// Envoie `request` avec les valeurs `extra` (cas d'un jeu de données, qui priment, R12) :
    /// propriétés des dossiers (R15) et scripts Bruno (R10) compris.
    pub async fn send_with(
        &self,
        ctx: &ProjectCtx,
        exec_id: Uuid,
        env: &str,
        request: Request,
        confirmed: bool,
        extra: &BTreeMap<String, String>,
    ) -> Result<SendOutcome, CommandError> {
        let (variables, environment) = self.variables(ctx, env)?;
        Self::check_protected(&environment, &[request.method], confirmed)?;
        let runtime = self.runtime_variables(ctx.id);
        let mut variables = Self::with_runtime(&run::layered(&variables, &request), &runtime);
        for (name, value) in extra {
            variables.values.insert(name.clone(), value.clone());
        }
        let cancel = Arc::new(Cancel::new());
        self.executions
            .lock()
            .map_err(|_| busy())?
            .insert(exec_id, cancel.clone());
        let extractions = request.extractions.clone();
        let result = self.exchange(ctx, &cancel, variables, request).await;
        self.executions.lock().map_err(|_| busy())?.remove(&exec_id);
        let exchange = result?;
        // Extractions lues dans la réponse reçue, avant le masquage des secrets (R6).
        let mut kept = BTreeMap::new();
        extract::store(&exchange.extracted, &mut kept);
        kept.extend(exchange.set.clone());
        self.keep_runtime(ctx.id, &kept);
        self.keep_in_environment(ctx, &environment.name, &extractions, &exchange.extracted);
        let mut outcome = Self::judge(ctx, &exchange);
        let mut sent = Sent::of(
            &exchange.prepared,
            &exchange.variables,
            &environment.locals,
            &runtime,
            exchange.extracted.clone(),
        );
        if let Some(token) = sent.oauth2.as_mut() {
            token.status = self.network.token_status(ctx.id, &environment.name);
        }
        outcome.sent = Some(sent);
        Ok(outcome)
    }

    /// Envoie `request` une fois par cas de son jeu de données (4.7-b, R12) ; `only` : ces cas
    /// seulement (numéros à partir de 1). Un cas en erreur n'arrête pas les suivants.
    pub async fn send_cases(
        &self,
        ctx: &ProjectCtx,
        exec_id: Uuid,
        env: &str,
        request: Request,
        confirmed: bool,
        only: Option<Vec<usize>>,
    ) -> Result<CasesOutcome, CommandError> {
        let Some(link) = request.data.clone() else {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                "Cette requête n'a pas de jeu de données.",
            ));
        };
        let set = crate::dataset::read(&ctx.root, &link);
        if let Some(error) = set.error {
            return Err(CommandError::new(CommandErrorKind::InvalidRequest, error));
        }
        let (variables, _) = self.variables(ctx, env)?;
        let mut cases = Vec::new();
        for (index, values) in crate::dataset::cases(&set.table).into_iter().enumerate() {
            let number = index + 1;
            if only.as_ref().is_some_and(|only| !only.contains(&number)) {
                continue;
            }
            let shown: Vec<(String, String)> = values
                .iter()
                .map(|(k, v)| (k.clone(), variables.mask(v)))
                .collect();
            let result = self
                .send_with(ctx, exec_id, env, request.clone(), confirmed, &values)
                .await;
            let cancelled = matches!(&result, Err(e) if e.kind == CommandErrorKind::Cancelled);
            cases.push(match result {
                Ok(outcome) => CaseOutcome {
                    number,
                    values: shown,
                    outcome: Some(outcome),
                    error: None,
                },
                Err(error) => {
                    if error.kind == CommandErrorKind::ProtectedEnvironment {
                        return Err(error);
                    }
                    CaseOutcome {
                        number,
                        values: shown,
                        outcome: None,
                        error: Some(error.message),
                    }
                }
            });
            if cancelled {
                break;
            }
        }
        let passed = !cases.is_empty()
            && cases
                .iter()
                .all(|c| c.outcome.as_ref().is_some_and(|o| o.passed));
        Ok(CasesOutcome { cases, passed })
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
        let mut prepared = resolve::prepare(request, &variables)?;
        let cancel = Arc::new(Cancel::new());
        self.executions
            .lock()
            .map_err(|_| busy())?
            .insert(exec_id, cancel.clone());
        let result = self.network.send(&mut prepared, None, &cancel).await;
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
            sent: None,
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
                run.tests = run_tests_with(&run.items, &run.ends);
                run
            })
    }

    fn save_last_run(&self, ctx: &ProjectCtx, view: &RunView) {
        if self.read_only() {
            return;
        }
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
        let (items, plan) = run::build(&ctx.root, folder, only.as_deref(), contract.is_some())?;
        let methods: Vec<Method> = items
            .iter()
            .filter(|i| i.flow.kind == StepKind::Request)
            .map(|i| i.method)
            .collect();
        Self::check_protected(&environment, &methods, confirmed)?;
        let started_at = now_ms();
        let mut view = RunView {
            id: Uuid::new_v4(),
            number: 0,
            folder: folder.to_owned(),
            environment: environment.name.clone(),
            concurrency: self.run_concurrency(),
            started_at,
            finished_at: items.is_empty().then_some(started_at),
            finished: items.is_empty(),
            cancelled: false,
            items,
            contract,
            revision: 0,
            groups: Vec::new(),
            tests: Vec::new(),
            ends: Vec::new(),
        };
        view.groups = run_groups(&view.folder, &view.items);
        view.tests = run_tests(&view.items);
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
            self.save_last_run(ctx, &view);
            return Ok(view);
        }
        // Runtime de l'application (Tauri) ou du cœur : la commande qui lance le run n'est pas
        // forcément exécutée dans un contexte tokio (commande synchrone, fil principal).
        self.spawner().spawn(Box::pin(self.clone().execute(
            ctx.clone(),
            handle,
            variables,
            plan,
        )));
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
        plan: run::Plan,
    ) {
        self.clone()
            .play_plan(ctx.clone(), handle.clone(), variables, plan)
            .await;
        let finished = handle.view.lock().ok().map(|mut view| {
            view.finished = true;
            view.finished_at = Some(now_ms());
            view.cancelled = handle.cancel.is_cancelled();
            view.touch();
            self.save_last_run(&ctx, &view);
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
            // Seule une requête a un statut dans la barre latérale (pas une étape d'un autre
            // genre, dont le chemin porte `#`).
            let done = match item.status {
                ItemStatus::Passed => Some(true),
                ItemStatus::Failed | ItemStatus::Error => Some(false),
                _ => None,
            }
            .filter(|_| item.flow.kind == StepKind::Request && !item.path.contains('#'))
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
        let missing = || not_found("Cette assertion n'est plus dans le run.");
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
                    self.save_last_run(ctx, &view);
                }
                (view.clone(), passed)
            }
            // Run oublié : seul le dernier run du projet, gardé sur disque, se relit.
            Err(error) => {
                let mut view = Self::last_run(ctx)
                    .filter(|run| run.id == run_id)
                    .ok_or(error)?;
                let passed = apply(&mut view)?;
                self.save_last_run(ctx, &view);
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
        // Un test en échec est rejoué en entier : ses valeurs extraites sont
        // recalculées (`docs/plan-mvp-e-tests-etapes.md`, TE5).
        let bad = |i: &RunItem| matches!(i.status, ItemStatus::Failed | ItemStatus::Error);
        let failed_tests: Vec<&String> = previous
            .items
            .iter()
            .filter(|i| bad(i))
            .filter_map(|i| i.test.as_ref())
            .collect();
        let failed: Vec<String> = previous
            .items
            .iter()
            .filter(|i| bad(i) || i.test.as_ref().is_some_and(|t| failed_tests.contains(&t)))
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

/// Unités d'un run (R2) : les étapes consécutives d'un même test ensemble,
/// chaque requête hors test seule.
pub fn run_units(items: &[RunItem]) -> Vec<Vec<usize>> {
    let mut units: Vec<Vec<usize>> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let joins = index > 0
            && item.test.is_some()
            && items.get(index - 1).and_then(|p| p.test.as_ref()) == item.test.as_ref();
        match units.last_mut() {
            Some(unit) if joins => unit.push(index),
            _ => units.push(vec![index]),
        }
    }
    units
}

/// Test et numéro d'étape de chaque requête d'un test de l'arborescence (R1).
pub fn test_steps(nodes: &[tree::TreeNode]) -> HashMap<String, (String, usize)> {
    let mut found = HashMap::new();
    for node in nodes {
        if let tree::TreeNode::Folder {
            path,
            children,
            test,
            ..
        } = node
        {
            if *test {
                // Une requête liée à un contrat reste un endpoint, rangé sous son contrat.
                let requests = children
                    .iter()
                    .filter(|c| matches!(c, tree::TreeNode::Request { contract: None, .. }));
                for (index, child) in requests.enumerate() {
                    if let tree::TreeNode::Request { path: step, .. } = child {
                        found.insert(step.clone(), (path.clone(), index + 1));
                    }
                }
            } else {
                found.extend(test_steps(children));
            }
        }
    }
    found
}

mod run;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod v4_tests;
