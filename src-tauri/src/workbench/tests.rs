//! Tests de l'établi : envoi, environnements protégés, secrets et runs.

use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::environment::Variable;
use crate::request::model::Request;
use crate::secrets::MemorySecretStore;

struct Fixture {
    _dir: tempfile::TempDir,
    ctx: ProjectCtx,
    bench: Arc<Workbench>,
    store: Arc<MemorySecretStore>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projet");
    std::fs::create_dir_all(&root).unwrap();
    environment::init(&root).unwrap();
    let store = Arc::new(MemorySecretStore::default());
    Fixture {
        ctx: ProjectCtx {
            id: Uuid::new_v4(),
            root,
            data_dir: dir.path().join("data"),
        },
        bench: Arc::new(Workbench::new(store.clone())),
        store,
        _dir: dir,
    }
}

fn set_base_url(f: &Fixture, url: &str) {
    let mut env = environment::list(&f.ctx.root).remove(0);
    env.variables = vec![Variable {
        name: "baseUrl".into(),
        value: url.into(),
    }];
    f.bench
        .save_environment(&f.ctx, &env, Some("Local"))
        .unwrap();
}

fn add_request(f: &Fixture, folder: &str, name: &str, url: &str, method: Method) -> String {
    let path = tree::create_request(&f.ctx.root, folder, Some(name)).unwrap();
    let full = tree::resolve(&f.ctx.root, &path).unwrap();
    let mut request = file::read(&full).unwrap();
    request.url = url.to_owned();
    request.method = method;
    file::write(&full, &request).unwrap();
    path
}

async fn wait(bench: &Workbench, id: Uuid) -> RunView {
    for _ in 0..200 {
        let view = bench.run_status(id).unwrap();
        if view.finished {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("run non terminé");
}

#[tokio::test]
async fn envoi_evalue_les_verifications_par_defaut() {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    let mut request = Request::new("ok");
    request.url = "{{baseUrl}}/ok".into();
    let outcome = f
        .bench
        .send(&f.ctx, Uuid::new_v4(), "Local", request, false)
        .await
        .unwrap();
    assert_eq!(outcome.response.status, 200);
    assert!(outcome.passed);
}

#[tokio::test]
async fn variable_inconnue_bloque_l_envoi() {
    let f = fixture();
    let mut request = Request::new("x");
    request.url = "{{absente}}/a".into();
    let error = f
        .bench
        .send(&f.ctx, Uuid::new_v4(), "Local", request, false)
        .await
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::UnknownVariable);
}

#[tokio::test]
async fn environnement_protege_demande_confirmation_pour_les_ecritures() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    let mut env = environment::list(&f.ctx.root).remove(0);
    env.protected = true;
    f.bench
        .save_environment(&f.ctx, &env, Some("Local"))
        .unwrap();
    let mut request = Request::new("creer");
    request.method = Method::Post;
    request.url = "{{baseUrl}}/x".into();
    let error = f
        .bench
        .send(&f.ctx, Uuid::new_v4(), "Local", request.clone(), false)
        .await
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::ProtectedEnvironment);
    let outcome = f
        .bench
        .send(&f.ctx, Uuid::new_v4(), "Local", request, true)
        .await
        .unwrap();
    assert_eq!(outcome.response.status, 201);
}

#[tokio::test]
async fn un_secret_local_est_envoye_mais_masque_dans_la_reponse() {
    let server = MockServer::start().await;
    Mock::given(path("/eco"))
        .respond_with(ResponseTemplate::new(200).set_body_string("jeton=abc123"))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    let mut env = environment::list(&f.ctx.root).remove(0);
    env.locals = vec!["token".into()];
    f.bench
        .save_environment(&f.ctx, &env, Some("Local"))
        .unwrap();
    f.bench
        .set_local(&f.ctx, "Local", "token", "abc123", true)
        .unwrap();

    let view = f.bench.environments(&f.ctx);
    let var = &view.environments[0].local_vars[0];
    assert!(var.secret && var.has_value && var.value.is_none());
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("abc123"));
    let local =
        std::fs::read_to_string(local::project_dir(&f.ctx.data_dir, f.ctx.id).join("local.json"))
            .unwrap();
    assert!(!local.contains("abc123"));
    assert_eq!(
        f.store
            .get(&secret_key(&f.ctx, "Local", "token"))
            .unwrap()
            .as_deref(),
        Some("abc123")
    );

    let mut request = Request::new("eco");
    request.url = "{{baseUrl}}/eco".into();
    let outcome = f
        .bench
        .send(&f.ctx, Uuid::new_v4(), "Local", request, false)
        .await
        .unwrap();
    assert_eq!(outcome.response.body, "jeton=••••");
}

#[tokio::test]
async fn un_run_execute_le_dossier_et_garde_le_dernier() {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(path("/ko"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "Dossier").unwrap();
    add_request(&f, "Dossier", "Bon", "{{baseUrl}}/ok", Method::Get);
    let bad = add_request(&f, "Dossier", "Mauvais", "{{baseUrl}}/ko", Method::Get);
    let full = tree::resolve(&f.ctx.root, &bad).unwrap();
    let mut request = file::read(&full).unwrap();
    request.assertions.push(Assertion {
        id: "statut".into(),
        target: "status".into(),
        operator: crate::request::model::Operator::Equals,
        value: Some(crate::request::model::Scalar::Int(200)),
        origin: crate::request::model::Origin::Manual,
    });
    file::write(&full, &request).unwrap();
    add_request(&f, "Dossier", "Variable", "{{inconnue}}/x", Method::Get);

    let started = f
        .bench
        .start_run(&f.ctx, "Dossier", "Local", None, false)
        .unwrap();
    let view = wait(&f.bench, started.id).await;
    let status = |name: &str| view.items.iter().find(|i| i.name == name).unwrap().status;
    assert_eq!(status("Bon"), ItemStatus::Passed);
    assert_eq!(status("Mauvais"), ItemStatus::Failed);
    assert_eq!(status("Variable"), ItemStatus::Error);
    assert_eq!(Workbench::last_run(&f.ctx).unwrap().id, started.id);

    let retry = f.bench.retry_failures(&f.ctx, started.id, false).unwrap();
    assert_eq!(retry.items.len(), 2);
    assert!(retry.items.iter().all(|i| i.name != "Bon"));
    wait(&f.bench, retry.id).await;
}

#[tokio::test]
async fn annuler_un_run_arrete_les_requetes_en_cours() {
    let server = MockServer::start().await;
    Mock::given(path("/lent"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "D").unwrap();
    add_request(&f, "D", "Lent", "{{baseUrl}}/lent", Method::Get);
    let started = f
        .bench
        .start_run(&f.ctx, "D", "Local", None, false)
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    f.bench.cancel_run(started.id).unwrap();
    let view = wait(&f.bench, started.id).await;
    assert!(view.cancelled);
    assert_eq!(view.items[0].status, ItemStatus::Cancelled);
}

#[tokio::test]
async fn dossier_vide_donne_un_run_termine() {
    let f = fixture();
    tree::create_folder(&f.ctx.root, "", "Vide").unwrap();
    let view = f
        .bench
        .start_run(&f.ctx, "Vide", "Local", None, false)
        .unwrap();
    assert!(view.finished && view.items.is_empty());
}

#[test]
fn le_dernier_environnement_utilise_est_retenu() {
    let f = fixture();
    let mut recette = environment::list(&f.ctx.root).remove(0);
    recette.name = "Recette".into();
    f.bench.save_environment(&f.ctx, &recette, None).unwrap();
    assert_eq!(f.bench.environments(&f.ctx).active, "Local");
    f.bench.select_environment(&f.ctx, "Recette").unwrap();
    assert_eq!(f.bench.environments(&f.ctx).active, "Recette");
    f.bench.delete_environment(&f.ctx, "Recette").unwrap();
    assert_eq!(f.bench.environments(&f.ctx).active, "Local");
}
