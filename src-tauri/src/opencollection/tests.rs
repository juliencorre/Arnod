//! Règles du format OpenCollection (docs/plan-opencollection.md, R1 à R8).

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use uuid::Uuid;

use super::collection;
use super::migrate;
use super::request::{
    OPERATORS, expression, from_value, operator_from, operator_name, scalar_from, target_from,
    value_text,
};
use super::schema::{self, Part};
use super::{ROOT_FILE, yaml};
use crate::contract::file as contract_file;
use crate::environment::{self, Environment, Variable};
use crate::request::file;
use crate::request::model::{Method, Operator, ParamKind, Scalar};
use crate::request::resolve::{Variables, prepare};
use crate::tree::{self, TreeNode};

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    collection::write(
        dir.path(),
        &collection::new_root("Boutique", Uuid::new_v4()),
    )
    .unwrap();
    dir
}

fn read(path: &Path) -> Value {
    yaml::read(path).unwrap()
}

fn assert_valid(part: Part, path: &Path) {
    let value = read(path);
    let errors = schema::errors(part, &value);
    assert!(
        errors.is_empty(),
        "{} : {errors:?}\n{}",
        path.display(),
        fs::read_to_string(path).unwrap()
    );
}

const CONTRACT: &str = r#"openapi: 3.0.3
info:
  title: Boutique
  version: "1.0"
servers:
  - url: http://localhost:8080
paths:
  /items/{id}:
    get:
      summary: Lire un article
      parameters:
        - { name: id, in: path, required: true, schema: { type: integer, example: 7 } }
        - { name: detail, in: query, schema: { type: boolean } }
        - { name: X-Trace, in: header, required: true, schema: { type: string } }
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: object
                properties:
                  id: { type: integer }
  /items:
    post:
      summary: Créer un article
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [nom]
              properties:
                nom: { type: string, example: Lampe }
      responses:
        "201":
          description: créé
"#;

// R1 : tout fichier écrit est valide pour le schéma officiel.

#[test]
fn r1_racine_neuve_valide() {
    let dir = project();
    let path = dir.path().join(ROOT_FILE);
    assert_valid(Part::Root, &path);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("opencollection: \"1.0.0\"\n"), "{text}");
    let meta = collection::read_meta(dir.path());
    assert!(meta.id.is_some());
}

#[test]
fn r1_requete_neuve_valide() {
    let dir = project();
    let path = tree::create_request(dir.path(), "", Some("Lire")).unwrap();
    let full = dir.path().join(&path);
    assert_valid(Part::HttpRequest, &full);
    let value = read(&full);
    assert_eq!(value.pointer("/info/type"), Some(&json!("http")));
    assert_eq!(value.pointer("/info/name"), Some(&json!("Lire")));
    assert_eq!(value.pointer("/http/method"), Some(&json!("GET")));
}

#[test]
fn r1_requetes_d_un_contrat_valides() {
    let dir = project();
    let path = contract_file::add(dir.path(), CONTRACT).unwrap();
    let report = contract_file::generate(
        dir.path(),
        &path,
        &["GET /items/{id}".to_owned(), "POST /items".to_owned()],
        None,
    )
    .unwrap();
    assert_eq!(report.created.len(), 2);
    for created in &report.created {
        let full = dir.path().join(created);
        assert_valid(Part::HttpRequest, &full);
        let value = read(&full);
        let tags = value.pointer("/info/tags").unwrap();
        assert!(
            tags.as_array().unwrap().iter().any(|t| t
                .as_str()
                .is_some_and(|t| t.starts_with("contract:contracts/"))),
            "{tags}"
        );
        // La conformité est déduite du lien : aucune vérification écrite pour elle (R8).
        let text = fs::read_to_string(&full).unwrap();
        assert!(!text.contains("matchesContract"), "{text}");
    }
    let show = file::read(&dir.path().join(&report.created[0])).unwrap();
    let link = show.contract.unwrap();
    assert_eq!(link.operation, "GET /items/{id}");
}

#[test]
fn r1_environnement_valide() {
    let dir = project();
    environment::save(
        dir.path(),
        &Environment {
            name: "Recette".to_owned(),
            color: "blue".to_owned(),
            protected: true,
            variables: vec![Variable {
                name: "baseUrl".to_owned(),
                value: "https://recette.test".to_owned(),
            }],
            locals: vec!["token".to_owned()],
        },
        None,
    )
    .unwrap();
    let path = dir.path().join("environments/recette.yml");
    assert_valid(Part::Environment, &path);
    assert_valid(Part::Root, &dir.path().join(ROOT_FILE));
    let value = read(&path);
    assert_eq!(
        value.pointer("/variables/1"),
        Some(&json!({ "name": "token", "secret": true }))
    );
    let meta = collection::read_meta(dir.path());
    assert_eq!(meta.protected, vec!["Recette".to_owned()]);
}

// R2 : un fichier illisible est listé comme tel et jamais réécrit.

#[test]
fn r2_fichier_non_conforme_liste_et_garde() {
    let dir = project();
    let path = dir.path().join("bizarre.yml");
    let text =
        "info:\n  name: Bizarre\n  type: http\nhttp:\n  method: GET\n  url: x\n  couleur: rouge\n";
    fs::write(&path, text).unwrap();
    let nodes = tree::read(dir.path()).unwrap();
    assert!(
        matches!(&nodes[..], [TreeNode::Request { invalid: true, .. }]),
        "{nodes:?}"
    );
    assert!(file::read(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
}

// R3 : un enregistrement ne change que les champs gérés par Arnod.

const BRUNO: &str = r#"info:
  name: Commande
  type: http
  seq: 3
  tags:
    - smoke
http:
  method: POST
  url: "{{baseUrl}}/orders/:id"
  params:
    - name: id
      value: "42"
      type: path
    - name: trace
      value: "1"
      type: query
      description: Trace côté serveur
  headers:
    - name: Accept
      value: application/json
  body:
    type: json
    data: '{"qty": 2}'
  auth:
    type: oauth2
    flow: client_credentials
    accessTokenUrl: https://auth.test/token
runtime:
  assertions:
    - expression: res.status
      operator: eq
      value: "201"
      description: Création
    - expression: res.body.items
      operator: isArray
    - expression: res.headers["content-type"]
      operator: contains
      value: json
      disabled: true
  scripts:
    - type: after-response
      code: bru.setVar("id", res.body.id);
docs: |-
  # Commande
  Crée une commande.
examples:
  - name: Créée
    response:
      status: 201
      body:
        type: json
        data: '{"id": 42}'
settings:
  timeout: 5000
"#;

#[test]
fn r3_le_contenu_non_gere_est_conserve() {
    let dir = project();
    let path = dir.path().join("commande.yml");
    fs::write(&path, BRUNO).unwrap();
    let mut request = file::read(&path).unwrap();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.timeout_ms, 5000);
    assert!(
        request.preserved.contains(&"scripts".to_owned()),
        "{:?}",
        request.preserved
    );
    assert!(request.preserved.contains(&"documentation".to_owned()));
    assert!(request.preserved.contains(&"exemples".to_owned()));
    assert_eq!(request.params[0].kind, ParamKind::Path);
    assert_eq!(request.params[1].kind, ParamKind::Query);
    assert!(!request.assertions[2].enabled);

    request.url = "{{baseUrl}}/commandes/:id".to_owned();
    request.assertions[0].value = Some(Scalar::Int(200));
    file::write(&path, &request).unwrap();

    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    assert_eq!(
        value.pointer("/http/url"),
        Some(&json!("{{baseUrl}}/commandes/:id"))
    );
    assert_eq!(value.pointer("/info/seq"), Some(&json!(3)));
    assert_eq!(value.pointer("/info/tags"), Some(&json!(["smoke"])));
    assert_eq!(value.pointer("/http/auth/type"), Some(&json!("oauth2")));
    assert_eq!(
        value.pointer("/http/auth/accessTokenUrl"),
        Some(&json!("https://auth.test/token"))
    );
    assert_eq!(
        value.pointer("/http/params/1/description"),
        Some(&json!("Trace côté serveur"))
    );
    assert_eq!(
        value.pointer("/runtime/assertions/0"),
        Some(&json!({
            "expression": "res.status",
            "operator": "eq",
            "value": "200",
            "description": "Création"
        }))
    );
    assert_eq!(
        value.pointer("/runtime/assertions/2/disabled"),
        Some(&json!(true))
    );
    assert!(value.pointer("/runtime/scripts/0/code").is_some());
    assert_eq!(
        value.pointer("/examples/0/response/status"),
        Some(&json!(201))
    );
    assert_eq!(
        value.pointer("/docs"),
        Some(&json!("# Commande\nCrée une commande."))
    );
    assert_eq!(value.pointer("/settings/timeout"), Some(&json!(5000)));
}

#[test]
fn r3_supprimer_une_verification_ne_garde_pas_sa_description() {
    let dir = project();
    let path = dir.path().join("commande.yml");
    fs::write(&path, BRUNO).unwrap();
    let mut request = file::read(&path).unwrap();
    request.assertions.remove(0);
    file::write(&path, &request).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("Création"), "{text}");
    assert_valid(Part::HttpRequest, &path);
}

// R4 : expressions, opérateurs et valeurs d'OpenCollection et de Bruno.

#[test]
fn r4_expressions_aller_retour() {
    for (target, expr) in [
        ("status", "res.status"),
        ("responseTime", "res.responseTime"),
        ("headers.content-type", "res.headers[\"content-type\"]"),
        ("body", "res.body"),
        ("body.items[0].id", "res.body.items[0].id"),
        ("body[0]", "res.body[0]"),
    ] {
        assert_eq!(expression(target), expr);
        assert_eq!(target_from(expr), target);
    }
    assert_eq!(target_from("res.headers.etag"), "headers.etag");
    assert_eq!(target_from("res.headers['X-Id']"), "headers.X-Id");
    assert_eq!(target_from("  res.status "), "status");
    // Une expression inconnue est gardée telle quelle.
    assert_eq!(target_from("res.getHeader('x')"), "res.getHeader('x')");
    assert_eq!(expression("res.getHeader('x')"), "res.getHeader('x')");
}

#[test]
fn r4_operateurs_aller_retour() {
    for (operator, name) in OPERATORS {
        assert_eq!(operator_name(operator), name);
        assert_eq!(operator_from(name), Some(operator));
    }
    assert_eq!(operator_from("isDefined"), Some(Operator::Exists));
    assert_eq!(operator_from("isString"), Some(Operator::IsText));
    assert_eq!(operator_from("plusGrandQue"), None);
}

#[test]
fn r4_valeurs_typees() {
    assert_eq!(scalar_from("200"), Scalar::Int(200));
    assert_eq!(scalar_from("1.5"), Scalar::Float(1.5));
    assert_eq!(scalar_from("true"), Scalar::Bool(true));
    assert_eq!(scalar_from("pending"), Scalar::Text("pending".to_owned()));
    assert_eq!(scalar_from("\"200\""), Scalar::Text("200".to_owned()));
    assert_eq!(scalar_from("'a b'"), Scalar::Text("a b".to_owned()));
    for value in [
        Scalar::Int(-3),
        Scalar::Bool(false),
        Scalar::Text("200".to_owned()),
        Scalar::Text("true".to_owned()),
        Scalar::Text("texte".to_owned()),
        Scalar::Text(String::new()),
    ] {
        assert_eq!(scalar_from(&value_text(&value)), value, "{value:?}");
    }
}

#[test]
fn r4_operateur_inconnu_refuse() {
    let value = json!({
        "info": { "name": "x", "type": "http" },
        "http": { "method": "GET", "url": "u" },
        "runtime": { "assertions": [
            { "expression": "res.status", "operator": "plusGrandQue", "value": "1" }
        ] }
    });
    let error = from_value(&value, "x").unwrap_err();
    assert!(error.contains("plusGrandQue"), "{error}");
}

// R5 : un projet est un dossier contenant opencollection.yml.

#[test]
fn r5_une_racine_non_conforme_n_est_pas_un_projet() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  nom: x\n",
    )
    .unwrap();
    let error = collection::read(dir.path()).unwrap_err();
    assert_eq!(error.kind, crate::error::CommandErrorKind::NotAProject);
}

#[test]
fn r5_les_autres_extensions_sont_gardees() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Partagée\nextensions:\n  bruno:\n    ignore: [node_modules]\n",
    )
    .unwrap();
    collection::update_meta(dir.path(), |meta| {
        meta.environment_order = vec!["Local".to_owned()];
    })
    .unwrap();
    let root = read(&dir.path().join(ROOT_FILE));
    assert_eq!(
        root.pointer("/extensions/bruno/ignore"),
        Some(&json!(["node_modules"]))
    );
    assert_eq!(
        root.pointer("/extensions/arnod/environments/order"),
        Some(&json!(["Local"]))
    );
    assert_valid(Part::Root, &dir.path().join(ROOT_FILE));
}

#[test]
fn r5_une_racine_sans_version_n_est_pas_un_projet() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(ROOT_FILE), "info:\n  name: x\n").unwrap();
    let error = collection::read(dir.path()).unwrap_err();
    assert_eq!(error.kind, crate::error::CommandErrorKind::NotAProject);
}

#[test]
fn r5_une_collection_regroupee_n_est_pas_prise_en_charge() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: x\nbundled: true\n",
    )
    .unwrap();
    let error = collection::read(dir.path()).unwrap_err();
    assert_eq!(error.kind, crate::error::CommandErrorKind::NotAProject);
    assert!(error.message.contains("regroupée"), "{}", error.message);
}

// R6 : un projet du format précédent est converti une fois.

fn legacy_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join("arnod.yml"),
        "version: 1\nid: 6c1f4e0a-1b2c-4d5e-8f90-123456789abc\nname: Ancien\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("environments")).unwrap();
    fs::write(
        root.join("environments/_order.yml"),
        "order:\n  - Recette\n  - Local\n",
    )
    .unwrap();
    fs::write(
        root.join("environments/local.yml"),
        "version: 1\nname: Local\ncolor: green\nvariables:\n  - name: baseUrl\n    value: http://localhost:8080\nlocals:\n  - token\n",
    )
    .unwrap();
    fs::write(
        root.join("environments/recette.yml"),
        "version: 1\nname: Recette\ncolor: orange\nprotected: true\nvariables: []\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("Commandes")).unwrap();
    fs::write(
        root.join("Commandes/lire.yml"),
        r#"version: 1
id: 0f4c2d1e-1111-4222-8333-444455556666
name: Lire
method: GET
url: "{{baseUrl}}/orders/1"
params:
  - name: detail
    value: "true"
    enabled: false
headers:
  - name: Accept
    value: application/json
auth:
  type: apiKey
  name: X-Key
  value: "{{cle}}"
  location: query
timeoutMs: 5000
assertions:
  - id: a1
    target: status
    op: equals
    value: 200
  - id: a2
    target: headers.Content-Type
    op: contains
    value: json
  - id: a3
    target: body
    op: matchesContract
contract:
  file: contracts/boutique.yaml
  operation: GET /orders/{id}
"#,
    )
    .unwrap();
    fs::write(
        root.join("Commandes/creer.yml"),
        "version: 1\nid: 0f4c2d1e-2222-4222-8333-444455556666\nname: Créer\nmethod: POST\nurl: /orders\nbody:\n  type: form\n  form:\n    - name: qty\n      value: \"2\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("contracts")).unwrap();
    fs::write(root.join("contracts/boutique.yaml"), CONTRACT).unwrap();
    fs::write(root.join("notes.yml"), "texte: libre\n").unwrap();
    dir
}

#[test]
fn r6_un_projet_du_format_precedent_est_converti() {
    let dir = legacy_project();
    let root = dir.path();
    assert!(migrate::run(root).unwrap());

    assert!(!root.join("arnod.yml").exists());
    assert!(!root.join("environments/_order.yml").exists());
    assert_valid(Part::Root, &root.join(ROOT_FILE));
    let collection_root = collection::read(root).unwrap();
    assert_eq!(
        collection::name(&collection_root).as_deref(),
        Some("Ancien")
    );
    let meta = collection::meta(&collection_root);
    assert_eq!(
        meta.id.map(|id| id.to_string()).as_deref(),
        Some("6c1f4e0a-1b2c-4d5e-8f90-123456789abc")
    );
    assert_eq!(meta.environment_order, vec!["Recette", "Local"]);
    assert_eq!(meta.protected, vec!["Recette"]);

    let envs = environment::list(root);
    let names: Vec<&str> = envs.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Recette", "Local"]);
    assert_eq!(envs[1].locals, vec!["token"]);
    assert_eq!(envs[1].color, "green");
    for env in ["local", "recette"] {
        assert_valid(
            Part::Environment,
            &root.join(format!("environments/{env}.yml")),
        );
    }

    let path = root.join("Commandes/lire.yml");
    assert_valid(Part::HttpRequest, &path);
    let request = file::read(&path).unwrap();
    assert_eq!(request.name, "Lire");
    assert_eq!(request.timeout_ms, 5000);
    assert!(!request.params[0].enabled);
    assert_eq!(request.assertions.len(), 2);
    assert_eq!(request.assertions[1].target, "headers.content-type");
    let link = request.contract.unwrap();
    assert_eq!(link.file, "contracts/boutique.yaml");
    assert_eq!(link.operation, "GET /orders/{id}");
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("contract:contracts/boutique.yaml#GET /orders/{id}"),
        "{text}"
    );
    assert!(text.contains("placement: query"), "{text}");
    assert!(!text.contains("matchesContract"), "{text}");
    assert!(!text.contains("version:"), "{text}");

    assert_valid(Part::HttpRequest, &root.join("Commandes/creer.yml"));
    // Ce qui n'est ni une requête ni un contrat est laissé tel quel.
    assert_eq!(
        fs::read_to_string(root.join("notes.yml")).unwrap(),
        "texte: libre\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("contracts/boutique.yaml")).unwrap(),
        CONTRACT
    );

    // Une seconde ouverture ne change rien.
    assert!(!migrate::run(root).unwrap());
}

#[test]
fn r6_un_manifeste_plus_recent_n_est_pas_converti() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!("version: 2\nid: {}\nname: futur\n", Uuid::new_v4());
    fs::write(dir.path().join("arnod.yml"), &text).unwrap();
    let error = migrate::run(dir.path()).unwrap_err();
    assert_eq!(error.kind, crate::error::CommandErrorKind::NewerVersion);
    assert_eq!(
        fs::read_to_string(dir.path().join("arnod.yml")).unwrap(),
        text
    );
    assert!(!dir.path().join(ROOT_FILE).exists());
}

#[test]
fn r6_un_projet_opencollection_n_est_pas_touche() {
    let dir = project();
    fs::write(dir.path().join("arnod.yml"), "version: 1\n").unwrap();
    assert!(!migrate::run(dir.path()).unwrap());
    assert!(dir.path().join("arnod.yml").exists());
}

// R7 : les fichiers de réglages ne sont jamais lus comme des requêtes.

#[test]
fn r7_racine_et_dossiers_ne_sont_pas_des_requetes() {
    let dir = project();
    tree::create_folder(dir.path(), "", "Commandes").unwrap();
    fs::write(
        dir.path().join("Commandes/folder.yml"),
        "info:\n  name: Commandes\n  type: folder\n",
    )
    .unwrap();
    assert_valid(Part::Folder, &dir.path().join("Commandes/folder.yml"));
    tree::create_request(dir.path(), "Commandes", Some("Lire")).unwrap();
    let nodes = tree::read(dir.path()).unwrap();
    assert!(
        matches!(
            &nodes[..],
            [TreeNode::Folder {
                request_count: 1,
                ..
            }]
        ),
        "{nodes:?}"
    );
}

#[test]
fn r7_renommer_un_dossier_met_a_jour_folder_yml() {
    let dir = project();
    tree::create_folder(dir.path(), "", "Commandes").unwrap();
    fs::write(
        dir.path().join("Commandes/folder.yml"),
        "info:\n  name: Commandes\n  type: folder\n  seq: 2\n",
    )
    .unwrap();
    let renamed = tree::rename(dir.path(), "Commandes", "Ventes").unwrap();
    let path = dir.path().join(renamed).join("folder.yml");
    let value = read(&path);
    assert_eq!(value.pointer("/info/name"), Some(&json!("Ventes")));
    assert_eq!(value.pointer("/info/seq"), Some(&json!(2)));
    assert_valid(Part::Folder, &path);
}

// S8 : paramètres de chemin.

#[test]
fn s8_parametres_de_chemin_remplaces() {
    let dir = project();
    let path = dir.path().join("commande.yml");
    fs::write(&path, BRUNO).unwrap();
    let request = file::read(&path).unwrap();
    let mut variables = Variables::default();
    variables
        .values
        .insert("baseUrl".to_owned(), "https://api.test".to_owned());
    let prepared = prepare(&request, &variables).unwrap();
    assert_eq!(prepared.url, "https://api.test/orders/42?trace=1");
}

#[test]
fn unions_typescript_concordantes() {
    use crate::request::model::{AuthKind, BodyKind};
    use crate::ts_contract::{serialized_name, union_members};
    use std::collections::BTreeSet;

    fn names<T: serde::Serialize>(values: impl IntoIterator<Item = T>) -> BTreeSet<String> {
        values.into_iter().map(serialized_name).collect()
    }
    assert_eq!(
        union_members("ParamKind"),
        names([ParamKind::Query, ParamKind::Path])
    );
    assert_eq!(
        union_members("AuthKind"),
        names([
            AuthKind::None,
            AuthKind::Basic,
            AuthKind::Bearer,
            AuthKind::ApiKey,
            AuthKind::Inherit,
            AuthKind::Other,
        ])
    );
    assert_eq!(
        union_members("BodyKind"),
        names([
            BodyKind::None,
            BodyKind::Json,
            BodyKind::Text,
            BodyKind::Xml,
            BodyKind::Form,
            BodyKind::Other,
        ])
    );
}
