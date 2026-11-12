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

// ------------------------------------------------- fin du premier usage

/// Notificateur qui garde ce qu'on lui diffuse.
#[derive(Default)]
struct Recorder {
    runs: Mutex<Vec<RunView>>,
    statuses: Mutex<Vec<Statuses>>,
}

impl Notifier for Recorder {
    fn run_changed(&self, _project: Uuid, run: &RunView) {
        self.runs.lock().unwrap().push(run.clone());
    }

    fn statuses_changed(&self, _project: Uuid, statuses: &Statuses) {
        self.statuses.lock().unwrap().push(statuses.clone());
    }
}

/// Serveur qui répond 200 sur `/ok` et 500 sur `/ko`.
async fn ok_ko_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"amount\":\"12.50\"}"))
        .mount(&server)
        .await;
    Mock::given(path("/ko"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    server
}

/// Requête qui vérifie que `body.amount` est un nombre (en échec contre `/ok`).
fn add_number_check(f: &Fixture, path: &str) {
    let full = tree::resolve(&f.ctx.root, path).unwrap();
    let mut request = file::read(&full).unwrap();
    request.assertions.push(Assertion {
        id: "x".into(),
        target: "body.amount".into(),
        operator: crate::request::model::Operator::IsNumber,
        value: None,
        origin: crate::request::model::Origin::Manual,
        enabled: true,
    });
    file::write(&full, &request).unwrap();
}

/// Requête qui exige le statut 200 (en échec contre `/ko`) : sans elle, seule la
/// durée est vérifiée et un 500 réussit.
fn add_status_check(f: &Fixture, path: &str) {
    let full = tree::resolve(&f.ctx.root, path).unwrap();
    let mut request = file::read(&full).unwrap();
    request.assertions.push(Assertion {
        id: "s".into(),
        target: "status".into(),
        operator: crate::request::model::Operator::Equals,
        value: Some(crate::request::model::Scalar::Int(200)),
        origin: crate::request::model::Origin::Manual,
        enabled: true,
    });
    file::write(&full, &request).unwrap();
}

#[tokio::test]
async fn r1_r2_un_run_porte_numero_duree_et_resultat_par_dossier() {
    let server = ok_ko_server().await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "Boutique").unwrap();
    tree::create_folder(&f.ctx.root, "Boutique", "Catalogue").unwrap();
    tree::create_folder(&f.ctx.root, "Boutique", "Commandes").unwrap();
    add_request(
        &f,
        "Boutique/Catalogue",
        "Lister",
        "{{baseUrl}}/ok",
        Method::Get,
    );
    let create = add_request(
        &f,
        "Boutique/Commandes",
        "Créer",
        "{{baseUrl}}/ko",
        Method::Post,
    );
    add_status_check(&f, &create);
    add_request(
        &f,
        "Boutique/Commandes",
        "Lire",
        "{{baseUrl}}/ok",
        Method::Get,
    );
    add_request(&f, "Boutique", "Santé", "{{baseUrl}}/ok", Method::Get);

    let first = f
        .bench
        .start_run(&f.ctx, "Boutique", "Local", None, false)
        .unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(first.concurrency, RUN_CONCURRENCY);
    assert!(first.finished_at.is_none());
    let done = wait(&f.bench, first.id).await;
    assert!(done.finished_at.is_some_and(|end| end >= done.started_at));
    let groups: Vec<(&str, &str, usize, usize, usize)> = done
        .groups
        .iter()
        .map(|g| {
            (
                g.name.as_str(),
                g.path.as_str(),
                g.passed,
                g.failed,
                g.total,
            )
        })
        .collect();
    assert_eq!(
        groups,
        [
            ("Catalogue", "Boutique/Catalogue", 1, 0, 1),
            ("Commandes", "Boutique/Commandes", 1, 1, 2),
            ("Boutique", "Boutique", 1, 0, 1),
        ]
    );

    // Le numéro suit celui du dernier run gardé, même après un redémarrage.
    let restarted = Arc::new(Workbench::new(f.store.clone()));
    let second = restarted
        .start_run(&f.ctx, "", "Local", None, false)
        .unwrap();
    assert_eq!(second.number, 2);
    let second = wait(&restarted, second.id).await;
    assert_eq!(second.groups[0].name, "Boutique");
    assert_eq!(second.groups[0].total, 4);
}

#[test]
fn r2_groupes_a_la_racine_du_projet() {
    let item = |path: &str, status| RunItem {
        path: path.into(),
        name: path.into(),
        method: Method::Get,
        status,
        http_status: None,
        duration_ms: None,
        slow: false,
        assertions: Vec::new(),
        error: None,
    };
    let groups = run_groups(
        "",
        &[
            item("a/x.yml", ItemStatus::Error),
            item("sante.yml", ItemStatus::Passed),
            item("a/b/y.yml", ItemStatus::Pending),
        ],
    );
    assert_eq!(groups.len(), 2);
    assert_eq!(
        (groups[0].name.as_str(), groups[0].failed, groups[0].total),
        ("a", 1, 2)
    );
    assert_eq!(
        (groups[1].name.as_str(), groups[1].path.as_str()),
        ("Projet", "")
    );
}

#[test]
fn un_dernier_run_d_une_version_precedente_se_relit() {
    let f = fixture();
    let dir = local::project_dir(&f.ctx.data_dir, f.ctx.id);
    std::fs::create_dir_all(&dir).unwrap();
    let old = serde_json::json!({
        "id": Uuid::new_v4(),
        "folder": "D",
        "environment": "Local",
        "startedAt": 5,
        "finished": true,
        "cancelled": false,
        "items": [{
            "path": "D/a.yml", "name": "a", "method": "GET", "status": "failed",
            "httpStatus": 500, "durationMs": 3, "slow": false, "error": null,
            "assertions": [{ "id": "x", "passed": false, "message": "m", "actual": null }]
        }]
    });
    std::fs::write(dir.join("last-run.json"), old.to_string()).unwrap();
    let run = Workbench::last_run(&f.ctx).unwrap();
    assert_eq!(run.number, 0);
    assert_eq!(run.concurrency, RUN_CONCURRENCY);
    assert_eq!(run.groups.len(), 1);
    assert_eq!(run.groups[0].failed, 1);
    assert_eq!(run.items[0].assertions[0].target, "");
    assert!(!run.items[0].assertions[0].ignored);
}

#[tokio::test]
async fn r3_r4_l_avancement_et_les_statuts_sont_diffuses() {
    let server = ok_ko_server().await;
    let f = fixture();
    let recorder = Arc::new(Recorder::default());
    f.bench.set_notifier(recorder.clone());
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "D").unwrap();
    let ok = add_request(&f, "D", "Bon", "{{baseUrl}}/ok", Method::Get);
    let ko = add_request(&f, "D", "Mauvais", "{{baseUrl}}/ko", Method::Get);
    add_status_check(&f, &ko);

    let started = f
        .bench
        .start_run(&f.ctx, "D", "Local", None, false)
        .unwrap();
    let done = wait(&f.bench, started.id).await;

    let runs = recorder.runs.lock().unwrap().clone();
    // En cours puis terminée, pour chaque requête, puis la fin du run.
    assert_eq!(runs.len(), 5);
    assert!(runs.windows(2).all(|w| w[0].revision < w[1].revision));
    assert!(runs.last().unwrap().finished);
    assert_eq!(runs.last().unwrap().revision, done.revision);

    let statuses = f.bench.statuses(&f.ctx);
    assert!(statuses[&ok].passed);
    assert!(!statuses[&ko].passed);
    assert_eq!(recorder.statuses.lock().unwrap().len(), 2);
    // Gardés sur disque (R5).
    let restarted = Workbench::new(f.store.clone());
    assert_eq!(restarted.statuses(&f.ctx), statuses);
}

#[test]
fn r5_les_statuts_suivent_renommage_et_suppression() {
    let f = fixture();
    let recorder = Arc::new(Recorder::default());
    f.bench.set_notifier(recorder.clone());
    f.bench.record_status(&f.ctx, "D/a.yml", true);
    f.bench.record_status(&f.ctx, "D/b.yml", false);
    f.bench.rename_status(&f.ctx, "D", "E");
    let statuses = f.bench.statuses(&f.ctx);
    assert_eq!(
        statuses.keys().map(String::as_str).collect::<Vec<_>>(),
        ["E/a.yml", "E/b.yml"]
    );
    f.bench.remove_status(&f.ctx, "E/b.yml");
    assert_eq!(f.bench.statuses(&f.ctx).len(), 1);
    // Rien ne change : rien n'est diffusé.
    f.bench.rename_status(&f.ctx, "absent", "ailleurs");
    assert_eq!(recorder.statuses.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn r7_ignorer_une_verification_dans_un_run() {
    let server = ok_ko_server().await;
    let f = fixture();
    set_base_url(&f, &server.uri());
    tree::create_folder(&f.ctx.root, "", "D").unwrap();
    let path = add_request(&f, "D", "Montant", "{{baseUrl}}/ok", Method::Get);
    add_number_check(&f, &path);
    let started = f
        .bench
        .start_run(&f.ctx, "D", "Local", None, false)
        .unwrap();
    let done = wait(&f.bench, started.id).await;
    let item = &done.items[0];
    assert_eq!(item.status, ItemStatus::Failed);
    let failed = item.assertions.iter().find(|a| !a.passed).unwrap();
    assert_eq!(failed.target, "body.amount");
    assert_eq!(
        failed.message.as_deref(),
        Some("attendu number, reçu \"12.50\"")
    );
    assert!(!f.bench.statuses(&f.ctx)[&path].passed);
    let id = failed.id.clone();

    let after = f
        .bench
        .ignore_in_run(&f.ctx, started.id, &path, &id)
        .unwrap();

    assert_eq!(after.items[0].status, ItemStatus::Passed);
    assert!(after.revision > done.revision);
    assert_eq!(after.groups[0].passed, 1);
    assert!(f.bench.statuses(&f.ctx)[&path].passed);
    // Gardé dans le dernier run, mais pas dans la requête.
    assert_eq!(
        Workbench::last_run(&f.ctx).unwrap().items[0].status,
        ItemStatus::Passed
    );
    let request = file::read(&tree::resolve(&f.ctx.root, &path).unwrap()).unwrap();
    assert!(request.assertions.iter().any(|a| a.target == "body.amount"));
    // Un run oublié se relit sur disque ; une vérification absente est refusée.
    let restarted = Workbench::new(f.store.clone());
    let error = restarted
        .ignore_in_run(&f.ctx, started.id, &path, "absente")
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert!(
        restarted
            .ignore_in_run(&f.ctx, started.id, &path, &id)
            .is_ok()
    );
}

#[test]
fn forme_json_du_run_et_des_statuts() {
    let mut run = RunView {
        id: Uuid::nil(),
        number: 3,
        folder: "D".into(),
        environment: "Recette".into(),
        concurrency: 8,
        started_at: 10,
        finished_at: Some(4_110),
        finished: true,
        cancelled: false,
        items: vec![RunItem {
            path: "D/a.yml".into(),
            name: "a".into(),
            method: Method::Post,
            status: ItemStatus::Failed,
            http_status: Some(201),
            duration_ms: Some(88),
            slow: false,
            assertions: vec![AssertionResult {
                id: "x".into(),
                target: "body.amount".into(),
                passed: false,
                message: Some("attendu number, reçu \"129.90\"".into()),
                actual: Some("\"129.90\"".into()),
                ignored: false,
            }],
            error: None,
        }],
        contract: None,
        revision: 0,
        groups: Vec::new(),
    };
    run.touch();
    let json = serde_json::to_value(&run).unwrap();
    assert_eq!(json["number"], 3);
    assert_eq!(json["concurrency"], 8);
    assert_eq!(json["finishedAt"], 4_110);
    assert_eq!(json["revision"], 1);
    assert_eq!(
        json["groups"],
        serde_json::json!([{ "name": "D", "path": "D", "passed": 0, "failed": 1, "total": 1 }])
    );
    assert_eq!(
        json["items"][0]["assertions"][0],
        serde_json::json!({
            "id": "x",
            "target": "body.amount",
            "passed": false,
            "message": "attendu number, reçu \"129.90\"",
            "actual": "\"129.90\"",
            "ignored": false
        })
    );
    let change = crate::windows::StatusesChange {
        project_id: Uuid::nil(),
        statuses: Statuses::from([(
            "D/a.yml".to_owned(),
            crate::statuses::RequestStatus {
                passed: false,
                at: 7,
            },
        )]),
    };
    assert_eq!(
        serde_json::to_value(change).unwrap(),
        serde_json::json!({
            "projectId": Uuid::nil().to_string(),
            "statuses": { "D/a.yml": { "passed": false, "at": 7 } }
        })
    );
}
