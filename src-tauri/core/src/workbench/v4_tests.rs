//! Runs de la V4 joués contre un serveur simulé (`docs/plan-v4.md`, R5, R10, R12, R13, R15, R17).

use std::collections::BTreeMap;
use std::time::Duration;

use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::environment::Variable;
use crate::flow::store as flows;
use crate::flow::{Action, Check, END, FAIL, Flow, FolderHooks, OnFailure, Step};
use crate::opencollection::ROOT_FILE;
use crate::request::file;
use crate::request::model::{Operator, Scalar};
use crate::secrets::MemorySecretStore;

struct Fixture {
    _dir: tempfile::TempDir,
    ctx: ProjectCtx,
    bench: Arc<Workbench>,
}

async fn fixture(server: &MockServer) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projet");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Boutique\n",
    )
    .unwrap();
    environment::init(&root).unwrap();
    let store = Arc::new(MemorySecretStore::default());
    let f = Fixture {
        ctx: ProjectCtx {
            id: Uuid::new_v4(),
            root,
            data_dir: dir.path().join("data"),
        },
        bench: Arc::new(Workbench::new(store)),
        _dir: dir,
    };
    let mut env = environment::list(&f.ctx.root).remove(0);
    env.variables = vec![Variable {
        name: "baseUrl".into(),
        value: server.uri(),
    }];
    f.bench
        .save_environment(&f.ctx, &env, Some("Local"))
        .unwrap();
    f
}

/// Requête `name` de `folder` vers `url` ; `edit` la complète.
fn request(
    f: &Fixture,
    folder: &str,
    name: &str,
    url: &str,
    edit: impl FnOnce(&mut Request),
) -> String {
    let path = tree::create_request(&f.ctx.root, folder, Some(name)).unwrap();
    let full = tree::resolve(&f.ctx.root, &path).unwrap();
    let mut request = file::read(&full).unwrap();
    request.url = url.to_owned();
    request.assertions.clear();
    edit(&mut request);
    file::write(&full, &request).unwrap();
    path
}

fn status_is(code: i64) -> crate::request::model::Assertion {
    crate::request::model::Assertion {
        id: "s".into(),
        target: "status".into(),
        operator: Operator::Equals,
        value: Some(Scalar::Int(code)),
        origin: crate::request::model::Origin::Manual,
        enabled: true,
    }
}

async fn run(f: &Fixture, folder: &str) -> RunView {
    let started = f
        .bench
        .start_run(&f.ctx, folder, "Local", None, false)
        .unwrap();
    for _ in 0..300 {
        let view = f.bench.run_status(started.id).unwrap();
        if view.finished {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("run non terminé");
}

fn line<'a>(view: &'a RunView, name: &str) -> &'a RunItem {
    view.items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("ligne « {name} » absente : {:?}", view.items))
}

fn file_of(path: &str) -> String {
    path.rsplit('/').next().unwrap().to_owned()
}

fn step(id: &str, name: &str, action: Action) -> Step {
    Step {
        id: id.into(),
        name: name.into(),
        action,
        next: None,
        on_failure: None,
    }
}

#[tokio::test]
async fn r5_un_test_suit_son_schema_et_ses_valeurs() {
    let server = MockServer::start().await;
    Mock::given(path("/customers"))
        .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"id":"cus_31"}"#))
        .mount(&server)
        .await;
    Mock::given(path("/orders"))
        .and(query_param("client", "cus_31"))
        .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"id":"ord_8","total":49.83}"#))
        .mount(&server)
        .await;
    Mock::given(path("/refus"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    tree::create_folder(&f.ctx.root, "", "parcours").unwrap();
    let client = request(
        &f,
        "parcours",
        "Créer un client",
        "{{baseUrl}}/customers",
        |r| {
            r.extractions.push(crate::request::model::Extraction {
                name: "customerId".into(),
                source: "body.id".into(),
                enabled: true,
                scope: Default::default(),
            });
        },
    );
    let order = request(
        &f,
        "parcours",
        "Créer une commande",
        "{{baseUrl}}/orders?client={{customerId}}",
        |r| {
            r.extractions.push(crate::request::model::Extraction {
                name: "order.total".into(),
                source: "body.total".into(),
                enabled: true,
                scope: Default::default(),
            });
        },
    );
    let refus = request(&f, "parcours", "Refus attendu", "{{baseUrl}}/refus", |_| {});
    let mut condition = step(
        "c",
        "Commande acceptée ?",
        Action::Condition {
            check: Check {
                target: "status".into(),
                operator: Operator::Equals,
                value: Some(Scalar::Int(201)),
            },
            otherwise: Some("r".into()),
        },
    );
    condition.next = Some("js".into());
    let mut script = step(
        "js",
        "Contrôler le montant TTC",
        Action::Script {
            code: "const ttc = Math.round(vars.order.total * 1.2 * 100) / 100; assert(ttc === 59.8, 'TTC ' + ttc); vars.set('order.ttc', ttc);".into(),
        },
    );
    script.next = Some(END.into());
    let mut refuse = Step::request(&file_of(&refus));
    refuse.id = "r".into();
    refuse.next = Some(FAIL.into());
    let flow = Flow {
        on_failure: OnFailure::Stop,
        steps: vec![
            Step::request(&file_of(&client)),
            Step::request(&file_of(&order)),
            condition,
            script,
            refuse,
        ],
    };
    flows::save_flow(&f.ctx.root, "parcours", Some(&flow)).unwrap();
    let view = run(&f, "parcours").await;
    assert_eq!(line(&view, "Créer un client").status, ItemStatus::Passed);
    assert_eq!(line(&view, "Créer une commande").status, ItemStatus::Passed);
    let cond = line(&view, "Commande acceptée ?");
    assert_eq!(cond.flow.branch, Some(true));
    assert_eq!(cond.flow.kind, StepKind::Condition);
    let js = line(&view, "Contrôler le montant TTC");
    assert_eq!(js.status, ItemStatus::Passed, "{:?}", js.error);
    assert!(js.extracted.iter().any(|e| e.name == "order.ttc"));
    // La branche non empruntée n'est pas exécutée (R8) ; le test réussit.
    assert_eq!(line(&view, "Refus attendu").status, ItemStatus::Skipped);
    assert_eq!(view.tests.len(), 1);
    assert_eq!(view.tests[0].status, ItemStatus::Passed);
    // Les valeurs passées s'affichent sur la ligne qui les extrait.
    let extracted = &line(&view, "Créer un client").extracted;
    assert_eq!(extracted[0].value.as_deref(), Some("cus_31"));
}

#[tokio::test]
async fn r15_proprietes_des_dossiers_et_r10_scripts_bruno() {
    let server = MockServer::start().await;
    Mock::given(path("/pay"))
        .and(query_param("devise", "EUR"))
        .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"ref":"PAY-1"}"#))
        .mount(&server)
        .await;
    Mock::given(path("/check"))
        .and(query_param("ref", "PAY-1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    tree::create_folder(&f.ctx.root, "", "paiements").unwrap();
    tree::set_folder_variables(
        &f.ctx.root,
        "paiements",
        &[("devise".into(), "EUR".into(), String::new())],
    )
    .unwrap();
    let pay = request(
        &f,
        "paiements",
        "Payer",
        "{{baseUrl}}/pay?devise={{devise}}",
        |r| {
            r.assertions.push(status_is(201));
        },
    );
    // Script après la réponse et test, comme l'import Bruno les garde (`runtime.scripts`).
    let full = tree::resolve(&f.ctx.root, &pay).unwrap();
    let mut value = crate::opencollection::yaml::read(&full).unwrap();
    value["runtime"]["scripts"] = serde_json::json!([
        { "type": "after-response", "code": "bru.setVar('ref', res.getBody().ref);" },
        { "type": "tests", "code": "test('créé', function () { expect(res.getStatus()).to.equal(201); });" }
    ]);
    crate::opencollection::yaml::write(&full, &value).unwrap();
    request(
        &f,
        "paiements",
        "Vérifier",
        "{{baseUrl}}/check?ref={{ref}}",
        |r| {
            r.assertions.push(status_is(200));
        },
    );
    let view = run(&f, "paiements").await;
    let paid = line(&view, "Payer");
    assert_eq!(
        paid.status,
        ItemStatus::Passed,
        "{:?} {:?}",
        paid.error,
        paid.assertions
    );
    assert!(
        paid.assertions
            .iter()
            .any(|a| a.target == "script" && a.passed)
    );
    assert_eq!(line(&view, "Vérifier").status, ItemStatus::Passed);
}

#[tokio::test]
async fn r12_une_requete_jouee_une_fois_par_cas() {
    let server = MockServer::start().await;
    for status in ["pending", "paid"] {
        Mock::given(path("/orders"))
            .and(query_param("status", status))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
    }
    Mock::given(path("/orders"))
        .and(query_param("status", "cancelled"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    let lister = request(
        &f,
        "",
        "Lister les commandes",
        "{{baseUrl}}/orders?status={{statut}}",
        |r| {
            r.assertions.push(crate::request::model::Assertion {
                id: "s".into(),
                target: "status".into(),
                operator: Operator::Equals,
                value: Some(Scalar::Text("{{attendu}}".into())),
                origin: crate::request::model::Origin::Manual,
                enabled: true,
            });
        },
    );
    let table = crate::dataset::DataTable {
        columns: vec!["statut".into(), "attendu".into()],
        rows: vec![
            vec!["pending".into(), "200".into()],
            vec!["paid".into(), "200".into()],
            vec!["cancelled".into(), "200".into()],
        ],
    };
    crate::dataset::save(&f.ctx.root, &lister, &table).unwrap();
    let view = run(&f, "").await;
    let item = line(&view, "Lister les commandes");
    assert_eq!(item.flow.cases.len(), 3);
    assert_eq!(item.flow.cases[0].status, ItemStatus::Passed);
    assert_eq!(item.flow.cases[2].status, ItemStatus::Failed);
    assert_eq!(item.status, ItemStatus::Failed);
    assert_eq!(item.flow.detail.as_deref(), Some("3 cas"));
    // Envoi depuis l'éditeur : un résultat par cas.
    let draft = file::read(&tree::resolve(&f.ctx.root, &lister).unwrap()).unwrap();
    let outcome = f
        .bench
        .send_cases(&f.ctx, Uuid::new_v4(), "Local", draft, false, None)
        .await
        .unwrap();
    assert_eq!(outcome.cases.len(), 3);
    assert!(!outcome.passed);
    assert_eq!(
        outcome.cases[2].outcome.as_ref().unwrap().response.status,
        400
    );
}

#[tokio::test]
async fn r13_avant_en_echec_et_apres_qui_nettoie() {
    let server = MockServer::start().await;
    Mock::given(path("/clients"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    tree::create_folder(&f.ctx.root, "", "v2").unwrap();
    tree::create_folder(&f.ctx.root, "v2", "cas-erreurs").unwrap();
    tree::create_folder(&f.ctx.root, "v2/cas-erreurs", "e404").unwrap();
    request(&f, "v2", "Lister", "{{baseUrl}}/ok", |_| {});
    request(
        &f,
        "v2/cas-erreurs/e404",
        "Erreur 404",
        "{{baseUrl}}/ok",
        |_| {},
    );
    let avant = request(
        &f,
        "v2",
        "Créer un client de test",
        "{{baseUrl}}/clients",
        |r| {
            r.assertions.push(status_is(201));
        },
    );
    let apres = request(
        &f,
        "v2",
        "Supprimer le client de test",
        "{{baseUrl}}/ok",
        |_| {},
    );
    let hooks = FolderHooks {
        before_all: vec![Step::request(&file_of(&avant))],
        after_all: vec![Step::request(&file_of(&apres))],
        ..FolderHooks::default()
    };
    flows::save_hooks(&f.ctx.root, "v2", &hooks).unwrap();
    // Les requêtes d'un Avant / Après n'apparaissent pas dans l'arborescence (R14).
    let nodes = tree::read(&f.ctx.root).unwrap();
    assert!(crate::template::find(&nodes, &avant).is_none());
    let view = run(&f, "v2").await;
    let before = line(&view, "Créer un client de test");
    assert_eq!(before.flow.hook, Some(HookPhase::BeforeAll));
    assert_eq!(before.status, ItemStatus::Failed);
    let direct = line(&view, "Lister");
    assert_eq!(direct.status, ItemStatus::Failed);
    assert_eq!(direct.error.as_deref(), Some("Avant en échec"));
    // Le sous-dossier se joue avec les siens ; « Après » nettoie quand même.
    assert_eq!(line(&view, "Erreur 404").status, ItemStatus::Passed);
    assert_eq!(
        line(&view, "Supprimer le client de test").status,
        ItemStatus::Passed
    );
}

#[tokio::test]
async fn r17_une_utilisation_joue_le_template_avec_ses_valeurs() {
    let server = MockServer::start().await;
    Mock::given(path("/payments"))
        .and(query_param("montant", "59.80"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    tree::create_folder(&f.ctx.root, "", "paiements").unwrap();
    request(
        &f,
        "paiements",
        "Créer un paiement",
        "{{baseUrl}}/payments?montant={{montant}}",
        |r| {
            r.assertions.push(status_is(201));
        },
    );
    let params = [crate::template::TemplateParam {
        name: "montant".into(),
        default: "129.90".into(),
        description: String::new(),
    }];
    let view =
        crate::template::create(&f.ctx.root, "paiements", "Payer", "", &params, true).unwrap();
    crate::template::set_values(
        &f.ctx.root,
        "paiements",
        &BTreeMap::from([("montant".to_owned(), "59.80".to_owned())]),
    )
    .unwrap();
    let run_view = run(&f, "").await;
    let instance = line(&run_view, "paiements");
    assert_eq!(instance.flow.kind, StepKind::Template);
    assert_eq!(instance.status, ItemStatus::Passed, "{:?}", instance.error);
    assert_eq!(instance.flow.detail.as_deref(), Some("1/1"));
    let _ = view;
}

#[tokio::test]
async fn r5_ignorer_l_echec_qui_a_arrete_le_test_rend_sa_fin() {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let f = fixture(&server).await;
    tree::create_folder(&f.ctx.root, "", "T").unwrap();
    // Un test joué comme la liste de ses requêtes : la première échoue, la seconde n'est pas jouée.
    let first = request(&f, "T", "Créer", "{{baseUrl}}/ok", |r| {
        r.seq = Some(1.0);
        r.assertions.push(status_is(201));
    });
    request(&f, "T", "Lire", "{{baseUrl}}/ok", |r| r.seq = Some(2.0));
    let done = run(&f, "T").await;
    assert_eq!(done.tests[0].status, ItemStatus::Failed);
    assert_eq!(done.tests[0].failed_step, Some(1));
    assert_eq!(done.ends.len(), 1);
    // Les identifiants des assertions ne sont pas écrits : celui du run vient de la lecture.
    let check = line(&done, "Créer")
        .assertions
        .iter()
        .find(|a| !a.passed)
        .unwrap()
        .id
        .clone();

    let after = f
        .bench
        .ignore_in_run(&f.ctx, done.id, &first, &check)
        .unwrap();

    // La fin en échec tenait à cette étape : le test se lit sur ses lignes, la suivante non jouée.
    assert!(after.ends.is_empty());
    assert_eq!(line(&after, "Créer").status, ItemStatus::Passed);
    assert_eq!(line(&after, "Lire").status, ItemStatus::Skipped);
    assert_eq!(after.tests[0].failed_step, None);
    assert_ne!(after.tests[0].status, ItemStatus::Failed);
    assert_eq!(after.tests[0].passed, 1);
}
