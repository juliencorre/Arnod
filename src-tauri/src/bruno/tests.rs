//! Conversion d'une vraie collection Bruno (`fixtures/boutique`), règle par règle.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::convert::{Converted, OutFile, convert};
use super::read::{self, dotenv};
use super::{ItemKind, ItemStatus, ReportItem};
use crate::opencollection::schema::{self, Part};

pub(crate) fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bruno/fixtures/boutique")
}

fn converted() -> Converted {
    convert(&read::read(&fixture()).unwrap())
}

fn file<'a>(converted: &'a Converted, path: &str) -> &'a OutFile {
    converted
        .files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| {
            panic!(
                "{path} absent : {:?}",
                converted.files.iter().map(|f| &f.path).collect::<Vec<_>>()
            )
        })
}

fn items(converted: &Converted, kind: ItemKind) -> Vec<&ReportItem> {
    converted.items.iter().filter(|i| i.kind == kind).collect()
}

#[test]
fn lecture_de_la_collection() {
    let collection = read::read(&fixture()).unwrap();
    assert_eq!(collection.name, "Boutique API");
    assert!(collection.settings.is_some(), "collection.bru");
    let envs: Vec<&str> = collection
        .environments
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(envs, ["Local", "Recette"]);
    let folders: Vec<&str> = collection
        .tree
        .folders
        .iter()
        .map(|f| f.dir.as_str())
        .collect();
    assert_eq!(folders, ["Authentification", "Commandes"]);
    assert_eq!(collection.tree.requests.len(), 1, "Santé à la racine");
    // collection.bru, 2 environnements, 2 folder.bru, 7 requêtes.
    assert_eq!(collection.bru_files, 12);
    assert!(
        collection.unreadable.is_empty(),
        "{:?}",
        collection.unreadable
    );
    assert_eq!(
        collection.dotenv.get("API_KEY").map(String::as_str),
        Some("ne-pas-lire")
    );
}

#[test]
fn tout_fichier_produit_est_valide_pour_le_schema() {
    let converted = converted();
    for file in &converted.files {
        assert!(
            schema::check(file.part, &file.value).is_ok(),
            "{}",
            file.path
        );
    }
    for env in &converted.environments {
        assert!(schema::check(Part::Environment, env).is_ok(), "{env}");
    }
    let defaults = json!({ "opencollection": "1.0.0", "info": { "name": "x" }, "request": converted.defaults });
    assert!(schema::check(Part::Root, &defaults).is_ok(), "{defaults}");
}

#[test]
fn dossiers_requetes_et_ordre() {
    let converted = converted();
    assert_eq!(converted.folders, ["Authentification", "Commandes"]);
    assert_eq!(converted.counts.folders, 2);
    // Profil (GraphQL) n'est pas repris : 6 requêtes sur 7.
    assert_eq!(converted.counts.requests, 6);
    let folder = &file(&converted, "Commandes/folder.yml").value;
    assert_eq!(
        folder["info"],
        json!({ "name": "Commandes", "type": "folder", "seq": 1.0 })
    );
    assert_eq!(
        folder["request"]["headers"][0],
        json!({ "name": "x-tenant", "value": "boutique" })
    );
    assert_eq!(folder["request"]["auth"], "inherit");
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(creer["info"]["name"], "Créer une commande");
    assert_eq!(creer["info"]["seq"], 1.0);
    let lire = &file(&converted, "Commandes/lire-une-commande.yml").value;
    assert_eq!(lire["info"]["seq"], 2.0);
}

#[test]
fn requete_methode_adresse_parametres_en_tetes() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(creer["http"]["method"], "POST");
    assert_eq!(creer["http"]["url"], "{{baseUrl}}/orders?source=web");
    assert_eq!(
        creer["http"]["params"],
        json!([
            { "name": "source", "value": "web", "type": "query" },
            { "name": "debug", "value": "true", "type": "query", "disabled": true },
        ])
    );
    assert_eq!(creer["http"]["headers"][1]["value"], "{{$randomUUID}}");
    let lire = &file(&converted, "Commandes/lire-une-commande.yml").value;
    assert_eq!(
        lire["http"]["params"],
        json!([{ "name": "id", "value": "{{orderId}}", "type": "path" }])
    );
}

#[test]
fn corps_du_mode_choisi_seulement() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(creer["http"]["body"]["type"], "json");
    assert!(
        creer["http"]["body"]["data"]
            .as_str()
            .unwrap()
            .starts_with("{\n  \"customer\"")
    );
    let annuler = &file(&converted, "Commandes/annuler-une-commande.yml").value;
    assert_eq!(
        annuler["http"]["body"],
        json!({ "type": "form-urlencoded", "data": [
            { "name": "status", "value": "cancelled" },
            { "name": "reason", "value": "test", "disabled": true },
        ] })
    );
    let jeton = &file(&converted, "Authentification/obtenir-un-jeton.yml").value;
    assert_eq!(
        jeton["http"]["body"]["data"][1],
        json!({ "name": "avatar", "type": "file", "value": ["images/avatar.png"] })
    );
    let note = &file(&converted, "Authentification/note-longue.yml").value;
    assert_eq!(note["http"]["body"]["data"], "Bonjour,\n  indenté\nfin");
    assert_eq!(note["http"]["headers"][0]["value"], "ligne 1\nligne 2");
}

#[test]
fn authentification_et_heritage() {
    let converted = converted();
    assert_eq!(
        converted.defaults["auth"],
        json!({ "type": "bearer", "token": "{{token}}" })
    );
    assert_eq!(converted.defaults["headers"][1]["disabled"], true);
    assert_eq!(
        converted.defaults["variables"],
        json!([{ "name": "apiVersion", "value": "v2" }])
    );
    let auth = &file(&converted, "Authentification/folder.yml").value;
    assert_eq!(
        auth["request"]["auth"],
        json!({ "type": "basic", "username": "{{user}}", "password": "{{password}}" })
    );
    let annuler = &file(&converted, "Commandes/annuler-une-commande.yml").value;
    assert_eq!(
        annuler["http"]["auth"],
        json!({ "type": "apikey", "key": "x-api-key", "value": "{{partnerKey}}", "placement": "header" })
    );
    let sante = &file(&converted, "sante.yml").value;
    assert!(sante["http"].get("auth").is_none(), "auth: none → absente");
    // OAuth 2 : non repris, signalé avec son bloc.
    let jeton = &file(&converted, "Authentification/obtenir-un-jeton.yml").value;
    assert!(jeton["http"].get("auth").is_none());
    let oauth = items(&converted, ItemKind::Auth);
    assert_eq!(oauth.len(), 1);
    assert_eq!(oauth[0].status, ItemStatus::Skipped);
    assert_eq!(oauth[0].block.as_deref(), Some("auth:oauth2"));
    assert!(
        oauth[0]
            .code
            .as_deref()
            .unwrap()
            .contains("client_id: arnod")
    );
}

#[test]
fn verifications_avec_les_operateurs_d_arnod() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(
        creer["runtime"]["assertions"],
        json!([
            { "expression": "res.status", "operator": "eq", "value": "201" },
            { "expression": "res.body.status", "operator": "eq", "value": "\"pending\"" },
            { "expression": "res.body.id", "operator": "isDefined" },
            { "expression": "res.body.total", "operator": "gt", "value": "0" },
            { "expression": "res.headers[\"content-type\"]", "operator": "contains", "value": "json" },
            { "expression": "res.body.reference", "operator": "matches", "value": "^CMD-[0-9]+$" },
        ])
    );
    // matches : repris, signalé non évalué.
    let checks = items(&converted, ItemKind::Assertion);
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].status, ItemStatus::ToCheck);
    assert_eq!(
        checks[0].request.as_deref(),
        Some("Commandes/creer-une-commande.yml")
    );
    // 6 + 2 (Lire) + 2 (Santé).
    assert_eq!(converted.counts.assertions, 10);
}

#[test]
fn valeur_sans_operateur_comparee_par_egalite() {
    let file = super::parse::parse(
        "meta {\n  name: A\n}\nget {\n  url: x\n}\nassert {\n  res.body.a: 12\n}\n",
    )
    .unwrap();
    let collection = read::Collection {
        name: "c".into(),
        tree: read::Folder {
            requests: vec![read::Source {
                path: "a.bru".into(),
                file,
            }],
            ..read::Folder::default()
        },
        ..read::Collection::default()
    };
    let converted = convert(&collection);
    assert_eq!(
        converted.files[0].value["runtime"]["assertions"][0],
        json!({ "expression": "res.body.a", "operator": "eq", "value": "12" })
    );
}

#[test]
fn extractions_en_actions_set_variable() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(
        creer["runtime"]["actions"],
        json!([
            {
                "type": "set-variable",
                "phase": "after-response",
                "selector": { "expression": "res.body.id", "method": "jsonq" },
                "variable": { "name": "orderId", "scope": "runtime" },
            },
            {
                "type": "set-variable",
                "phase": "after-response",
                "selector": { "expression": "res.body.legacyId", "method": "jsonq" },
                "variable": { "name": "oldId", "scope": "runtime" },
                "disabled": true,
            },
        ])
    );
    assert_eq!(
        creer["runtime"]["variables"],
        json!([{ "name": "customerId", "value": "42" }])
    );
    assert_eq!(converted.counts.extractions, 2);
}

#[test]
fn valeurs_dynamiques_gardees_telles_quelles() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert!(
        creer["http"]["body"]["data"]
            .as_str()
            .unwrap()
            .contains("{{$isoTimestamp}}")
    );
    assert_eq!(converted.counts.dynamic_values, 2);
    assert!(
        converted
            .examples
            .iter()
            .any(|e| e.before == "{{$randomUUID}}")
    );
}

#[test]
fn scripts_et_tests_gardes_et_signales() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    let scripts = creer["runtime"]["scripts"].as_array().unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0]["type"], "tests");
    assert!(
        scripts[0]["code"]
            .as_str()
            .unwrap()
            .starts_with("test(\"total cohérent\"")
    );
    let folder = &file(&converted, "Commandes/folder.yml").value;
    assert_eq!(folder["request"]["scripts"][0]["type"], "before-request");
    // tests (Créer), script:pre-request (Commandes), script:post-response (Obtenir un jeton).
    assert_eq!(converted.counts.scripts, 3);
    let tests = items(&converted, ItemKind::Tests);
    assert_eq!(tests[0].status, ItemStatus::NotRun);
    assert_eq!(tests[0].name, "Créer une commande");
    assert_eq!(tests[0].place, "Commandes");
    assert_eq!(tests[0].file, "Commandes/Créer une commande.bru");
    assert!(
        tests[0]
            .code
            .as_deref()
            .unwrap()
            .contains("expect(o.total)")
    );
    let folder_script = items(&converted, ItemKind::PreRequest)[0];
    assert_eq!(
        (folder_script.name.as_str(), folder_script.place.as_str()),
        ("Commandes", "réglages du dossier")
    );
    assert_eq!(folder_script.file, "Commandes/folder.bru");
    assert_eq!(
        items(&converted, ItemKind::PostResponse)[0].name,
        "Obtenir un jeton"
    );
}

#[test]
fn reglages_et_documentation() {
    let converted = converted();
    let creer = &file(&converted, "Commandes/creer-une-commande.yml").value;
    assert_eq!(
        creer["settings"],
        json!({ "encodeUrl": true, "timeout": 5000 })
    );
    assert_eq!(
        creer["docs"],
        "Crée une commande pour le client `customerId`."
    );
    assert_eq!(
        converted.docs.as_deref(),
        Some("# Boutique API\n\nCollection de démonstration.")
    );
}

#[test]
fn environnements_et_secrets_en_variables_locales() {
    let converted = converted();
    assert_eq!(converted.counts.environments, 2);
    assert_eq!(
        converted.environments[1],
        json!({ "name": "Recette", "color": "gray", "variables": [
            { "name": "baseUrl", "value": "https://recette.boutique.dev" },
            { "name": "partnerKey", "secret": true },
            { "name": "ancien", "value": "https://old.boutique.dev", "disabled": true },
            { "name": "token", "secret": true },
            { "name": "password", "secret": true },
        ] })
    );
    assert_eq!(converted.counts.secrets, 3);
    // La valeur de .env n'est jamais écrite : elle va au coffre.
    let text = serde_json::to_string(&converted.environments).unwrap();
    assert!(!text.contains("ne-pas-lire"));
    assert_eq!(converted.secrets.len(), 1);
    assert_eq!(converted.secrets[0].name, "partnerKey");
    assert_eq!(converted.secrets[0].value, "ne-pas-lire");
    let env_items = items(&converted, ItemKind::Env);
    assert!(
        env_items
            .iter()
            .any(|i| i.status == ItemStatus::Stored && i.name == "partnerKey")
    );
    assert!(
        env_items
            .iter()
            .any(|i| i.status == ItemStatus::ToFill && i.name == "token, password")
    );
}

#[test]
fn requetes_non_prises_en_charge_signalees() {
    let converted = converted();
    let skipped = items(&converted, ItemKind::Request);
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].name, "Profil");
    assert!(skipped[0].detail.contains("GraphQL"));
    assert_eq!(skipped[0].request, None);
    // Corps multipart : repris, pas encore envoyé.
    assert!(
        items(&converted, ItemKind::Body)
            .iter()
            .any(|i| i.status == ItemStatus::ToCheck)
    );
}

#[test]
fn fichier_illisible_ne_bloque_pas_le_reste() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("bruno.json"),
        r#"{"name":"c","type":"collection"}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("casse.bru"), "meta {\n  name: A\n").unwrap();
    std::fs::write(dir.path().join("ok.bru"), "get {\n  url: x\n}\n").unwrap();
    let converted = convert(&read::read(dir.path()).unwrap());
    assert_eq!(converted.counts.requests, 1);
    let skipped = items(&converted, ItemKind::Request);
    assert_eq!(skipped[0].file, "casse.bru");
    assert!(
        skipped[0].detail.contains("ligne 1"),
        "{}",
        skipped[0].detail
    );
}

#[test]
fn exemples_de_conversion() {
    let converted = converted();
    let pairs: Vec<(&str, &str)> = converted
        .examples
        .iter()
        .map(|e| (e.before.as_str(), e.after.as_str()))
        .collect();
    assert!(
        pairs.contains(&("assert { res.status: eq 201 }", "status égal à 201")),
        "{pairs:?}"
    );
    assert!(pairs.contains(&(
        "vars:post-response { orderId: res.body.id }",
        "extrait orderId ← body.id"
    )));
}

#[test]
fn fichier_env() {
    let vars = dotenv("# commentaire\nA=1\nexport B=\"deux\"\n\nC='trois'\n=x\n");
    assert_eq!(vars.len(), 3);
    assert_eq!(vars["B"], "deux");
    assert_eq!(vars["C"], "trois");
}

#[test]
fn forme_json_du_rapport() {
    let item = &items(&converted(), ItemKind::Tests)[0].clone();
    let value: Value = serde_json::to_value(item).unwrap();
    assert_eq!(value["kind"], "tests");
    assert_eq!(value["status"], "notRun");
    assert_eq!(value["request"], "Commandes/creer-une-commande.yml");
    assert!(value.get("block").is_some());
    let counts = serde_json::to_value(converted().counts).unwrap();
    assert!(counts.get("dynamicValues").is_some());
}
