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
use crate::request::model::{Extraction, Method, Operator, Origin, ParamKind, Request, Scalar};
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
    // La durée par défaut se reconnaît à la relecture ; une autre limite est « manuelle » (VT4).
    let mut request = file::read(&full).unwrap();
    assert_eq!(request.assertions[0].origin, Origin::Default);
    request.assertions[0].value = Some(Scalar::Int(800));
    file::write(&full, &request).unwrap();
    let request = file::read(&full).unwrap();
    assert_eq!(request.assertions[0].origin, Origin::Manual);
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
    assert_eq!(request.settings.timeout_ms, Some(5000));
    assert!(
        request.preserved.contains(&"scripts".to_owned()),
        "{:?}",
        request.preserved
    );
    // Documentation et réponses attendues sont gérées (R9 de la configuration d'un endpoint).
    assert_eq!(request.preserved, ["scripts"]);
    assert_eq!(request.docs, "# Commande\nCrée une commande.");
    assert_eq!(request.examples[0].status, Some(201));
    assert_eq!(request.params[1].description, "Trace côté serveur");
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
    assert_eq!(request.settings.timeout_ms, Some(5000));
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
    {
        use crate::request::model::ExampleBodyKind as K;
        assert_eq!(
            union_members("ExampleBodyKind"),
            names([K::None, K::Json, K::Text, K::Xml, K::Html, K::Binary])
        );
    }
    assert_eq!(
        union_members("AuthKind"),
        names([
            AuthKind::None,
            AuthKind::Basic,
            AuthKind::Bearer,
            AuthKind::ApiKey,
            AuthKind::Inherit,
            AuthKind::Other,
            AuthKind::OAuth2,
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

// Configuration d'un endpoint (docs/plan-configuration-endpoint.md).

#[test]
fn ep_description_etiquettes_docs_et_descriptions_ecrites() {
    use crate::request::model::{ContractLink, Pair, Request};
    let dir = project();
    let path = dir.path().join("lire.yml");
    let mut request = Request::new("Lire");
    request.description = "Lit une commande.".to_owned();
    request.tags = vec!["commandes".to_owned(), "critique".to_owned()];
    request.docs = "## Lire\n\nLe client doit exister.".to_owned();
    request.url = "{{baseUrl}}/orders/{id}".to_owned();
    request.params = vec![Pair {
        description: "Envoie l'e-mail".to_owned(),
        ..Pair::new("notify", "true")
    }];
    request.headers = vec![Pair {
        description: "Évite les doublons".to_owned(),
        ..Pair::new("Idempotency-Key", "{{cle}}")
    }];
    request.contract = Some(ContractLink {
        file: "contracts/boutique.yaml".to_owned(),
        operation: "GET /orders/{id}".to_owned(),
    });
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    assert_eq!(
        value.pointer("/info/description"),
        Some(&json!("Lit une commande."))
    );
    assert_eq!(
        value.pointer("/info/tags"),
        Some(&json!([
            "commandes",
            "critique",
            "contract:contracts/boutique.yaml#GET /orders/{id}"
        ]))
    );
    assert_eq!(
        value.pointer("/docs"),
        Some(&json!("## Lire\n\nLe client doit exister."))
    );
    assert_eq!(
        value.pointer("/http/headers/0/description"),
        Some(&json!("Évite les doublons"))
    );
    // R1 : le paramètre de chemin « id » est ajouté, avant ceux de requête.
    assert_eq!(
        value.pointer("/http/params"),
        Some(&json!([
            { "name": "id", "value": "", "type": "path" },
            { "name": "notify", "value": "true", "description": "Envoie l'e-mail", "type": "query" }
        ]))
    );
    let back = file::read(&path).unwrap();
    assert_eq!(back.tags, ["commandes", "critique"]);
    assert_eq!(back.description, "Lit une commande.");
    assert!(back.contract.is_some());
}

#[test]
fn ep_r2_variantes_de_corps() {
    use crate::request::model::{Body, BodyKind, BodyVariant, Request};
    let dir = project();
    let path = dir.path().join("creer.yml");
    let mut request = Request::new("Créer");
    request.method = Method::Post;
    request.body = Body {
        kind: BodyKind::Json,
        variants: vec![
            BodyVariant {
                title: "Complète".to_owned(),
                content: "{\"coupon\": \"BIENVENUE\"}".to_owned(),
                ..BodyVariant::default()
            },
            BodyVariant {
                title: "Sans coupon".to_owned(),
                content: "{}".to_owned(),
                ..BodyVariant::default()
            },
        ],
        selected: 1,
        ..Body::default()
    };
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    assert_eq!(
        value.pointer("/http/body"),
        Some(&json!([
            { "title": "Complète", "body": { "type": "json", "data": "{\"coupon\": \"BIENVENUE\"}" } },
            { "title": "Sans coupon", "selected": true, "body": { "type": "json", "data": "{}" } }
        ]))
    );
    let back = file::read(&path).unwrap();
    assert_eq!(back.body.selected, 1);
    assert_eq!(back.body.content(), "{}");

    // Seule la variante sélectionnée part.
    let mut variables = Variables::default();
    variables
        .values
        .insert("baseUrl".into(), "https://api.test".into());
    let prepared = prepare(&back, &variables).unwrap();
    assert_eq!(prepared.body.as_deref(), Some("{}"));

    // Une seule variante sans nom : corps simple.
    let mut simple = back.clone();
    simple.body = Body::raw(BodyKind::Text, "bonjour");
    file::write(&path, &simple).unwrap();
    assert_eq!(
        read(&path).pointer("/http/body"),
        Some(&json!({ "type": "text", "data": "bonjour" }))
    );
}

#[test]
fn ep_r2_variantes_de_types_differents_conservees() {
    let dir = project();
    let path = dir.path().join("mixte.yml");
    let text = "info:\n  name: Mixte\n  type: http\nhttp:\n  method: POST\n  url: https://h/x\n  body:\n    - title: A\n      body:\n        type: json\n        data: \"{}\"\n    - title: B\n      body:\n        type: text\n        data: b\n";
    fs::write(&path, text).unwrap();
    let mut request = file::read(&path).unwrap();
    assert_eq!(request.body.kind, crate::request::model::BodyKind::Other);
    request.url = "https://h/y".to_owned();
    file::write(&path, &request).unwrap();
    assert_eq!(
        read(&path).pointer("/http/body/1/body/type"),
        Some(&json!("text"))
    );
}

#[test]
fn ep_r3_reglages_propres_ou_herites() {
    use crate::request::model::{Request, Settings};
    let dir = project();
    let path = dir.path().join("r.yml");
    let mut request = Request::new("R");
    request.settings = Settings {
        timeout_ms: Some(5000),
        follow_redirects: Some(false),
        max_redirects: Some(3),
        encode_url: None,
    };
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    assert_eq!(
        read(&path).get("settings"),
        Some(&json!({ "timeout": 5000, "followRedirects": false, "maxRedirects": 3 }))
    );
    // « inherit » se lit comme hérité.
    fs::write(
        &path,
        "info:\n  name: R\n  type: http\nhttp:\n  method: GET\n  url: https://h\nsettings:\n  timeout: inherit\n  encodeUrl: false\n",
    )
    .unwrap();
    let back = file::read(&path).unwrap();
    assert_eq!(back.settings.timeout_ms, None);
    assert_eq!(back.settings.encode_url, Some(false));

    // R5 : adresse non encodée, les paramètres partent tels quels.
    let mut sent = back.clone();
    sent.params = vec![crate::request::model::Pair::new("q", "a b")];
    let prepared = prepare(&sent, &Variables::default()).unwrap();
    assert_eq!(prepared.url, "https://h?q=a b");
}

#[test]
fn ep_r6_reponses_attendues() {
    use crate::request::model::{Example, ExampleBodyKind, ResponseHeader};
    let dir = project();
    let path = dir.path().join("commande.yml");
    // Exemple existant avec une partie « request », conservée.
    fs::write(
        &path,
        "info:\n  name: Commande\n  type: http\nhttp:\n  method: POST\n  url: https://h/orders\nexamples:\n  - name: Créée\n    request:\n      url: https://h/orders\n      method: POST\n    response:\n      status: 201\n",
    )
    .unwrap();
    let mut request = file::read(&path).unwrap();
    request.examples[0].description = "La commande est enregistrée.".to_owned();
    request.examples[0].body_kind = ExampleBodyKind::Json;
    request.examples[0].body = "{\n  \"id\": 1\n}".to_owned();
    request.examples.push(Example {
        name: "Erreur de validation".to_owned(),
        status: Some(400),
        status_text: "Bad Request".to_owned(),
        headers: vec![ResponseHeader {
            name: "Content-Type".to_owned(),
            value: "application/json".to_owned(),
        }],
        ..Example::default()
    });
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    assert_eq!(
        value.pointer("/examples/0/request/method"),
        Some(&json!("POST"))
    );
    assert_eq!(
        value.pointer("/examples/0/response"),
        Some(&json!({ "status": 201, "body": { "type": "json", "data": "{\n  \"id\": 1\n}" } }))
    );
    assert_eq!(
        value.pointer("/examples/1"),
        Some(&json!({
            "name": "Erreur de validation",
            "response": {
                "status": 400,
                "statusText": "Bad Request",
                "headers": [{ "name": "Content-Type", "value": "application/json" }]
            }
        }))
    );
    let back = file::read(&path).unwrap();
    assert_eq!(back.examples, request.examples);

    // Toutes supprimées : plus d'« examples ».
    let mut none = back;
    none.examples.clear();
    file::write(&path, &none).unwrap();
    assert!(read(&path).get("examples").is_none());
}

#[test]
fn ep_r8_endpoint_d_un_contrat_valide_avec_ses_exemples() {
    let dir = project();
    let path = contract_file::add(dir.path(), CONTRACT).unwrap();
    let report =
        contract_file::generate(dir.path(), &path, &["GET /items/{id}".to_owned()], None).unwrap();
    let full = dir.path().join(&report.created[0]);
    assert_valid(Part::HttpRequest, &full);
    let request = file::read(&full).unwrap();
    assert_eq!(request.url, "{{baseUrl}}/items/:id");
    assert_eq!(request.params[0].kind, ParamKind::Path);
    assert_eq!(request.params[0].value, "7");
    assert_eq!(request.examples[0].status, Some(200));
    assert_eq!(request.examples[0].name, "ok");
}

// Lot E (docs/plan-mvp-e-tests-etapes.md, R8) : extractions lues et écrites,
// les autres actions conservées.

const ACTIONS: &str = r#"info:
  name: Créer le client
  type: http
http:
  method: POST
  url: "{{baseUrl}}/customers"
runtime:
  actions:
    - type: set-variable
      phase: before-request
      selector:
        expression: res.body.avant
        method: jsonq
      variable:
        name: avant
        scope: request
    - type: set-variable
      description: Identifiant du client créé
      phase: after-response
      selector:
        expression: res.body.id
        method: jsonq
      variable:
        name: client.id
        scope: collection
    - type: set-variable
      phase: after-response
      selector:
        expression: res.headers["location"]
        method: jsonq
      variable:
        name: lieu
        scope: runtime
      disabled: true
"#;

#[test]
fn lot_e_r8_extractions_lues_ecrites_et_autres_actions_gardees() {
    let dir = project();
    let path = dir.path().join("creer.yml");
    fs::write(&path, ACTIONS).unwrap();
    let mut request = file::read(&path).unwrap();
    assert_eq!(
        request.extractions,
        [
            Extraction {
                name: "client.id".into(),
                source: "body.id".into(),
                enabled: true
            },
            Extraction {
                name: "lieu".into(),
                source: "headers.location".into(),
                enabled: false
            },
        ]
    );
    // L'action avant la requête n'est pas une extraction : seule elle reste « conservée ».
    assert_eq!(request.preserved, ["actions"]);

    request.extractions[1].enabled = true;
    request.extractions.push(Extraction {
        name: "ref".into(),
        source: "request.body.externalRef".into(),
        enabled: true,
    });
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    let actions = value.pointer("/runtime/actions").unwrap();
    assert_eq!(actions.as_array().unwrap().len(), 4);
    assert_eq!(actions[0]["phase"], json!("before-request"));
    assert_eq!(
        actions[1],
        json!({
            "type": "set-variable",
            "description": "Identifiant du client créé",
            "phase": "after-response",
            "selector": { "expression": "res.body.id", "method": "jsonq" },
            "variable": { "name": "client.id", "scope": "collection" }
        })
    );
    assert!(actions[2].get("disabled").is_none());
    assert_eq!(
        actions[3]["selector"]["expression"],
        json!("req.body.externalRef")
    );
    assert_eq!(actions[3]["variable"]["scope"], json!("runtime"));
    assert_eq!(file::read(&path).unwrap().extractions, {
        let mut list = request.extractions.clone();
        list[1].enabled = true;
        list
    });
}

#[test]
fn lot_e_r8_sans_extraction_les_actions_disparaissent_et_seq_est_ecrit() {
    let dir = project();
    let path = dir.path().join("simple.yml");
    let mut request = Request::new("Étape 1");
    request.extractions.push(Extraction {
        name: "id".into(),
        source: "body.id".into(),
        enabled: true,
    });
    request.seq = Some(2.0);
    file::write(&path, &request).unwrap();
    let value = read(&path);
    assert_eq!(value.pointer("/info/seq"), Some(&json!(2)));
    assert!(file::read(&path).unwrap().preserved.is_empty());
    request.extractions.clear();
    request.seq = None;
    file::write(&path, &request).unwrap();
    let value = read(&path);
    assert!(value.pointer("/runtime/actions").is_none());
    // Sans rang dans le brouillon, celui du fichier reste.
    assert_eq!(value.pointer("/info/seq"), Some(&json!(2)));
    assert_eq!(file::read(&path).unwrap().seq, Some(2.0));
}

#[test]
fn lot_e_expressions_des_extractions() {
    use crate::opencollection::request::{extraction_expression, extraction_source};
    for (source, expression) in [
        ("body.id", "res.body.id"),
        ("headers.Location", "res.headers[\"location\"]"),
        ("status", "res.status"),
        ("request.body.ref", "req.body.ref"),
        ("request.headers.X-Ref", "req.headers[\"x-ref\"]"),
    ] {
        assert_eq!(extraction_expression(source), expression);
    }
    assert_eq!(
        extraction_source("req.body.items[0].id"),
        "request.body.items[0].id"
    );
    // Expressions écrites par l'import Bruno (lot C), telles quelles.
    for (expression, source) in [
        ("res.body.token", "body.token"),
        ("res.body.items[0].id", "body.items[0].id"),
        ("res.headers['x-request-id']", "headers.x-request-id"),
        ("res.status", "status"),
        ("res(\"data.id\")", "body.data.id"),
        ("res('items[0].id')", "body.items[0].id"),
        ("res.cookies.x", "res.cookies.x"),
    ] {
        assert_eq!(extraction_source(expression), source, "{expression}");
    }
    assert_eq!(
        extraction_source("res.headers[\"location\"]"),
        "headers.location"
    );
}

#[test]
fn lot_e_extractions_de_bruno_lues_et_appliquees() {
    use crate::request::assertions::ResponseData;
    use crate::request::extract;
    let dir = project();
    let path = dir.path().join("jeton.yml");
    fs::write(
        &path,
        r#"info:
  name: Jeton
  type: http
  seq: 2
http:
  method: POST
  url: "{{baseUrl}}/login"
runtime:
  actions:
    - type: set-variable
      phase: after-response
      selector:
        expression: res.body.token
        method: jsonq
      variable:
        name: token
        scope: runtime
    - type: set-variable
      phase: after-response
      selector:
        expression: res("data.id")
        method: jsonq
      variable:
        name: id
        scope: runtime
    - type: set-variable
      phase: after-response
      selector:
        expression: res.headers['x-request-id']
        method: jsonq
      variable:
        name: trace
        scope: runtime
    - type: set-variable
      phase: after-response
      selector:
        expression: res.cookies.session
        method: jsonq
      variable:
        name: session
        scope: runtime
"#,
    )
    .unwrap();
    let request = file::read(&path).unwrap();
    assert_eq!(request.seq, Some(2.0));
    let response = ResponseData {
        status: 200,
        duration_ms: 1,
        headers: vec![("X-Request-Id".into(), "r-9".into())],
        body: r#"{"token":"t0k","data":{"id":7}}"#.into(),
    };
    let sent = ResponseData {
        status: 0,
        duration_ms: 0,
        headers: Vec::new(),
        body: String::new(),
    };
    let values = extract::apply(&request.extractions, &response, &sent);
    let got: Vec<Option<&str>> = values.iter().map(|v| v.value.as_deref()).collect();
    assert_eq!(got, [Some("t0k"), Some("7"), Some("r-9"), None]);
    assert!(
        values[3]
            .message
            .as_deref()
            .unwrap()
            .contains("non reconnue")
    );
    // Réécrite telle quelle si elle n'est pas modifiée, et conforme au schéma.
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let value = read(&path);
    assert_eq!(
        value.pointer("/runtime/actions/3/selector/expression"),
        Some(&json!("res.cookies.session"))
    );
}

// Lot H - Réseau et authentification (docs/plan-mvp-h-reseau-auth.md, R8).

#[test]
fn lot_h_r8_oauth2_identifiants_client_aller_retour_conforme() {
    use crate::request::model::{Auth, AuthKind, CredentialsPlacement, OAuth2, TokenPlacement};
    let dir = project();
    let path = dir.path().join("commandes.yml");
    let mut request = Request::new("Lister");
    request.auth = Auth {
        kind: AuthKind::OAuth2,
        oauth2: OAuth2 {
            access_token_url: "{{authUrl}}/oauth/token".into(),
            client_id: "{{oauthClientId}}".into(),
            client_secret: "{{oauth2ClientSecret}}".into(),
            scope: "orders:read orders:write".into(),
            credentials: CredentialsPlacement::Body,
            token_placement: TokenPlacement::Query,
            token_name: "access_token".into(),
        },
        ..Auth::default()
    };
    file::write(&path, &request).unwrap();
    assert_valid(Part::HttpRequest, &path);
    let written = read(&path);
    let auth = written.pointer("/http/auth").unwrap();
    assert_eq!(auth["type"], "oauth2");
    assert_eq!(auth["flow"], "client_credentials");
    assert_eq!(auth["accessTokenUrl"], "{{authUrl}}/oauth/token");
    assert_eq!(auth["credentials"]["placement"], "body");
    assert_eq!(auth["tokenConfig"]["placement"]["query"], "access_token");
    assert_eq!(file::read(&path).unwrap().auth, request.auth);

    // Un autre flux OAuth 2 reste conservé, non géré (R8).
    let other = super::request::auth_from(Some(&json!({
        "type": "oauth2",
        "flow": "password",
        "accessTokenUrl": "https://a.test/token",
    })));
    assert_eq!(other.kind, AuthKind::Other);
    assert_eq!(other.other, "oauth2");
}
