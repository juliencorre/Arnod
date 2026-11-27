//! `arnod run` de bout en bout (`docs/plan-v2.md`, lot D) : un vrai projet sur disque, un serveur
//! HTTP local, l'exécutable lancé comme dans un terminal. Sortie (LC1, LC3), rapports (LC2, R8),
//! variables (LC4, R6), erreurs (LC5), codes de sortie (R7) et rien d'écrit hors des rapports
//! (R5).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use arnod_core::environment::{self, Environment, Variable};
use arnod_core::local::{self, LocalData, LocalVar};
use arnod_core::manifest::{Manifest, create_project_folder};
use arnod_core::request::file;
use arnod_core::request::model::{Assertion, Method, Pair};
use arnod_core::tree;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PROJECT_ID: u128 = 0xa1;
const SECRET: &str = "s3cret-du-poste";

struct Fixture {
    server: MockServer,
    project: TempDir,
    data: TempDir,
}

impl Fixture {
    fn root(&self) -> &Path {
        self.project.path()
    }

    /// Lance `arnod-cli` dans le dossier du projet.
    async fn arnod(&self, args: &[&str], vars: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_arnod-cli"));
        command
            .args(args)
            .current_dir(self.root())
            .env("ARNOD_DONNEES", self.data.path())
            .env_remove("ARNOD_VAR_token");
        for (name, value) in vars {
            command.env(name, value);
        }
        tokio::task::spawn_blocking(move || command.output().expect("arnod-cli lancé"))
            .await
            .expect("arnod-cli terminé")
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn pair(name: &str, value: &str) -> Pair {
    serde_json::from_value(json!({ "name": name, "value": value })).unwrap()
}

fn check(target: &str, operator: &str, value: serde_json::Value) -> Assertion {
    let mut assertion = json!({ "id": "", "target": target, "operator": operator });
    if !value.is_null() {
        assertion["value"] = value;
    }
    serde_json::from_value(assertion).unwrap()
}

/// Écrit une requête du projet.
fn request(
    root: &Path,
    folder: &str,
    name: &str,
    method: Method,
    url: &str,
    edit: impl FnOnce(&mut arnod_core::request::model::Request),
) -> String {
    let path = tree::create_request(root, folder, Some(name)).unwrap();
    let full = tree::resolve(root, &path).unwrap();
    let mut request = file::read(&full).unwrap();
    request.method = method;
    request.url = url.to_owned();
    request.assertions = vec![check("status", "equals", json!(200))];
    edit(&mut request);
    file::write(&full, &request).unwrap();
    path
}

/// Projet « boutique-tests » : `commandes` (une requête « smoke », un test « parcours » dont la
/// première étape échoue), `sante` (étiqueté « smoke », il lit la variable locale `token`).
async fn fixture() -> Fixture {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/commandes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": [] })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/commandes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 7 })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sante"))
        .and(header("authorization", format!("Bearer {SECRET}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!("ok {SECRET}")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sante"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let manifest = Manifest::new(Uuid::from_u128(PROJECT_ID), "boutique-tests");
    let _ = create_project_folder(root, &manifest).unwrap();
    for (name, protected) in [("Recette fonctionnelle", false), ("Production", true)] {
        let env = Environment {
            name: name.to_owned(),
            color: "gray".to_owned(),
            protected,
            variables: vec![Variable {
                name: "baseUrl".to_owned(),
                value: server.uri(),
            }],
            locals: vec!["token".to_owned()],
        };
        environment::save(root, &env, None).unwrap();
    }
    tree::create_folder(root, "", "commandes").unwrap();
    request(
        root,
        "commandes",
        "Lister les commandes",
        Method::Get,
        "{{baseUrl}}/commandes",
        |r| {
            r.tags = vec!["smoke".to_owned()];
        },
    );
    let test = tree::create_test_named(root, "commandes", Some("parcours")).unwrap();
    let first = tree::resolve(root, &test.step).unwrap();
    let mut step = file::read(&first).unwrap();
    step.name = "Créer une commande".to_owned();
    step.method = Method::Post;
    step.url = "{{baseUrl}}/commandes".to_owned();
    step.assertions = vec![
        check("status", "equals", json!(201)),
        check("headers.Location", "exists", serde_json::Value::Null),
    ];
    file::write(&first, &step).unwrap();
    let second = tree::add_step(root, &test.folder).unwrap();
    let full = tree::resolve(root, &second).unwrap();
    let mut step = file::read(&full).unwrap();
    step.name = "Lire la commande".to_owned();
    step.url = "{{baseUrl}}/commandes".to_owned();
    file::write(&full, &step).unwrap();

    tree::create_folder(root, "", "sante").unwrap();
    tree::set_folder_tags(root, "sante", &["smoke".to_owned()]).unwrap();
    request(
        root,
        "sante",
        "Santé",
        Method::Get,
        "{{baseUrl}}/sante",
        |r| {
            r.headers = vec![pair("Authorization", "Bearer {{token}}")];
        },
    );

    Fixture {
        server,
        project,
        data: tempfile::tempdir().unwrap(),
    }
}

/// Contenu de chaque fichier sous `dir`, hors `rapports/`.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let relative = path.strip_prefix(base).unwrap().to_path_buf();
            if relative.starts_with("rapports") {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(relative, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[tokio::test(flavor = "multi_thread")]
async fn lc1_lc2_dossier_resume_rapports_et_code_de_sortie() {
    let f = fixture().await;
    let project_before = snapshot(f.root());
    let output = f
        .arnod(
            &[
                "run",
                "commandes",
                "--env",
                "recette fonctionnelle",
                "--rapport",
                "junit,allure,html",
            ],
            &[],
        )
        .await;
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}\n{}", stderr(&output));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        format!(
            "Arnod {} · projet boutique-tests · environnement Recette fonctionnelle",
            env!("CARGO_PKG_VERSION")
        )
    );
    assert_eq!(lines[1], "Dossier commandes · 2 tests · 8 en parallèle");
    let mut cases: Vec<&str> = lines[2..4].to_vec();
    cases.sort_unstable();
    assert!(cases[0].starts_with("  ✓ Lister les commandes "), "{text}");
    assert!(cases[0].ends_with(" ms"), "{text}");
    assert!(cases[1].starts_with("  ✗ parcours "), "{text}");
    assert!(
        cases[1].ends_with("   étape 1 · headers.location — en-tête absent de la réponse"),
        "{text}"
    );
    assert!(lines[4].starts_with("1 réussi · 1 en échec · "), "{text}");
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(
        lines[5],
        format!(
            "Rapports : rapports{sep}junit.xml · rapports{sep}allure-results{sep} · \
             rapports{sep}rapport.html"
        )
    );
    assert_eq!(lines[6], "Code de sortie : 1");
    assert_eq!(lines.len(), 7);

    let reports = f.root().join("rapports");
    let junit = fs::read_to_string(reports.join("junit.xml")).unwrap();
    assert!(junit.contains("<testsuites name=\"boutique-tests\" tests=\"2\" failures=\"1\""));
    assert!(junit.contains("<skipped") || junit.contains("non jouée"));
    let results: Vec<_> = fs::read_dir(reports.join("allure-results"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with("-result.json"))
        .collect();
    assert_eq!(results.len(), 2);
    let html = fs::read_to_string(reports.join("rapport.html")).unwrap();
    assert!(html.contains("boutique-tests · Dossier commandes"));

    // R5 : rien d'écrit hors des rapports, ni dans le projet, ni sur le poste.
    assert_eq!(snapshot(f.root()), project_before);
    assert_eq!(fs::read_dir(f.data.path()).unwrap().count(), 0);

    // Un second run ne garde pas les résultats Allure du premier.
    f.arnod(&["run", "commandes", "-r", "allure"], &[]).await;
    let again = fs::read_dir(reports.join("allure-results"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with("-result.json"))
        .count();
    assert_eq!(again, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn lc1_un_test_lance_seul_montre_ses_etapes() {
    let f = fixture().await;
    let output = f.arnod(&["run", "commandes/parcours"], &[]).await;
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], "Test commandes/parcours · 2 étapes");
    assert!(lines[2].starts_with("  ✗ 1  Créer une commande "), "{text}");
    assert!(
        lines[2].ends_with("   headers.location — en-tête absent de la réponse"),
        "{text}"
    );
    assert!(lines[3].starts_with("  – 2  Lire la commande "), "{text}");
    assert!(lines[3].ends_with("non jouée"), "{text}");
    assert!(lines[4].starts_with("0 réussi · 1 en échec · "), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn lc4_r6_variable_locale_manquante_puis_fournie() {
    let f = fixture().await;
    let output = f.arnod(&["run", "sante"], &[]).await;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output).trim(),
        "Variable locale sans valeur sur ce poste pour l'environnement « Recette fonctionnelle » : \
         « token ». Donnez-la par ARNOD_VAR_token."
    );
    assert!(stdout(&output).is_empty());
    let sent = f.server.received_requests().await.unwrap_or_default();
    assert!(
        sent.is_empty(),
        "rien n'est envoyé avant la fin des vérifications"
    );

    let output = f
        .arnod(
            &["run", "sante", "-r", "junit,html"],
            &[("ARNOD_VAR_token", SECRET)],
        )
        .await;
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}\n{}", stderr(&output));
    // « sante » ne range qu'une requête, sans sous-dossier : c'est un test.
    assert_eq!(text.lines().nth(1), Some("Test sante · 1 étape"));
    assert!(text.contains("  ✓ 1  Santé "), "{text}");
    assert!(text.ends_with("Code de sortie : 0\n"));
    // R8 : le secret n'apparaît ni dans la sortie ni dans les rapports (réponse masquée).
    for report in ["junit.xml", "rapport.html"] {
        let content = fs::read_to_string(f.root().join("rapports").join(report)).unwrap();
        assert!(!content.contains(SECRET), "{report}");
    }
    assert!(!text.contains(SECRET));
}

#[tokio::test(flavor = "multi_thread")]
async fn r6_valeur_locale_du_poste_lue_sans_etre_reecrite() {
    let f = fixture().await;
    let data = LocalData {
        locals: BTreeMap::from([(
            "Recette fonctionnelle".to_owned(),
            vec![LocalVar {
                name: "token".to_owned(),
                value: Some(SECRET.to_owned()),
                secret: false,
            }],
        )]),
        ..LocalData::default()
    };
    local::save(f.data.path(), Uuid::from_u128(PROJECT_ID), &data).unwrap();
    let before = snapshot(f.data.path());
    let output = f.arnod(&["run", "sante"], &[]).await;
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        snapshot(f.data.path()),
        before,
        "rien n'est écrit sur le poste (R5)"
    );
    // ARNOD_VAR_ prime sur la valeur du poste : un mauvais jeton échoue.
    let output = f
        .arnod(&["run", "sante"], &[("ARNOD_VAR_token", "faux")])
        .await;
    assert_eq!(output.status.code(), Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn lc3_etiquette_heritee_d_un_dossier() {
    let f = fixture().await;
    let output = f
        .arnod(
            &["run", "--etiquette", "smoke"],
            &[("ARNOD_VAR_token", SECRET)],
        )
        .await;
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}\n{}", stderr(&output));
    assert_eq!(
        text.lines().nth(1),
        Some("Étiquette smoke · 2 tests dans 2 dossiers · 8 en parallèle")
    );
    assert!(
        text.contains("  ✓ Lister les commandes ") && text.contains("  ✓ sante "),
        "{text}"
    );
    let output = f
        .arnod(
            &["run", "-t", "smoke", "-p", "1"],
            &[("ARNOD_VAR_token", SECRET)],
        )
        .await;
    assert!(stdout(&output).contains("· 1 en parallèle"));
}

#[tokio::test(flavor = "multi_thread")]
async fn lc5_r7_erreurs_de_commande() {
    let f = fixture().await;
    let output = f.arnod(&["run", "commande"], &[]).await;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output).trim(),
        "La cible « commande » n'existe pas dans ce projet. Vouliez-vous dire « commandes » ?"
    );
    let output = f
        .arnod(&["run", "commandes", "--env", "Recete fonctionelle"], &[])
        .await;
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Vouliez-vous dire « Recette fonctionnelle » ?"));
    let output = f.arnod(&["run", "commandes", "--env", "Qualif"], &[]).await;
    assert_eq!(
        stderr(&output).trim(),
        "L'environnement « Qualif » n'existe pas. Environnements : « Recette fonctionnelle », \
         « Production »."
    );
    let output = f
        .arnod(&["run", "commandes", "--env", "Production"], &[])
        .await;
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).ends_with("Ajoutez --confirmer pour envoyer quand même.\n"));
    let output = f
        .arnod(
            &["run", "commandes", "--env", "Production", "--confirmer"],
            &[],
        )
        .await;
    assert_eq!(output.status.code(), Some(1));
    let output = f.arnod(&["--version"], &[]).await;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        format!("arnod {}\n", env!("CARGO_PKG_VERSION"))
    );
    let output = f.arnod(&["run", "--help"], &[]).await;
    assert!(stdout(&output).starts_with("arnod run - lance"));
    let output = f.arnod(&["run", "--rapprt", "junit"], &[]).await;
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Vouliez-vous dire --rapport ?"));
    // Hors d'un projet.
    let elsewhere = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_arnod-cli"))
        .args(["run"])
        .current_dir(elsewhere.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).starts_with("Aucun projet Arnod dans "));
    // --projet donne le dossier du projet depuis ailleurs.
    let output = Command::new(env!("CARGO_BIN_EXE_arnod-cli"))
        .args(["run", "commandes/parcours", "--projet"])
        .arg(f.root())
        .current_dir(elsewhere.path())
        .env("ARNOD_DONNEES", f.data.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
}

/// V4 (`docs/plan-v4.md`, critères d'acceptation) : un test en schéma (une condition, une étape
/// script, « Fin ✓ ») et une requête à jeu de données se jouent hors de l'application, sans rien
/// écrire dans le projet (R5).
#[tokio::test(flavor = "multi_thread")]
async fn v4_schema_et_jeu_de_donnees() {
    use arnod_core::dataset::{self, DataTable};
    use arnod_core::flow::{Action, Check, END, Flow, OnFailure, Step, store};
    use arnod_core::request::model::{Operator, Scalar};

    let f = fixture().await;
    let root = f.root();
    // Le test « parcours » : la commande créée (201) mène au script, puis à la fin ; la lecture
    // n'est pas empruntée.
    let mut condition = Step {
        id: "c".to_owned(),
        name: "Commande créée ?".to_owned(),
        action: Action::Condition {
            check: Check {
                target: "status".to_owned(),
                operator: Operator::Equals,
                value: Some(Scalar::Int(201)),
            },
            otherwise: Some(END.to_owned()),
        },
        next: Some("js".to_owned()),
        on_failure: None,
    };
    condition.next = Some("js".to_owned());
    let script = Step {
        id: "js".to_owned(),
        name: "Contrôler l'identifiant".to_owned(),
        action: Action::Script {
            code: "assert(response.body.id === 7, 'identifiant 7');".to_owned(),
        },
        next: Some(END.to_owned()),
        on_failure: None,
    };
    let first = tree::resolve(root, "commandes/parcours").unwrap();
    let mut created = file::read(&first.join("etape-1.yml")).unwrap();
    created.assertions = vec![check("status", "equals", json!(201))];
    file::write(&first.join("etape-1.yml"), &created).unwrap();
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            Step::request("etape-1.yml"),
            condition,
            script,
            Step::request("etape-2.yml"),
        ],
    };
    store::save_flow(root, "commandes/parcours", Some(&flow)).unwrap();
    // « Lister les commandes » joué deux fois : un cas par ligne.
    dataset::save(
        root,
        "commandes/lister-les-commandes.yml",
        &DataTable {
            columns: vec!["page".to_owned()],
            rows: vec![vec!["1".to_owned()], vec!["2".to_owned()]],
        },
    )
    .unwrap();
    let before = snapshot(root);
    let output = f
        .arnod(&["run", "commandes", "--env", "Recette fonctionnelle"], &[])
        .await;
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}\n{}", stderr(&output));
    assert!(text.contains("✓ parcours"), "{text}");
    assert!(text.contains("✓ Lister les commandes"), "{text}");
    // Le test lancé seul montre ses étapes : la condition, le script, la lecture non jouée.
    let alone = f
        .arnod(
            &[
                "run",
                "commandes/parcours",
                "--env",
                "Recette fonctionnelle",
            ],
            &[],
        )
        .await;
    let steps = stdout(&alone);
    assert_eq!(alone.status.code(), Some(0), "{steps}");
    assert!(steps.contains("Commande créée ?"), "{steps}");
    assert!(steps.contains("Contrôler l'identifiant"), "{steps}");
    assert_eq!(
        snapshot(root),
        before,
        "rien n'est écrit dans le projet (R5)"
    );
    let received = f.server.received_requests().await.unwrap_or_default();
    let listed = received
        .iter()
        .filter(|r| r.method.as_str() == "GET" && r.url.path() == "/commandes")
        .count();
    assert_eq!(
        listed, 2,
        "une requête par cas, la lecture jamais empruntée"
    );
}
