//! Runs de la V4 (`docs/plan-v4.md`) : un run se construit par unités (test et son schéma,
//! requête seule et ses cas, utilisation d'un template), groupées par dossier à Avant / Après
//! (R13) ; chaque test suit son schéma (R5 à R8) ; chaque envoi passe par les scripts Bruno de sa
//! requête, de ses dossiers et de la racine (R10) et prend les propriétés des dossiers (R15).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use tokio::sync::Semaphore;

use super::{
    CaseResult, FlowInfo, ItemStatus, ProjectCtx, RunHandle, RunItem, SendOutcome, TestEnd,
    Workbench, sent_data,
};
use crate::dataset;
use crate::error::{CommandError, CommandErrorKind};
use crate::flow::engine::{
    self, BoxFuture, Call, CallDone, Report, RequestDone, Runner, StepState,
};
use crate::flow::store::Extensions;
use crate::flow::{Action, Flow, FolderHooks, HookPhase, MAX_DEPTH, OnFailure, Step, StepKind};
use crate::http::{Cancel, HttpResponse};
use crate::network::Session;
use crate::request::assertions::{AssertionResult, ResponseData};
use crate::request::extract::{self, ExtractedValue};
use crate::request::model::{Method, Pair, Request, Scalar, ScriptSource};
use crate::request::resolve::{self, Variables};
use crate::request::{file, inherit};
use crate::script::{self, Phase, ScriptRequest};
use crate::template;
use crate::tree::{self, TreeNode};

/// Une unité d'un run : jouée d'un bloc, en parallèle des autres.
#[derive(Debug, Clone)]
pub(super) enum Unit {
    /// Requête seule, et son jeu de données (R12).
    Request { item: usize },
    /// Test : son dossier, son schéma, la ligne de chaque étape.
    Test {
        path: String,
        flow: Flow,
        items: HashMap<String, usize>,
    },
    /// Utilisation d'un template (R17).
    Instance {
        path: String,
        template: String,
        item: usize,
    },
}

/// Unités directes d'un dossier qui a des Avant / Après (R13).
#[derive(Debug, Clone)]
pub(super) struct Group {
    pub folder: String,
    pub hooks: FolderHooks,
    /// Ligne de chaque étape d'Avant / Après, par phase.
    pub items: HashMap<HookPhase, HashMap<String, usize>>,
    pub units: Vec<Unit>,
}

/// Ce qu'un run joue.
#[derive(Debug, Clone, Default)]
pub(super) struct Plan {
    pub units: Vec<Unit>,
    pub groups: Vec<Group>,
}

fn item(path: &str, name: &str, method: Method) -> RunItem {
    RunItem {
        path: path.to_owned(),
        name: name.to_owned(),
        method,
        status: ItemStatus::Pending,
        http_status: None,
        duration_ms: None,
        slow: false,
        assertions: Vec::new(),
        error: None,
        test: None,
        step: None,
        extracted: Vec::new(),
        flow: FlowInfo::default(),
    }
}

fn join(folder: &str, file: &str) -> String {
    if folder.is_empty() {
        file.to_owned()
    } else {
        format!("{folder}/{file}")
    }
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// Nom et méthode d'une requête du projet (le nom du fichier si elle est illisible).
fn request_label(root: &Path, path: &str) -> (String, Method) {
    let fallback = Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_owned(), |s| s.to_string_lossy().into_owned());
    tree::resolve(root, path)
        .and_then(|p| file::read(&p))
        .map_or((fallback, Method::Get), |r| (r.name, r.method))
}

/// Libellé d'une étape : son nom, sinon celui de sa nature (une requête : son nom).
fn step_label(root: &Path, folder: &str, step: &Step) -> (String, Method, String) {
    match step.file() {
        Some(file) if matches!(step.action, Action::Request { .. }) => {
            let path = join(folder, file);
            let (name, method) = request_label(root, &path);
            (name, method, path)
        }
        _ => {
            let name = if step.name.trim().is_empty() {
                match &step.action {
                    Action::Test { test } => file_name(test).to_owned(),
                    Action::Template { template, .. } => file_name(template).to_owned(),
                    other => other.step_kind().label().to_owned(),
                }
            } else {
                step.name.trim().to_owned()
            };
            let method = step
                .file()
                .map(|f| request_label(root, &join(folder, f)).1)
                .unwrap_or(Method::Get);
            (name, method, format!("{folder}#{}", step.id))
        }
    }
}

/// Schéma réconcilié d'un test (R2).
pub(super) fn flow_of(root: &Path, ext: &Extensions, path: &str, children: &[TreeNode]) -> Flow {
    crate::flow::edit::test_flow(root, ext, path, children)
}

struct Builder<'a> {
    root: &'a Path,
    ext: Extensions,
    only: Option<&'a [String]>,
    items: Vec<RunItem>,
    plan: Plan,
}

impl Builder<'_> {
    fn wanted(&self, paths: &[&str]) -> bool {
        self.only
            .is_none_or(|only| paths.iter().any(|p| only.iter().any(|o| o == p)))
    }

    fn push(&mut self, parent: &str, unit: Unit) {
        match self.plan.groups.iter_mut().find(|g| g.folder == parent) {
            Some(group) => group.units.push(unit),
            None => self.plan.units.push(unit),
        }
    }

    fn hook_items(&mut self, folder: &str, phases: &[HookPhase]) {
        let Some(index) = self.plan.groups.iter().position(|g| g.folder == folder) else {
            return;
        };
        for phase in phases {
            let steps: Vec<Step> = self.plan.groups[index].hooks.phase(*phase).to_vec();
            let mut map = HashMap::new();
            for step in &steps {
                let (name, method, path) = step_label(self.root, folder, step);
                let path = if path.contains('#') {
                    format!("{folder}#{}-{}", phase_key(*phase), step.id)
                } else {
                    path
                };
                let mut line = item(&path, &name, method);
                line.flow.kind = step.action.step_kind();
                line.flow.step_id = Some(step.id.clone());
                line.flow.hook = Some(*phase);
                line.flow.folder = Some(folder.to_owned());
                map.insert(step.id.clone(), self.items.len());
                self.items.push(line);
            }
            self.plan.groups[index].items.insert(*phase, map);
        }
    }

    fn open_group(&mut self, folder: &str) {
        let hooks = self.ext.hooks(folder);
        if hooks.is_empty() || self.plan.groups.iter().any(|g| g.folder == folder) {
            return;
        }
        self.plan.groups.push(Group {
            folder: folder.to_owned(),
            hooks,
            items: HashMap::new(),
            units: Vec::new(),
        });
        self.hook_items(folder, &[HookPhase::BeforeAll, HookPhase::BeforeEach]);
    }

    fn close_group(&mut self, folder: &str) {
        self.hook_items(folder, &[HookPhase::AfterEach, HookPhase::AfterAll]);
    }

    fn node(&mut self, parent: &str, node: &TreeNode) {
        match node {
            TreeNode::Folder {
                path,
                template: Some(template),
                name,
                ..
            } => {
                if !self.wanted(&[path]) {
                    return;
                }
                let mut line = item(path, name, Method::Get);
                line.flow.kind = StepKind::Template;
                let index = self.items.len();
                self.items.push(line);
                self.push(
                    parent,
                    Unit::Instance {
                        path: path.clone(),
                        template: template.clone(),
                        item: index,
                    },
                );
            }
            TreeNode::Folder {
                path,
                children,
                test: true,
                ..
            } => {
                let flow = flow_of(self.root, &self.ext, path, children);
                let mut lines = Vec::new();
                for (number, step) in flow.steps.iter().enumerate() {
                    let (name, method, line_path) = step_label(self.root, path, step);
                    let mut line = item(&line_path, &name, method);
                    line.test = Some(path.clone());
                    line.step = Some(number + 1);
                    line.flow.kind = step.action.step_kind();
                    line.flow.step_id = Some(step.id.clone());
                    lines.push(line);
                }
                let paths: Vec<&str> = lines.iter().map(|l| l.path.as_str()).collect();
                if !self.wanted(&paths) {
                    return;
                }
                let mut items = HashMap::new();
                for (step, line) in flow.steps.iter().zip(lines) {
                    items.insert(step.id.clone(), self.items.len());
                    self.items.push(line);
                }
                self.push(
                    parent,
                    Unit::Test {
                        path: path.clone(),
                        flow,
                        items,
                    },
                );
            }
            TreeNode::Folder { path, children, .. } => self.walk(path, children),
            TreeNode::Request {
                path,
                invalid: false,
                ..
            } => {
                if !self.wanted(&[path]) {
                    return;
                }
                let (name, method) = request_label(self.root, path);
                let index = self.items.len();
                self.items.push(item(path, &name, method));
                self.push(parent, Unit::Request { item: index });
            }
            TreeNode::Request { .. } => {}
        }
    }

    fn walk(&mut self, folder: &str, nodes: &[TreeNode]) {
        self.open_group(folder);
        for node in nodes {
            self.node(folder, node);
        }
        self.close_group(folder);
    }

    /// Retire les groupes sans unité (relance des échecs) et leurs lignes.
    fn finish(mut self) -> (Vec<RunItem>, Plan) {
        let empty: Vec<Group> = self
            .plan
            .groups
            .iter()
            .filter(|g| g.units.is_empty())
            .cloned()
            .collect();
        self.plan.groups.retain(|g| !g.units.is_empty());
        if empty.is_empty() {
            return (self.items, self.plan);
        }
        let dropped: HashSet<usize> = empty
            .iter()
            .flat_map(|g| g.items.values().flat_map(|m| m.values().copied()))
            .collect();
        let mut remap = vec![0usize; self.items.len()];
        let mut kept = Vec::new();
        for (index, line) in self.items.into_iter().enumerate() {
            if dropped.contains(&index) {
                continue;
            }
            remap[index] = kept.len();
            kept.push(line);
        }
        let fix_unit = |unit: &mut Unit| match unit {
            Unit::Request { item } | Unit::Instance { item, .. } => *item = remap[*item],
            Unit::Test { items, .. } => {
                for value in items.values_mut() {
                    *value = remap[*value];
                }
            }
        };
        for unit in &mut self.plan.units {
            fix_unit(unit);
        }
        for group in &mut self.plan.groups {
            for unit in &mut group.units {
                fix_unit(unit);
            }
            for map in group.items.values_mut() {
                for value in map.values_mut() {
                    *value = remap[*value];
                }
            }
        }
        (kept, self.plan)
    }
}

fn phase_key(phase: HookPhase) -> &'static str {
    match phase {
        HookPhase::BeforeAll => "avant",
        HookPhase::BeforeEach => "avant-chaque",
        HookPhase::AfterEach => "apres-chaque",
        HookPhase::AfterAll => "apres",
    }
}

/// Lignes et unités du run du dossier `folder` (`""` : tout le projet). `only` restreint le run
/// à ces chemins (« Relancer les échecs ») ; `contract` : les requêtes liées à un contrat, jouées
/// seules (jamais des étapes).
pub(super) fn build(
    root: &Path,
    folder: &str,
    only: Option<&[String]>,
    contract: bool,
) -> Result<(Vec<RunItem>, Plan), CommandError> {
    if contract {
        let mut items = Vec::new();
        let mut plan = Plan::default();
        for path in only.unwrap_or_default() {
            let (name, method) = request_label(root, path);
            plan.units.push(Unit::Request { item: items.len() });
            items.push(item(path, &name, method));
        }
        return Ok((items, plan));
    }
    let nodes = tree::read(root)?;
    let mut builder = Builder {
        root,
        ext: Extensions::read(root),
        only,
        items: Vec::new(),
        plan: Plan::default(),
    };
    if folder.is_empty() {
        builder.walk("", &nodes);
    } else {
        match template::find(&nodes, folder) {
            Some(node @ TreeNode::Folder { test: true, .. })
            | Some(
                node @ TreeNode::Folder {
                    template: Some(_), ..
                },
            )
            | Some(node @ TreeNode::Request { .. }) => {
                // Un test ou une requête joué seul garde les Avant / Après de son dossier (R13).
                let parent = parent_of(folder).to_owned();
                builder.open_group(&parent);
                let node = node.clone();
                builder.node(&parent, &node);
                builder.close_group(&parent);
            }
            Some(TreeNode::Folder { children, .. }) => {
                let children = children.clone();
                builder.walk(folder, &children);
            }
            None => {}
        }
    }
    Ok(builder.finish())
}

// ---------------------------------------------------------------- échange

/// Ce qu'un envoi a donné.
pub(super) struct Exchange {
    pub request: Request,
    pub raw: HttpResponse,
    pub extracted: Vec<ExtractedValue>,
    /// Variables posées par les scripts, non masquées.
    pub set: BTreeMap<String, String>,
    /// Tests des scripts et erreurs de script, en assertions.
    pub tests: Vec<AssertionResult>,
    /// Variables de l'envoi, valeurs ajoutées à l'envoi masquées comme des secrets.
    pub variables: Variables,
    pub prepared: resolve::Prepared,
}

/// Variables de l'envoi d'une requête (R15) : celles de la collection sous l'environnement, puis
/// les propriétés des dossiers (du plus lointain au plus proche) au-dessus.
pub(super) fn layered(variables: &Variables, request: &Request) -> Variables {
    let mut merged = variables.clone();
    let inherited = &request.inherited.variables;
    for variable in inherited.iter().rev() {
        if variable.folder.is_empty() {
            merged
                .values
                .entry(variable.name.clone())
                .or_insert_with(|| variable.value.clone());
        } else {
            merged
                .values
                .insert(variable.name.clone(), variable.value.clone());
            merged.secrets.remove(&variable.name);
        }
    }
    merged
}

/// Scripts d'une phase dans l'ordre de Bruno (R10) : avant la requête, la racine puis les dossiers
/// puis la requête ; après, la requête puis les dossiers puis la racine.
fn scripts_of<'a>(request: &'a Request, phase: Phase) -> Vec<&'a ScriptSource> {
    let inherited: Vec<&ScriptSource> = request
        .inherited
        .scripts
        .iter()
        .filter(|s| Phase::parse(&s.phase) == Some(phase))
        .collect();
    let own: Vec<&ScriptSource> = request
        .scripts
        .iter()
        .filter(|s| Phase::parse(&s.phase) == Some(phase))
        .collect();
    match phase {
        Phase::BeforeRequest => inherited.into_iter().chain(own).collect(),
        _ => own.into_iter().chain(inherited.into_iter().rev()).collect(),
    }
}

fn script_label(source: &ScriptSource) -> String {
    if source.source.is_empty() {
        "script de la requête".to_owned()
    } else {
        format!("script du {}", source.source)
    }
}

/// Applique à la requête ce qu'un script d'avant l'envoi a changé (méthode, adresse, en-têtes,
/// corps), avant la résolution des variables.
fn apply_script_request(request: &mut Request, changed: &ScriptRequest) {
    if let Some(method) = Method::from_name(&changed.method) {
        request.method = method;
    }
    request.url = changed.url.clone();
    request
        .params
        .retain(|p| p.kind != crate::request::model::ParamKind::Query);
    let mut headers: Vec<Pair> = Vec::new();
    for (name, value) in &changed.headers {
        headers.push(Pair::new(name.clone(), value.clone()));
    }
    request.headers = headers;
    if let Some(body) = &changed.body {
        let selected = request.body.selected;
        if let Some(variant) = request.body.variants.get_mut(selected) {
            variant.content = body.clone();
        }
    }
}

fn script_request(request: &Request) -> ScriptRequest {
    ScriptRequest {
        method: request.method.as_str().to_owned(),
        url: request.url.clone(),
        headers: request
            .headers
            .iter()
            .filter(|h| h.enabled && !h.name.trim().is_empty())
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect(),
        body: Some(request.body.content().to_owned()).filter(|b| !b.is_empty()),
    }
}

/// Valeur attendue d'un texte résolu : nombre, booléen, sinon texte.
fn scalar_of(text: &str) -> Scalar {
    let trimmed = text.trim();
    if let Ok(n) = trimmed.parse::<i64>() {
        Scalar::Int(n)
    } else if let Ok(n) = trimmed.parse::<f64>() {
        Scalar::Float(n)
    } else if trimmed == "true" || trimmed == "false" {
        Scalar::Bool(trimmed == "true")
    } else {
        Scalar::Text(text.to_owned())
    }
}

fn script_failure(label: &str, message: &str, index: usize) -> AssertionResult {
    AssertionResult {
        id: format!("script-erreur-{index}"),
        target: "script".to_owned(),
        passed: false,
        message: Some(format!("{label} : {message}")),
        actual: None,
        ignored: false,
    }
}

impl Workbench {
    /// Envoie `request` avec `variables` (environnement, propriétés et variables d'exécution
    /// déjà fusionnées), scripts Bruno compris (R10).
    pub(super) async fn exchange(
        &self,
        ctx: &ProjectCtx,
        cancel: &Cancel,
        variables: Variables,
        mut request: Request,
    ) -> Result<Exchange, CommandError> {
        let mut variables = variables;
        let mut set = BTreeMap::new();
        let mut tests = Vec::new();
        for (index, source) in scripts_of(&request, Phase::BeforeRequest)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            let current = script_request(&request);
            let vars = variables.values.clone();
            let code = source.code.clone();
            let outcome = tokio::task::spawn_blocking(move || {
                script::run_bruno(Phase::BeforeRequest, &code, &vars, Some(&current), None)
            })
            .await
            .unwrap_or_default();
            if let Some(changed) = &outcome.request {
                apply_script_request(&mut request, changed);
            }
            for (name, value) in &outcome.set {
                variables.values.insert(name.clone(), value.clone());
            }
            set.extend(outcome.set);
            if let Some(error) = outcome.error {
                tests.push(script_failure(&script_label(&source), &error, index));
            }
        }
        let mut prepared = resolve::prepare(&request, &variables)?;
        let session = Session {
            project: ctx.id,
            environment: variables.environment.clone(),
        };
        let raw = self
            .network
            .send(&mut prepared, Some(&session), cancel)
            .await;
        let variables = variables.hiding(&prepared.hidden);
        let raw = raw.map_err(|mut e| {
            e.message = variables.mask(&e.message);
            e
        })?;
        let data = raw.data();
        let extracted = extract::apply(&request.extractions, &data, &sent_data(&prepared));
        let mut vars: BTreeMap<String, String> = variables.values.clone();
        extract::store(&extracted, &mut vars);
        for phase in [Phase::AfterResponse, Phase::Tests] {
            for (index, source) in scripts_of(&request, phase)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .enumerate()
            {
                let snapshot = vars.clone();
                let response = data.clone();
                let code = source.code.clone();
                let outcome = tokio::task::spawn_blocking(move || {
                    script::run_bruno(phase, &code, &snapshot, None, Some(&response))
                })
                .await
                .unwrap_or_default();
                for (name, value) in &outcome.set {
                    vars.insert(name.clone(), value.clone());
                }
                set.extend(outcome.set);
                for (n, test) in outcome.tests.into_iter().enumerate() {
                    tests.push(AssertionResult {
                        id: format!("script-{}-{n}", tests.len()),
                        target: "script".to_owned(),
                        passed: test.passed,
                        message: (!test.passed).then(|| {
                            format!(
                                "« {} » : {}",
                                test.name,
                                test.error.unwrap_or_else(|| "en échec".to_owned())
                            )
                        }),
                        actual: Some(test.name),
                        ignored: false,
                    });
                }
                if let Some(error) = outcome.error {
                    tests.push(script_failure(&script_label(&source), &error, index));
                }
            }
        }
        Ok(Exchange {
            request,
            raw,
            extracted,
            set,
            tests,
            variables,
            prepared,
        })
    }

    /// Évalue un échange : assertions de la requête (valeurs attendues `{{…}}` résolues, 4.7-a),
    /// conformité au contrat, tests des scripts.
    pub(super) fn judge(ctx: &ProjectCtx, exchange: &Exchange) -> SendOutcome {
        let response = Self::mask_response(&exchange.variables, exchange.raw.clone());
        let mut request = exchange.request.clone();
        for assertion in &mut request.assertions {
            if let Some(Scalar::Text(text)) = &assertion.value
                && text.contains("{{")
                && let Ok(resolved) = resolve::substitute(text, &exchange.variables)
            {
                assertion.value = Some(scalar_of(&resolved));
            }
        }
        let mut outcome = Self::outcome(&ctx.root, &request, response);
        outcome.assertions.extend(exchange.tests.iter().cloned());
        outcome.passed = outcome.passed && exchange.tests.iter().all(|t| t.passed || t.ignored);
        outcome
    }
}

// ---------------------------------------------------------------- exécution

/// Requête jouée par une ligne de run.
pub(super) struct Played {
    pub passed: bool,
    pub cancelled: bool,
    pub error: Option<String>,
    pub outcome: Option<SendOutcome>,
    pub raw: Option<ResponseData>,
    /// Valeurs extraites et posées par les scripts, non masquées.
    pub values: BTreeMap<String, String>,
    /// Valeurs extraites, masquées.
    pub shown: Vec<ExtractedValue>,
}

impl Workbench {
    /// Joue la requête `path` avec les variables d'exécution `runtime` et les valeurs `extra`
    /// (cas d'un jeu de données, qui priment).
    pub(super) async fn play_file(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        variables: &Variables,
        path: &str,
        runtime: &BTreeMap<String, String>,
        extra: &BTreeMap<String, String>,
    ) -> Played {
        let failed = |message: String| Played {
            passed: false,
            cancelled: false,
            error: Some(message),
            outcome: None,
            raw: None,
            values: BTreeMap::new(),
            shown: Vec::new(),
        };
        if handle.cancel.is_cancelled() {
            return Played {
                cancelled: true,
                ..failed(String::new())
            };
        }
        let request = match tree::resolve(&ctx.root, path).and_then(|p| file::read(&p)) {
            Ok(mut request) => {
                // Héritage des dossiers et de la collection (R3, R4, R15).
                request.inherited = inherit::resolve(&ctx.root, path);
                request
            }
            Err(error) => return failed(variables.mask(&error.message)),
        };
        let mut merged = Self::with_runtime(&layered(variables, &request), runtime);
        for (name, value) in extra {
            merged.values.insert(name.clone(), value.clone());
        }
        let extractions = request.extractions.clone();
        match self.exchange(ctx, &handle.cancel, merged, request).await {
            Ok(exchange) => {
                self.keep_in_environment(
                    ctx,
                    &exchange.variables.environment,
                    &extractions,
                    &exchange.extracted,
                );
                let outcome = Self::judge(ctx, &exchange);
                let mut values = BTreeMap::new();
                extract::store(&exchange.extracted, &mut values);
                values.extend(exchange.set.clone());
                let shown = exchange
                    .extracted
                    .iter()
                    .cloned()
                    .chain(exchange.set.iter().map(|(name, value)| ExtractedValue {
                        name: name.clone(),
                        source: "script".to_owned(),
                        value: Some(value.clone()),
                        message: None,
                    }))
                    .map(|mut e| {
                        e.value = e.value.map(|v| exchange.variables.mask(&v));
                        e
                    })
                    .collect();
                Played {
                    passed: outcome.passed,
                    cancelled: false,
                    error: None,
                    raw: Some(exchange.raw.data()),
                    outcome: Some(outcome),
                    values,
                    shown,
                }
            }
            Err(error) => {
                let cancelled = error.kind == CommandErrorKind::Cancelled;
                Played {
                    cancelled,
                    ..failed(variables.mask(&error.message))
                }
            }
        }
    }

    /// Reporte une requête jouée sur sa ligne ; `status` : la ligne prend son verdict (une
    /// attente garde le sien). Un échec déjà noté reste (boucle, Continuer).
    fn apply_played(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        index: usize,
        played: &Played,
        status: bool,
    ) {
        let outcome = played.outcome.clone();
        let error = played.error.clone();
        let shown = played.shown.clone();
        let (passed, cancelled) = (played.passed, played.cancelled);
        self.update(ctx, handle, index, move |i| {
            let sticky =
                i.flow.passes > 0 && matches!(i.status, ItemStatus::Failed | ItemStatus::Error);
            i.flow.passes += 1;
            if let Some(outcome) = outcome {
                i.http_status = Some(outcome.response.status);
                i.duration_ms = Some(outcome.response.total_ms);
                i.slow = outcome.slow;
                i.assertions = outcome.assertions;
            }
            i.extracted = shown;
            if !status {
                return;
            }
            if cancelled {
                i.status = ItemStatus::Cancelled;
            } else if !sticky {
                i.status = if passed {
                    ItemStatus::Passed
                } else if error.is_some() {
                    ItemStatus::Error
                } else {
                    ItemStatus::Failed
                };
                i.error = error;
            }
        });
    }

    /// Requête seule : une fois, ou une fois par cas de son jeu de données (R12).
    async fn play_request(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        variables: &Variables,
        index: usize,
        runtime: &mut BTreeMap<String, String>,
    ) -> bool {
        let path = handle
            .view
            .lock()
            .ok()
            .and_then(|v| v.items.get(index).map(|i| i.path.clone()))
            .unwrap_or_default();
        self.update(ctx, handle, index, |i| i.status = ItemStatus::Running);
        let data = tree::resolve(&ctx.root, &path)
            .and_then(|p| file::read(&p))
            .ok()
            .and_then(|r| r.data);
        let Some(link) = data else {
            let played = self
                .play_file(ctx, handle, variables, &path, runtime, &BTreeMap::new())
                .await;
            runtime.extend(played.values.clone());
            self.apply_played(ctx, handle, index, &played, true);
            return played.passed;
        };
        let set = dataset::read(&ctx.root, &link);
        if let Some(error) = set.error {
            self.update(ctx, handle, index, move |i| {
                i.status = ItemStatus::Error;
                i.error = Some(error);
            });
            return false;
        }
        let cases = dataset::cases(&set.table);
        let mut results: Vec<CaseResult> = Vec::new();
        let mut all = true;
        let mut first_failed: Option<Played> = None;
        let mut last: Option<Played> = None;
        for (number, values) in cases.iter().enumerate() {
            let played = self
                .play_file(ctx, handle, variables, &path, runtime, values)
                .await;
            if played.cancelled {
                break;
            }
            let shown: Vec<(String, String)> = values
                .iter()
                .map(|(k, v)| (k.clone(), variables.mask(v)))
                .collect();
            results.push(CaseResult {
                number: number + 1,
                values: shown,
                status: if played.passed {
                    ItemStatus::Passed
                } else if played.error.is_some() {
                    ItemStatus::Error
                } else {
                    ItemStatus::Failed
                },
                http_status: played.outcome.as_ref().map(|o| o.response.status),
                duration_ms: played.outcome.as_ref().map(|o| o.response.total_ms),
                assertions: played
                    .outcome
                    .as_ref()
                    .map(|o| o.assertions.clone())
                    .unwrap_or_default(),
                error: played.error.clone(),
            });
            all &= played.passed;
            if !played.passed && first_failed.is_none() {
                first_failed = Some(played);
            } else {
                last = Some(played);
            }
            let snapshot = results.clone();
            self.update(ctx, handle, index, move |i| i.flow.cases = snapshot);
        }
        let cancelled = handle.cancel.is_cancelled();
        let shown = first_failed.or(last);
        let total: u64 = results.iter().filter_map(|c| c.duration_ms).sum();
        let empty = results.is_empty();
        self.update(ctx, handle, index, move |i| {
            i.flow.passes = u32::try_from(results.len()).unwrap_or(u32::MAX);
            if let Some(played) = &shown
                && let Some(outcome) = &played.outcome
            {
                i.http_status = Some(outcome.response.status);
                i.assertions = outcome.assertions.clone();
                i.slow = outcome.slow;
            }
            i.duration_ms = Some(total);
            i.flow.detail = Some(format!("{} cas", results.len()));
            i.status = if cancelled {
                ItemStatus::Cancelled
            } else if all && !empty {
                ItemStatus::Passed
            } else if empty {
                i.error = Some("Le jeu de données n'a aucun cas.".to_owned());
                ItemStatus::Error
            } else {
                ItemStatus::Failed
            };
            i.flow.cases = results;
        });
        all && !empty
    }

    /// Joue une unité avec les variables d'exécution `vars`, qu'elle complète.
    pub(super) async fn play_unit(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        variables: &Variables,
        unit: &Unit,
        vars: &mut BTreeMap<String, String>,
    ) -> bool {
        match unit {
            Unit::Request { item } => self.play_request(ctx, handle, variables, *item, vars).await,
            Unit::Test { path, flow, items } => {
                let runner = StepRunner::new(self, ctx, handle, variables, path, Some(items));
                let done = engine::run(flow, &runner, vars, 0).await;
                let cancelled = done.cancelled || handle.cancel.is_cancelled();
                let indexes: Vec<usize> = handle
                    .view
                    .lock()
                    .map(|view| {
                        items
                            .values()
                            .copied()
                            .filter(|i| {
                                view.items
                                    .get(*i)
                                    .is_some_and(|line| line.status == ItemStatus::Pending)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // Étapes non empruntées : « non exécutées » (R8).
                for index in indexes {
                    self.update(ctx, handle, index, |i| {
                        if matches!(i.status, ItemStatus::Pending) {
                            i.status = if cancelled {
                                ItemStatus::Cancelled
                            } else {
                                ItemStatus::Skipped
                            };
                        }
                    });
                }
                if !cancelled {
                    self.end_test(
                        ctx,
                        handle,
                        path,
                        done.passed,
                        done.reason,
                        done.failed_step,
                    );
                }
                done.passed
            }
            Unit::Instance {
                path,
                template,
                item,
            } => {
                self.update(ctx, handle, *item, |i| i.status = ItemStatus::Running);
                let runner = StepRunner::new(self, ctx, handle, variables, path, None);
                let given: BTreeMap<String, String> = tree::resolve(&ctx.root, path)
                    .map(|dir| {
                        tree::folder_variables_raw(&dir)
                            .into_iter()
                            .map(|(n, v, _)| (n, v))
                            .collect()
                    })
                    .unwrap_or_default();
                let result = runner
                    .play_template(template, &given, parent_of(path), vars, 1)
                    .await;
                let detail = format!("{}/{}", result.passed_steps, result.total_steps);
                let passed = result.passed;
                let cancelled = handle.cancel.is_cancelled();
                self.update(ctx, handle, *item, move |i| {
                    i.flow.detail = Some(detail);
                    i.error = result.message;
                    i.status = if cancelled {
                        ItemStatus::Cancelled
                    } else if passed {
                        ItemStatus::Passed
                    } else {
                        ItemStatus::Failed
                    };
                });
                passed
            }
        }
    }

    /// Note la fin d'un test joué par son schéma (R5).
    fn end_test(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        path: &str,
        passed: bool,
        reason: Option<String>,
        failed_step: Option<String>,
    ) {
        // Diffusé avec le changement suivant (ligne d'un autre test, fin du run).
        let _ = ctx;
        if let Ok(mut view) = handle.view.lock() {
            view.ends.retain(|e| e.test != path);
            view.ends.push(TestEnd {
                test: path.to_owned(),
                passed,
                reason,
                failed_step,
            });
            view.tests = super::run_tests_with(&view.items, &view.ends);
        }
    }

    /// Une unité qui n'est pas jouée parce qu'un Avant a échoué (R13).
    fn skip_unit(&self, ctx: &ProjectCtx, handle: &RunHandle, unit: &Unit, reason: &str) {
        match unit {
            Unit::Request { item } | Unit::Instance { item, .. } => {
                let reason = reason.to_owned();
                self.update(ctx, handle, *item, move |i| {
                    i.status = ItemStatus::Failed;
                    i.error = Some(reason);
                });
            }
            Unit::Test { path, items, .. } => {
                for index in items.values() {
                    self.update(ctx, handle, *index, |i| i.status = ItemStatus::Skipped);
                }
                self.end_test(ctx, handle, path, false, Some(reason.to_owned()), None);
            }
        }
    }

    /// Joue les étapes d'une phase d'Avant / Après (R13) ; rend `true` si elles réussissent.
    async fn play_hooks(
        &self,
        ctx: &ProjectCtx,
        handle: &RunHandle,
        variables: &Variables,
        group: &Group,
        phase: HookPhase,
        vars: &mut BTreeMap<String, String>,
    ) -> bool {
        let steps = group.hooks.phase(phase).to_vec();
        if steps.is_empty() {
            return true;
        }
        let flow = Flow {
            // « Après » se joue en entier, même après un échec (R13).
            on_failure: if matches!(phase, HookPhase::AfterEach | HookPhase::AfterAll) {
                OnFailure::Continue
            } else {
                OnFailure::Stop
            },
            steps,
        };
        let empty = HashMap::new();
        let items = group.items.get(&phase).unwrap_or(&empty);
        let runner = StepRunner::new(self, ctx, handle, variables, &group.folder, Some(items));
        let done = engine::run(&flow, &runner, vars, 0).await;
        done.passed
    }

    /// Joue un groupe : « Avant le dossier », ses tests directs encadrés par « Avant / Après
    /// chaque test », puis « Après le dossier », même après un échec (R13).
    pub(super) async fn play_group(
        self: Arc<Self>,
        ctx: ProjectCtx,
        handle: Arc<RunHandle>,
        variables: Variables,
        group: Arc<Group>,
        semaphore: Arc<Semaphore>,
    ) {
        let mut seed = BTreeMap::new();
        let before = {
            let Ok(_permit) = semaphore.clone().acquire_owned().await else {
                return;
            };
            self.play_hooks(
                &ctx,
                &handle,
                &variables,
                &group,
                HookPhase::BeforeAll,
                &mut seed,
            )
            .await
        };
        if before {
            let mut tasks = Vec::new();
            for index in 0..group.units.len() {
                let (bench, ctx, handle, variables, group, semaphore, seed) = (
                    self.clone(),
                    ctx.clone(),
                    handle.clone(),
                    variables.clone(),
                    group.clone(),
                    semaphore.clone(),
                    seed.clone(),
                );
                tasks.push(tokio::spawn(async move {
                    let Ok(_permit) = semaphore.acquire_owned().await else {
                        return;
                    };
                    let Some(unit) = group.units.get(index) else {
                        return;
                    };
                    let mut vars = seed;
                    let ready = bench
                        .play_hooks(
                            &ctx,
                            &handle,
                            &variables,
                            &group,
                            HookPhase::BeforeEach,
                            &mut vars,
                        )
                        .await;
                    if ready {
                        bench
                            .play_unit(&ctx, &handle, &variables, unit, &mut vars)
                            .await;
                    } else {
                        bench.skip_unit(&ctx, &handle, unit, "Avant en échec");
                    }
                    bench
                        .play_hooks(
                            &ctx,
                            &handle,
                            &variables,
                            &group,
                            HookPhase::AfterEach,
                            &mut vars,
                        )
                        .await;
                    bench.keep_runtime(ctx.id, &vars);
                }));
            }
            for task in tasks {
                let _ = task.await;
            }
        } else {
            for unit in &group.units {
                self.skip_unit(&ctx, &handle, unit, "Avant en échec");
            }
            // Les lignes « Avant chaque test » ne sont pas jouées.
            if let Some(map) = group.items.get(&HookPhase::BeforeEach) {
                for index in map.values() {
                    self.update(&ctx, &handle, *index, |i| i.status = ItemStatus::Skipped);
                }
            }
            if let Some(map) = group.items.get(&HookPhase::AfterEach) {
                for index in map.values() {
                    self.update(&ctx, &handle, *index, |i| i.status = ItemStatus::Skipped);
                }
            }
        }
        let Ok(_permit) = semaphore.acquire_owned().await else {
            return;
        };
        self.play_hooks(
            &ctx,
            &handle,
            &variables,
            &group,
            HookPhase::AfterAll,
            &mut seed,
        )
        .await;
    }

    /// Joue tout le plan d'un run, unités en parallèle (`run_concurrency`).
    pub(super) async fn play_plan(
        self: Arc<Self>,
        ctx: ProjectCtx,
        handle: Arc<RunHandle>,
        variables: Variables,
        plan: Plan,
    ) {
        let semaphore = Arc::new(Semaphore::new(self.run_concurrency()));
        let mut tasks = Vec::new();
        for unit in plan.units {
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
                let mut vars = BTreeMap::new();
                bench
                    .play_unit(&ctx, &handle, &variables, &unit, &mut vars)
                    .await;
                bench.keep_runtime(ctx.id, &vars);
            }));
        }
        for group in plan.groups {
            tasks.push(tokio::spawn(self.clone().play_group(
                ctx.clone(),
                handle.clone(),
                variables.clone(),
                Arc::new(group),
                semaphore.clone(),
            )));
        }
        for task in tasks {
            let _ = task.await;
        }
    }
}

// ---------------------------------------------------------------- parcours

/// Ce que l'établi fournit au parcours d'un schéma.
struct StepRunner<'a> {
    bench: &'a Workbench,
    ctx: &'a ProjectCtx,
    handle: &'a RunHandle,
    variables: &'a Variables,
    /// Dossier des fichiers des étapes (le test, ou le dossier d'un Avant / Après).
    folder: String,
    /// Ligne de chaque étape ; `None` pour un test appelé (rien n'est rendu).
    items: Option<&'a HashMap<String, usize>>,
    /// Variables de l'environnement et des propriétés du dossier.
    base: BTreeMap<String, String>,
}

impl<'a> StepRunner<'a> {
    fn new(
        bench: &'a Workbench,
        ctx: &'a ProjectCtx,
        handle: &'a RunHandle,
        variables: &'a Variables,
        folder: &str,
        items: Option<&'a HashMap<String, usize>>,
    ) -> Self {
        let mut base = variables.values.clone();
        for variable in inherit::folder_variables(&ctx.root, folder).iter().rev() {
            if variable.folder.is_empty() {
                base.entry(variable.name.clone())
                    .or_insert_with(|| variable.value.clone());
            } else {
                base.insert(variable.name.clone(), variable.value.clone());
            }
        }
        Self {
            bench,
            ctx,
            handle,
            variables,
            folder: folder.to_owned(),
            items,
            base,
        }
    }

    fn index(&self, step: &str) -> Option<usize> {
        self.items.and_then(|items| items.get(step).copied())
    }

    fn kind_of(&self, index: usize) -> StepKind {
        self.handle
            .view
            .lock()
            .ok()
            .and_then(|v| v.items.get(index).map(|i| i.flow.kind))
            .unwrap_or_default()
    }

    /// Joue un test, une requête, un dossier ou une utilisation du chemin `path` (test existant,
    /// contenu d'un template), sans rendre compte de ses étapes.
    fn play_path<'b>(
        &'b self,
        path: &'b str,
        vars: &'b mut BTreeMap<String, String>,
        depth: usize,
    ) -> BoxFuture<'b, CallDone> {
        Box::pin(async move {
            if depth > MAX_DEPTH {
                return CallDone {
                    message: Some(format!(
                        "Plus de {MAX_DEPTH} tests appelés les uns dans les autres : un test s'appelle-t-il lui-même ?"
                    )),
                    ..CallDone::default()
                };
            }
            let Some(node) = template::node(&self.ctx.root, path) else {
                return CallDone {
                    message: Some(format!("« {path} » n'existe plus.")),
                    ..CallDone::default()
                };
            };
            match node {
                TreeNode::Request { path, .. } => {
                    let played = self
                        .bench
                        .play_file(
                            self.ctx,
                            self.handle,
                            self.variables,
                            &path,
                            vars,
                            &BTreeMap::new(),
                        )
                        .await;
                    vars.extend(played.values);
                    CallDone {
                        passed: played.passed,
                        passed_steps: usize::from(played.passed),
                        total_steps: 1,
                        message: played.error,
                    }
                }
                TreeNode::Folder {
                    path,
                    template: Some(template),
                    ..
                } => {
                    let given: BTreeMap<String, String> = tree::resolve(&self.ctx.root, &path)
                        .map(|dir| {
                            tree::folder_variables_raw(&dir)
                                .into_iter()
                                .map(|(n, v, _)| (n, v))
                                .collect()
                        })
                        .unwrap_or_default();
                    self.play_template(&template, &given, parent_of(&path), vars, depth + 1)
                        .await
                }
                TreeNode::Folder {
                    path,
                    children,
                    test: true,
                    ..
                } => {
                    let ext = Extensions::read(&self.ctx.root);
                    let flow = flow_of(&self.ctx.root, &ext, &path, &children);
                    let nested = StepRunner::new(
                        self.bench,
                        self.ctx,
                        self.handle,
                        self.variables,
                        &path,
                        None,
                    );
                    let done = engine::run(&flow, &nested, vars, depth).await;
                    CallDone {
                        passed: done.passed,
                        passed_steps: done.passed_steps,
                        total_steps: done.played,
                        message: done.reason,
                    }
                }
                TreeNode::Folder { children, .. } => {
                    let mut total = CallDone {
                        passed: true,
                        ..CallDone::default()
                    };
                    for child in &children {
                        let child_path = match child {
                            TreeNode::Folder { path, .. } | TreeNode::Request { path, .. } => {
                                path.clone()
                            }
                        };
                        let done = self.play_path(&child_path, vars, depth + 1).await;
                        total.passed &= done.passed;
                        total.passed_steps += done.passed_steps;
                        total.total_steps += done.total_steps;
                        if total.message.is_none() {
                            total.message = done.message;
                        }
                        if self.handle.cancel.is_cancelled() {
                            break;
                        }
                    }
                    total
                }
            }
        })
    }

    /// Joue le template `template` avec les valeurs `given`, complétées des propriétés du dossier
    /// `parent` puis des valeurs par défaut (R17).
    fn play_template<'b>(
        &'b self,
        template: &'b str,
        given: &'b BTreeMap<String, String>,
        parent: &'b str,
        vars: &'b mut BTreeMap<String, String>,
        depth: usize,
    ) -> BoxFuture<'b, CallDone> {
        Box::pin(async move {
            let inherited: BTreeMap<String, String> =
                inherit::folder_variables(&self.ctx.root, parent)
                    .into_iter()
                    .map(|v| (v.name, v.value))
                    .collect();
            match template::effective_values(&self.ctx.root, template, given, &inherited) {
                Ok(values) => {
                    vars.extend(values);
                    self.play_path(template, vars, depth).await
                }
                Err(message) => CallDone {
                    message: Some(message),
                    ..CallDone::default()
                },
            }
        })
    }
}

impl Runner for StepRunner<'_> {
    fn request<'a>(
        &'a self,
        step: &'a str,
        file: &'a str,
        vars: &'a BTreeMap<String, String>,
        _attempt: u32,
    ) -> BoxFuture<'a, RequestDone> {
        Box::pin(async move {
            let path = join(&self.folder, file);
            let index = self.index(step);
            let status = index.is_some_and(|i| self.kind_of(i) == StepKind::Request);
            if let (Some(index), true) = (index, status) {
                self.bench.update(self.ctx, self.handle, index, |i| {
                    i.status = ItemStatus::Running
                });
            }
            let played = self
                .bench
                .play_file(
                    self.ctx,
                    self.handle,
                    self.variables,
                    &path,
                    vars,
                    &BTreeMap::new(),
                )
                .await;
            if let Some(index) = index {
                self.bench
                    .apply_played(self.ctx, self.handle, index, &played, status);
            }
            RequestDone {
                passed: played.passed,
                response: played.raw,
                extracted: played.values,
            }
        })
    }

    fn call<'a>(
        &'a self,
        _step: &'a str,
        call: Call<'a>,
        vars: &'a mut BTreeMap<String, String>,
        depth: usize,
    ) -> BoxFuture<'a, CallDone> {
        match call {
            Call::Test(test) => self.play_path(test, vars, depth),
            Call::Template { template, values } => {
                let parent = self.folder.as_str();
                self.play_template(template, values, parent, vars, depth)
            }
        }
    }

    fn script<'a>(
        &'a self,
        code: &'a str,
        vars: &'a BTreeMap<String, String>,
        response: Option<&'a ResponseData>,
    ) -> BoxFuture<'a, script::ScriptOutcome> {
        let (code, vars, response) = (code.to_owned(), vars.clone(), response.cloned());
        Box::pin(async move {
            tokio::task::spawn_blocking(move || script::run_step(&code, &vars, response.as_ref()))
                .await
                .unwrap_or_else(|_| script::ScriptOutcome {
                    error: Some("Le script s'est arrêté sans résultat.".to_owned()),
                    ..script::ScriptOutcome::default()
                })
        })
    }

    fn substitute(&self, text: &str, vars: &BTreeMap<String, String>) -> Result<String, String> {
        let mut variables = self.variables.clone();
        variables.values.extend(self.base.clone());
        variables
            .values
            .extend(vars.iter().map(|(k, v)| (k.clone(), v.clone())));
        resolve::substitute(text, &variables).map_err(|e| variables.mask(&e.message))
    }

    fn base(&self) -> BTreeMap<String, String> {
        self.base.clone()
    }

    fn report(&self, step: &str, report: Report) {
        let Some(index) = self.index(step) else {
            return;
        };
        let variables = self.variables;
        self.bench.update(self.ctx, self.handle, index, move |i| {
            match report.state {
                StepState::Running => {
                    i.status = ItemStatus::Running;
                    if report.detail.is_some() {
                        i.flow.detail = report.detail;
                    }
                    return;
                }
                StepState::Passed => i.status = ItemStatus::Passed,
                StepState::Failed => i.status = ItemStatus::Failed,
                StepState::Error => i.status = ItemStatus::Error,
            }
            i.flow.passes += 1;
            i.flow.detail = report.detail;
            i.flow.branch = report.branch;
            if report.duration_ms.is_some() {
                i.duration_ms = report.duration_ms;
            }
            i.error = report.error.map(|e| variables.mask(&e));
            if !report.values.is_empty() {
                i.extracted = report
                    .values
                    .into_iter()
                    .map(|mut e| {
                        e.value = e.value.map(|v| variables.mask(&v));
                        e
                    })
                    .collect();
            }
        });
    }

    fn cancelled(&self) -> bool {
        self.handle.cancel.is_cancelled()
    }

    fn sleep(&self, ms: u64) -> BoxFuture<'_, ()> {
        Box::pin(tokio::time::sleep(std::time::Duration::from_millis(ms)))
    }
}
