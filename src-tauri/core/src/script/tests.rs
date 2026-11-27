//! Règles des scripts (`docs/plan-v4.md`, R9, R10).

use std::collections::BTreeMap;

use super::*;

fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn response(status: u16, body: &str) -> ResponseData {
    ResponseData {
        status,
        duration_ms: 42,
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        body: body.to_owned(),
    }
}

#[test]
fn r9_vars_response_et_assert_comme_sur_l_ecran_4_3_g() {
    let code = r#"
const total = vars.order.total;
const ttc = Math.round(total * 1.2 * 100) / 100;
assert(ttc === response.body.amount, "montant TTC");
vars.set("order.ttc", ttc);
"#;
    let outcome = run_step(
        code,
        &vars(&[("order.total", "49.83"), ("order.id", "ord_8F2kQ")]),
        Some(&response(200, r#"{"amount": 59.8}"#)),
    );
    assert!(outcome.passed, "{:?}", outcome.error);
    assert_eq!(
        outcome.set.get("order.ttc").map(String::as_str),
        Some("59.8")
    );
}

#[test]
fn r9_un_assert_faux_fait_echouer_l_etape_avec_son_message() {
    let outcome = run_step(
        r#"assert(response.status === 201, "commande créée");"#,
        &BTreeMap::new(),
        Some(&response(400, "{}")),
    );
    assert!(!outcome.passed);
    assert_eq!(
        outcome.error.as_deref(),
        Some("Assertion fausse : commande créée")
    );
}

#[test]
fn r9_une_exception_fait_echouer_l_etape() {
    let outcome = run_step("null.x;", &BTreeMap::new(), None);
    assert!(!outcome.passed);
    assert!(outcome.error.is_some());
}

#[test]
fn r9_un_script_sans_fin_est_interrompu() {
    let started = Instant::now();
    let outcome = run_step("while (true) {}", &BTreeMap::new(), None);
    assert!(!outcome.passed);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|e| e.contains("interrompu")),
        "{:?}",
        outcome.error
    );
    assert!(started.elapsed() < TIMEOUT + Duration::from_secs(2));
}

#[test]
fn r9_ni_disque_ni_reseau() {
    for code in [
        "require('fs')",
        "fetch('https://exemple.test')",
        "new XMLHttpRequest()",
    ] {
        let outcome = run_step(code, &BTreeMap::new(), None);
        assert!(!outcome.passed, "{code} ne doit pas réussir");
    }
}

#[test]
fn r9_console_et_return() {
    let outcome = run_step(
        "console.log('bonjour', 2); return; vars.set('x', 1);",
        &BTreeMap::new(),
        None,
    );
    assert!(outcome.passed);
    assert_eq!(outcome.logs, vec!["bonjour 2".to_owned()]);
    assert!(outcome.set.is_empty());
}

#[test]
fn r10_bruno_apres_la_reponse_pose_une_variable() {
    let outcome = run_bruno(
        Phase::AfterResponse,
        r#"bru.setVar("orderId", res.getBody().id); bru.setVar("code", res.status);"#,
        &BTreeMap::new(),
        None,
        Some(&response(201, r#"{"id": "ord_1"}"#)),
    );
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.set.get("orderId").map(String::as_str),
        Some("ord_1")
    );
    assert_eq!(outcome.set.get("code").map(String::as_str), Some("201"));
}

#[test]
fn r10_bruno_tests_et_expect() {
    let code = r#"
test("statut 201", function () { expect(res.getStatus()).to.equal(201); });
test("identifiant", function () { expect(res.getBody()).to.have.property("id"); });
test("faux", function () { expect(res.getBody().id).to.equal("autre"); });
"#;
    let outcome = run_bruno(
        Phase::Tests,
        code,
        &BTreeMap::new(),
        None,
        Some(&response(201, r#"{"id": "ord_1"}"#)),
    );
    assert_eq!(outcome.tests.len(), 3);
    assert!(outcome.tests[0].passed && outcome.tests[1].passed);
    assert!(!outcome.tests[2].passed);
    assert_eq!(
        outcome.tests[2].error.as_deref(),
        Some(r#"attendu "autre", reçu "ord_1""#)
    );
}

#[test]
fn r10_bruno_avant_la_requete_modifie_la_requete() {
    let request = ScriptRequest {
        method: "GET".to_owned(),
        url: "https://exemple.test/a".to_owned(),
        headers: vec![("Accept".to_owned(), "*/*".to_owned())],
        body: None,
    };
    let outcome = run_bruno(
        Phase::BeforeRequest,
        r#"req.setHeader("X-Trace", bru.getVar("trace")); req.setUrl(req.getUrl() + "?b=1");"#,
        &vars(&[("trace", "t-1")]),
        Some(&request),
        None,
    );
    let changed = outcome.request.expect("requête modifiée");
    assert_eq!(changed.url, "https://exemple.test/a?b=1");
    assert!(
        changed
            .headers
            .contains(&("X-Trace".to_owned(), "t-1".to_owned()))
    );
}
