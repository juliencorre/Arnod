//! Tests de l'établi : envoi, environnements protégés, secrets et runs.

use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::environment::Variable;
use crate::request::model::{Assertion, Request};
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
        enabled: true,
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

/// Une annulation qui tombe entre le démarrage d'une requête et son envoi
/// (lecture du fichier, héritage) l'annule aussi : aucune ne part après elle.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn annuler_un_run_n_envoie_plus_aucune_requete() {
    let server = MockServer::start().await;
    Mock::given(path("/lent"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(400)))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "D").unwrap();
    for n in 0..24 {
        add_request(
            &f,
            "D",
            &format!("Lent {n}"),
            "{{baseUrl}}/lent",
            Method::Get,
        );
    }
    // Annulations à des instants variés du démarrage : de 0 à 6 ms.
    for attempt in 0..30u64 {
        let started = f
            .bench
            .start_run(&f.ctx, "D", "Local", None, false)
            .unwrap();
        tokio::time::sleep(Duration::from_micros(attempt * 200)).await;
        f.bench.cancel_run(started.id).unwrap();
        let view = wait(&f.bench, started.id).await;
        let sent: Vec<&str> = view
            .items
            .iter()
            .filter(|i| i.status != ItemStatus::Cancelled)
            .map(|i| i.name.as_str())
            .collect();
        assert!(
            sent.is_empty(),
            "essai {attempt} : envoyées malgré l'annulation : {sent:?}"
        );
    }
}

#[tokio::test]
async fn les_runs_termines_anciens_sont_oublies() {
    let f = fixture();
    tree::create_folder(&f.ctx.root, "", "Vide").unwrap();
    let mut ids = Vec::new();
    for _ in 0..MAX_FINISHED_RUNS + 15 {
        let view = f
            .bench
            .start_run(&f.ctx, "Vide", "Local", None, false)
            .unwrap();
        ids.push(view.id);
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(f.bench.runs.lock().unwrap().len(), MAX_FINISHED_RUNS);
    // Les plus récents restent lisibles, le plus ancien est oublié.
    assert!(f.bench.run_status(*ids.last().unwrap()).is_ok());
    assert_eq!(
        f.bench.run_status(ids[0]).unwrap_err().kind,
        CommandErrorKind::NotFound
    );
}

#[tokio::test]
async fn le_dernier_run_se_relance_apres_un_redemarrage() {
    let f = fixture();
    tree::create_folder(&f.ctx.root, "", "D").unwrap();
    // Variable inconnue : la requête finit en erreur, donc à relancer.
    add_request(&f, "D", "Mauvais", "{{absente}}/ko", Method::Get);
    let started = f
        .bench
        .start_run(&f.ctx, "D", "Local", None, false)
        .unwrap();
    wait(&f.bench, started.id).await;

    // Nouvel établi, comme après un redémarrage : le run n'est plus en mémoire.
    let restarted = Arc::new(Workbench::new(f.store.clone()));
    let retry = restarted.retry_failures(&f.ctx, started.id, false).unwrap();
    assert_eq!(retry.items.len(), 1);
    wait(&restarted, retry.id).await;
    // Un identifiant inconnu reste refusé.
    assert_eq!(
        restarted
            .retry_failures(&f.ctx, Uuid::new_v4(), false)
            .unwrap_err()
            .kind,
        CommandErrorKind::NotFound
    );
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

/// Petit contrat : `GET /animaux/{id}` renvoie un animal `{ id, nom }`.
const CONTRAT: &str = r#"openapi: 3.0.3
info: { title: Animaux, version: 1.0.0 }
paths:
  /animaux/{id}:
    get:
      summary: Un animal
      parameters:
        - { name: id, in: path, required: true, schema: { type: integer }, example: 1 }
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: object
                required: [id, nom]
                properties:
                  id: { type: integer }
                  nom: { type: string }
"#;

#[tokio::test]
async fn c8_c10_le_run_du_contrat_verifie_la_conformite() {
    use crate::contract::file as contracts;

    let server = MockServer::start().await;
    Mock::given(path("/animaux/1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{ "id": 1, "nom": "Rex" }"#))
        .mount(&server)
        .await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    let contract = contracts::add(&f.ctx.root, CONTRAT).unwrap();
    contracts::generate(
        &f.ctx.root,
        &contract,
        &["GET /animaux/{id}".to_owned()],
        None,
    )
    .unwrap();
    add_request(&f, "", "Hors contrat", "{{baseUrl}}/autre", Method::Get);

    let run = f
        .bench
        .start_contract_run(&f.ctx, &contract, "Local", false)
        .unwrap();
    assert_eq!(run.contract.as_deref(), Some("Animaux"));
    assert_eq!(run.items.len(), 1, "seules les requêtes liées");
    let done = wait(&f.bench, run.id).await;
    let item = &done.items[0];
    assert_eq!(item.status, ItemStatus::Passed, "{:?}", item.assertions);
    let conformity = item.assertions.iter().find(|a| a.id == "contrat").unwrap();
    assert!(conformity.passed);

    // Un écart de schéma fait échouer la seule vérification de conformité.
    server.reset().await;
    Mock::given(path("/animaux/1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{ "id": "un" }"#))
        .mount(&server)
        .await;
    let retry = f
        .bench
        .start_contract_run(&f.ctx, &contract, "Local", false)
        .unwrap();
    let done = wait(&f.bench, retry.id).await;
    let item = &done.items[0];
    assert_eq!(item.status, ItemStatus::Failed);
    let conformity = item.assertions.iter().find(|a| a.id == "contrat").unwrap();
    assert_eq!(
        conformity.message.as_deref(),
        Some("body.nom : champ obligatoire absent ; body.id : attendu entier, reçu \"un\"")
    );
    // Les vérifications écrites (durée, statut) passent toujours.
    assert!(
        item.assertions
            .iter()
            .filter(|a| a.id != "contrat")
            .all(|a| a.passed),
        "{:?}",
        item.assertions
    );
}
