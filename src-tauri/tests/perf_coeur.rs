//! Banc de performance du cœur (audit de novembre 2026, plan-qualite-mvp T7).
//!
//! Tests ignorés : ils ne tournent pas dans `cargo test` ; la CI les lance chaque semaine et à la
//! demande (`.github/workflows/qualite.yml`). Chaque scénario compare ses mesures du projet de
//! 2 000 requêtes aux budgets de R7 (`budget`) : un dépassement est signalé, et fait échouer le
//! banc avec `ARNOD_BANC_STRICT=1` (activé quand les lots F et G auront tenu ces budgets). À
//! lancer en profil optimisé, un scénario à la fois, sur un fil :
//!
//! ```sh
//! cargo test --release --manifest-path src-tauri/Cargo.toml --locked --test perf_coeur \
//!   -- --ignored --nocapture --test-threads=1 perf_1
//! ```
//!
//! `CARGO_PROFILE_RELEASE_PANIC=unwind` est nécessaire devant la commande : le profil release
//! fixe `panic = "abort"`, les tests exigent `unwind`, et la bibliothèque (`cdylib`, sans
//! empreinte dans son nom de fichier) serait compilée deux fois dans le même
//! `libarnod_lib.rlib` ; le test se lierait alors à la mauvaise. `dist/index.html` doit exister
//! (`npm run build`, ou un fichier vide) pour `generate_context!`.
//!
//! Les données sont engendrées par les fonctions du cœur (projet, environnements, contrats,
//! requêtes liées) dans `target/tmp` (disque réel ; `ARNOD_BANC_DIR` pour un autre dossier).
//! Les commandes Tauri ne sont pas appelables sans application : chaque scénario rejoue le
//! corps de la commande avec les mêmes fonctions publiques. Sans crate ajoutée : `Instant`,
//! médiane de plusieurs passes, `/proc/self/io` et `/proc/self/status` (Linux).

#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use arnod_lib::bruno::convert::SecretValue;
use arnod_lib::commands::tree::TreeView;
use arnod_lib::contract;
use arnod_lib::environment::{self, Environment, Variable};
use arnod_lib::error::CommandError;
use arnod_lib::http::{HttpResponse, MAX_BODY_BYTES, Timeline};
use arnod_lib::import::{self, ImportTarget};
use arnod_lib::input::{self, RequestInput};
use arnod_lib::layout::windows::{MAIN, Windows};
use arnod_lib::layout::{Content, Preset};
use arnod_lib::local;
use arnod_lib::opencollection::schema::{self, Part};
use arnod_lib::opencollection::yaml;
use arnod_lib::request::model::{
    Assertion, Auth, AuthKind, Body, BodyKind, Extraction, Method, Operator, Origin, Pair, Request,
    Scalar,
};
use arnod_lib::request::{file, inherit};
use arnod_lib::response;
use arnod_lib::results::{ResultBook, ResultChange};
use arnod_lib::secrets::MemorySecretStore;
use arnod_lib::state::AppState;
use arnod_lib::statuses::{self, RequestStatus, StatusBook, Statuses};
use arnod_lib::tree::batch::{self, BatchAction};
use arnod_lib::tree::transfer::{self, Mode, Target};
use arnod_lib::tree::undo::Undo;
use arnod_lib::tree::{self, TreeNode};
use arnod_lib::watch::service::{Sink, Watchers};
use arnod_lib::watch::{self, FilesChange};
use arnod_lib::windows::{RunChange, StatusesChange, WindowsState};
use arnod_lib::workbench::{
    ItemStatus, Notifier, ProjectCtx, RunItem, RunView, SendOutcome, Workbench, run_groups,
    run_tests,
};

// ------------------------------------------------------------------------------------ mesures

/// Passes d'une mesure courante.
const PASSES: usize = 9;

struct Stat {
    median: f64,
    min: f64,
    max: f64,
}

impl Stat {
    fn of(mut times: Vec<f64>) -> Self {
        times.sort_by(f64::total_cmp);
        Self {
            median: times[times.len() / 2],
            min: times[0],
            max: times[times.len() - 1],
        }
    }

    fn text(&self) -> String {
        let unit = |ms: f64| {
            if ms >= 1000.0 {
                format!("{:.2} s", ms / 1000.0)
            } else if ms >= 10.0 {
                format!("{ms:.1} ms")
            } else if ms >= 0.1 {
                format!("{ms:.2} ms")
            } else {
                format!("{:.1} µs", ms * 1000.0)
            }
        };
        format!(
            "{} ({} à {})",
            unit(self.median),
            unit(self.min),
            unit(self.max)
        )
    }
}

/// Compteurs d'entrées-sorties du processus (`/proc/self/io`).
#[derive(Clone, Copy, Default)]
struct Io {
    rchar: u64,
    syscr: u64,
    wchar: u64,
    syscw: u64,
}

fn io() -> Io {
    let mut out = Io::default();
    for line in fs::read_to_string("/proc/self/io")
        .unwrap_or_default()
        .lines()
    {
        let mut parts = line.split(':');
        let key = parts.next().unwrap_or_default();
        let value: u64 = parts
            .next()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        match key {
            "rchar" => out.rchar = value,
            "syscr" => out.syscr = value,
            "wchar" => out.wchar = value,
            "syscw" => out.syscw = value,
            _ => {}
        }
    }
    out
}

struct Sample<T> {
    stat: Stat,
    out: T,
    /// Octets lus et écrits par passe, et appels `read` par passe.
    read: u64,
    reads: u64,
    written: u64,
    writes: u64,
}

impl<T> Sample<T> {
    fn io_text(&self) -> String {
        let mut text = format!("lit {} en {} read()", bytes(self.read as usize), self.reads);
        if self.written > 0 {
            let _ = write!(
                text,
                ", écrit {} en {} write()",
                bytes(self.written as usize),
                self.writes
            );
        }
        text
    }
}

/// Médiane de `passes` appels de `work` ; `prepare` n'est pas chronométré.
fn measure_prep<T>(
    passes: usize,
    mut prepare: impl FnMut(),
    mut work: impl FnMut() -> T,
) -> Sample<T> {
    let mut times = Vec::with_capacity(passes);
    let mut last = None;
    let mut total = Io::default();
    for _ in 0..passes {
        prepare();
        let before = io();
        let started = Instant::now();
        let out = work();
        times.push(started.elapsed().as_secs_f64() * 1e3);
        let after = io();
        total.rchar += after.rchar - before.rchar;
        total.syscr += after.syscr - before.syscr;
        total.wchar += after.wchar - before.wchar;
        total.syscw += after.syscw - before.syscw;
        last = Some(out);
    }
    let n = passes as u64;
    Sample {
        stat: Stat::of(times),
        out: last.expect("au moins une passe"),
        // La lecture de /proc/self/io elle-même compte pour une centaine d'octets.
        read: (total.rchar / n).saturating_sub(120),
        reads: (total.syscr / n).saturating_sub(1),
        written: total.wchar / n,
        writes: total.syscw / n,
    }
}

fn measure<T>(passes: usize, work: impl FnMut() -> T) -> Sample<T> {
    measure_prep(passes, || {}, work)
}

fn bytes(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.2} Mo", n as f64 / 1e6)
    } else if n >= 1000 {
        format!("{:.1} ko", n as f64 / 1e3)
    } else {
        format!("{n} o")
    }
}

fn json_len<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(0, |v| v.len())
}

fn title(text: &str) {
    println!("\n## {text}\n");
    println!("| opération | taille | temps médian (min à max) | JSON vers l'interface | détail |");
    println!("| --- | --- | --- | --- | --- |");
}

fn row(op: &str, size: &str, stat: &Stat, ipc: Option<usize>, note: &str) {
    println!(
        "| {op} | {size} | {} | {} | {note} |",
        stat.text(),
        ipc.map_or_else(|| "—".to_owned(), bytes)
    );
}

fn status_kb(key: &str) -> f64 {
    fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1).map(str::to_owned))
        .and_then(|kb| kb.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// Mémoire résidente, en Mo.
fn rss() -> f64 {
    status_kb("VmRSS:") / 1024.0
}

/// Pic de mémoire résidente depuis la dernière remise à zéro, en Mo.
fn peak() -> f64 {
    status_kb("VmHWM:") / 1024.0
}

fn reset_peak() {
    let _ = fs::write("/proc/self/clear_refs", "5");
}

fn load_average() -> String {
    fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

fn workdir(label: &str) -> tempfile::TempDir {
    let base = std::env::var_os("ARNOD_BANC_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")), PathBuf::from);
    fs::create_dir_all(&base).expect("dossier du banc");
    tempfile::Builder::new()
        .prefix(&format!("banc-{label}-"))
        .tempdir_in(base)
        .expect("dossier temporaire")
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1e3
}

// ------------------------------------------------------------------------------------ budgets

/// Compare une mesure à son budget (R7 et critères d'acceptation du plan-qualite-mvp, sur le
/// projet de 2 000 requêtes). Le dépassement est signalé ; avec `ARNOD_BANC_STRICT=1`, il fait
/// échouer le scénario.
fn budget(label: &str, measured: f64, limit: f64, unit: &str) {
    let held = measured <= limit;
    println!(
        "> Budget {} : {label} : {measured:.1} {unit} (au plus {limit} {unit})",
        if held { "tenu" } else { "DÉPASSÉ" }
    );
    let strict = std::env::var("ARNOD_BANC_STRICT").is_ok_and(|v| !v.is_empty() && v != "0");
    assert!(
        held || !strict,
        "budget dépassé : {label} : {measured:.1} {unit} pour {limit} {unit} au plus"
    );
}

// ------------------------------------------------------------------------------------ serveur

/// Serveur HTTP local : même réponse à toute requête, connexions gardées ouvertes.
struct Server {
    url: String,
    connections: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
}

const RUN_BODY: &str = r#"{"id":42,"status":"ok","name":"Banc","total":12.5,"items":[{"sku":"A-1","price":12.5,"qty":2}]}"#;

fn serve(body: Vec<u8>, content_type: &'static str, set_cookie: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("port local");
    let url = format!("http://{}", listener.local_addr().expect("adresse"));
    let connections = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let (conns, reqs) = (connections.clone(), requests.clone());
    let body = Arc::new(body);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            conns.fetch_add(1, Ordering::SeqCst);
            let (body, reqs) = (body.clone(), reqs.clone());
            std::thread::spawn(move || answer(stream, &body, content_type, set_cookie, &reqs));
        }
    });
    Server {
        url,
        connections,
        requests,
    }
}

fn answer(
    stream: TcpStream,
    body: &[u8],
    content_type: &str,
    set_cookie: bool,
    requests: &AtomicUsize,
) {
    let _ = stream.set_nodelay(true);
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        let mut length = 0usize;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
        if length > 0 {
            let mut sent = vec![0u8; length];
            if reader.read_exact(&mut sent).is_err() {
                return;
            }
        }
        let number = requests.fetch_add(1, Ordering::SeqCst);
        let mut head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
            body.len()
        );
        if set_cookie {
            let _ = write!(
                head,
                "Set-Cookie: session=s{number}; Path=/; Max-Age=3600\r\n"
            );
        }
        head.push_str("\r\n");
        if writer.write_all(head.as_bytes()).is_err() || writer.write_all(body).is_err() {
            return;
        }
    }
}

// ------------------------------------------------------------------------------------ données

/// Forme d'un projet engendré.
struct Shape {
    label: &'static str,
    /// Dossiers « Domaine », chacun avec ses requêtes libres et un test.
    domains: usize,
    /// Requêtes hors test et hors contrat.
    free: usize,
    tests: usize,
    steps: usize,
    /// Opérations du grand contrat (environ 3,4 ko chacune).
    big_ops: usize,
    /// Opérations des neuf autres contrats.
    small_ops: Vec<usize>,
    envs: usize,
    vars: usize,
}

impl Shape {
    /// 2 000 requêtes, 100 dossiers, 30 tests de 8 étapes, 10 contrats (un de 300
    /// opérations), 600 requêtes liées, 20 environnements de 40 variables.
    fn large() -> Self {
        Self {
            label: "grand",
            domains: 30,
            free: 1160,
            tests: 30,
            steps: 8,
            big_ops: 300,
            small_ops: vec![34, 34, 34, 34, 34, 34, 32, 32, 32],
            envs: 20,
            vars: 40,
        }
    }

    /// Le même projet au quart : 500 requêtes.
    fn medium() -> Self {
        Self {
            label: "moyen",
            domains: 8,
            free: 286,
            tests: 8,
            steps: 8,
            big_ops: 75,
            small_ops: vec![9, 9, 9, 8, 8, 8, 8, 8, 8],
            envs: 20,
            vars: 40,
        }
    }

    fn requests(&self) -> usize {
        self.free + self.tests * self.steps + self.big_ops + self.small_ops.iter().sum::<usize>()
    }
}

struct Project {
    dir: tempfile::TempDir,
    state: AppState,
    ctx: ProjectCtx,
    bench: Arc<Workbench>,
    requests: usize,
    free: Vec<String>,
    /// Étapes du premier test.
    steps: Vec<String>,
    /// Requêtes liées au grand contrat.
    linked_big: Vec<String>,
    big_contract: String,
    big_contract_bytes: usize,
}

fn variables(count: usize, base_url: &str) -> Vec<Variable> {
    let mut list = vec![
        Variable {
            name: "baseUrl".to_owned(),
            value: base_url.to_owned(),
        },
        Variable {
            name: "tenant".to_owned(),
            value: "boutique".to_owned(),
        },
        Variable {
            name: "pageSize".to_owned(),
            value: "50".to_owned(),
        },
    ];
    for index in list.len()..count {
        list.push(Variable {
            name: format!("variable{index:02}"),
            value: format!("valeur-{index:02}-de-demonstration"),
        });
    }
    list
}

fn check(target: &str, operator: Operator, value: Scalar, id: &str) -> Assertion {
    Assertion {
        id: id.to_owned(),
        target: target.to_owned(),
        operator,
        value: Some(value),
        origin: Origin::Manual,
        enabled: true,
    }
}

fn free_request(index: usize, domain: usize) -> Request {
    let mut request = Request::new(format!("Requête {index:04}"));
    request.description = format!("Vérifie la ressource {index} du domaine {domain:02}.");
    request.tags = vec![
        format!("domaine-{domain:02}"),
        if index.is_multiple_of(3) {
            "fumee"
        } else {
            "regression"
        }
        .to_owned(),
    ];
    request.method = match index % 4 {
        0 => Method::Get,
        1 => Method::Post,
        2 => Method::Put,
        _ => Method::Delete,
    };
    request.url = format!("{{{{baseUrl}}}}/api/v1/domaine-{domain:02}/ressources/{index}");
    request.params = vec![Pair::new("page", "1"), Pair::new("limit", "{{pageSize}}")];
    request.headers = vec![
        Pair::new("Accept", "application/json"),
        Pair::new("X-Tenant", "{{tenant}}"),
        Pair::new("X-Request-Id", "{{$randomUUID}}"),
    ];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "{{token}}".to_owned(),
        ..Auth::default()
    };
    if matches!(request.method, Method::Post | Method::Put) {
        request
            .headers
            .push(Pair::new("Content-Type", "application/json"));
        request.body = Body::raw(
            BodyKind::Json,
            format!(
                "{{\n  \"name\": \"Ressource {index}\",\n  \"quantity\": {},\n  \"tenant\": \"{{{{tenant}}}}\",\n  \"createdAt\": \"{{{{$isoTimestamp}}}}\"\n}}",
                index % 7
            ),
        );
    }
    request
        .assertions
        .push(check("status", Operator::Equals, Scalar::Int(200), "a1"));
    request.assertions.push(check(
        "body.status",
        Operator::Equals,
        Scalar::Text("ok".to_owned()),
        "a2",
    ));
    request.docs = "Requête du banc de performance.\n\nElle ressemble à une requête réelle : en-têtes, corps et assertions.".to_owned();
    request
}

fn step_request(test: usize, step: usize) -> Request {
    let mut request = Request::new(format!("Étape {step}"));
    request.seq = Some(step as f64);
    request.headers = vec![
        Pair::new("Accept", "application/json"),
        Pair::new("X-Tenant", "{{tenant}}"),
    ];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "{{token}}".to_owned(),
        ..Auth::default()
    };
    if step == 1 {
        request.method = Method::Post;
        request.url = format!("{{{{baseUrl}}}}/parcours/{test}/commandes");
        request
            .headers
            .push(Pair::new("Content-Type", "application/json"));
        request.body = Body::raw(BodyKind::Json, "{\n  \"customer\": 42,\n  \"qty\": 2\n}");
        request.extractions = vec![Extraction {
            name: format!("parcours{test}Id"),
            source: "body.id".to_owned(),
            scope: arnod_lib::request::model::ExtractionScope::Runtime,
            enabled: true,
        }];
    } else {
        request.url = format!(
            "{{{{baseUrl}}}}/parcours/{test}/commandes/{{{{parcours{test}Id}}}}/etape/{step}"
        );
    }
    request
        .assertions
        .push(check("status", Operator::Equals, Scalar::Int(200), "a1"));
    request
}

/// Une opération OpenAPI, avec `props` champs documentés dans sa réponse.
fn operation(out: &mut String, method: &str, summary: &str, id: &str, tag: &str, props: usize) {
    let with_id = matches!(method, "put" | "delete") || id.starts_with("lire");
    let with_body = matches!(method, "post" | "put");
    let _ = write!(
        out,
        "    {method}:\n      summary: {summary}\n      operationId: {id}\n      description: Opération {id} du banc de performance, documentée comme une vraie.\n      tags:\n        - {tag}\n      parameters:\n"
    );
    if with_id {
        out.push_str("        - name: id\n          in: path\n          required: true\n          description: Identifiant de la ressource.\n          schema:\n            type: integer\n            example: 42\n");
    }
    out.push_str("        - name: page\n          in: query\n          required: false\n          description: Numéro de page, à partir de 1.\n          schema:\n            type: integer\n            example: 1\n        - name: X-Tenant\n          in: header\n          required: true\n          description: Locataire de l'appel.\n          schema:\n            type: string\n            example: boutique\n");
    if with_body {
        out.push_str("      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              required:\n                - name\n              properties:\n                name:\n                  type: string\n                  example: Exemple\n                quantity:\n                  type: integer\n                  example: 2\n");
    }
    out.push_str("      responses:\n        \"200\":\n          description: Réponse nominale.\n          content:\n            application/json:\n              schema:\n                type: object\n                required:\n                  - id\n                  - status\n                properties:\n                  id:\n                    type: integer\n                    description: Identifiant de la ressource.\n                    example: 42\n                  status:\n                    type: string\n                    example: ok\n");
    for prop in 0..props {
        let _ = write!(
            out,
            "                  champ{prop:02}:\n                    type: string\n                    description: Champ métier numéro {prop} de la ressource, documenté pour le banc.\n                    example: valeur-{prop:02}\n"
        );
    }
    out.push_str("        \"404\":\n          description: Ressource introuvable.\n");
}

/// Contrat OpenAPI 3.0 de `ops` opérations : cinq par ressource (lister, créer, lire,
/// modifier, supprimer), trois tags.
fn contract_text(index: usize, ops: usize, props: usize) -> String {
    let mut out = format!(
        "openapi: 3.0.3\ninfo:\n  title: \"Service {index:02}\"\n  version: 1.{index}.0\n  description: Contrat du banc de performance, service {index:02}.\nservers:\n  - url: http://localhost:8080\npaths:\n"
    );
    let mut left = ops;
    let mut resource = 0;
    while left > 0 {
        let count = left.min(5);
        left -= count;
        let tag = ["Lecture", "Ecriture", "Gestion"][resource % 3];
        let name = format!("res{index}x{resource}");
        let _ = writeln!(out, "  /{name}:");
        operation(
            &mut out,
            "get",
            &format!("Lister {name}"),
            &format!("lister{name}"),
            tag,
            props,
        );
        if count >= 2 {
            operation(
                &mut out,
                "post",
                &format!("Créer {name}"),
                &format!("creer{name}"),
                tag,
                props,
            );
        }
        if count >= 3 {
            let _ = writeln!(out, "  /{name}/{{id}}:");
            operation(
                &mut out,
                "get",
                &format!("Lire {name}"),
                &format!("lire{name}"),
                tag,
                props,
            );
        }
        if count >= 4 {
            operation(
                &mut out,
                "put",
                &format!("Modifier {name}"),
                &format!("modifier{name}"),
                tag,
                props,
            );
        }
        if count >= 5 {
            operation(
                &mut out,
                "delete",
                &format!("Supprimer {name}"),
                &format!("supprimer{name}"),
                tag,
                props,
            );
        }
        resource += 1;
    }
    out
}

/// Grand contrat : assez de champs documentés pour peser environ 3,4 ko par opération.
fn big_contract_text(ops: usize) -> String {
    let mut props = 4;
    loop {
        let text = contract_text(0, ops, props);
        if text.len() >= ops * 3400 || props >= 40 {
            return text;
        }
        props += 1;
    }
}

/// Engendre le projet avec les fonctions du cœur ; `base_url` va dans « Local ».
fn build(shape: &Shape, base_url: &str) -> Project {
    let started = Instant::now();
    let dir = workdir(shape.label);
    let data_dir = dir.path().join("donnees");
    let state = AppState::load(data_dir.clone(), dir.path().join("projets"));
    let created = state
        .create_project("Grand projet", None, None)
        .expect("projet créé");
    let ctx = state.project_ctx(created.project_id).expect("projet");
    let bench = Arc::new(Workbench::new(Arc::new(MemorySecretStore::default())));
    bench.network().attach(&data_dir);
    let root = ctx.root.clone();

    // Environnements : « Local » complété, puis les autres.
    let mut local_env = environment::list(&root).remove(0);
    local_env.variables = variables(shape.vars, base_url);
    local_env.locals = vec!["token".to_owned()];
    bench
        .save_environment(&ctx, &local_env, Some("Local"))
        .expect("Local");
    bench
        .set_local(
            &ctx,
            "Local",
            "token",
            "jeton-secret-0123456789abcdef",
            true,
        )
        .expect("secret");
    for index in 1..shape.envs {
        let env = Environment {
            name: format!("Env {index:02}"),
            color: "gray".to_owned(),
            protected: false,
            variables: variables(shape.vars, &format!("https://env{index:02}.exemple.test")),
            locals: vec!["token".to_owned()],
        };
        environment::save(&root, &env, None).expect("environnement");
    }

    // Contrats et requêtes liées, créées par « Créer les requêtes ».
    let mut linked_big = Vec::new();
    let mut big_contract = String::new();
    let mut big_contract_bytes = 0;
    let all_ops = std::iter::once(shape.big_ops).chain(shape.small_ops.iter().copied());
    for (index, ops) in all_ops.enumerate() {
        let text = if index == 0 {
            big_contract_text(ops)
        } else {
            contract_text(index, ops, 4)
        };
        let path = contract::file::add(&root, &text).expect("contrat ajouté");
        let keys: Vec<String> = contract::file::read(&root, &path)
            .expect("contrat lisible")
            .operations
            .iter()
            .map(|operation| operation.key.clone())
            .collect();
        assert_eq!(keys.len(), ops);
        let report = contract::file::generate(&root, &path, &keys, None).expect("requêtes créées");
        assert_eq!(report.created.len(), ops, "{:?}", report.skipped);
        if index == 0 {
            linked_big = report.created;
            big_contract = path;
            big_contract_bytes = text.len();
        }
    }

    // Domaines : réglages de dossier, requêtes libres, un test chacun.
    for domain in 0..shape.domains {
        let folder =
            tree::create_folder(&root, "", &format!("Domaine {domain:02}")).expect("dossier");
        let settings = json!({
            "info": { "name": format!("Domaine {domain:02}"), "type": "folder" },
            "request": { "headers": [{ "name": "X-Domaine", "value": format!("domaine-{domain:02}") }] },
        });
        schema::check(Part::Folder, &settings).expect("folder.yml conforme");
        yaml::write(&root.join(&folder).join("folder.yml"), &settings).expect("folder.yml");
    }
    let mut free = Vec::new();
    for index in 0..shape.free {
        let domain = index % shape.domains;
        let path = format!("Domaine {domain:02}/requete-{index:04}.yml");
        let absolute = tree::resolve(&root, &path).expect("chemin");
        file::write(&absolute, &free_request(index, domain)).expect("requête écrite");
        free.push(path);
    }
    let mut steps = Vec::new();
    for test in 0..shape.tests {
        let domain = test % shape.domains;
        let folder = tree::create_folder(
            &root,
            &format!("Domaine {domain:02}"),
            &format!("Parcours {test:02}"),
        )
        .expect("test");
        for step in 1..=shape.steps {
            let path = format!("{folder}/etape-{step}.yml");
            let absolute = tree::resolve(&root, &path).expect("chemin");
            file::write(&absolute, &step_request(test, step)).expect("étape écrite");
            if test == 0 {
                steps.push(path);
            }
        }
    }
    let requests = tree::requests_in(&root, "").expect("arborescence").len();
    assert_eq!(requests, shape.requests());
    println!(
        "\n> Projet « {} » engendré en {:.1} s : {requests} requêtes, {} contrats (grand contrat : {}), {} environnements de {} variables.",
        shape.label,
        started.elapsed().as_secs_f64(),
        shape.small_ops.len() + 1,
        bytes(big_contract_bytes),
        shape.envs,
        shape.vars,
    );
    Project {
        dir,
        state,
        ctx,
        bench,
        requests,
        free,
        steps,
        linked_big,
        big_contract,
        big_contract_bytes,
    }
}

fn folder_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder { children, .. } => 1 + folder_count(children),
            TreeNode::Request { .. } => 0,
        })
        .sum()
}

fn test_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Folder { children, test, .. } => usize::from(*test) + test_count(children),
            TreeNode::Request { .. } => 0,
        })
        .sum()
}

struct Silent;

impl Sink for Silent {
    fn changed(&self, _change: FilesChange) {}
}

/// Run terminé de toutes les requêtes, tel que `last-run.json` le garde.
fn finished_run(paths: &[String]) -> RunView {
    let items: Vec<RunItem> = paths
        .iter()
        .map(|path| RunItem {
            path: path.clone(),
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            method: Method::Get,
            status: ItemStatus::Passed,
            http_status: Some(200),
            duration_ms: Some(12),
            slow: false,
            assertions: serde_json::from_value(json!([
                { "id": "duree", "target": "responseTime", "passed": true, "message": null, "actual": "12", "ignored": false },
                { "id": "a1", "target": "status", "passed": true, "message": null, "actual": "200", "ignored": false },
                { "id": "a2", "target": "body.status", "passed": true, "message": null, "actual": "ok", "ignored": false },
            ]))
            .expect("résultats"),
            error: None,
            test: None,
            step: None,
            extracted: Vec::new(),
            flow: Default::default(),
        })
        .collect();
    RunView {
        id: Uuid::new_v4(),
        number: 1,
        folder: String::new(),
        environment: "Local".to_owned(),
        concurrency: 8,
        started_at: 1_800_000_000_000,
        finished_at: Some(1_800_000_010_000),
        finished: true,
        cancelled: false,
        groups: run_groups("", &items),
        tests: run_tests(&items),
        ends: Vec::new(),
        items,
        contract: None,
        revision: 1,
    }
}

fn seed_statuses(project: &Project, paths: &[String]) {
    let requests: Statuses = paths
        .iter()
        .map(|path| {
            (
                path.clone(),
                RequestStatus {
                    passed: true,
                    at: 1_800_000_000_000,
                },
            )
        })
        .collect();
    statuses::save(
        &project.ctx.data_dir,
        project.ctx.id,
        &StatusBook {
            requests,
            read_only: false,
        },
    )
    .expect("statuts");
}

fn seed_last_run(project: &Project, paths: &[String]) {
    let dir = local::project_dir(&project.ctx.data_dir, project.ctx.id);
    fs::create_dir_all(&dir).expect("dossier local");
    let text = serde_json::to_string(&finished_run(paths)).expect("run");
    fs::write(dir.join("last-run.json"), text).expect("last-run.json");
}

/// Corps de `tree_load` (`commands/tree.rs`).
fn tree_load(project: &Project, watchers: &Watchers) -> TreeView {
    let ctx = project.state.project_ctx(project.ctx.id).expect("projet");
    let _ = watchers.watch(ctx.id, &ctx.root);
    let nodes = tree::read(&ctx.root).expect("arborescence");
    let tests = tree::tests_view(&nodes, &contract::file::paths(&ctx.root));
    let data = local::load(&ctx.data_dir, ctx.id);
    TreeView {
        nodes,
        tests,
        collapsed: data.collapsed,
        templates: arnod_lib::template::list(&ctx.root),
        sections: data.sections,
    }
}

// ------------------------------------------------------------------------- 1. ouverture

#[test]
#[ignore = "banc de performance"]
fn perf_1_ouverture() {
    println!("\n> Charge de la machine : {}", load_average());
    for shape in [Shape::medium(), Shape::large()] {
        let project = build(&shape, "http://127.0.0.1:9");
        let root = project.ctx.root.clone();
        let size = format!("{} req.", project.requests);
        let paths = tree::requests_in(&root, "").expect("arborescence");
        seed_statuses(&project, &paths);
        seed_last_run(&project, &paths);
        let watchers = Watchers::new(Arc::new(Silent));
        let nodes = tree::read(&root).expect("arborescence");
        println!(
            "> {} dossiers, {} tests, {} fichiers de requête.",
            folder_count(&nodes),
            test_count(&nodes),
            project.requests
        );
        title(&format!("1. Ouverture d'un projet ({size})"));

        let ctx = measure(2001, || {
            project.state.project_ctx(project.ctx.id).expect("ctx")
        });
        row(
            "`project_ctx` (chaque commande)",
            &size,
            &ctx.stat,
            None,
            "verrou d'AppState, is_dir, migrate::run",
        );

        let read = measure(PASSES, || tree::read(&root).expect("arborescence"));
        row(
            "`tree::read`",
            &size,
            &read.stat,
            Some(json_len(&read.out)),
            &read.io_text(),
        );

        let contracts = contract::file::paths(&root);
        let view = measure(PASSES, || tree::tests_view(&nodes, &contracts));
        row(
            "`tree::tests_view`",
            &size,
            &view.stat,
            Some(json_len(&view.out)),
            "copie de l'arbre sans les requêtes liées",
        );

        let watch_only = measure_prep(
            PASSES,
            || watchers.unwatch(project.ctx.id),
            || watchers.watch(project.ctx.id, &root).expect("surveillance"),
        );
        row(
            "`Watchers::watch` (registre d'empreintes + inotify)",
            &size,
            &watch_only.stat,
            None,
            &format!("{} ; verrou global du registre tenu", watch_only.io_text()),
        );

        let cold = measure_prep(
            PASSES,
            || watchers.unwatch(project.ctx.id),
            || tree_load(&project, &watchers),
        );
        let tree_json = json_len(&cold.out);
        row(
            "**`tree_load` à l'ouverture** (observateur à démarrer)",
            &size,
            &cold.stat,
            Some(tree_json),
            &cold.io_text(),
        );

        let warm = measure(PASSES, || tree_load(&project, &watchers));
        row(
            "`tree_load` suivant (observateur déjà là)",
            &size,
            &warm.stat,
            Some(tree_json),
            &warm.io_text(),
        );

        let encode = measure(PASSES, || {
            serde_json::to_vec(&warm.out).expect("json").len()
        });
        row(
            "sérialisation JSON de `TreeView`",
            &size,
            &encode.stat,
            Some(encode.out),
            "",
        );

        let list = measure(PASSES, || contract::file::list(&root));
        let list_json = json_len(&list.out);
        row(
            "**`contract_list`** (résumés, ressources, requêtes liées)",
            &size,
            &list.stat,
            Some(list_json),
            &list.io_text(),
        );

        let linked = measure(PASSES, || contract::file::linked(&root).len());
        row(
            "  dont `contract::file::linked`",
            &size,
            &linked.stat,
            None,
            &format!("{} liées ; {}", linked.out, linked.io_text()),
        );

        let text = fs::read_to_string(root.join(&project.big_contract)).expect("contrat");
        let parse = measure(PASSES, || {
            contract::parse::load(&text)
                .expect("contrat")
                .operations
                .len()
        });
        row(
            "  dont lecture du grand contrat (`parse::load`)",
            &bytes(text.len()),
            &parse.stat,
            None,
            &format!("{} opérations", parse.out),
        );

        let envs = measure(PASSES, || project.bench.environments(&project.ctx));
        row(
            "`environments_load`",
            &format!("{} × {}", shape.envs, shape.vars),
            &envs.stat,
            Some(json_len(&envs.out)),
            &envs.io_text(),
        );

        let load = measure(PASSES, || {
            statuses::load(&project.ctx.data_dir, project.ctx.id)
        });
        row(
            "`request_statuses` (première lecture de statuses.json)",
            &size,
            &load.stat,
            Some(json_len(&load.out.requests)),
            &load.io_text(),
        );

        let last = measure(PASSES, || {
            Workbench::last_run(&project.ctx).expect("dernier run")
        });
        row(
            "`run_last` (last-run.json)",
            &size,
            &last.stat,
            Some(json_len(&last.out)),
            &last.io_text(),
        );

        let total = cold.stat.median
            + list.stat.median
            + envs.stat.median
            + load.stat.median
            + last.stat.median;
        let total_json = tree_json
            + list_json
            + json_len(&envs.out)
            + json_len(&load.out.requests)
            + json_len(&last.out);
        println!(
            "| **Total à l'ouverture** (tree_load + contract_list + environments_load + request_statuses + run_last, commandes synchrones) | {size} | {total:.1} ms | {} | somme des médianes |",
            bytes(total_json)
        );
        if shape.label == "grand" {
            budget(
                "ouverture d'un projet (somme des commandes)",
                total,
                250.0,
                "ms",
            );
        }
    }
}

// ------------------------------------------------------------------------- 2. frappe

/// Corps de `request_input` (`commands/input.rs`).
fn request_input(
    project: &Project,
    environment: &str,
    path: Option<&str>,
    request: Request,
) -> RequestInput {
    let ctx = project.state.project_ctx(project.ctx.id).expect("projet");
    let all = environment::list(&ctx.root);
    let active = all
        .iter()
        .find(|env| env.name.eq_ignore_ascii_case(environment))
        .cloned()
        .expect("environnement");
    let mut request = request;
    let (inherited, inherited_vars) = path.map_or_else(Default::default, |path| {
        (
            inherit::resolve(&ctx.root, path),
            inherit::variable_names(&ctx.root, path),
        )
    });
    request.inherited = inherited;
    let data = local::load(&ctx.data_dir, ctx.id);
    let locals = data.locals.get(&active.name).cloned().unwrap_or_default();
    let mut known = input::known_names(&active, &locals, &inherited_vars);
    let extracted = path
        .map(|path| tree::extracted_before(&ctx.root, path))
        .unwrap_or_default();
    for name in extracted
        .into_iter()
        .chain(project.bench.runtime_variables(ctx.id).into_keys())
    {
        if !known.contains(&name) {
            known.push(name);
        }
    }
    let unknown = input::unknown(&input::used_names(&request), &known, &all, &active.name);
    let mut values: BTreeMap<String, String> = active
        .variables
        .iter()
        .map(|v| (v.name.clone(), v.value.clone()))
        .collect();
    for var in locals.iter().filter(|v| !v.secret) {
        if let Some(value) = &var.value {
            values.insert(var.name.clone(), value.clone());
        }
    }
    RequestInput {
        environment: active.name.clone(),
        added: input::added_headers(&request, &values),
        secrets: input::secret::clear_secrets(&request, &known),
        known,
        unknown,
    }
}

#[test]
#[ignore = "banc de performance"]
fn perf_2_frappe() {
    println!("\n> Charge de la machine : {}", load_average());
    for shape in [Shape::medium(), Shape::large()] {
        let project = build(&shape, "http://127.0.0.1:9");
        let root = project.ctx.root.clone();
        let size = format!("{} req.", project.requests);
        let watchers = Watchers::new(Arc::new(Silent));
        watchers.watch(project.ctx.id, &root).expect("surveillance");
        title(&format!("2. Frappe dans l'éditeur ({size})"));

        let cases = [
            ("requête libre", project.free[5].clone()),
            ("étape 6 d'un test", project.steps[5].clone()),
            ("requête liée à un contrat", project.linked_big[10].clone()),
        ];
        for (label, path) in &cases {
            let request =
                file::read(&tree::resolve(&root, path).expect("chemin")).expect("requête");
            let sample = measure(PASSES, || {
                request_input(&project, "Local", Some(path.as_str()), request.clone())
            });
            assert!(sample.out.unknown.is_empty(), "{:?}", sample.out.unknown);
            row(
                &format!("**`request_input`**, {label}"),
                &size,
                &sample.stat,
                Some(json_len(&sample.out)),
                &format!(
                    "{} ; argument : {}",
                    sample.io_text(),
                    bytes(json_len(&request))
                ),
            );
            if shape.label == "grand" {
                budget(
                    &format!("`request_input`, {label}"),
                    sample.stat.median,
                    16.0,
                    "ms",
                );
            }
        }
        let path = &cases[1].1;
        let envs = measure(PASSES, || environment::list(&root).len());
        row(
            "  dont `environment::list`",
            &format!("{} × {}", shape.envs, shape.vars),
            &envs.stat,
            None,
            &envs.io_text(),
        );
        let chain = measure(PASSES, || {
            (
                inherit::resolve(&root, path),
                inherit::variable_names(&root, path),
            )
        });
        row(
            "  dont héritage (`inherit::resolve` + `variable_names`)",
            &size,
            &chain.stat,
            None,
            &chain.io_text(),
        );
        let before = measure(PASSES, || tree::extracted_before(&root, path));
        row(
            "  dont **`tree::extracted_before`**",
            &size,
            &before.stat,
            None,
            &format!("{} ; {} variable(s)", before.io_text(), before.out.len()),
        );
        let local = measure(PASSES, || {
            local::load(&project.ctx.data_dir, project.ctx.id)
        });
        row(
            "  dont `local::load`",
            &size,
            &local.stat,
            None,
            &local.io_text(),
        );

        // Enregistrement automatique : deux contenus alternés, pour écrire à chaque passe.
        let absolute = tree::resolve(&root, path).expect("chemin");
        let request = file::read(&absolute).expect("requête");
        watch::acknowledge(&absolute);
        let mut flip = false;
        let save = measure(25, || {
            flip = !flip;
            let mut next = request.clone();
            next.description = if flip {
                "Première version."
            } else {
                "Seconde version."
            }
            .to_owned();
            let ctx = project.state.project_ctx(project.ctx.id).expect("projet");
            let target = tree::resolve(&ctx.root, path).expect("chemin");
            watch::check_unchanged(&target).expect("inchangé");
            file::write(&target, &next).expect("écriture");
        });
        row(
            "**`request_save`** (fusion, schéma, écriture, observateur)",
            &size,
            &save.stat,
            None,
            &save.io_text(),
        );
        if shape.label == "grand" {
            budget("`request_save`", save.stat.median, 5.0, "ms");
        }

        let same = measure(25, || {
            let target = tree::resolve(&root, path).expect("chemin");
            watch::check_unchanged(&target).expect("inchangé");
            file::write(&target, &file::read(&target).expect("requête")).expect("écriture");
        });
        row(
            "`request_save` sans changement",
            &size,
            &same.stat,
            None,
            &same.io_text(),
        );

        let read = measure(25, || {
            let mut request = file::read(&absolute).expect("requête");
            watch::acknowledge(&absolute);
            request.inherited = inherit::resolve(&root, path);
            request
        });
        row(
            "`request_read`",
            &size,
            &read.stat,
            Some(json_len(&read.out)),
            &read.io_text(),
        );

        // Ce que l'interface enchaîne après chaque enregistrement : `tree_load`, puis
        // `contract_list` (stores `workbench` et `contracts`).
        let after = measure(PASSES, || {
            let view = tree_load(&project, &watchers);
            let list = contract::file::list(&root);
            json_len(&view) + json_len(&list)
        });
        row(
            "**suite d'un enregistrement** : `tree_load` + `contract_list`",
            &size,
            &after.stat,
            Some(after.out),
            &after.io_text(),
        );
        println!(
            "| **Total par enregistrement automatique** | {size} | {:.1} ms | {} | request_save + tree_load + contract_list |",
            save.stat.median + after.stat.median,
            bytes(after.out)
        );
    }
}

// ------------------------------------------------------------------------- 3. réponse

fn json_body(objects: usize) -> String {
    let mut out = format!("{{\"total\":{objects},\"items\":[");
    for index in 0..objects {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"id\":{index},\"sku\":\"SKU-{index:06}\",\"name\":\"Article de démonstration numéro {index:05}\",\"price\":{}.{:02},\"qty\":{},\"available\":{},\"tags\":[\"alpha\",\"beta\",\"gamma\"],\"warehouse\":{{\"code\":\"W{:02}\",\"city\":\"Rennes\"}},\"description\":\"Lot de démonstration pour le banc.\"}}",
            index % 20,
            index % 100,
            index % 9,
            index % 2 == 0,
            index % 40,
        );
    }
    out.push_str("]}");
    out
}

fn html_body(rows: usize) -> String {
    let mut out = String::from(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Rapport</title><style>td{padding:4px}</style><script>window.x = 1;</script></head><body><header><h1>Rapport des commandes</h1><p>Page de démonstration du banc.</p></header><main><table>\n",
    );
    for index in 0..rows {
        let _ = writeln!(
            out,
            "<tr class=\"ligne\"><td class=\"id\">{index}</td><td>Commande numéro {index:06}</td><td><a href=\"/commandes/{index}\">Détail</a></td><td><strong>{}</strong> €</td></tr>",
            index % 500
        );
    }
    out.push_str("</table></main><footer><small>Fin du rapport</small></footer></body></html>");
    out
}

/// Décodage côté Rust des arguments d'une commande : le message est lu en `Value`, puis
/// l'argument est copié (ce que fait Tauri pour un `String`).
fn decode_argument(payload: &[u8]) -> usize {
    let value: serde_json::Value = serde_json::from_slice(payload).expect("arguments");
    let body: String = serde_json::from_value(value["body"].clone()).expect("corps");
    body.len()
}

#[test]
#[ignore = "banc de performance"]
fn perf_3_reponse() {
    println!("\n> Charge de la machine : {}", load_average());
    title("3. Corps de la réponse");
    let json_headers = vec![("content-type".to_owned(), "application/json".to_owned())];
    let html_headers = vec![(
        "content-type".to_owned(),
        "text/html; charset=utf-8".to_owned(),
    )];

    // Mise en forme : jusqu'à 1 Mo, chaque valeur porte son chemin.
    for objects in [1_000, 4_000] {
        let body = json_body(objects);
        let sample = measure(PASSES, || response::render(&body, &json_headers, None));
        let out = json_len(&sample.out);
        assert!(sample.out.notice.is_none());
        row(
            "`response_render` JSON (jetons et chemins)",
            &format!("{} ({objects} objets)", bytes(body.len())),
            &sample.stat,
            Some(out),
            &format!(
                "facteur {:.1} du corps ; {} lignes ; argument reçu : {}",
                out as f64 / body.len() as f64,
                sample.out.lines.len(),
                bytes(json_len(&body))
            ),
        );
    }
    let body = json_body(20_000);
    assert!(body.len() < MAX_BODY_BYTES, "{}", body.len());
    let size = format!("{} (20 000 objets)", bytes(body.len()));
    let argument = json_len(&body);
    reset_peak();
    let before = rss();
    let sample = measure(PASSES, || response::render(&body, &json_headers, None));
    let out = json_len(&sample.out);
    row(
        "**`response_render` JSON de 5 Mo** (au-delà de 1 Mo : sans mise en forme)",
        &size,
        &sample.stat,
        Some(out),
        &format!(
            "facteur {:.2} ; {} ligne(s) ; argument reçu : {} ; pic mémoire +{:.0} Mo",
            out as f64 / body.len() as f64,
            sample.out.lines.len(),
            bytes(argument),
            peak() - before
        ),
    );
    let encode = measure(PASSES, || {
        serde_json::to_vec(&sample.out).expect("json").len()
    });
    row(
        "  sérialisation de la réponse",
        &size,
        &encode.stat,
        Some(encode.out),
        "",
    );
    let payload =
        serde_json::to_vec(&json!({ "body": body, "headers": json_headers, "format": null }))
            .expect("arguments");
    let decode = measure(PASSES, || decode_argument(&payload));
    row(
        "  décodage de l'argument `body` côté Rust (estimation)",
        &size,
        &decode.stat,
        None,
        &format!("message de {}", bytes(payload.len())),
    );

    let pretty = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&json_body(12_000)).expect("json"),
    )
    .expect("json");
    let sample = measure(PASSES, || response::render(&pretty, &json_headers, None));
    let out = json_len(&sample.out);
    row(
        "`response_render` JSON indenté de plus de 1 Mo",
        &bytes(pretty.len()),
        &sample.stat,
        Some(out),
        &format!(
            "facteur {:.2} ; {} lignes d'un jeton",
            out as f64 / pretty.len() as f64,
            sample.out.lines.len()
        ),
    );

    // Filtre JSONPath.
    for objects in [5_000, 20_000] {
        let body = json_body(objects);
        let size = format!("{} ({objects} objets)", bytes(body.len()));
        for expression in ["$..sku", "$.items[?@.price > 10]"] {
            reset_peak();
            let before = rss();
            let sample = measure(PASSES, || response::filter(&body, expression));
            assert!(sample.out.error.is_none(), "{:?}", sample.out.error);
            row(
                &format!("**`response_filter`** `{expression}`"),
                &size,
                &sample.stat,
                Some(json_len(&sample.out)),
                &format!(
                    "{} correspondances, {} renvoyées ; argument reçu : {} ; pic mémoire +{:.0} Mo",
                    sample.out.total,
                    sample.out.matches.len(),
                    bytes(json_len(&body)),
                    peak() - before
                ),
            );
        }
        let parse = measure(PASSES, || {
            serde_json::from_str::<serde_json::Value>(&body).expect("json");
        });
        row(
            "  dont lecture du corps en JSON, refaite à chaque filtre",
            &size,
            &parse.stat,
            None,
            "",
        );
    }

    // HTML : aperçu et mise en forme.
    for rows in [5_000, 12_500] {
        let body = html_body(rows);
        let size = format!("{} ({rows} lignes)", bytes(body.len()));
        let preview = measure(PASSES, || response::preview(&body));
        let out = json_len(&preview.out);
        row(
            if rows > 5_000 {
                "**`response_preview` HTML de 2 Mo**"
            } else {
                "`response_preview` HTML"
            },
            &size,
            &preview.stat,
            Some(out),
            &format!(
                "facteur {:.2} ; argument reçu : {}",
                out as f64 / body.len() as f64,
                bytes(json_len(&body))
            ),
        );
        let render = measure(PASSES, || response::render(&body, &html_headers, None));
        let out = json_len(&render.out);
        row(
            "`response_render` HTML",
            &size,
            &render.stat,
            Some(out),
            &format!(
                "facteur {:.2} ; {} lignes{}",
                out as f64 / body.len() as f64,
                render.out.lines.len(),
                if render.out.notice.is_some() {
                    " ; sans mise en forme"
                } else {
                    ""
                }
            ),
        );
    }
    // Envoi d'une requête dont la réponse pèse 5 Mo : corps de `request_send`.
    let server = serve(json_body(20_000).into_bytes(), "application/json", false);
    let shape = Shape {
        label: "envoi",
        domains: 1,
        free: 4,
        tests: 1,
        steps: 2,
        big_ops: 5,
        small_ops: vec![5],
        envs: 2,
        vars: 40,
    };
    let project = build(&shape, &server.url);
    let path = project.free[0].clone();
    let request =
        file::read(&tree::resolve(&project.ctx.root, &path).expect("chemin")).expect("requête");
    let book = Mutex::new(ResultBook::default());
    let mut sizes = (0usize, 0usize, 0usize);
    let mut parts = (Vec::new(), Vec::new(), Vec::new());
    title("3 bis. Envoi d'une requête dont la réponse pèse 5 Mo (corps de `request_send`)");
    reset_peak();
    let before = rss();
    let sample = measure(7, || {
        let version = request.clone();
        let mut sent = request.clone();
        sent.inherited = inherit::resolve(&project.ctx.root, &path);
        project
            .bench
            .precheck(&project.ctx, "Local", &sent, false)
            .expect("précontrôle");
        let step = Instant::now();
        let started = book.lock().expect("résultats").start(project.ctx.id, &path);
        let first = json_len(&ResultChange {
            result: started,
            origin: MAIN.to_owned(),
        });
        let start_ms = ms(step);
        let step = Instant::now();
        let outcome = tauri::async_runtime::block_on(project.bench.send(
            &project.ctx,
            Uuid::new_v4(),
            "Local",
            sent,
            false,
        ));
        let send_ms = ms(step);
        let step = Instant::now();
        let finished = {
            let mut book = book.lock().expect("résultats");
            let result = book.finish(project.ctx.id, &path, &outcome, 1);
            book.keep_request(project.ctx.id, &path, version)
                .unwrap_or(result)
        };
        let second = json_len(&ResultChange {
            result: finished,
            origin: MAIN.to_owned(),
        });
        let reply = json_len(&outcome.as_ref().expect("réponse"));
        let finish_ms = ms(step);
        sizes = (first, second, reply);
        parts.0.push(start_ms);
        parts.1.push(send_ms);
        parts.2.push(finish_ms);
    });
    let size = bytes(json_body(20_000).len());
    row(
        "**`request_send`, réponse de 5 Mo** (envoi, résultats partagés, 2 événements, réponse)",
        &size,
        &sample.stat,
        Some(sizes.0 + sizes.1 + sizes.2),
        &format!(
            "`arnod://resultat` au début : {} (la réponse précédente repart) ; à la fin : {} ; réponse de la commande : {} ; pic mémoire +{:.0} Mo",
            bytes(sizes.0),
            bytes(sizes.1),
            bytes(sizes.2),
            peak() - before
        ),
    );
    row(
        "  dont début (`ResultBook::start` + événement)",
        &size,
        &Stat::of(parts.0),
        Some(sizes.0),
        "copie et sérialisation du résultat précédent",
    );
    row(
        "  dont `Workbench::send` (HTTP local, masquage, assertions)",
        &size,
        &Stat::of(parts.1),
        None,
        "",
    );
    row(
        "  dont fin (`finish`, `keep_request`, événement, réponse)",
        &size,
        &Stat::of(parts.2),
        Some(sizes.1 + sizes.2),
        "",
    );
    drop(project);
}

// ------------------------------------------------------------------------- 4. run

/// Compte et pèse les événements d'un run, comme `windows::EventNotifier` les émet.
struct Meter {
    serialize: bool,
    run_events: AtomicUsize,
    run_bytes: AtomicU64,
    run_max: AtomicU64,
    status_events: AtomicUsize,
    status_bytes: AtomicU64,
    status_max: AtomicU64,
    /// Temps passé à copier et sérialiser, tous fils confondus, en nanosecondes.
    nanos: AtomicU64,
    done: Mutex<mpsc::Sender<()>>,
}

impl Meter {
    fn new(serialize: bool, done: mpsc::Sender<()>) -> Self {
        Self {
            serialize,
            run_events: AtomicUsize::new(0),
            run_bytes: AtomicU64::new(0),
            run_max: AtomicU64::new(0),
            status_events: AtomicUsize::new(0),
            status_bytes: AtomicU64::new(0),
            status_max: AtomicU64::new(0),
            nanos: AtomicU64::new(0),
            done: Mutex::new(done),
        }
    }
}

impl Notifier for Meter {
    fn run_changed(&self, project_id: Uuid, run: &RunView) {
        self.run_events.fetch_add(1, Ordering::SeqCst);
        if self.serialize {
            let started = Instant::now();
            let change = RunChange {
                project_id,
                run: run.clone(),
                origin: String::new(),
            };
            let len = json_len(&change) as u64;
            self.run_bytes.fetch_add(len, Ordering::SeqCst);
            self.run_max.fetch_max(len, Ordering::SeqCst);
            self.nanos
                .fetch_add(started.elapsed().as_nanos() as u64, Ordering::SeqCst);
        }
        if run.finished
            && let Ok(done) = self.done.lock()
        {
            let _ = done.send(());
        }
    }

    fn statuses_changed(&self, project_id: Uuid, statuses: &Statuses) {
        self.status_events.fetch_add(1, Ordering::SeqCst);
        if self.serialize {
            let started = Instant::now();
            let change = StatusesChange {
                project_id,
                statuses: statuses.clone(),
            };
            let len = json_len(&change) as u64;
            self.status_bytes.fetch_add(len, Ordering::SeqCst);
            self.status_max.fetch_max(len, Ordering::SeqCst);
            self.nanos
                .fetch_add(started.elapsed().as_nanos() as u64, Ordering::SeqCst);
        }
    }
}

struct RunReport {
    items: usize,
    launch_ms: f64,
    total_ms: f64,
    /// Octets sérialisés de `arnod://run` et `arnod://statuts`.
    event_bytes: usize,
    view: RunView,
}

/// Lance un run et attend sa fin ; affiche sa ligne.
fn run(
    label: &str,
    project: &Project,
    server: &Server,
    serialize: bool,
    start: impl FnOnce(&Arc<Workbench>) -> Result<RunView, CommandError>,
) -> RunReport {
    let (done, finished) = mpsc::channel();
    let meter = Arc::new(Meter::new(serialize, done));
    project.bench.set_notifier(meter.clone());
    let (connections, requests) = (
        server.connections.load(Ordering::SeqCst),
        server.requests.load(Ordering::SeqCst),
    );
    let before = io();
    let rss_before = rss();
    reset_peak();
    let started = Instant::now();
    let view = start(&project.bench).expect("run lancé");
    let launch_ms = ms(started);
    if !view.finished {
        finished
            .recv_timeout(Duration::from_secs(1800))
            .expect("run terminé");
    }
    let total_ms = ms(started);
    let after = io();
    let view = project.bench.run_status(view.id).expect("run");
    let items = view.items.len();
    let passed = view
        .items
        .iter()
        .filter(|item| item.status == ItemStatus::Passed)
        .count();
    let run_events = meter.run_events.load(Ordering::SeqCst);
    let status_events = meter.status_events.load(Ordering::SeqCst);
    let run_bytes = meter.run_bytes.load(Ordering::SeqCst) as usize;
    let status_bytes = meter.status_bytes.load(Ordering::SeqCst) as usize;
    println!(
        "| {label} | {items} req. | {:.2} s (lancement {:.0} ms) | {:.0} req/s | {run_events} `arnod://run`{} ; {status_events} `arnod://statuts`{} ; sérialisation cumulée {:.2} s ; écrit sur disque {} en {} write() ; {} connexion(s) pour {} requêtes ; {passed} réussies ; mémoire {:.0} → {:.0} Mo (pic {:.0}) |",
        total_ms / 1000.0,
        launch_ms,
        items as f64 / (total_ms / 1000.0),
        if serialize {
            format!(
                " (max {}, total {})",
                bytes(meter.run_max.load(Ordering::SeqCst) as usize),
                bytes(run_bytes)
            )
        } else {
            String::new()
        },
        if serialize {
            format!(
                " (max {}, total {})",
                bytes(meter.status_max.load(Ordering::SeqCst) as usize),
                bytes(status_bytes)
            )
        } else {
            String::new()
        },
        meter.nanos.load(Ordering::SeqCst) as f64 / 1e9,
        bytes((after.wchar - before.wchar) as usize),
        after.syscw - before.syscw,
        server.connections.load(Ordering::SeqCst) - connections,
        server.requests.load(Ordering::SeqCst) - requests,
        rss_before,
        rss(),
        peak(),
    );
    RunReport {
        items,
        launch_ms,
        total_ms,
        event_bytes: run_bytes + status_bytes,
        view,
    }
}

fn run_title(text: &str) {
    println!("\n## {text}\n");
    println!("| run | lignes | durée | débit | détail |");
    println!("| --- | --- | --- | --- | --- |");
}

#[test]
#[ignore = "banc de performance"]
fn perf_4_run() {
    println!("\n> Charge de la machine : {}", load_average());
    let server = serve(RUN_BODY.as_bytes().to_vec(), "application/json", false);
    let mut totals = Vec::new();
    for shape in [Shape::medium(), Shape::large()] {
        let project = build(&shape, &server.url);
        let size = format!("{} req.", project.requests);
        run_title(&format!("4. Run ({size})"));
        let domain = run(
            "un dossier (référence par ligne)",
            &project,
            &server,
            true,
            |bench| bench.start_run(&project.ctx, "Domaine 01", "Local", None, false),
        );
        let per_item = (domain.total_ms - domain.launch_ms) / domain.items as f64;
        let free: Vec<String> = project
            .free
            .iter()
            .take(project.linked_big.len())
            .cloned()
            .collect();
        let free_run = run(
            "requêtes libres (autant que de liées)",
            &project,
            &server,
            true,
            |bench| bench.start_run(&project.ctx, "", "Local", Some(free.clone()), false),
        );
        let contract_run = run(
            &format!(
                "run du grand contrat ({})",
                bytes(project.big_contract_bytes)
            ),
            &project,
            &server,
            true,
            |bench| bench.start_contract_run(&project.ctx, &project.big_contract, "Local", false),
        );
        let quiet = run(
            "projet entier, sans sérialiser les événements",
            &project,
            &server,
            false,
            |bench| bench.start_run(&project.ctx, "", "Local", None, false),
        );
        let full = run("**projet entier**", &project, &server, true, |bench| {
            bench.start_run(&project.ctx, "", "Local", None, false)
        });
        println!(
            "\n> {size} : lancement {:.0} ms ; hors lancement, {per_item:.2} ms par ligne pour un dossier, {:.2} ms pour une ligne liée au grand contrat, {:.2} ms pour une ligne libre ; projet entier : {:.2} s.",
            full.launch_ms,
            (contract_run.total_ms - contract_run.launch_ms) / contract_run.items as f64,
            (free_run.total_ms - free_run.launch_ms) / free_run.items as f64,
            full.total_ms / 1000.0,
        );
        let _ = quiet.launch_ms;
        if shape.label == "grand" {
            budget(
                "événements d'un run du projet entier",
                full.event_bytes as f64 / 1e6,
                20.0,
                "Mo",
            );
        }
        totals.push((project.requests, full.total_ms, quiet.total_ms));

        // Coûts unitaires qui expliquent la croissance.
        title(&format!(
            "4 bis. Coûts unitaires d'une ligne de run ({size})"
        ));
        let view = full.view.clone();
        let touch = measure(PASSES, || {
            let mut copy = view.clone();
            copy.touch();
            copy
        });
        row(
            "`RunView::touch` + copie (à chaque changement de ligne)",
            &size,
            &touch.stat,
            None,
            "verrou du run tenu",
        );
        let change = RunChange {
            project_id: project.ctx.id,
            run: view.clone(),
            origin: String::new(),
        };
        let encode = measure(PASSES, || json_len(&change));
        row(
            "sérialisation de `arnod://run`",
            &size,
            &encode.stat,
            Some(encode.out),
            "l'état complet du run, à chaque changement",
        );
        let book = statuses::load(&project.ctx.data_dir, project.ctx.id);
        let save = measure(PASSES, || {
            statuses::save(&project.ctx.data_dir, project.ctx.id, &book).expect("statuts")
        });
        row(
            "`statuses::save` (à chaque ligne terminée)",
            &size,
            &save.stat,
            None,
            &format!("{} ; verrou des statuts tenu", save.io_text()),
        );
        let status_change = StatusesChange {
            project_id: project.ctx.id,
            statuses: book.requests.clone(),
        };
        let encode = measure(PASSES, || json_len(&status_change));
        row(
            "sérialisation de `arnod://statuts`",
            &size,
            &encode.stat,
            Some(encode.out),
            "tous les statuts, à chaque ligne terminée",
        );
        let root = project.ctx.root.clone();
        let contract_read = measure(PASSES, || {
            contract::file::read(&root, &project.big_contract)
                .expect("contrat")
                .operations
                .len()
        });
        row(
            "`contract::file::read` du grand contrat (à chaque ligne liée)",
            &bytes(project.big_contract_bytes),
            &contract_read.stat,
            None,
            &contract_read.io_text(),
        );
        let launch_reads = measure(PASSES, || {
            let paths = tree::requests_in(&root, "").expect("arborescence");
            let nodes = tree::read(&root).expect("arborescence");
            for path in &paths {
                let _ = tree::resolve(&root, path).and_then(|p| file::read(&p));
            }
            nodes.len()
        });
        row(
            "lectures du lancement (`requests_in`, `tree::read`, chaque fichier)",
            &size,
            &launch_reads.stat,
            None,
            &launch_reads.io_text(),
        );
        let last = fs::metadata(
            local::project_dir(&project.ctx.data_dir, project.ctx.id).join("last-run.json"),
        )
        .map_or(0, |m| m.len());
        println!(
            "> last-run.json : {} ; statuses.json : {}.",
            bytes(last as usize),
            bytes(
                fs::metadata(
                    local::project_dir(&project.ctx.data_dir, project.ctx.id).join("statuses.json")
                )
                .map_or(0, |m| m.len()) as usize
            )
        );

        if project.requests >= 2000 {
            // Mémoire gardée : MAX_FINISHED_RUNS runs terminés.
            let before = rss();
            let kept: Vec<RunView> = (0..20).map(|_| view.clone()).collect();
            println!(
                "> 20 runs terminés de {} lignes gardés en mémoire : +{:.1} Mo (un run : {} en JSON).",
                view.items.len(),
                rss() - before,
                bytes(json_len(&view))
            );
            drop(kept);
        }
    }
    if let [
        (small, small_ms, small_quiet),
        (large, large_ms, large_quiet),
    ] = totals.as_slice()
    {
        println!(
            "\n> Croissance du run : {small} → {large} requêtes (× {:.1}) ; durée × {:.1} avec les événements sérialisés, × {:.1} sans.",
            *large as f64 / *small as f64,
            large_ms / small_ms,
            large_quiet / small_quiet,
        );
    }

    // Cookies : une réponse qui en pose un réécrit cookies.json.
    let cookie_server = serve(RUN_BODY.as_bytes().to_vec(), "application/json", true);
    let project = build(&Shape::medium(), &cookie_server.url);
    run_title("4 ter. Run de 500 requêtes dont chaque réponse pose un cookie");
    run(
        "projet entier, `Set-Cookie` à chaque réponse",
        &project,
        &cookie_server,
        true,
        |bench| bench.start_run(&project.ctx, "", "Local", None, false),
    );
}

// ------------------------------------------------------------------------- 5. catalogue

#[test]
#[ignore = "banc de performance"]
fn perf_5_catalogue() {
    println!("\n> Charge de la machine : {}", load_average());
    title("5. Catalogue");
    for count in [50usize, 200] {
        let dir = workdir("catalogue");
        let data_dir = dir.path().join("donnees");
        let state = AppState::load(data_dir.clone(), dir.path().join("projets"));
        let started = Instant::now();
        let mut ids = Vec::new();
        for index in 0..count {
            let created = state
                .create_project(&format!("Projet {index:03}"), None, None)
                .expect("projet");
            ids.push(created.project_id);
        }
        let creation = ms(started) / count as f64;
        let mut workspaces = Vec::new();
        for index in 0..15 {
            let (id, _) = state
                .mutate(|catalog| catalog.create_workspace(&format!("Espace {index:02}")))
                .expect("workspace");
            // Chaque projet dans un ou deux workspaces.
            let members: Vec<Uuid> = ids
                .iter()
                .enumerate()
                .filter(|(n, _)| n % 15 == index || (n + 7) % 15 == index)
                .map(|(_, id)| *id)
                .collect();
            state
                .mutate(|catalog| catalog.add_projects(id, &members))
                .expect("projets ajoutés");
            workspaces.push(id);
        }
        let size = format!("{count} projets, 15 workspaces");
        let emitted = Arc::new(AtomicUsize::new(0));
        let seen = emitted.clone();
        // Comme `windows::catalog_changed` : l'instantané est sérialisé pour toutes les fenêtres.
        state.on_change(Box::new(move |snapshot| {
            seen.store(json_len(snapshot), Ordering::SeqCst);
        }));
        let mut turn = 0usize;
        let sample = measure(31, || {
            turn += 1;
            let target = workspaces[turn % workspaces.len()];
            state
                .mutate(|catalog| catalog.open_workspace(Some(target)))
                .expect("workspace ouvert")
                .1
        });
        let snapshot = json_len(&sample.out);
        row(
            "**`AppState::mutate`** (changer de workspace)",
            &size,
            &sample.stat,
            Some(snapshot),
            &format!(
                "{} ; plus l'événement `arnod://catalogue` de {} ; arnod.json : {}",
                sample.io_text(),
                bytes(emitted.load(Ordering::SeqCst)),
                bytes(fs::metadata(data_dir.join("arnod.json")).map_or(0, |m| m.len()) as usize)
            ),
        );
        let snap = measure(31, || state.snapshot().expect("instantané"));
        row(
            "  dont instantané (`is_dir` par projet)",
            &size,
            &snap.stat,
            Some(json_len(&snap.out)),
            "verrou d'AppState tenu",
        );
        let ctx = measure(2001, || state.project_ctx(ids[count / 2]).expect("ctx"));
        row("`project_ctx`", &size, &ctx.stat, None, "");
        println!(
            "| création d'un projet (moyenne) | {size} | {creation:.2} ms | — | dossier, racine, « Local », arnod.json |"
        );

        // « Tous les projets » : dernier run de chaque projet.
        let paths: Vec<String> = (0..500)
            .map(|n| format!("Dossier {:02}/requete-{n:04}.yml", n % 20))
            .collect();
        let text = serde_json::to_string(&finished_run(&paths)).expect("run");
        for id in &ids {
            let folder = local::project_dir(&data_dir, *id);
            fs::create_dir_all(&folder).expect("dossier local");
            fs::write(folder.join("last-run.json"), &text).expect("last-run.json");
        }
        let last = measure(PASSES, || {
            // Corps de `project_last_runs` (`commands/runs.rs`).
            let snapshot = state.snapshot().expect("instantané");
            snapshot
                .projects
                .iter()
                .filter_map(|project| {
                    let ctx = ProjectCtx {
                        id: project.id,
                        root: project.path.clone().into(),
                        data_dir: data_dir.clone(),
                    };
                    Workbench::last_run(&ctx)
                        .map(|run| arnod_lib::workbench::ProjectLastRun::of(project.id, &run))
                })
                .collect::<Vec<_>>()
        });
        row(
            "`project_last_runs` (un dernier run de 500 lignes par projet)",
            &size,
            &last.stat,
            Some(json_len(&last.out)),
            &last.io_text(),
        );
    }
}

// ------------------------------------------------------------------------- 6. disposition

/// Fenêtres : `zones` zones dans la principale, `tabs` onglets, `detached` fenêtres détachées.
fn layout(
    state: &WindowsState,
    project: Uuid,
    zones: Preset,
    tabs: usize,
    detached: usize,
) -> (Uuid, Vec<Uuid>) {
    let count = zones.zone_count();
    let ((zone, ids), _, _) = state
        .update(|windows: &mut Windows| {
            windows.apply_preset(MAIN, zones)?;
            let zone_ids: Vec<Uuid> = windows
                .get(MAIN)?
                .layout
                .as_ref()
                .expect("zones")
                .zones()
                .iter()
                .map(|zone| zone.id)
                .collect();
            let mut ids = Vec::new();
            for index in 0..tabs {
                let content = Content::Request {
                    project_id: project,
                    path: format!("Domaine {:02}/requete-{index:04}.yml", index % 30),
                };
                ids.push(
                    windows
                        .layout_mut(MAIN)?
                        .open(content, Some(zone_ids[index % count]))?,
                );
            }
            for tab in ids.iter().rev().take(detached) {
                windows.detach_tab(MAIN, *tab, None)?;
            }
            ids.truncate(tabs - detached);
            Ok((zone_ids[0], ids))
        })
        .expect("disposition");
    (zone, ids)
}

#[test]
#[ignore = "banc de performance"]
fn perf_6_disposition() {
    println!("\n> Charge de la machine : {}", load_average());
    title("6. Disposition");
    let project = Uuid::new_v4();
    for (label, preset, tabs, detached) in [
        (
            "10 onglets, 1 zone, 1 fenêtre",
            Preset::One,
            10usize,
            0usize,
        ),
        ("40 onglets, 4 zones, 3 fenêtres", Preset::Four, 40, 2),
    ] {
        let dir = workdir("fenetres");
        let state = WindowsState::load(dir.path().to_path_buf());
        let (zone, ids) = layout(&state, project, preset, tabs, detached);
        // Onglets de la première zone : un sur `zone_count` dans la fenêtre principale.
        let count = preset.zone_count();
        let in_zone: Vec<Uuid> = ids.iter().copied().step_by(count).collect();
        let mut turn = 0usize;
        let sample = measure(101, || {
            turn += 1;
            let tab = in_zone[turn % in_zone.len()];
            let before = state.read(Windows::active_request).expect("lecture");
            let (_, _, file) = state
                .update(|windows| windows.layout_mut(MAIN)?.activate(zone, tab))
                .expect("activation");
            // `windows::apply` : l'instantané part en événement, puis en réponse de la commande.
            let event = json_len(&file);
            let after = state.read(Windows::active_request).expect("lecture");
            (event, before != after)
        });
        row(
            "**`layout_activate`** (copie, windows.json, événement, réponse)",
            label,
            &sample.stat,
            Some(sample.out.0 * 2),
            &format!(
                "{} ; `arnod://fenetres` : {} vers chaque fenêtre, autant en réponse ; windows.json : {}",
                sample.io_text(),
                bytes(sample.out.0),
                bytes(
                    fs::metadata(dir.path().join("windows.json")).map_or(0, |m| m.len()) as usize
                )
            ),
        );
        let mut turn = 0usize;
        let open = measure(101, || {
            turn += 1;
            let content = Content::Request {
                project_id: project,
                path: format!("Autre/apercu-{turn}.yml"),
            };
            let (_, _, file) = state
                .update(|windows| windows.open_selection(content, true))
                .expect("aperçu");
            json_len(&file)
        });
        row(
            "`layout_open_selection` en aperçu (un clic dans la barre latérale)",
            label,
            &open.stat,
            Some(open.out * 2),
            &open.io_text(),
        );
    }
}

// ------------------------------------------------------------------------- 7. opérations groupées

fn flat_project(count: usize) -> (tempfile::TempDir, PathBuf, Vec<String>) {
    let dir = workdir("vrac");
    let state = AppState::load(dir.path().join("donnees"), dir.path().join("projets"));
    let created = state.create_project("Vrac", None, None).expect("projet");
    let root = state.project_ctx(created.project_id).expect("ctx").root;
    tree::create_folder(&root, "", "Lot").expect("dossier");
    tree::create_folder(&root, "", "Archive").expect("dossier");
    let mut paths = Vec::new();
    for index in 0..count {
        let path = format!("Lot/requete-{index:04}.yml");
        file::write(
            &tree::resolve(&root, &path).expect("chemin"),
            &free_request(index, 0),
        )
        .expect("requête");
        paths.push(path);
    }
    (dir, root, paths)
}

/// Ce que fait `tree::request_name_taken` : lire le nom de chaque requête du dossier.
fn name_check(dir: &Path) -> usize {
    fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| file::read_name(&entry.path()).is_some())
            .count()
    })
}

fn bruno_collection(dir: &Path, name: &str, folders: usize, per_folder: usize) -> PathBuf {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bruno/fixtures/boutique");
    let read = |relative: &str| fs::read_to_string(fixtures.join(relative)).expect("fixture Bruno");
    let source = dir.join(name);
    fs::create_dir_all(source.join("environments")).expect("dossier");
    fs::write(
        source.join("bruno.json"),
        format!("{{\"version\":\"1\",\"name\":\"{name}\",\"type\":\"collection\",\"ignore\":[\"node_modules\",\".git\"]}}"),
    )
    .expect("bruno.json");
    fs::write(source.join("collection.bru"), read("collection.bru")).expect("collection.bru");
    for env in ["Local.bru", "Recette.bru"] {
        fs::write(
            source.join("environments").join(env),
            read(&format!("environments/{env}")),
        )
        .expect("environnement");
    }
    let models = [
        (
            "Créer une commande",
            read("Commandes/Créer une commande.bru"),
        ),
        ("Lire une commande", read("Commandes/Lire une commande.bru")),
        (
            "Annuler une commande",
            read("Commandes/Annuler une commande.bru"),
        ),
    ];
    let folder_settings = read("Commandes/folder.bru");
    for folder in 0..folders {
        let folder_name = format!("Commandes {folder:02}");
        let target = source.join(&folder_name);
        fs::create_dir_all(&target).expect("dossier");
        fs::write(
            target.join("folder.bru"),
            folder_settings.replace("name: Commandes", &format!("name: {folder_name}")),
        )
        .expect("folder.bru");
        for index in 0..per_folder {
            let (model, text) = &models[index % models.len()];
            let request_name = format!("{model} {index:03}");
            fs::write(
                target.join(format!("{request_name}.bru")),
                text.replace(&format!("name: {model}"), &format!("name: {request_name}")),
            )
            .expect("requête Bruno");
        }
    }
    source
}

#[test]
#[ignore = "banc de performance"]
fn perf_7_groupes() {
    println!("\n> Charge de la machine : {}", load_average());
    title("7. Opérations groupées : sélection d'un même dossier");
    for count in [50usize, 200] {
        let (_dir, root, paths) = flat_project(count);
        let size = format!("{count} requêtes d'un dossier");
        let norm = measure(PASSES, || batch::normalize(&paths).len());
        row("`batch::normalize`", &size, &norm.stat, None, "");
        let undo: RefCell<Option<Undo>> = RefCell::new(None);
        let revert = || {
            if let Some(undo) = undo.borrow_mut().take() {
                undo.apply().expect("annulation");
            }
        };
        let duplicate = measure_prep(3, revert, || {
            let (done, kept) =
                batch::apply(&root, &paths, &BatchAction::Duplicate).expect("duplication");
            *undo.borrow_mut() = Some(kept);
            done
        });
        row(
            "**`tree_batch` dupliquer**",
            &size,
            &duplicate.stat,
            Some(json_len(&duplicate.out.items)),
            &duplicate.io_text(),
        );
        revert();
        let target = BatchAction::Move {
            target: "Archive".to_owned(),
        };
        let moved = measure_prep(3, revert, || {
            let (done, kept) = batch::apply(&root, &paths, &target).expect("déplacement");
            *undo.borrow_mut() = Some(kept);
            done
        });
        row(
            "**`tree_batch` déplacer** vers un dossier",
            &size,
            &moved.stat,
            Some(json_len(&moved.out.items)),
            &moved.io_text(),
        );
        revert();
        let lot = root.join("Lot");
        let one = measure(PASSES, || name_check(&lot));
        row(
            "  une vérification de nom dans le dossier",
            &size,
            &one.stat,
            None,
            &one.io_text(),
        );
    }

    title("7 bis. Opérations groupées dans le grand projet");
    let project = build(&Shape::large(), "http://127.0.0.1:9");
    let root = project.ctx.root.clone();
    let size = "200 requêtes de 30 dossiers";
    let paths: Vec<String> = project.free.iter().take(200).cloned().collect();
    tree::create_folder(&root, "", "Archive").expect("dossier");
    let watchers = Watchers::new(Arc::new(Silent));
    watchers.watch(project.ctx.id, &root).expect("surveillance");
    let (done, undo) = {
        let started = Instant::now();
        let out = batch::apply(&root, &paths, &BatchAction::Duplicate).expect("duplication");
        println!(
            "| `tree_batch` dupliquer | {size} | {:.1} ms | {} | une passe |",
            ms(started),
            bytes(json_len(&out.0.items))
        );
        out
    };
    let _ = done;
    undo.apply().expect("annulation");
    let (done, undo) = {
        let started = Instant::now();
        let out = batch::apply(
            &root,
            &paths,
            &BatchAction::Move {
                target: "Archive".to_owned(),
            },
        )
        .expect("déplacement");
        println!(
            "| `tree_batch` déplacer vers un dossier | {size} | {:.1} ms | {} | une passe |",
            ms(started),
            bytes(json_len(&out.0.items))
        );
        out
    };

    // Suite de la commande pour chaque élément déplacé : onglets, résultats, statuts.
    let all = tree::requests_in(&root, "").expect("arborescence");
    seed_statuses(&project, &all);
    seed_statuses(
        &project,
        &paths
            .iter()
            .cloned()
            .chain(all.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let wdir = workdir("fenetres");
    let wstate = WindowsState::load(wdir.path().to_path_buf());
    layout(&wstate, project.ctx.id, Preset::Four, 40, 2);
    let started = Instant::now();
    let before = io();
    for (from, to) in &done.items {
        let to = to.as_deref().expect("déplacé");
        let _ = wstate.update(|windows| {
            windows.rename_path(project.ctx.id, from, to);
            Ok(())
        });
        project.bench.rename_status(&project.ctx, from, to);
    }
    let after = io();
    println!(
        "| suite de `tree_batch` : `follow_path` par élément (windows.json, statuses.json) | {size} | {:.1} ms | — | écrit {} en {} write() ; un `arnod://fenetres` et un `arnod://statuts` par élément |",
        ms(started),
        bytes((after.wchar - before.wchar) as usize),
        after.syscw - before.syscw
    );
    undo.apply().expect("annulation");

    // Copier ou déplacer une requête : dialogue, aperçu, transfert.
    let path = project.free[3].clone();
    let targets = measure(PASSES, || {
        transfer::targets(&root, &path, &root, true).expect("destinations")
    });
    row(
        "**`transfer_targets`** (ouverture du dialogue)",
        "2000 req.",
        &targets.stat,
        Some(json_len(&targets.out)),
        &targets.io_text(),
    );
    let target = Target::Folder {
        folder: "Archive".to_owned(),
    };
    let preview = measure(PASSES, || {
        transfer::preview(&root, &path, &root, &target, Mode::Move, None).expect("aperçu")
    });
    row(
        "`transfer_preview`",
        "2000 req.",
        &preview.stat,
        Some(json_len(&preview.out)),
        &preview.io_text(),
    );
    let undo: RefCell<Option<Undo>> = RefCell::new(None);
    let revert = || {
        if let Some(undo) = undo.borrow_mut().take() {
            undo.apply().expect("annulation");
        }
    };
    let apply = measure_prep(PASSES, revert, || {
        let (done, kept) =
            transfer::transfer(&root, &path, &root, &target, Mode::Move).expect("transfert");
        *undo.borrow_mut() = Some(kept);
        done
    });
    row(
        "`transfer_apply` (déplacer une requête)",
        "2000 req.",
        &apply.stat,
        Some(json_len(&apply.out)),
        &apply.io_text(),
    );
    revert();
    let mut there = false;
    let folder = measure(PASSES, || {
        there = !there;
        if there {
            tree::move_to(&root, "Domaine 05", "Archive").expect("déplacement")
        } else {
            tree::move_to(&root, "Archive/Domaine 05", "").expect("retour")
        }
    });
    row(
        "`tree_move` d'un dossier de 47 requêtes (observateur actif)",
        "2000 req.",
        &folder.stat,
        None,
        &folder.io_text(),
    );

    title("7 ter. Import Bruno");
    for (folders, per_folder) in [(25usize, 5usize), (25, 20)] {
        let dir = workdir("bruno");
        let count = folders * per_folder;
        let size = format!("{count} requêtes .bru");
        let state = AppState::load(dir.path().join("donnees"), dir.path().join("projets"));
        let source = bruno_collection(dir.path(), "Collection modele", folders, per_folder);
        let detect = measure(5, || import::detect(&source).expect("format reconnu"));
        row(
            "`import_detect`",
            &size,
            &detect.stat,
            Some(json_len(&detect.out)),
            &format!("{} fichiers ; {}", detect.out.files, detect.io_text()),
        );
        let pass = RefCell::new(0usize);
        let next = RefCell::new(PathBuf::new());
        let run = measure_prep(
            5,
            || {
                *pass.borrow_mut() += 1;
                let name = format!("Collection {count} passe {}", pass.borrow());
                *next.borrow_mut() = bruno_collection(dir.path(), &name, folders, per_folder);
            },
            || {
                import::run(
                    &state,
                    &next.borrow(),
                    &ImportTarget::NewProject {
                        location: None,
                        workspace_id: None,
                    },
                    &import::ImportOptions::default(),
                    &|_: &ProjectCtx, _: &SecretValue| Ok(()),
                )
                .expect("import")
            },
        );
        row(
            "**`import_run`** (nouveau projet)",
            &size,
            &run.stat,
            Some(json_len(&run.out)),
            &format!(
                "{} éléments au rapport ; {}",
                run.out
                    .report
                    .as_ref()
                    .map_or(0, |report| report.items.len()),
                run.io_text()
            ),
        );
    }
}

// ------------------------------------------------------------------------- 8. mémoire

fn outcome_of(body: String) -> SendOutcome {
    let size_bytes = body.len();
    SendOutcome {
        response: HttpResponse {
            status: 200,
            status_text: "OK".to_owned(),
            headers: vec![("content-type".to_owned(), "application/json".to_owned())],
            body,
            truncated: false,
            size_bytes,
            wait_ms: 3,
            receive_ms: 9,
            total_ms: 12,
            timeline: Timeline::default(),
        },
        assertions: Vec::new(),
        passed: true,
        slow: false,
        sent: None,
    }
}

#[test]
#[ignore = "banc de performance"]
fn perf_8_memoire() {
    println!("\n> Charge de la machine : {}", load_average());
    println!("\n## 8. Mémoire\n");
    let project = Uuid::new_v4();
    let body = json_body(20_000);
    let base = rss();
    println!("- Mémoire résidente au départ (corps de 5 Mo en main) : {base:.0} Mo.");
    let mut book = ResultBook::default();
    reset_peak();
    for index in 0..60 {
        let path = format!("Domaine 00/requete-{index:04}.yml");
        book.start(project, &path);
        let sent = Ok(outcome_of(body.clone()));
        book.finish(project, &path, &sent, 1);
        book.keep_request(project, &path, free_request(index, 0));
        if index == 0 || index == 9 || index == 49 || index == 59 {
            println!(
                "- `ResultBook` après {} résultats de {} : +{:.0} Mo (pic +{:.0} Mo).",
                index + 1,
                bytes(body.len()),
                rss() - base,
                peak() - base
            );
        }
    }
    // Renvoyer une requête qui a déjà un résultat de 5 Mo : copies pendant `start` et `finish`.
    let path = "Domaine 00/requete-0059.yml";
    let before = rss();
    reset_peak();
    let started = Instant::now();
    let first = book.start(project, path);
    let sent = Ok(outcome_of(body.clone()));
    let second = book.finish(project, path, &sent, 2);
    let kept = book.keep_request(project, path, free_request(59, 0));
    let elapsed = ms(started);
    println!(
        "- Renvoi d'une requête au résultat de 5 Mo (`start`, `finish`, `keep_request`, sans réseau ni sérialisation) : {elapsed:.1} ms, pic +{:.0} Mo ; trois copies du résultat rendues à l'appelant ({}, {}, {}).",
        peak() - before,
        bytes(json_len(&first)),
        bytes(json_len(&second)),
        bytes(kept.as_ref().map_or(0, json_len)),
    );
    drop((first, second, kept, sent));
    drop(book);
    println!("- Après abandon du carnet : {:.0} Mo.", rss());

    // Statuts et registre de l'observateur d'un grand projet.
    let big = build(&Shape::large(), "http://127.0.0.1:9");
    let before = rss();
    let watchers = Watchers::new(Arc::new(Silent));
    watchers
        .watch(big.ctx.id, &big.ctx.root)
        .expect("surveillance");
    println!(
        "- Registre de l'observateur pour {} requêtes : +{:.1} Mo.",
        big.requests,
        rss() - before
    );
    let paths = tree::requests_in(&big.ctx.root, "").expect("arborescence");
    let before = rss();
    let runs: Vec<RunView> = (0..20).map(|_| finished_run(&paths)).collect();
    println!(
        "- 20 runs terminés de {} lignes (`MAX_FINISHED_RUNS`) : +{:.1} Mo ; un run en JSON : {}.",
        paths.len(),
        rss() - before,
        bytes(json_len(&runs[0]))
    );
    println!("- Dossier du banc : {}.", big.dir.path().display());
}
