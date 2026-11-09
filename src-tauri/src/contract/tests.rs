//! Tests des contrats, règle par règle (docs/plan-contrats-services.md).

use std::fs;

use serde_json::json;

use super::conformance::{check, validate};
use super::file::{self, ContractSource};
use super::generate::{STATUS_ASSERTION, request_for, safe_name};
use super::parse::{SpecKind, load};
use super::sample::sample;
use crate::environment;
use crate::error::CommandErrorKind;
use crate::request::assertions::ResponseData;
use crate::request::file as request_file;
use crate::request::model::{ApiKeyLocation, AuthKind, BodyKind, Method, Operator};
use crate::tree::{self, TreeNode};

/// Contrat OpenAPI 3.0 : tags, références, cycle, sécurité, formulaire.
pub const PETSTORE: &str = r##"openapi: 3.0.3
info:
  title: Petstore
  version: 1.2.0
  description: Animalerie de démonstration.
servers:
  - url: "{scheme}://api.exemple.fr/v1"
    variables:
      scheme:
        default: https
tags:
  - name: animaux
  - name: magasin
paths:
  /store/inventory:
    get:
      tags: [magasin]
      summary: Inventaire
      responses:
        200:
          description: ok
          content:
            application/json:
              schema:
                type: object
                additionalProperties:
                  type: integer
  /pets:
    get:
      tags: [animaux]
      summary: Lister les animaux
      operationId: listPets
      parameters:
        - name: limit
          in: query
          schema: { type: integer, maximum: 100 }
        - name: espece
          in: query
          required: true
          schema: { type: string, enum: [chien, chat] }
        - name: X-Trace
          in: header
          schema: { type: string }
      responses:
        "200":
          description: Liste
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Pet" }
        default:
          description: Erreur
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Error" }
    post:
      tags: [animaux]
      summary: Créer un animal
      security:
        - cle: []
      requestBody:
        content:
          application/json:
            schema: { $ref: "#/components/schemas/NewPet" }
      responses:
        "201":
          description: Créé
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Pet" }
        "400":
          description: Invalide
  /pets/{petId}:
    parameters:
      - name: petId
        in: path
        required: true
        schema: { type: integer }
        example: 42
    get:
      tags: [animaux]
      operationId: showPetById
      responses:
        "200":
          description: Un animal
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Pet" }
        "404":
          description: Absent
    delete:
      tags: [animaux]
      deprecated: true
      summary: "Supprimer: un animal ?"
      responses:
        "204":
          description: Supprimé
    trace:
      responses:
        "200":
          description: Trace
  /login:
    post:
      summary: Connexion
      requestBody:
        content:
          application/x-www-form-urlencoded:
            schema:
              type: object
              properties:
                utilisateur: { type: string, example: alice }
                motDePasse: { type: string, format: password }
      responses:
        "200":
          description: ok
security:
  - jeton: []
components:
  securitySchemes:
    jeton:
      type: http
      scheme: bearer
    cle:
      type: apiKey
      in: header
      name: X-API-Key
  schemas:
    Pet:
      type: object
      required: [id, name]
      properties:
        id: { type: integer, readOnly: true }
        name: { type: string, minLength: 2 }
        tag: { type: string, nullable: true }
        parent: { $ref: "#/components/schemas/Pet" }
    NewPet:
      allOf:
        - $ref: "#/components/schemas/Pet"
        - type: object
          properties:
            puce: { type: string, format: uuid }
    Error:
      type: object
      required: [code]
      properties:
        code: { type: integer }
        message: { type: string }
"##;

/// Contrat Swagger 2.0 en JSON : hôte, corps, formulaire, authentification basique.
const SWAGGER: &str = r##"{
  "swagger": "2.0",
  "info": { "title": "Commandes", "version": "2.0.1" },
  "host": "commandes.exemple.fr",
  "basePath": "/api/",
  "schemes": ["https", "http"],
  "consumes": ["application/json"],
  "produces": ["application/json"],
  "securityDefinitions": { "basique": { "type": "basic" } },
  "security": [{ "basique": [] }],
  "paths": {
    "/orders/{id}": {
      "put": {
        "summary": "Modifier une commande",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "type": "string", "format": "uuid" },
          { "name": "body", "in": "body", "schema": { "$ref": "#/definitions/Order" } }
        ],
        "responses": { "200": { "description": "ok", "schema": { "$ref": "#/definitions/Order" } } }
      }
    },
    "/upload": {
      "post": {
        "operationId": "envoyer",
        "consumes": ["application/x-www-form-urlencoded"],
        "parameters": [
          { "name": "nom", "in": "formData", "type": "string", "required": true },
          { "name": "taille", "in": "formData", "type": "integer" }
        ],
        "responses": { "202": { "description": "accepté" } }
      }
    }
  },
  "definitions": {
    "Order": {
      "type": "object",
      "required": ["qty"],
      "properties": { "qty": { "type": "integer", "minimum": 1 }, "note": { "type": "string" } }
    }
  }
}"##;

fn response(status: u16, body: &str) -> ResponseData {
    ResponseData {
        status,
        duration_ms: 10,
        headers: Vec::new(),
        body: body.to_owned(),
    }
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    environment::init(dir.path()).unwrap();
    dir
}

// --- R1 : lecture ---

#[test]
fn r1_lit_openapi_3_en_yaml() {
    let contract = load(PETSTORE).unwrap();
    assert_eq!(contract.title, "Petstore");
    assert_eq!(contract.version, "1.2.0");
    assert_eq!(contract.kind, SpecKind::OpenApi3);
    assert_eq!(contract.spec_version, "3.0.3");
    assert_eq!(
        contract.servers,
        vec!["https://api.exemple.fr/v1".to_owned()]
    );
    // Groupes dans l'ordre de `tags`, ordre du document dans chaque groupe,
    // opérations sans tag à la fin.
    let keys: Vec<&str> = contract.operations.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "GET /pets",
            "POST /pets",
            "GET /pets/{petId}",
            "DELETE /pets/{petId}",
            "GET /store/inventory",
            "TRACE /pets/{petId}",
            "POST /login",
        ]
    );
    let delete = contract.operation("DELETE /pets/{petId}").unwrap();
    assert!(delete.deprecated);
}

#[test]
fn r1_lit_swagger_2_en_json() {
    let contract = load(SWAGGER).unwrap();
    assert_eq!(contract.kind, SpecKind::Swagger2);
    assert_eq!(contract.version, "2.0.1");
    assert_eq!(
        contract.servers,
        vec![
            "https://commandes.exemple.fr/api".to_owned(),
            "http://commandes.exemple.fr/api".to_owned()
        ]
    );
    assert_eq!(contract.operations.len(), 2);
}

#[test]
fn r1_refuse_ce_qui_n_est_pas_un_contrat() {
    for (text, expected) in [
        ("", "vide"),
        ("name: [", "YAML illisible"),
        ("{ \"a\": ", "JSON illisible"),
        ("titre: Bonjour", "champ « openapi » ou « swagger » absent"),
        ("openapi: 2.5.0\npaths: {}", "n'est pas prise en charge"),
        ("swagger: '1.2'\npaths: {}", "n'est pas prise en charge"),
        (
            "openapi: 3.0.0\ninfo: { title: x }",
            "pas d'objet « paths »",
        ),
        ("- 1\n- 2", "pas un objet"),
    ] {
        let error = load(text).unwrap_err();
        assert!(error.contains(expected), "{text:?} → {error}");
    }
    // OpenAPI 3.1 autorise un contrat sans `paths` (webhooks seuls).
    assert!(
        load("openapi: 3.1.0\ninfo: { title: x, version: '1' }")
            .unwrap()
            .operations
            .is_empty()
    );
}

// --- R2, R10 : fichiers ---

#[test]
fn r2_ajoute_le_texte_tel_quel_sous_un_nom_libre() {
    let dir = project();
    let first = file::add(dir.path(), PETSTORE).unwrap();
    assert_eq!(first, "contracts/petstore.yaml");
    assert_eq!(
        fs::read_to_string(dir.path().join(&first)).unwrap(),
        PETSTORE
    );
    assert_eq!(
        file::add(dir.path(), PETSTORE).unwrap(),
        "contracts/petstore-2.yaml"
    );
    assert_eq!(
        file::add(dir.path(), SWAGGER).unwrap(),
        "contracts/commandes.json"
    );
    let names: Vec<(String, String)> = file::list(dir.path())
        .into_iter()
        .map(|c| (c.name, c.path))
        .collect();
    assert_eq!(
        names,
        [
            (
                "Commandes".to_owned(),
                "contracts/commandes.json".to_owned()
            ),
            (
                "Petstore".to_owned(),
                "contracts/petstore-2.yaml".to_owned()
            ),
            ("Petstore".to_owned(), "contracts/petstore.yaml".to_owned()),
        ]
    );
}

#[test]
fn r1_un_texte_refuse_n_ecrit_rien() {
    let dir = project();
    let error = file::add(dir.path(), "bonjour: 1").unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert!(
        error
            .message
            .starts_with("Ce texte n'est pas un contrat lisible")
    );
    assert!(!dir.path().join("contracts").exists());
}

#[test]
fn r10_enregistre_seulement_une_source_lisible() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let changed = PETSTORE.replace("version: 1.2.0", "version: 1.3.0");
    file::save(dir.path(), &path, &changed).unwrap();
    assert_eq!(file::view(dir.path(), &path).unwrap().version, "1.3.0");

    let error = file::save(dir.path(), &path, "openapi: [").unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert_eq!(fs::read_to_string(dir.path().join(&path)).unwrap(), changed);
}

#[test]
fn r10_un_fichier_illisible_est_liste_avec_son_erreur() {
    let dir = project();
    fs::create_dir_all(dir.path().join("contracts")).unwrap();
    fs::write(dir.path().join("contracts/casse.yaml"), "openapi: [").unwrap();
    let list = file::list(dir.path());
    assert_eq!(list[0].name, "casse");
    assert!(list[0].error.as_deref().unwrap().contains("YAML illisible"));
    let view = file::view(dir.path(), "contracts/casse.yaml").unwrap();
    assert_eq!(view.source, "openapi: [");
    assert!(view.error.is_some());
    assert!(view.operations.is_empty());
}

#[test]
fn les_chemins_hors_de_contracts_sont_refuses() {
    let dir = project();
    for path in [
        "environments/local.yml",
        "../x.yaml",
        "contracts/../a.yaml",
        "a.yaml",
    ] {
        assert!(file::view(dir.path(), path).is_err(), "{path}");
    }
}

#[test]
fn le_dossier_contracts_est_reserve_dans_l_arborescence() {
    let dir = project();
    file::add(dir.path(), PETSTORE).unwrap();
    assert!(tree::read(dir.path()).unwrap().is_empty());
    let error = tree::create_folder(dir.path(), "", "Contracts").unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NameTaken);
}

#[test]
fn modele_vide_lisible() {
    let contract = load(&file::template("Mon \"service\"")).unwrap();
    assert_eq!(contract.title, "Mon 'service'");
    assert_eq!(contract.operations.len(), 1);
    assert_eq!(contract.servers, vec!["http://localhost:8080".to_owned()]);
}

#[test]
fn source_d_un_contrat_ajoute() {
    let source: ContractSource =
        serde_json::from_value(json!({ "kind": "template", "name": "Paiements" })).unwrap();
    assert_eq!(
        source,
        ContractSource::Template {
            name: "Paiements".to_owned()
        }
    );
    let source: ContractSource =
        serde_json::from_value(json!({ "kind": "url", "url": "https://x" })).unwrap();
    assert!(matches!(source, ContractSource::Url { .. }));
}

// --- R3 : références ---

#[test]
fn r3_references_locales_et_cycles() {
    let contract = load(PETSTORE).unwrap();
    // NewPet = allOf(Pet, puce) ; `id` est en lecture seule ; `parent` boucle sur Pet.
    let schema = json!({ "$ref": "#/components/schemas/NewPet" });
    let value = sample(&contract.doc, &schema);
    assert_eq!(value["name"], json!("texte"));
    assert_eq!(value["puce"], json!("00000000-0000-0000-0000-000000000001"));
    assert!(value.get("id").is_none());
    assert_eq!(value["parent"]["parent"], json!(null));
}

#[test]
fn r3_reference_externe_signalee() {
    let text = PETSTORE.replace(
        "schema: { $ref: \"#/components/schemas/NewPet\" }",
        "schema: { $ref: \"commun.yaml#/NewPet\" }",
    );
    let contract = load(&text).unwrap();
    assert!(contract.operation("POST /pets").unwrap().external_refs);
    assert!(!contract.operation("GET /pets").unwrap().external_refs);
}

// --- R5 à R7 : requêtes créées ---

#[test]
fn r6_requete_prete_a_envoyer() {
    let contract = load(PETSTORE).unwrap();
    let list = contract.operation("GET /pets").unwrap();
    let request = request_for(&contract, list, "contracts/petstore.yaml").unwrap();
    assert_eq!(request.name, "Lister les animaux");
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.url, "{{baseUrl}}/pets");
    let params: Vec<(&str, &str, bool)> = request
        .params
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str(), p.enabled))
        .collect();
    assert_eq!(params, [("limit", "1", false), ("espece", "chien", true)]);
    let headers: Vec<(&str, &str)> = request
        .headers
        .iter()
        .map(|h| (h.name.as_str(), h.value.as_str()))
        .collect();
    assert_eq!(
        headers,
        [("X-Trace", "texte"), ("Accept", "application/json")]
    );
    // Sécurité globale : jeton porteur en variable.
    assert_eq!(request.auth.kind, AuthKind::Bearer);
    assert_eq!(request.auth.token, "{{token}}");

    let show = contract.operation("GET /pets/{petId}").unwrap();
    let request = request_for(&contract, show, "contracts/petstore.yaml").unwrap();
    // Sans résumé : l'operationId ; paramètre de chemin commun, avec son exemple.
    assert_eq!(request.name, "showPetById");
    assert_eq!(request.url, "{{baseUrl}}/pets/42");
}

#[test]
fn r6_corps_json_et_formulaire() {
    let contract = load(PETSTORE).unwrap();
    let create = contract.operation("POST /pets").unwrap();
    let request = request_for(&contract, create, "c").unwrap();
    assert_eq!(request.body.kind, BodyKind::Json);
    let body: serde_json::Value = serde_json::from_str(&request.body.content).unwrap();
    assert_eq!(body["name"], json!("texte"));
    assert!(
        request
            .headers
            .iter()
            .any(|h| h.name == "Content-Type" && h.value == "application/json")
    );
    // Sécurité de l'opération : clé d'API en en-tête.
    assert_eq!(request.auth.kind, AuthKind::ApiKey);
    assert_eq!(request.auth.name, "X-API-Key");
    assert_eq!(request.auth.location, ApiKeyLocation::Header);

    let login = contract.operation("POST /login").unwrap();
    let request = request_for(&contract, login, "c").unwrap();
    assert_eq!(request.body.kind, BodyKind::Form);
    let fields: Vec<(&str, &str)> = request
        .body
        .form
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str()))
        .collect();
    assert_eq!(
        fields,
        [("utilisateur", "alice"), ("motDePasse", "{{motDePasse}}")]
    );
}

#[test]
fn r6_swagger_2_corps_formulaire_et_basic() {
    let contract = load(SWAGGER).unwrap();
    let put = contract.operation("PUT /orders/{id}").unwrap();
    let request = request_for(&contract, put, "c").unwrap();
    assert_eq!(
        request.url,
        "{{baseUrl}}/orders/00000000-0000-0000-0000-000000000001"
    );
    assert_eq!(request.body.kind, BodyKind::Json);
    assert!(request.body.content.contains("\"qty\": 1"));
    assert_eq!(request.auth.kind, AuthKind::Basic);
    assert_eq!(request.auth.username, "{{username}}");

    let upload = contract.operation("POST /upload").unwrap();
    let request = request_for(&contract, upload, "c").unwrap();
    assert_eq!(request.name, "envoyer");
    assert_eq!(request.body.kind, BodyKind::Form);
    let fields: Vec<(&str, bool)> = request
        .body
        .form
        .iter()
        .map(|p| (p.name.as_str(), p.enabled))
        .collect();
    assert_eq!(fields, [("nom", true), ("taille", false)]);
}

#[test]
fn r6_parametre_de_chemin_non_declare_devient_une_variable() {
    let text = "openapi: 3.0.0\ninfo: { title: x, version: '1' }\npaths:\n  /a/{inconnu}/b:\n    get:\n      responses: { '200': { description: ok } }\n";
    let contract = load(text).unwrap();
    let request = request_for(&contract, &contract.operations[0], "c").unwrap();
    assert_eq!(request.url, "{{baseUrl}}/a/{{inconnu}}/b");
}

#[test]
fn r7_verifications_de_la_requete_creee() {
    let contract = load(PETSTORE).unwrap();
    let create = contract.operation("POST /pets").unwrap();
    let request = request_for(&contract, create, "contracts/petstore.yaml").unwrap();
    let checks: Vec<(&str, Operator)> = request
        .assertions
        .iter()
        .map(|a| (a.id.as_str(), a.operator))
        .collect();
    assert_eq!(
        checks,
        [
            ("duree", Operator::LessThan),
            (STATUS_ASSERTION, Operator::Equals),
        ]
    );
    assert_eq!(
        request.assertions[1].value,
        Some(crate::request::model::Scalar::Int(201))
    );
    let link = request.contract.unwrap();
    assert_eq!(link.file, "contracts/petstore.yaml");
    assert_eq!(link.operation, "POST /pets");
}

#[test]
fn r5_noms_sans_caracteres_interdits() {
    assert_eq!(
        safe_name("Supprimer: un animal ?", "x"),
        "Supprimer un animal"
    );
    assert_eq!(safe_name("GET /pets/{petId}", "x"), "GET pets {petId}");
    assert_eq!(safe_name(" / ", "Opération"), "Opération");
    assert_eq!(safe_name(&"a".repeat(80), "x").chars().count(), 64);
}

#[test]
fn trace_n_est_pas_envoyable() {
    let contract = load(PETSTORE).unwrap();
    let trace = contract.operation("TRACE /pets/{petId}").unwrap();
    assert!(request_for(&contract, trace, "c").is_err());
}

// --- R4, R5, R8 : création sur disque ---

fn all_keys() -> Vec<String> {
    load(PETSTORE)
        .unwrap()
        .operations
        .into_iter()
        .map(|o| o.key)
        .collect()
}

#[test]
fn r5_requetes_rangees_par_contrat_et_tag() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let report = file::generate(dir.path(), &path, &all_keys(), None).unwrap();
    assert_eq!(report.folder, "Petstore");
    assert_eq!(
        report.created,
        [
            "Petstore/animaux/lister-les-animaux.yml",
            "Petstore/animaux/creer-un-animal.yml",
            "Petstore/animaux/showpetbyid.yml",
            "Petstore/animaux/supprimer-un-animal.yml",
            "Petstore/magasin/inventaire.yml",
            "Petstore/connexion.yml",
        ]
    );
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].operation, "TRACE /pets/{petId}");
    let request = request_file::read(&dir.path().join("Petstore/animaux/showpetbyid.yml")).unwrap();
    assert_eq!(request.contract.unwrap().operation, "GET /pets/{petId}");
    // Les requêtes créées apparaissent dans l'arborescence.
    assert!(
        matches!(&tree::read(dir.path()).unwrap()[0], TreeNode::Folder { name, request_count: 6, .. } if name == "Petstore")
    );
}

#[test]
fn r4_une_operation_ne_cree_qu_une_requete() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let keys = vec!["GET /pets".to_owned()];
    file::generate(dir.path(), &path, &keys, None).unwrap();
    // Même déplacée et renommée, la requête liée compte.
    tree::create_folder(dir.path(), "", "Ailleurs").unwrap();
    let moved = tree::move_to(
        dir.path(),
        "Petstore/animaux/lister-les-animaux.yml",
        "Ailleurs",
    )
    .unwrap();
    tree::rename(dir.path(), &moved, "Ma liste").unwrap();
    let report = file::generate(dir.path(), &path, &keys, None).unwrap();
    assert!(report.created.is_empty());
    assert_eq!(
        report.skipped[0].reason,
        "une requête existe déjà pour cette opération"
    );
    let view = file::view(dir.path(), &path).unwrap();
    let op = view
        .operations
        .iter()
        .find(|o| o.key == "GET /pets")
        .unwrap();
    assert_eq!(op.request.as_deref(), Some("Ailleurs/ma-liste.yml"));
}

#[test]
fn r5_nom_pris_numerote() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    fs::create_dir_all(dir.path().join("Petstore/animaux")).unwrap();
    tree::create_request(dir.path(), "Petstore/animaux", Some("Lister les animaux")).unwrap();
    let report = file::generate(dir.path(), &path, &["GET /pets".to_owned()], None).unwrap();
    assert_eq!(
        report.created,
        ["Petstore/animaux/lister-les-animaux-2.yml"]
    );
    let request = request_file::read(&dir.path().join(&report.created[0])).unwrap();
    assert_eq!(request.name, "Lister les animaux (2)");
}

#[test]
fn r8_base_url_vide_prend_le_premier_serveur() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let report =
        file::generate(dir.path(), &path, &["GET /pets".to_owned()], Some("local")).unwrap();
    let set = report.base_url.unwrap();
    assert_eq!(set.environment, "Local");
    assert_eq!(set.value, "https://api.exemple.fr/v1");
    let local = &environment::list(dir.path())[0];
    assert_eq!(local.variables[0].value, "https://api.exemple.fr/v1");

    // Déjà renseignée : inchangée.
    let report =
        file::generate(dir.path(), &path, &["POST /pets".to_owned()], Some("Local")).unwrap();
    assert!(report.base_url.is_none());
}

#[test]
fn r8_variable_locale_jamais_modifiee() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let mut local = environment::list(dir.path()).remove(0);
    local.variables.clear();
    local.locals = vec!["baseUrl".to_owned()];
    environment::save(dir.path(), &local, Some("Local")).unwrap();
    let report =
        file::generate(dir.path(), &path, &["GET /pets".to_owned()], Some("Local")).unwrap();
    assert!(report.base_url.is_none());
    assert!(environment::list(dir.path())[0].variables.is_empty());
}

#[test]
fn r8_serveur_relatif_ignore() {
    let dir = project();
    let text = PETSTORE.replace("\"{scheme}://api.exemple.fr/v1\"", "/v1");
    let path = file::add(dir.path(), &text).unwrap();
    let report =
        file::generate(dir.path(), &path, &["GET /pets".to_owned()], Some("Local")).unwrap();
    assert!(report.base_url.is_none());
}

// --- R9, R11 : conformité ---

#[test]
fn r9_reponse_conforme() {
    let contract = load(PETSTORE).unwrap();
    let body = r#"[{ "id": 1, "name": "Rex", "tag": null }, { "id": 2, "name": "Tom" }]"#;
    assert_eq!(check(&contract, "GET /pets", &response(200, body)), Ok(()));
    // Réponse `default` pour un statut non listé.
    assert_eq!(
        check(&contract, "GET /pets", &response(500, r#"{ "code": 500 }"#)),
        Ok(())
    );
    // Sans schéma : rien à valider.
    assert_eq!(
        check(&contract, "GET /pets/{petId}", &response(404, "")),
        Ok(())
    );
}

#[test]
fn r9_ecarts_decrits_avec_le_chemin() {
    let contract = load(PETSTORE).unwrap();
    let body = r#"[{ "id": "un", "name": "R" }, { "name": "Tom" }]"#;
    let error = check(&contract, "GET /pets", &response(200, body)).unwrap_err();
    assert_eq!(
        error,
        "body[0].id : attendu entier, reçu \"un\" ; body[0].name : au moins 2 caractère(s) attendu(s), reçu 1 ; body[1].id : champ obligatoire absent"
    );
}

#[test]
fn r9_statut_non_prevu_et_corps_absent() {
    let contract = load(PETSTORE).unwrap();
    let error = check(&contract, "POST /pets", &response(418, "")).unwrap_err();
    assert_eq!(
        error,
        "statut 418 non prévu par le contrat (prévus : 201, 400)"
    );
    let error = check(&contract, "POST /pets", &response(201, "")).unwrap_err();
    assert_eq!(error, "corps vide, le contrat prévoit un corps JSON");
    let error = check(&contract, "POST /pets", &response(201, "<pet/>")).unwrap_err();
    assert!(error.contains("pas du JSON"));
}

#[test]
fn r9_mots_cles_du_sous_ensemble() {
    let doc = json!({});
    let cases = [
        (json!({ "type": "integer" }), json!(2.0), true),
        (json!({ "type": "integer" }), json!(2.5), false),
        (json!({ "type": ["string", "null"] }), json!(null), true),
        (
            json!({ "type": "string", "nullable": true }),
            json!(null),
            true,
        ),
        (json!({ "type": "string" }), json!(null), false),
        (json!({ "enum": ["a", "b"] }), json!("c"), false),
        (json!({ "const": 3 }), json!(3), true),
        (
            json!({ "type": "object", "additionalProperties": false, "properties": { "a": {} } }),
            json!({ "a": 1, "b": 2 }),
            false,
        ),
        (
            json!({ "additionalProperties": { "type": "integer" } }),
            json!({ "x": "y" }),
            false,
        ),
        (json!({ "type": "array", "minItems": 2 }), json!([1]), false),
        (
            json!({ "type": "array", "maxItems": 1 }),
            json!([1, 2]),
            false,
        ),
        (json!({ "maxLength": 2 }), json!("abc"), false),
        (json!({ "minimum": 1 }), json!(0), false),
        (
            json!({ "maximum": 5, "exclusiveMaximum": true }),
            json!(5),
            false,
        ),
        (json!({ "exclusiveMinimum": 0 }), json!(0), false),
        (
            json!({ "anyOf": [{ "type": "string" }, { "type": "integer" }] }),
            json!(1),
            true,
        ),
        (
            json!({ "oneOf": [{ "type": "string" }, { "type": "boolean" }] }),
            json!(1),
            false,
        ),
        (
            json!({ "allOf": [{ "required": ["a"] }, { "required": ["b"] }] }),
            json!({ "a": 1 }),
            false,
        ),
        (json!(false), json!(1), false),
        (json!(true), json!(1), true),
    ];
    for (schema, value, ok) in cases {
        let errors = validate(&doc, &schema, &value);
        assert_eq!(errors.is_empty(), ok, "{schema} / {value} → {errors:?}");
    }
}

#[test]
fn r9_schema_recursif_borne() {
    let contract = load(PETSTORE).unwrap();
    let mut body = json!({ "id": 1, "name": "Rex" });
    for _ in 0..100 {
        body = json!({ "id": 1, "name": "Rex", "parent": body });
    }
    let schema = json!({ "$ref": "#/components/schemas/Pet" });
    assert!(validate(&contract.doc, &schema, &body).is_empty());
}

#[test]
fn r9_au_plus_trois_ecarts_affiches() {
    let contract = load(PETSTORE).unwrap();
    let body = r#"[{}, {}, {}]"#;
    let error = check(&contract, "GET /pets", &response(200, body)).unwrap_err();
    assert!(error.ends_with("(+3 autres écarts)"), "{error}");
}

#[test]
fn r11_operation_disparue_ou_contrat_absent() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    let report = file::generate(dir.path(), &path, &["GET /pets".to_owned()], None).unwrap();
    let request = request_file::read(&dir.path().join(&report.created[0])).unwrap();
    let ok = file::conformance_result(
        dir.path(),
        request.contract.as_ref(),
        "contrat",
        &response(200, "[]"),
    );
    assert!(ok.passed);

    let without = PETSTORE.replace(
        "    get:\n      tags: [animaux]\n      summary: Lister les animaux",
        "    patch:\n      tags: [animaux]\n      summary: Lister les animaux",
    );
    file::save(dir.path(), &path, &without).unwrap();
    let missing = file::conformance_result(
        dir.path(),
        request.contract.as_ref(),
        "contrat",
        &response(200, "[]"),
    );
    assert!(!missing.passed);
    assert_eq!(
        missing.message.as_deref(),
        Some("l'opération « GET /pets » n'est plus dans le contrat")
    );
    let view = file::view(dir.path(), &path).unwrap();
    assert_eq!(view.orphans.len(), 1);
    assert_eq!(view.orphans[0].name, "Lister les animaux");

    file::delete(dir.path(), &path, true).unwrap();
    let gone = file::conformance_result(
        dir.path(),
        request.contract.as_ref(),
        "contrat",
        &response(200, "[]"),
    );
    assert_eq!(
        gone.message.as_deref(),
        Some("le contrat contracts/petstore.yaml est introuvable")
    );
    let none = file::conformance_result(dir.path(), None, "contrat", &response(200, "[]"));
    assert_eq!(
        none.message.as_deref(),
        Some("aucun contrat associé à cette requête")
    );
}

#[test]
fn r12_requetes_liees_dans_l_ordre_de_l_arborescence() {
    let dir = project();
    let path = file::add(dir.path(), PETSTORE).unwrap();
    file::generate(
        dir.path(),
        &path,
        &["POST /login".to_owned(), "GET /pets".to_owned()],
        None,
    )
    .unwrap();
    tree::create_request(dir.path(), "", Some("Hors contrat")).unwrap();
    assert_eq!(
        file::linked_paths(dir.path(), &path),
        [
            "Petstore/animaux/lister-les-animaux.yml",
            "Petstore/connexion.yml"
        ]
    );
}

#[test]
fn unions_typescript_concordantes() {
    use crate::ts_contract::{serialized_name, union_members};
    use std::collections::BTreeSet;

    fn names<T: serde::Serialize>(values: impl IntoIterator<Item = T>) -> BTreeSet<String> {
        values.into_iter().map(serialized_name).collect()
    }
    assert_eq!(
        union_members("SpecKind"),
        names([SpecKind::OpenApi3, SpecKind::Swagger2])
    );
    assert_eq!(
        union_members("AssertionOperator"),
        names(
            crate::opencollection::request::OPERATORS
                .iter()
                .map(|(op, _)| *op)
        )
    );
}
