//! Règles de la saisie, une par une (`docs/plan-mvp-b-saisie-curl.md`, R1 à R3, R11 à R13).

use std::collections::BTreeMap;

use super::*;
use crate::environment::{Environment, Variable};
use crate::http::USER_AGENT;
use crate::local::LocalVar;
use crate::request::model::*;

fn env(name: &str, shared: &[(&str, &str)], locals: &[&str]) -> Environment {
    Environment {
        name: name.to_owned(),
        color: "gray".to_owned(),
        protected: false,
        variables: shared
            .iter()
            .map(|(n, v)| Variable {
                name: (*n).to_owned(),
                value: (*v).to_owned(),
            })
            .collect(),
        locals: locals.iter().map(|n| (*n).to_owned()).collect(),
    }
}

fn local(name: &str, value: Option<&str>, secret: bool) -> LocalVar {
    LocalVar {
        name: name.to_owned(),
        value: value.map(str::to_owned),
        secret,
    }
}

// --- Variables connues (R1) -----------------------------------------------------

#[test]
fn r1_connues_partagees_locales_renseignees_et_heritees() {
    let recette = env(
        "Recette",
        &[("baseUrl", "https://r.test")],
        &["token", "user", "vide"],
    );
    let locals = [
        local("token", None, true),
        local("user", Some("ana"), false),
    ];
    let known = known_names(&recette, &locals, &["tenant".to_owned()]);
    assert_eq!(known, ["baseUrl", "tenant", "token", "user"]);
}

#[test]
fn r1_valeurs_dynamiques_jamais_inconnues() {
    let mut request = Request::new("t");
    request.url = "{{baseUrl}}/{{ $guid }}?t={{$timestamp}}".to_owned();
    assert_eq!(used_names(&request), ["baseUrl"]);
    assert_eq!(written_names(&request), ["baseUrl", "$guid", "$timestamp"]);
}

// --- Variables inconnues (R2) -----------------------------------------------------

#[test]
fn r2_utilisees_dans_tout_ce_qui_part() {
    let mut request = Request::new("t");
    request.url = "{{baseUrl}}/{{apiVersion}}/orders".to_owned();
    request.params = vec![
        Pair::new("q", "{{q}}"),
        Pair {
            enabled: false,
            ..Pair::new("off", "{{off}}")
        },
    ];
    request.headers = vec![Pair::new("X-Tenant", "{{tenant}}")];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "{{token}}".to_owned(),
        ..Auth::default()
    };
    request.body = Body::raw(BodyKind::Json, "{\"a\":\"{{montant}}\"}");
    request.inherited.headers = vec![InheritedHeader {
        name: "X-Client".to_owned(),
        value: "{{client}}".to_owned(),
        source: "collection".to_owned(),
    }];
    assert_eq!(
        used_names(&request),
        [
            "baseUrl",
            "apiVersion",
            "q",
            "tenant",
            "client",
            "token",
            "montant"
        ]
    );
    // Corps d'un autre type que celui choisi : rien n'en part.
    request.body.kind = BodyKind::None;
    assert!(!used_names(&request).contains(&"montant".to_owned()));
}

#[test]
fn r2_inconnues_avec_les_environnements_qui_les_definissent() {
    let all = [
        env("Recette", &[("baseUrl", "https://r.test")], &[]),
        env("Local", &[("apiVersion", "v2")], &[]),
        env("Prod", &[], &["apiVersion"]),
    ];
    let used = [
        "baseUrl".to_owned(),
        "apiVersion".to_owned(),
        "x".to_owned(),
    ];
    let known = ["baseUrl".to_owned()];
    let unknown = unknown(&used, &known, &all, "Recette");
    assert_eq!(unknown.len(), 2);
    assert_eq!(unknown[0].name, "apiVersion");
    assert_eq!(
        unknown[0].defined_in,
        [
            VariableSource {
                environment: "Local".to_owned(),
                value: Some("v2".to_owned()),
            },
            VariableSource {
                environment: "Prod".to_owned(),
                value: None,
            },
        ]
    );
    assert!(unknown[1].defined_in.is_empty());
    assert_eq!(
        serde_json::to_value(&unknown[0]).unwrap(),
        serde_json::json!({
            "name": "apiVersion",
            "definedIn": [
                { "environment": "Local", "value": "v2" },
                { "environment": "Prod", "value": null }
            ]
        })
    );
}

// --- En-têtes ajoutés (R3) ---------------------------------------------------------

fn names(added: &[AddedHeader]) -> Vec<&str> {
    added.iter().map(|h| h.name.as_str()).collect()
}

#[test]
fn r3_ajoutes_par_arnod() {
    let mut request = Request::new("t");
    request.method = Method::Post;
    request.url = "{{baseUrl}}/orders".to_owned();
    request.body = Body::raw(BodyKind::Json, "{}");
    let values = BTreeMap::from([(
        "baseUrl".to_owned(),
        "https://recette.boutique.dev".to_owned(),
    )]);
    let added = added_headers(&request, &values);
    assert_eq!(
        names(&added),
        [
            "Content-Type",
            "User-Agent",
            "Accept",
            "Content-Length",
            "Host"
        ]
    );
    assert_eq!(added[0].value, "application/json");
    assert_eq!(added[0].origin, AddedOrigin::Body);
    assert_eq!(added[1].value, USER_AGENT);
    assert_eq!(added[4].value, "recette.boutique.dev");
    // Sans corps : ni type ni longueur ; hôte inconnu : ce qui le donnera.
    let get = added_headers(&Request::new("t"), &BTreeMap::new());
    assert_eq!(names(&get), ["User-Agent", "Accept", "Host"]);
    assert_eq!(get[2].value, "d'après l'adresse");
}

#[test]
fn r3_un_en_tete_saisi_ou_herite_remplace_celui_d_arnod() {
    let mut request = Request::new("t");
    request.url = "http://localhost:8080/x".to_owned();
    request.body = Body::raw(BodyKind::Xml, "<a/>");
    request.headers = vec![
        Pair::new("content-type", "application/soap+xml"),
        Pair {
            enabled: false,
            ..Pair::new("User-Agent", "x")
        },
    ];
    request.inherited.headers = vec![InheritedHeader {
        name: "Accept".to_owned(),
        value: "application/xml".to_owned(),
        source: "collection".to_owned(),
    }];
    let added = added_headers(&request, &BTreeMap::new());
    assert_eq!(names(&added), ["User-Agent", "Content-Length", "Host"]);
    assert_eq!(added[2].value, "localhost:8080");
}

#[test]
fn r3_authentification_masquee_et_heritee() {
    let mut request = Request::new("t");
    request.url = "https://a.test".to_owned();
    request.headers = vec![Pair::new("Authorization", "x")];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "secret".to_owned(),
        ..Auth::default()
    };
    let added = added_headers(&request, &BTreeMap::new());
    // L'authentification s'ajoute même si l'en-tête est saisi.
    assert_eq!(added[0].name, "Authorization");
    assert_eq!(added[0].value, "Bearer ••••");
    assert_eq!(added[0].origin, AddedOrigin::Auth);
    assert_eq!(added[0].source, None);

    request.auth.kind = AuthKind::Inherit;
    request.inherited.auth = Some(InheritedAuth {
        auth: Auth {
            kind: AuthKind::ApiKey,
            name: "X-Key".to_owned(),
            value: "k".to_owned(),
            ..Auth::default()
        },
        source: "collection « B »".to_owned(),
    });
    let added = added_headers(&request, &BTreeMap::new());
    assert_eq!(added[0].name, "X-Key");
    assert_eq!(added[0].value, "••••");
    assert_eq!(added[0].source.as_deref(), Some("collection « B »"));
    assert_eq!(
        serde_json::to_value(&added[0]).unwrap(),
        serde_json::json!({
            "name": "X-Key",
            "value": "••••",
            "origin": "auth",
            "source": "collection « B »"
        })
    );
}

// --- Texte (R11 à R13) -------------------------------------------------------------

#[test]
fn r11_texte_d_une_liste() {
    let pairs = vec![
        Pair::new("Content-Type", "application/json"),
        Pair {
            enabled: false,
            ..Pair::new("X-Debug", "true")
        },
    ];
    assert_eq!(
        to_text(&pairs, PairsFormat::Header),
        "Content-Type: application/json\n# X-Debug: true"
    );
    assert_eq!(
        to_text(&[Pair::new("page", "2")], PairsFormat::Query),
        "page=2"
    );
}

#[test]
fn r12_lignes_lues_et_lignes_illisibles() {
    let text = "Content-Type: application/json\nX-Client: arnod-tests\n# X-Debug: true\n\nAccept-Language: fr-FR\nCache-Control no-cache\n:authority: a.test\nHost: localhost:8080";
    let read = from_text(text, PairsFormat::Header, &[]);
    let lines: Vec<(&str, &str, bool)> = read
        .pairs
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str(), p.enabled))
        .collect();
    assert_eq!(
        lines,
        [
            ("Content-Type", "application/json", true),
            ("X-Client", "arnod-tests", true),
            ("X-Debug", "true", false),
            ("Accept-Language", "fr-FR", true),
            ("Host", "localhost:8080", true),
        ]
    );
    assert_eq!(read.errors.len(), 1);
    assert_eq!(read.errors[0].line, 6);
    assert_eq!(
        read.errors[0].message,
        "Ligne 6 : « : » manquant entre le nom et la valeur."
    );
    assert_eq!(
        serde_json::to_value(&read.errors[0]).unwrap(),
        serde_json::json!({ "line": 6, "message": "Ligne 6 : « : » manquant entre le nom et la valeur." })
    );
}

#[test]
fn r12_parametres_egal_deux_points_ou_sans_valeur() {
    let read = from_text("page=2\nfiltre: a=b\ndebug\n=x", PairsFormat::Query, &[]);
    let lines: Vec<(&str, &str)> = read
        .pairs
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str()))
        .collect();
    assert_eq!(lines, [("page", "2"), ("filtre", "a=b"), ("debug", "")]);
    assert_eq!(read.errors.len(), 1);
    assert!(read.errors[0].message.contains("nom manquant"));
    assert!(read.pairs.iter().all(|p| p.kind == ParamKind::Query));
}

#[test]
fn r12_nom_d_en_tete_avec_espace() {
    let read = from_text("Mon en-tete: x", PairsFormat::Header, &[]);
    assert!(read.errors[0].message.contains("espace"));
}

#[test]
fn r13_descriptions_gardees_par_nom() {
    let previous = vec![
        Pair {
            description: "Format du corps".to_owned(),
            ..Pair::new("Content-Type", "text/plain")
        },
        Pair {
            description: "Client".to_owned(),
            ..Pair::new("X-Client", "a")
        },
    ];
    let read = from_text(
        "x-client: b\ncontent-type: application/json\nNouveau: 1",
        PairsFormat::Header,
        &previous,
    );
    let descriptions: Vec<&str> = read.pairs.iter().map(|p| p.description.as_str()).collect();
    assert_eq!(descriptions, ["Client", "Format du corps", ""]);
}

#[test]
fn r11_aller_retour() {
    let pairs = vec![
        Pair::new("a", "1"),
        Pair {
            enabled: false,
            ..Pair::new("b", "x=y")
        },
    ];
    let read = from_text(
        &to_text(&pairs, PairsFormat::Query),
        PairsFormat::Query,
        &pairs,
    );
    assert_eq!(read.pairs, pairs);
}
