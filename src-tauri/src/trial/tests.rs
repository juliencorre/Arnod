//! Essai enregistré dans « Non classés » (docs/plan-accueil-non-classes.md, R6 à R11).

use std::fs;

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
            }],
            request_count: 1,
        },
        TreeNode::Request {
            name: "Essai rapide".to_owned(),
            path: "c.yml".to_owned(),
            method: None,
            assertion_count: 0,
            invalid: false,
        },
    ];
    assert_eq!(next_number(&nodes), 8);
    assert_eq!(next_number(&[]), 1);
}
