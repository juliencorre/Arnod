//! Rapports d'un run (R8) : JUnit au schéma d'Apache Ant, Allure au modèle « TestResult »,
//! HTML autonome ; aucun secret, verdicts et durées tels que le cœur les a calculés.

use serde_json::Value;
use uuid::Uuid;

use super::*;
use crate::contract::conformance;
use crate::request::assertions::AssertionResult;
use crate::request::model::Method;
use crate::response::markup::{Item, tokenize};
use crate::workbench::{ItemStatus, RunItem, RunView, run_groups, run_tests};

fn item(path: &str, name: &str, status: ItemStatus, ms: u64) -> RunItem {
    RunItem {
        path: path.to_owned(),
        name: name.to_owned(),
        method: Method::Get,
        status,
        http_status: (status != ItemStatus::Error).then_some(200),
        duration_ms: (status != ItemStatus::Skipped).then_some(ms),
        slow: false,
        assertions: Vec::new(),
        error: None,
        test: None,
        step: None,
        extracted: Vec::new(),
    }
}

fn step(test: &str, number: usize, mut item: RunItem) -> RunItem {
    item.test = Some(test.to_owned());
    item.step = Some(number);
    item
}

fn failed_check(target: &str, message: &str) -> AssertionResult {
    AssertionResult {
        id: format!("c-{target}"),
        target: target.to_owned(),
        passed: false,
        message: Some(message.to_owned()),
        actual: Some("\"<absent & caché>\"".to_owned()),
        ignored: false,
    }
}

/// Run du dossier `commandes` : une requête seule réussie, un test dont l'étape 2 échoue (la 3
/// n'est pas jouée), une requête en erreur dans un sous-dossier.
fn view() -> RunView {
    let mut create = item(
        "commandes/parcours/b.yml",
        "Créer une commande",
        ItemStatus::Failed,
        64,
    );
    create.method = Method::Post;
    create.http_status = Some(201);
    create.assertions = vec![
        AssertionResult {
            id: "ok".into(),
            target: "status".into(),
            passed: true,
            message: None,
            actual: Some("201".into()),
            ignored: false,
        },
        failed_check("headers.Location", "absent"),
    ];
    let mut broken = item(
        "commandes/v2/paiement.yml",
        "Payer <vite>",
        ItemStatus::Error,
        0,
    );
    broken.error = Some("Connexion refusée par 127.0.0.1:1 (••••••)".into());
    let items = vec![
        item(
            "commandes/lister.yml",
            "Lister les commandes",
            ItemStatus::Passed,
            42,
        ),
        step(
            "commandes/parcours",
            1,
            item(
                "commandes/parcours/a.yml",
                "Créer un client",
                ItemStatus::Passed,
                30,
            ),
        ),
        step("commandes/parcours", 2, create),
        step(
            "commandes/parcours",
            3,
            item("commandes/parcours/c.yml", "Lire", ItemStatus::Skipped, 0),
        ),
        broken,
    ];
    RunView {
        id: Uuid::nil(),
        number: 3,
        folder: "commandes".into(),
        environment: "Recette fonctionnelle".into(),
        concurrency: 4,
        started_at: 1_764_235_271_000,
        finished_at: Some(1_764_235_274_400),
        finished: true,
        cancelled: false,
        groups: run_groups("commandes", &items),
        tests: run_tests(&items),
        items,
        contract: None,
        revision: 9,
    }
}

fn meta() -> Meta {
    Meta {
        version: "0.0.4".into(),
        project: "boutique-tests".into(),
        target: "Dossier commandes".into(),
    }
}

#[test]
fn r8_cas_verdicts_et_echecs_tels_que_le_coeur_les_a_calcules() {
    let view = view();
    let all = cases(&view);
    let summary: Vec<_> = all
        .iter()
        .map(|c| (c.name.as_str(), c.verdict, c.duration_ms, c.failure.clone()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Lister les commandes", Verdict::Passed, 42, None),
            (
                "parcours",
                Verdict::Failed,
                94,
                Some("étape 2 · headers.Location — absent".into())
            ),
            (
                "Payer <vite>",
                Verdict::Error,
                0,
                Some("Connexion refusée par 127.0.0.1:1 (••••••)".into())
            ),
        ]
    );
    let summary = Summary::of(&view, &all);
    assert_eq!(
        (
            summary.total,
            summary.passed,
            summary.failed,
            summary.errors
        ),
        (3, 1, 1, 1)
    );
    assert_eq!(summary.duration_ms, 3400);
    assert!(summary.failing());
    // Le test « parcours » appartient au dossier qui le contient (« commandes »).
    assert_eq!(folder_count(&all), 2);
    assert_eq!(duration_fr(summary.duration_ms), "3,4 s");
    assert_eq!(duration_fr(42), "42 ms");
    assert_eq!(duration_fr(125_400), "2 min 05 s");
    assert_eq!(iso8601(1_764_235_271_000), "2025-11-27T09:21:11");
    assert_eq!(iso8601(0), "1970-01-01T00:00:00");
    assert_eq!(iso8601(951_782_400_000), "2000-02-29T00:00:00");
}

/// Élément XML lu : nom, attributs (valeur sans guillemets), enfants.
#[derive(Debug)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Arbre d'un document XML bien formé (sinon, le test échoue).
fn parse(xml: &str) -> Node {
    let mut stack: Vec<Node> = vec![Node {
        name: "#document".into(),
        attrs: vec![],
        children: vec![],
    }];
    for token in tokenize(xml, false) {
        match token {
            Item::Open {
                name,
                attrs,
                self_closing,
            } => {
                let node = Node {
                    name: name.to_owned(),
                    attrs: attrs
                        .into_iter()
                        .map(|(n, v)| {
                            let raw = v.expect("attribut sans valeur");
                            assert!(raw.starts_with('"') && raw.ends_with('"'), "{raw}");
                            (n.to_owned(), raw.trim_matches('"').to_owned())
                        })
                        .collect(),
                    children: vec![],
                };
                if self_closing {
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(node);
                }
            }
            Item::Close { name } => {
                let node = stack.pop().unwrap();
                assert_eq!(node.name, name, "balises mal imbriquées");
                stack.last_mut().unwrap().children.push(node);
            }
            Item::Text(text) => {
                assert!(
                    !text.contains('<') && !text.contains('>'),
                    "texte non échappé : {text}"
                );
                let unescaped = text
                    .replace("&amp;", "")
                    .replace("&lt;", "")
                    .replace("&gt;", "")
                    .replace("&quot;", "")
                    .replace("&apos;", "");
                assert!(!unescaped.contains('&'), "entité inconnue : {text}");
            }
            Item::Comment(_) | Item::Declaration(_) => {}
        }
    }
    assert_eq!(stack.len(), 1, "balise non fermée");
    let mut document = stack.pop().unwrap();
    assert_eq!(document.children.len(), 1, "une seule racine");
    document.children.pop().unwrap()
}

fn is_count(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
}

fn is_seconds(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let (whole, decimals) = v.split_once('.').unwrap_or((v, "0"));
        is_count(Some(whole)) && is_count(Some(decimals))
    })
}

/// `ISO8601_DATETIME_PATTERN` du schéma : `AAAA-MM-JJThh:mm:ss`, sans fuseau.
fn is_timestamp(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let b = v.as_bytes();
        b.len() == 19
            && b.iter().enumerate().all(|(i, c)| match i {
                4 | 7 => *c == b'-',
                10 => *c == b'T',
                13 | 16 => *c == b':',
                _ => c.is_ascii_digit(),
            })
    })
}

/// Règles du schéma JUnit d'Apache Ant (`JUnit.xsd`, repris par Jenkins) : attributs
/// obligatoires et leur forme, ordre des enfants d'une suite, contenu d'un cas.
fn check_junit(root: &Node) {
    assert_eq!(root.name, "testsuites");
    for suite in &root.children {
        assert_eq!(suite.name, "testsuite");
        for name in ["name", "timestamp", "hostname", "package", "id"] {
            assert!(
                suite.attr(name).is_some_and(|v| !v.is_empty()),
                "testsuite@{name}"
            );
        }
        for name in ["tests", "failures", "errors", "skipped", "id"] {
            assert!(is_count(suite.attr(name)), "testsuite@{name}");
        }
        assert!(is_seconds(suite.attr("time")));
        assert!(is_timestamp(suite.attr("timestamp")));
        let order: Vec<&str> = suite.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(order.first(), Some(&"properties"));
        assert_eq!(&order[order.len() - 2..], ["system-out", "system-err"]);
        assert!(order[1..order.len() - 2].iter().all(|n| *n == "testcase"));
        for property in &suite.children[0].children {
            assert_eq!(property.name, "property");
            assert!(property.attr("name").is_some() && property.attr("value").is_some());
        }
        let cases: Vec<&Node> = suite
            .children
            .iter()
            .filter(|c| c.name == "testcase")
            .collect();
        assert_eq!(suite.attr("tests"), Some(cases.len().to_string().as_str()));
        for case in cases {
            assert!(case.attr("name").is_some_and(|v| !v.is_empty()));
            assert!(case.attr("classname").is_some_and(|v| !v.is_empty()));
            assert!(is_seconds(case.attr("time")));
            let kinds: Vec<&str> = case.children.iter().map(|c| c.name.as_str()).collect();
            assert!(kinds.len() <= 2 && kinds.last() == Some(&"system-out"));
            if let Some(kind) = kinds.first().filter(|k| **k != "system-out") {
                assert!(["failure", "error", "skipped"].contains(kind), "{kind}");
                if *kind != "skipped" {
                    let outcome = &case.children[0];
                    assert!(outcome.attr("message").is_some() && outcome.attr("type").is_some());
                }
            }
        }
    }
}

#[test]
fn r8_junit_conforme_au_schema_d_ant() {
    let xml = junit::render(&view(), &meta());
    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites "));
    let root = parse(&xml);
    check_junit(&root);
    assert_eq!(root.attr("tests"), Some("3"));
    assert_eq!(root.attr("failures"), Some("1"));
    assert_eq!(root.attr("errors"), Some("1"));
    assert_eq!(root.attr("time"), Some("3.400"));
    let suites: Vec<_> = root
        .children
        .iter()
        .map(|s| s.attr("name").unwrap())
        .collect();
    assert_eq!(suites, ["commandes", "commandes/v2"]);
    let failure = &root.children[0].children[2].children[0];
    assert_eq!(failure.name, "failure");
    assert_eq!(
        failure.attr("message"),
        Some("étape 2 · headers.Location — absent")
    );
    assert!(xml.contains("Payer &lt;vite&gt;"));
    assert!(xml.contains("classname=\"boutique-tests.commandes.v2\""));
    assert!(
        xml.contains("<property name=\"arnod.environnement\" value=\"Recette fonctionnelle\"/>")
    );
}

#[test]
fn r8_allure_conforme_au_modele_test_result() {
    let schema: Value = serde_json::from_str(include_str!("allure-result.schema.json")).unwrap();
    let files = allure::render(&view(), &meta());
    assert_eq!(files.len(), 4, "trois résultats et environment.properties");
    let results: Vec<Value> = files
        .iter()
        .filter(|(name, _)| name.ends_with("-result.json"))
        .map(|(name, text)| {
            let value: Value = serde_json::from_str(text).unwrap();
            assert_eq!(
                name,
                &format!("{}-result.json", value["uuid"].as_str().unwrap())
            );
            let errors = conformance::validate_at(&schema, &schema, &value, "résultat");
            assert!(errors.is_empty(), "{errors:?}");
            value
        })
        .collect();
    let statuses: Vec<&str> = results
        .iter()
        .map(|r| r["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["passed", "failed", "broken"]);
    let parcours = &results[1];
    assert_eq!(parcours["fullName"], "boutique-tests/commandes/parcours");
    assert_eq!(parcours["steps"].as_array().unwrap().len(), 3);
    assert_eq!(
        parcours["steps"][1]["name"],
        "2. POST Créer une commande · 201"
    );
    assert_eq!(parcours["steps"][1]["steps"][1]["status"], "failed");
    assert_eq!(
        parcours["stop"].as_u64().unwrap() - parcours["start"].as_u64().unwrap(),
        94
    );
    // Même historique d'un run à l'autre (même projet, même chemin).
    let again = allure::render(&view(), &meta());
    let again: Value = serde_json::from_str(&again[1].1).unwrap();
    assert_eq!(again["historyId"], parcours["historyId"]);
    assert_ne!(again["uuid"], parcours["uuid"]);
    let (_, properties) = files.last().unwrap();
    assert_eq!(
        properties,
        "Arnod=0.0.4\nProjet=boutique-tests\nEnvironnement=Recette fonctionnelle\nCible=Dossier commandes\n"
    );
}

#[test]
fn r8_html_autonome_echappe_et_lisible_sans_couleur() {
    let html = html::render(&view(), &meta());
    assert!(html.starts_with("<!doctype html>\n<html lang=\"fr\">"));
    for forbidden in [
        "<script", "src=", "href=", "url(", "@import", "http://", "https://",
    ] {
        assert!(!html.contains(forbidden), "{forbidden}");
    }
    assert!(html.contains("Payer &lt;vite&gt;"));
    assert!(!html.contains("<vite>"));
    assert!(html.contains("&quot;&lt;absent &amp; caché&gt;&quot;"));
    // Le verdict se lit au signe et au texte, pas seulement à la couleur.
    assert!(
        html.contains("✕</span><span class=\"name\">parcours</span><span class=\"sr\">en échec")
    );
    assert!(html.contains("✓</span><span class=\"name\">Lister les commandes"));
    assert!(html.contains("<h2>En échec</h2>"));
    assert!(html.contains("3 tests dans 2 dossiers"));
}
