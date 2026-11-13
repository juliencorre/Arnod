//! Essai enregistré dans « Non classés » (docs/plan-accueil-non-classes.md, R6 à R11).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::contract::file as contract_file;
use crate::environment;
use crate::manifest::{self, Manifest};
use crate::request::model::Method;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let _created = manifest::create_project_folder(
        dir.path(),
        &Manifest::new(uuid::Uuid::from_u128(1), "Non classés"),
    )
    .unwrap();
    environment::init(dir.path()).unwrap();
    dir
}

fn response(status: u16, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        status_text: "OK".to_owned(),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        body: body.to_owned(),
        truncated: false,
        size_bytes: body.len(),
        wait_ms: 10,
        receive_ms: 2,
        total_ms: 12,
    }
}

fn trial(method: Method, url: &str) -> Request {
    let mut request = Request::new(TRIAL_PREFIX);
    request.method = method;
    request.url = url.to_owned();
    request
}

#[test]
fn r11_premier_essai_contrat_et_requete() {
    let dir = project();
    let root = dir.path();

    let recorded = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, r#"{"id":"prd_101","name":"T-shirt"}"#),
    )
    .unwrap();

    assert_eq!(recorded.contract, "contracts/api-boutique-dev.yaml");
    assert_eq!(recorded.path, "api.boutique.dev/essai-1.yml");
    let request = request_file::read(&root.join(&recorded.path)).unwrap();
    assert_eq!(request.name, "Essai 1");
    assert_eq!(request.url, "https://api.boutique.dev/products");
    assert_eq!(
        request.contract,
        Some(ContractLink {
            file: recorded.contract.clone(),
            operation: "GET /products".to_owned(),
        })
    );
    // Durée par défaut, statut et champs du corps proposés.
    let targets: Vec<&str> = request
        .assertions
        .iter()
        .map(|a| a.target.as_str())
        .collect();
    assert_eq!(targets, ["responseTime", "status", "body.id", "body.name"]);
    assert_eq!(request.examples.len(), 1);
    assert_eq!(request.examples[0].name, "200 OK");

    let contract = contract_file::read(root, &recorded.contract).unwrap();
    assert_eq!(contract.title, "api.boutique.dev");
    assert_eq!(contract.servers, ["https://api.boutique.dev"]);
    assert_eq!(contract.operations.len(), 1);
    // Le contrat liste la requête liée à son opération.
    let view = contract_file::view(root, &recorded.contract).unwrap();
    assert_eq!(
        view.operations[0].request.as_deref(),
        Some(recorded.path.as_str())
    );
}

#[test]
fn r9_essais_suivants_enrichissent_le_meme_contrat() {
    let dir = project();
    let root = dir.path();
    record(
        root,
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, "[]"),
    )
    .unwrap();

    let second = record(
        root,
        trial(Method::Post, "https://api.boutique.dev/orders"),
        &response(201, r#"{"id":42}"#),
    )
    .unwrap();
    let other = record(
        root,
        trial(Method::Get, "http://localhost:8080/sante"),
        &response(200, "{}"),
    )
    .unwrap();

    assert_eq!(second.contract, "contracts/api-boutique-dev.yaml");
    assert_eq!(second.request.name, "Essai 2");
    assert_eq!(other.contract, "contracts/localhost-8080.yaml");
    // « : » est interdit dans un nom de dossier sous Windows.
    assert_eq!(other.path, "localhost 8080/essai-3.yml");
    let contract = contract_file::read(root, &second.contract).unwrap();
    let keys: Vec<&str> = contract.operations.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, ["GET /products", "POST /orders"]);
}

#[test]
fn r9_contrat_ecrit_a_la_main_jamais_modifie() {
    let dir = project();
    let root = dir.path();
    let folder = contract_file::dir(root);
    fs::create_dir_all(&folder).unwrap();
    let mine = "openapi: 3.1.0\ninfo:\n  title: Boutique\n  version: 2.1.0\nservers:\n  - url: https://api.boutique.dev\npaths: {}\n";
    fs::write(folder.join("api-boutique-dev.yaml"), mine).unwrap();

    let recorded = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, "[]"),
    )
    .unwrap();

    assert_eq!(recorded.contract, "contracts/api-boutique-dev-2.yaml");
    assert_eq!(
        fs::read_to_string(folder.join("api-boutique-dev.yaml")).unwrap(),
        mine
    );
}

#[test]
fn r10_contrat_deduit_illisible_refuse_l_essai() {
    let dir = project();
    let root = dir.path();
    let folder = contract_file::dir(root);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("api-boutique-dev.yaml"), "openapi: [\n").unwrap();

    let error = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, "[]"),
    )
    .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert!(error.message.contains("illisible"), "{}", error.message);
    assert!(tree::read(root).unwrap().is_empty());
}

#[test]
fn r11_numero_d_essai_libre() {
    let nodes = vec![
        TreeNode::Request {
            name: "Essai 3".to_owned(),
            path: "a.yml".to_owned(),
            method: None,
            assertion_count: 0,
            invalid: false,
            contract: None,
            url: String::new(),
            tags: Vec::new(),
        },
        TreeNode::Folder {
            name: "d".to_owned(),
            path: "d".to_owned(),
            children: vec![TreeNode::Request {
                name: "Essai 7".to_owned(),
                path: "d/b.yml".to_owned(),
                method: None,
                assertion_count: 0,
                invalid: false,
                contract: None,
                url: String::new(),
                tags: Vec::new(),
            }],
            request_count: 1,
            test: true,
        },
        TreeNode::Request {
            name: "Essai rapide".to_owned(),
            path: "c.yml".to_owned(),
            method: None,
            assertion_count: 0,
            invalid: false,
            contract: None,
            url: String::new(),
            tags: Vec::new(),
        },
    ];
    assert_eq!(next_number(&nodes), 8);
    assert_eq!(next_number(&[]), 1);
}

#[test]
fn c_generaliser_le_chemin_d_un_essai() {
    let dir = project();
    let root = dir.path();
    let first = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/orders/42/tracking"),
        &response(200, r#"{"orderId":42}"#),
    )
    .unwrap();
    let proposed = generalize::suggestion(root, &first.path).unwrap().unwrap();
    assert_eq!(proposed.path, "/orders/42/tracking");
    assert_eq!(proposed.template.path, "/orders/{id}/tracking");

    let changed = generalize::generalize(root, &first.path).unwrap();
    assert_eq!(changed, std::slice::from_ref(&first.path));
    let request = request_file::read(&root.join(&first.path)).unwrap();
    assert_eq!(request.url, "https://api.boutique.dev/orders/{id}/tracking");
    assert_eq!(
        request.contract.map(|l| l.operation).as_deref(),
        Some("GET /orders/{id}/tracking")
    );
    let id = request.params.iter().find(|p| p.name == "id").unwrap();
    assert_eq!(id.value, "42");
    let contract = contract_file::read(root, &first.contract).unwrap();
    assert_eq!(contract.operations.len(), 1);
    // Plus rien à proposer, et l'essai suivant rejoint l'opération généralisée.
    assert_eq!(generalize::suggestion(root, &first.path).unwrap(), None);
    let second = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/orders/43/tracking"),
        &response(404, r#"{"error":"absent"}"#),
    )
    .unwrap();
    let request = request_file::read(&root.join(&second.path)).unwrap();
    assert_eq!(
        request.contract.map(|l| l.operation).as_deref(),
        Some("GET /orders/{id}/tracking")
    );
    assert_eq!(
        contract_file::read(root, &first.contract)
            .unwrap()
            .operations
            .len(),
        1
    );
}

#[test]
fn c_garder_le_chemin_tel_quel() {
    let dir = project();
    let root = dir.path();
    let recorded = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/orders/42"),
        &response(200, "{}"),
    )
    .unwrap();
    generalize::keep(root, &recorded.path).unwrap();
    assert_eq!(generalize::suggestion(root, &recorded.path).unwrap(), None);
    assert!(generalize::generalize(root, &recorded.path).is_err());
}

#[test]
fn b_transferer_un_essai_au_referentiel_de_tests() {
    let dir = project();
    let root = dir.path();
    let recorded = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, r#"{"id":"prd_101"}"#),
    )
    .unwrap();
    let before = request_file::read(&root.join(&recorded.path)).unwrap();
    let moved = transfer(root, &recorded.path, "Lister les produits", "").unwrap();
    assert_eq!(
        moved,
        Transferred {
            test: "Lister les produits".to_owned(),
            step: "Lister les produits/essai-1.yml".to_owned(),
        }
    );
    let step = request_file::read(&root.join(&moved.step)).unwrap();
    assert_eq!(step.contract, None);
    assert_eq!(step.seq, Some(1.0));
    assert_eq!(step.assertions.len(), before.assertions.len());
    assert_eq!(step.url, before.url);
    assert!(!root.join(&recorded.path).exists());
    // Le dossier est un test d'une étape, dans le référentiel.
    let nodes = tree::read(root).unwrap();
    let tests = tree::tests_view(&nodes, &contract_file::paths(root));
    assert!(tests.iter().any(|n| matches!(n,
        TreeNode::Folder { name, test: true, .. } if name == "Lister les produits")));
    // Un nom déjà pris est refusé sans rien déplacer.
    let other = record(
        root,
        trial(Method::Get, "https://api.boutique.dev/orders"),
        &response(200, "[]"),
    )
    .unwrap();
    assert!(transfer(root, &other.path, "lister les produits", "").is_err());
    assert!(root.join(&other.path).exists());
}

fn target_project(dir: &Path, contract: Option<&str>) -> std::path::PathBuf {
    let root = dir.join("boutique-tests");
    std::fs::create_dir_all(&root).unwrap();
    let _created = manifest::create_project_folder(
        &root,
        &Manifest::new(uuid::Uuid::from_u128(2), "boutique-tests"),
    )
    .unwrap();
    if let Some(text) = contract {
        std::fs::create_dir_all(root.join("contracts")).unwrap();
        std::fs::write(root.join("contracts/commandes.yaml"), text).unwrap();
    }
    root
}

const COMMANDES: &str = "openapi: 3.0.3\ninfo:\n  title: API Commandes\n  version: 2.4.0\nservers:\n  - url: https://api.boutique.dev\npaths:\n  /orders:\n    get:\n      responses:\n        '200':\n          description: OK\n  /orders/{id}/tracking:\n    get:\n      parameters:\n        - name: id\n          in: path\n          required: true\n          schema:\n            type: integer\n      responses:\n        '200':\n          description: OK\n";

#[test]
fn d_ranger_les_essais_avec_une_copie_du_contrat() {
    let inbox = project();
    let other = tempfile::tempdir().unwrap();
    let target = target_project(other.path(), None);
    let recorded = record(
        inbox.path(),
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, "[]"),
    )
    .unwrap();
    let (filed, undo) = filing::file_into(inbox.path(), &recorded.contract, &target, None).unwrap();
    assert_eq!(filed.contract, "contracts/api-boutique-dev.yaml");
    assert_eq!(
        filed.moved,
        [(
            recorded.path.clone(),
            "api.boutique.dev/essai-1.yml".to_owned()
        )]
    );
    let moved = request_file::read(&target.join("api.boutique.dev/essai-1.yml")).unwrap();
    assert_eq!(moved.url, "https://api.boutique.dev/products");
    assert_eq!(moved.contract.unwrap().file, filed.contract);
    // La boîte est vide de cet essai et de son contrat.
    assert!(!inbox.path().join(&recorded.path).exists());
    assert!(!inbox.path().join(&recorded.contract).exists());
    // « Annuler » remet tout.
    undo.apply().unwrap();
    assert!(inbox.path().join(&recorded.path).exists());
    assert!(inbox.path().join(&recorded.contract).exists());
    assert!(!target.join("api.boutique.dev/essai-1.yml").exists());
    assert!(!target.join(&filed.contract).exists());
}

#[test]
fn d_ranger_les_essais_sous_un_contrat_du_projet() {
    let inbox = project();
    let other = tempfile::tempdir().unwrap();
    let target = target_project(other.path(), Some(COMMANDES));
    let tracking = record(
        inbox.path(),
        trial(Method::Get, "https://api.boutique.dev/orders/42/tracking"),
        &response(200, "{}"),
    )
    .unwrap();
    let products = record(
        inbox.path(),
        trial(Method::Get, "https://api.boutique.dev/products"),
        &response(200, "[]"),
    )
    .unwrap();
    let (filed, _undo) = filing::file_into(
        inbox.path(),
        &tracking.contract,
        &target,
        Some("contracts/commandes.yaml"),
    )
    .unwrap();
    assert_eq!(filed.contract_name, "API Commandes");
    assert_eq!(filed.left, std::slice::from_ref(&products.path));
    let (_, to) = &filed.moved[0];
    let moved = request_file::read(&target.join(to)).unwrap();
    assert_eq!(moved.url, "{{baseUrl}}/orders/42/tracking");
    assert_eq!(
        moved.contract.unwrap().operation,
        "GET /orders/{id}/tracking"
    );
    // Un essai reste : le contrat déduit reste avec lui ; le contrat du projet n'est pas touché.
    assert!(inbox.path().join(&tracking.contract).exists());
    assert_eq!(
        std::fs::read_to_string(target.join("contracts/commandes.yaml")).unwrap(),
        COMMANDES
    );
}
