//! Règles du catalogue des étapes (docs/plan-accueil-non-classes.md, R1 à R3).

use std::collections::BTreeMap;

use super::definition::{self, Engine, Family, FieldKind, Rule};
use super::*;
use crate::request::model::{BodyKind, Method};

fn rest() -> Definition {
    find("rest").expect("définition REST intégrée")
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn with(mut definition: Definition, change: impl FnOnce(&mut Definition)) -> Result<(), String> {
    change(&mut definition);
    definition::validate(&definition)
}

// ------------------------------------------------------------------ R1

#[test]
fn r1_la_definition_rest_integree_est_valide() {
    let catalog = catalog();
    assert_eq!(catalog.natures.len(), 1);
    let nature = &catalog.natures[0];
    assert_eq!(nature.definition.id, "rest");
    assert_eq!(nature.definition.family, Family::Calls);
    assert_eq!(nature.definition.action, "Envoyer");
    assert_eq!(
        nature.definition.engine,
        Engine::Builtin {
            id: "http".to_owned(),
            abi: 1
        }
    );
    assert_eq!(nature.origin, Origin::Builtin);
    assert!(!catalog.source.synced);
}

#[test]
fn r1_format_plus_recent_refuse() {
    let error = with(rest(), |d| d.format = 2).unwrap_err();
    assert!(error.contains("format 2"), "{error}");
}

#[test]
fn r1_identifiant_et_version_verifies() {
    assert!(with(rest(), |d| d.id = "Rest".to_owned()).is_err());
    assert!(with(rest(), |d| d.version = "1.0".to_owned()).is_err());
    assert!(definition::is_version("10.2.30"));
}

#[test]
fn r1_moteur_inconnu_ou_webassembly_refuse() {
    let unknown = with(rest(), |d| {
        d.engine = Engine::Builtin {
            id: "kafka".to_owned(),
            abi: 1,
        }
    });
    assert!(unknown.unwrap_err().contains("inconnu"));
    let wasm = with(rest(), |d| {
        d.engine = Engine::Wasm {
            module: "engine.wasm".to_owned(),
            sha256: "00".to_owned(),
            abi: 1,
        }
    });
    assert!(wasm.unwrap_err().contains("WebAssembly"));
}

#[test]
fn r1_champs_uniques_et_choix_coherents() {
    let duplicate = with(rest(), |d| d.form[1].key = "method".to_owned());
    assert!(duplicate.unwrap_err().contains("en double"));
    let subject = with(rest(), |d| d.form[0].key = "url".to_owned());
    assert!(subject.is_err(), "le champ principal ne se répète pas");
    let default = with(rest(), |d| d.form[0].default = Some("TRACE".to_owned()));
    assert!(default.unwrap_err().contains("par défaut"));
    let empty = with(rest(), |d| d.form[0].options.clear());
    assert!(empty.unwrap_err().contains("pas d'option"));
}

#[test]
fn r1_visible_when_vers_un_choix_place_avant() {
    let after = with(rest(), |d| d.form.swap(0, 2));
    assert!(after.unwrap_err().contains("placé avant"));
    let value = with(rest(), |d| {
        if let Some(condition) = &mut d.form[2].visible_when {
            condition.values.push("TRACE".to_owned());
        }
    });
    assert!(value.unwrap_err().contains("valeur inconnue"));
}

#[test]
fn r1_regles_de_reconnaissance_verifiees() {
    let no_scheme = with(rest(), |d| {
        d.detect = vec![Rule::Url {
            schemes: Vec::new(),
            method_field: None,
        }]
    });
    assert!(no_scheme.is_err());
    let not_choice = with(rest(), |d| {
        d.detect = vec![Rule::Url {
            schemes: vec!["https".to_owned()],
            method_field: Some("body".to_owned()),
        }]
    });
    assert!(not_choice.unwrap_err().contains("n'est pas un choix"));
}

#[test]
fn r1_definition_illisible() {
    assert!(definition::parse("{").unwrap_err().contains("illisible"));
}

#[test]
fn familles_identiques_cote_typescript() {
    let rust: std::collections::BTreeSet<String> = Family::ALL
        .into_iter()
        .map(crate::ts_contract::serialized_name)
        .collect();
    assert_eq!(rust, crate::ts_contract::union_members("StepFamily"));
}

#[test]
fn natures_de_champ_identiques_cote_typescript() {
    let rust: std::collections::BTreeSet<String> =
        [FieldKind::Choice, FieldKind::Text, FieldKind::Multiline]
            .into_iter()
            .map(crate::ts_contract::serialized_name)
            .collect();
    assert_eq!(rust, crate::ts_contract::union_members("StepFieldKind"));
}

#[test]
fn forme_json_d_une_nature() {
    let json = serde_json::to_value(catalog()).unwrap();
    let nature = &json["natures"][0];
    assert_eq!(nature["id"], "rest");
    assert_eq!(nature["origin"], "builtin");
    assert_eq!(nature["family"], "calls");
    assert_eq!(nature["engine"]["kind"], "builtin");
    assert_eq!(nature["subject"]["key"], "url");
    assert_eq!(nature["form"][2]["visibleWhen"]["in"][0], "POST");
    assert_eq!(nature["detect"][1]["kind"], "curl");
    assert_eq!(json["source"]["name"], "arnod-steps");
}

// ------------------------------------------------------------------ R2

#[test]
fn r2_une_adresse_http_est_reconnue_rest() {
    let found = detect("  https://api.boutique.dev/products  ").unwrap();
    assert_eq!(found.nature, "rest");
    assert_eq!(found.subject, "https://api.boutique.dev/products");
    assert!(found.values.is_empty());
    assert_eq!(detect("http://localhost:8080").unwrap().nature, "rest");
}

#[test]
fn r2_la_methode_en_tete_est_reprise() {
    let found = detect("post https://api.boutique.dev/orders").unwrap();
    assert_eq!(found.subject, "https://api.boutique.dev/orders");
    assert_eq!(found.values, map(&[("method", "POST")]));
}

#[test]
fn r2_autre_texte_sans_nature() {
    for input in [
        "",
        "   ",
        "api.boutique.dev/products",
        "ftp://serveur/fichier",
        "SELECT * FROM orders",
        "https://",
        "https:///chemin",
        "TRACE https://api.boutique.dev",
    ] {
        assert_eq!(detect(input), None, "{input:?}");
    }
}

#[test]
fn r2_la_regle_la_plus_precise_l_emporte() {
    let mut sql = rest();
    sql.id = "sql".to_owned();
    sql.detect = vec![Rule::Keywords {
        words: vec!["select".to_owned()],
    }];
    let mut kafka = rest();
    kafka.id = "kafka".to_owned();
    kafka.detect = vec![Rule::Prefix {
        text: "https://kafka".to_owned(),
    }];
    let all = [rest(), sql, kafka];
    assert_eq!(
        detect::detect(&all, "SELECT 1").map(|d| d.nature),
        Some("sql".to_owned())
    );
    assert_eq!(
        detect::detect(&all, "https://kafka.local").map(|d| d.nature),
        Some("kafka".to_owned())
    );
    assert_eq!(
        detect::detect(&all, "https://api.local").map(|d| d.nature),
        Some("rest".to_owned())
    );
}

// ------------------------------------------------------------------ R3

#[test]
fn r3_defauts_appliques_et_champs_masques_ignores() {
    let values = values::normalize(&rest(), &map(&[("body", "{}"), ("other", "x")])).unwrap();
    assert_eq!(values, map(&[("method", "GET"), ("headers", "")]));
}

#[test]
fn r3_champs_visibles_pour_un_post() {
    let values = values::normalize(
        &rest(),
        &map(&[("method", "post"), ("body", "{\"a\":1}\n")]),
    )
    .unwrap();
    assert_eq!(
        values,
        map(&[
            ("method", "POST"),
            ("headers", ""),
            ("bodyType", "JSON"),
            ("body", "{\"a\":1}")
        ])
    );
}

#[test]
fn r3_option_inconnue_refusee() {
    let error = values::normalize(&rest(), &map(&[("method", "TRACE")])).unwrap_err();
    assert!(error.contains("Méthode"), "{error}");
}

#[test]
fn r3_requete_rest_d_un_essai() {
    let values = values::normalize(
        &rest(),
        &map(&[("method", "PUT"), ("bodyType", "XML"), ("body", "<a/>")]),
    )
    .unwrap();
    let request = http::request("Essai 1", " https://api.local/a?b=1 ", &values).unwrap();
    assert_eq!(request.name, "Essai 1");
    assert_eq!(request.method, Method::Put);
    assert_eq!(request.url, "https://api.local/a?b=1");
    assert_eq!(request.body.kind, BodyKind::Xml);
    assert_eq!(request.body.content(), "<a/>");
}

#[test]
fn r3_adresse_invalide_refusee() {
    let values = values::normalize(&rest(), &BTreeMap::new()).unwrap();
    assert!(http::request("Essai 1", "api.local", &values).is_err());
    assert!(http::request("Essai 1", "ftp://api.local", &values).is_err());
}

// ------------------------------------------------------------------ cURL collée
// (docs/plan-mvp-b-saisie-curl.md, R4 à R6)

#[test]
fn r4_une_commande_curl_est_une_requete_rest() {
    let detection = detect(
        "curl -X POST https://api.boutique.dev/orders -H 'Content-Type: application/json' -H 'Authorization: Bearer eyJ.x' -d '{\"a\":1}' --compressed",
    )
    .unwrap();
    assert_eq!(detection.nature, "rest");
    assert_eq!(detection.subject, "https://api.boutique.dev/orders");
    assert_eq!(
        detection.values,
        map(&[
            ("method", "POST"),
            (
                "headers",
                "Content-Type: application/json\nAuthorization: Bearer {{token}}"
            ),
            ("bodyType", "JSON"),
            ("body", "{\"a\":1}"),
        ])
    );
    let curl = detection.curl.unwrap();
    assert_eq!(curl.secrets.len(), 1);
    assert_eq!(curl.secrets[0].name, "token");
    assert_eq!(curl.secrets[0].value, "eyJ.x");
    assert_eq!(curl.ignored[0].option, "--compressed");
}

#[test]
fn r4_curl_sans_corps_et_forme_json() {
    let detection = detect("curl https://api.local/x").unwrap();
    assert_eq!(detection.values, map(&[("method", "GET")]));
    let json = serde_json::to_value(&detection).unwrap();
    assert_eq!(
        json["curl"],
        serde_json::json!({ "secrets": [], "ignored": [] })
    );
    // Une adresse seule n'a pas de champ `curl`.
    let url = serde_json::to_value(detect("https://api.local/x").unwrap()).unwrap();
    assert!(url.get("curl").is_none());
    // Un texte qui ressemble mais n'est pas une commande lisible : rien de reconnu.
    assert!(detect("curl 'https://api.local").is_none());
}

#[test]
fn r4_regle_curl_verifiee() {
    let missing = with(rest(), |d| d.form.retain(|f| f.key != "headers"));
    assert!(missing.unwrap_err().contains("curl"));
}

#[test]
fn r4_en_tetes_du_formulaire() {
    let values = values::normalize(
        &rest(),
        &map(&[("headers", "Accept: text/plain\n# X-Off: 1\n")]),
    )
    .unwrap();
    let request = http::request("Essai 1", "https://api.local", &values).unwrap();
    assert_eq!(request.headers.len(), 2);
    assert_eq!(request.headers[0].name, "Accept");
    assert!(!request.headers[1].enabled);
    let bad = values::normalize(&rest(), &map(&[("headers", "Accept text/plain")])).unwrap();
    let error = http::request("Essai 1", "https://api.local", &bad).unwrap_err();
    assert!(error.starts_with("en-têtes, ligne 1 :"), "{error}");
}

#[test]
fn r5_secrets_remplaces_a_l_envoi_seulement() {
    let values = values::normalize(
        &rest(),
        &map(&[("headers", "Authorization: Bearer {{token}}")]),
    )
    .unwrap();
    let request = http::request("Essai 1", "https://api.local", &values).unwrap();
    let sent = http::with_secrets(&request, &map(&[("token", "s3cret")]));
    assert_eq!(sent.headers[0].value, "Bearer s3cret");
    assert_eq!(request.headers[0].value, "Bearer {{token}}");
}
