//! Règles du corps de la réponse (`docs/plan-mvp-a-reponse.md`, R1 à R8), une par test.

use serde_json::json;

use super::*;
use crate::request::model::{Operator, Scalar};

fn ct(value: &str) -> Vec<(String, String)> {
    vec![("Content-Type".to_owned(), value.to_owned())]
}

/// Texte d'une ligne mise en forme.
fn text(line: &[Token]) -> String {
    line.iter().map(|t| t.text.as_str()).collect()
}

fn texts(body: &RenderedBody) -> Vec<String> {
    body.lines.iter().map(|l| text(l)).collect()
}

const ORDER: &str = r#"{"id":"ord_8F2kQ","status":"pending","items":[{"sku":"TSH-BLU-M","qty":2,"price":19.9},{"sku":"MUG-NOIR","qty":1,"price":12.5}],"coupon":null,"total":47.82}"#;

// --- R1 : format détecté -------------------------------------------------------------------

#[test]
fn r1_format_d_apres_content_type() {
    assert_eq!(
        detect(&ct("application/json; charset=utf-8"), ""),
        ResponseFormat::Json
    );
    assert_eq!(
        detect(&ct("application/problem+json"), ""),
        ResponseFormat::Json
    );
    assert_eq!(detect(&ct("text/xml"), ""), ResponseFormat::Xml);
    assert_eq!(detect(&ct("application/soap+xml"), ""), ResponseFormat::Xml);
    assert_eq!(detect(&ct("text/html"), "{}"), ResponseFormat::Html);
    assert_eq!(detect(&ct("text/csv"), "{}"), ResponseFormat::Text);
}

#[test]
fn r1_sans_en_tete_parlant_le_corps_decide() {
    assert_eq!(detect(&[], r#"  {"a":1}"#), ResponseFormat::Json);
    assert_eq!(detect(&ct("text/plain"), "[1,2]"), ResponseFormat::Json);
    assert_eq!(detect(&[], "{pas du json"), ResponseFormat::Text);
    assert_eq!(
        detect(&[], "<!DOCTYPE html><html></html>"),
        ResponseFormat::Html
    );
    assert_eq!(
        detect(&[], "<?xml version=\"1.0\"?><a/>"),
        ResponseFormat::Xml
    );
    assert_eq!(detect(&[], "bonjour"), ResponseFormat::Text);
}

#[test]
fn r1_le_format_choisi_l_emporte_sur_le_format_detecte() {
    let body = render(ORDER, &ct("application/json"), Some(ResponseFormat::Text));
    assert_eq!(body.format, ResponseFormat::Text);
    assert_eq!(body.detected, ResponseFormat::Json);
    assert_eq!(texts(&body), vec![ORDER.to_owned()]);
}

#[test]
fn r1_forme_json_serialisee() {
    let body = render(r#"{"a":1}"#, &[], None);
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["format"], "json");
    assert_eq!(value["detected"], "json");
    assert_eq!(value["notice"], serde_json::Value::Null);
    assert_eq!(
        value["lines"][0][0],
        json!({ "text": "{", "kind": "punct", "path": "$" })
    );
    // `{ "a": 1 }` tient sur une ligne ; un jeton sans chemin n'écrit pas `path`.
    assert_eq!(
        value["lines"][0][2],
        json!({ "text": "\"a\"", "kind": "key" })
    );
}

// --- R2 : JSON mis en forme ----------------------------------------------------------------

#[test]
fn r2_json_indente_et_objets_courts_sur_une_ligne() {
    let body = render(ORDER, &ct("application/json"), None);
    assert_eq!(
        texts(&body),
        vec![
            "{",
            "  \"id\": \"ord_8F2kQ\",",
            "  \"status\": \"pending\",",
            "  \"items\": [",
            "    { \"sku\": \"TSH-BLU-M\", \"qty\": 2, \"price\": 19.9 },",
            "    { \"sku\": \"MUG-NOIR\", \"qty\": 1, \"price\": 12.5 }",
            "  ],",
            "  \"coupon\": null,",
            "  \"total\": 47.82",
            "}",
        ]
    );
    assert_eq!(body.notice, None);
}

#[test]
fn r2_chaque_valeur_porte_son_chemin_canonique() {
    let body = render(ORDER, &[], None);
    let sku = body.lines[4]
        .iter()
        .find(|t| t.text == "\"TSH-BLU-M\"")
        .unwrap();
    assert_eq!(sku.kind, TokenKind::String);
    assert_eq!(sku.path.as_deref(), Some("$.items[0].sku"));
    let null = body.lines[7].iter().find(|t| t.text == "null").unwrap();
    assert_eq!(null.kind, TokenKind::Literal);
    assert_eq!(null.path.as_deref(), Some("$.coupon"));
    // Les clés n'ont pas de chemin : seule la valeur est surlignée.
    assert!(
        body.lines[1]
            .iter()
            .all(|t| t.kind != TokenKind::Key || t.path.is_none())
    );
}

#[test]
fn r2_nom_special_entre_crochets() {
    let body = render(r#"{"content-type":"x","l'a":1}"#, &[], None);
    let paths: Vec<_> = body
        .lines
        .iter()
        .flatten()
        .filter_map(|t| t.path.clone())
        .collect();
    assert!(paths.contains(&"$['content-type']".to_owned()), "{paths:?}");
    assert!(paths.contains(&"$['l\\'a']".to_owned()), "{paths:?}");
}

#[test]
fn r2_json_invalide_affiche_tel_quel_avec_la_raison() {
    let body = render("{\"a\": 1,\n oups}", &ct("application/json"), None);
    assert_eq!(texts(&body), vec!["{\"a\": 1,", " oups}"]);
    assert!(
        body.notice
            .as_deref()
            .is_some_and(|n| n.contains("pas du JSON valide") && n.contains("ligne 2")),
        "{:?}",
        body.notice
    );
}

#[test]
fn r2_corps_vide_et_corps_volumineux() {
    assert!(render("", &ct("application/json"), None).lines.is_empty());
    let big = format!("[{}0]", "0,".repeat(render::RENDER_LIMIT / 2 + 1));
    let body = render(&big, &[], None);
    assert_eq!(body.lines.len(), 1);
    assert!(body.notice.is_some_and(|n| n.contains("1 Mo")));
}

// --- R3 : XML et HTML mis en forme ---------------------------------------------------------

#[test]
fn r3_xml_une_balise_par_ligne_texte_court_sur_sa_ligne() {
    let body = render(
        r#"<?xml version="1.0"?><commande id="42"><article sku="TSH">T-shirt</article><vide/><note></note></commande>"#,
        &ct("application/xml"),
        None,
    );
    assert_eq!(
        texts(&body),
        vec![
            "<?xml version=\"1.0\"?>",
            "<commande id=\"42\">",
            "  <article sku=\"TSH\">T-shirt</article>",
            "  <vide/>",
            "  <note></note>",
            "</commande>",
        ]
    );
    let line = &body.lines[1];
    assert_eq!(line[1].kind, TokenKind::Tag);
    assert_eq!(line[3].kind, TokenKind::Attr);
    assert_eq!(line[5].kind, TokenKind::AttrValue);
}

#[test]
fn r3_html_elements_vides_et_script_brut() {
    let body = render(
        "<html><body><p>Un<br>deux</p><script>if (a < b) { x(); }</script></body></html>",
        &ct("text/html"),
        None,
    );
    assert_eq!(
        texts(&body),
        vec![
            "<html>",
            "  <body>",
            "    <p>",
            "      Un",
            "      <br>",
            "      deux",
            "    </p>",
            "    <script>if (a < b) { x(); }</script>",
            "  </body>",
            "</html>",
        ]
    );
}

#[test]
fn r3_balise_jamais_fermee_reste_du_texte() {
    let body = render("<a>texte <b", &ct("text/xml"), None);
    assert_eq!(texts(&body), vec!["<a>", "  texte <b"]);
}

// --- R4 : aperçu HTML sans script ----------------------------------------------------------

fn element(tag: &str, children: Vec<HtmlNode>) -> HtmlNode {
    HtmlNode::Element {
        tag: tag.to_owned(),
        children,
    }
}

fn t(text: &str) -> HtmlNode {
    HtmlNode::Text {
        text: text.to_owned(),
    }
}

#[test]
fn r4_apercu_sans_script_ni_style_ni_attribut() {
    let nodes = preview(
        r#"<!DOCTYPE html><html><head><title>T</title><style>p{}</style></head><body onload="x()"><h1 class="t">Bonjour &amp; bienvenue</h1><script>alert(1)</script><p>Un <a href="https://ailleurs.example">lien</a></p><iframe src="https://x"><p>caché</p></iframe></body></html>"#,
    );
    assert_eq!(
        nodes,
        vec![
            element("h1", vec![t("Bonjour & bienvenue")]),
            element("p", vec![t("Un "), element("a", vec![t("lien")])]),
        ]
    );
}

#[test]
fn r4_image_remplacee_par_son_texte_et_balise_inconnue_deballee() {
    let nodes =
        preview(r#"<custom-el><img src="https://x/a.png" alt="Logo"><img src=x></custom-el><br/>"#);
    assert_eq!(
        nodes,
        vec![t("[image : Logo]"), t("[image]"), element("br", vec![])]
    );
}

#[test]
fn r4_balises_mal_fermees_et_forme_serialisee() {
    let nodes = preview("<ul><li>a<li>b</ul><p>fin");
    assert_eq!(
        nodes,
        vec![
            element(
                "ul",
                vec![element("li", vec![t("a"), element("li", vec![t("b")])])]
            ),
            element("p", vec![t("fin")]),
        ]
    );
    assert_eq!(
        serde_json::to_value(&nodes[1]).unwrap(),
        json!({ "kind": "element", "tag": "p", "children": [{ "kind": "text", "text": "fin" }] })
    );
}

#[test]
fn r4_apercu_borne() {
    let nodes = preview(&"<span>x</span>".repeat(preview::MAX_NODES + 10));
    assert!(nodes.len() <= preview::MAX_NODES + 1);
    assert!(matches!(nodes.last(), Some(HtmlNode::Text { text }) if text.contains("limité")));
}

// --- R5 : filtre JSONPath ------------------------------------------------------------------

#[test]
fn r5_correspondances_avec_chemin_et_valeur() {
    let result = filter(ORDER, "$.items[*].sku");
    assert_eq!(result.error, None);
    assert_eq!(result.total, 2);
    let found: Vec<_> = result
        .matches
        .iter()
        .map(|m| (m.path.as_str(), m.value.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            ("$.items[0].sku", "\"TSH-BLU-M\""),
            ("$.items[1].sku", "\"MUG-NOIR\"")
        ]
    );
}

#[test]
fn r5_les_chemins_du_filtre_sont_ceux_de_la_mise_en_forme() {
    let rendered = render(ORDER, &[], None);
    let paths: std::collections::HashSet<_> = rendered
        .lines
        .iter()
        .flatten()
        .filter_map(|t| t.path.clone())
        .collect();
    for expression in ["$..price", "$.items[?@.qty > 1]", "$.*", "$['items'][1]"] {
        let result = filter(ORDER, expression);
        assert!(result.total > 0, "{expression}");
        for found in &result.matches {
            assert!(paths.contains(&found.path), "{expression} : {}", found.path);
        }
    }
}

#[test]
fn r5_expression_vide_et_aucun_resultat() {
    assert_eq!(filter(ORDER, "  ").total, 0);
    assert_eq!(filter(ORDER, "  ").error, None);
    let none = filter(ORDER, "$.absent");
    assert_eq!((none.total, none.error), (0, None));
}

#[test]
fn r5_correspondances_bornees() {
    let body = serde_json::to_string(&vec![1; filter::MAX_MATCHES + 5]).unwrap();
    let result = filter(&body, "$[*]");
    assert_eq!(result.total, filter::MAX_MATCHES + 5);
    assert_eq!(result.matches.len(), filter::MAX_MATCHES);
}

#[test]
fn r5_forme_json_serialisee() {
    let value = serde_json::to_value(filter(ORDER, "$.items[0].qty")).unwrap();
    assert_eq!(
        value,
        json!({
            "matches": [{
                "path": "$.items[0].qty",
                "value": "2",
                "assertion": {
                    "id": "filtre-body-items-0--qty",
                    "target": "body.items[0].qty",
                    "operator": "equals",
                    "value": 2,
                    "origin": "proposed",
                    "enabled": true
                }
            }],
            "total": 1,
            "error": null
        })
    );
}

// --- R6 : vérification tirée d'une correspondance ------------------------------------------

#[test]
fn r6_verification_selon_la_valeur() {
    let result = filter(ORDER, "$['status','coupon','items','total']");
    let assertions: Vec<_> = result
        .matches
        .iter()
        .map(|m| m.assertion.clone().unwrap())
        .map(|a| (a.target, a.operator, a.value))
        .collect();
    assert_eq!(
        assertions,
        vec![
            (
                "body.status".to_owned(),
                Operator::Equals,
                Some(Scalar::Text("pending".into()))
            ),
            ("body.coupon".to_owned(), Operator::IsNull, None),
            (
                "body.items".to_owned(),
                Operator::Length,
                Some(Scalar::Int(2))
            ),
            (
                "body.total".to_owned(),
                Operator::Equals,
                Some(Scalar::Float(47.82))
            ),
        ]
    );
    let object = filter(ORDER, "$.items[0]").matches.remove(0);
    assert_eq!(object.assertion.map(|a| a.operator), Some(Operator::Exists));
    let root = filter(ORDER, "$").matches.remove(0);
    assert_eq!(root.assertion.map(|a| a.target), Some("body".to_owned()));
}

#[test]
fn r6_la_verification_reussit_sur_la_reponse_filtree() {
    use crate::request::assertions::{ResponseData, evaluate_all};
    let response = ResponseData {
        status: 200,
        duration_ms: 1,
        headers: Vec::new(),
        body: ORDER.to_owned(),
    };
    let assertions: Vec<_> = [
        "$.items[1].sku",
        "$.coupon",
        "$.items",
        "$.total",
        "$.items[0]",
    ]
    .iter()
    .filter_map(|e| filter(ORDER, e).matches.remove(0).assertion)
    .collect();
    assert_eq!(assertions.len(), 5);
    for result in evaluate_all(&assertions, &response) {
        assert!(result.passed, "{result:?}");
    }
}

#[test]
fn r6_nom_non_pointe_sans_verification() {
    let result = filter(r#"{"content-type":"x"}"#, "$['content-type']");
    assert_eq!(result.matches[0].path, "$['content-type']");
    assert_eq!(result.matches[0].assertion, None);
}

// --- R7 : erreurs en français ---------------------------------------------------------------

#[test]
fn r7_expression_invalide_expliquee() {
    let cases = [
        ("items[*]", "commence par $"),
        ("$.items[0", "Crochet « [ » non fermé"),
        ("$.items[?(@.qty > 1]", "Parenthèse « ( » non fermée"),
        ("$['a", "Guillemet non fermé"),
        ("$.items.", "après le dernier point"),
        ("$.items[*]sku", "au caractère 11 : « sku »"),
    ];
    for (expression, expected) in cases {
        let result = filter(ORDER, expression);
        assert_eq!(result.total, 0, "{expression}");
        let error = result.error.unwrap_or_default();
        assert!(error.contains(expected), "{expression} : {error}");
    }
}

#[test]
fn r7_corps_qui_n_est_pas_du_json() {
    let error = filter("<a/>", "$.a").error.unwrap_or_default();
    assert!(error.contains("corps JSON"), "{error}");
    let empty = filter("", "$.a").error.unwrap_or_default();
    assert!(empty.contains("pas de corps"), "{empty}");
}

// --- R8 : nom du fichier enregistré --------------------------------------------------------

#[test]
fn r8_nom_de_fichier_propose() {
    assert_eq!(
        file_name("Créer une commande", ResponseFormat::Json),
        "Créer une commande.json"
    );
    assert_eq!(file_name("a/b:c?", ResponseFormat::Xml), "a-b-c-.xml");
    assert_eq!(file_name("  ", ResponseFormat::Html), "reponse.html");
    assert_eq!(file_name("fin.", ResponseFormat::Text), "fin.txt");
}

// --- Lecture des balises -------------------------------------------------------------------

#[test]
fn entites_decodees() {
    assert_eq!(
        markup::decode_entities("a &lt;b&gt; &#233;t&#xE9; &inconnu; & fin"),
        "a <b> été &inconnu; & fin"
    );
}

// --- Miroirs TypeScript ---------------------------------------------------------------------

#[test]
fn unions_miroirs_cote_typescript() {
    use crate::ts_contract::{serialized_name, union_members};
    use std::collections::BTreeSet;
    let formats: BTreeSet<_> = [
        ResponseFormat::Json,
        ResponseFormat::Xml,
        ResponseFormat::Html,
        ResponseFormat::Text,
    ]
    .into_iter()
    .map(serialized_name)
    .collect();
    assert_eq!(formats, union_members("ResponseFormat"));
    let kinds: BTreeSet<_> = [
        TokenKind::Key,
        TokenKind::String,
        TokenKind::Number,
        TokenKind::Literal,
        TokenKind::Punct,
        TokenKind::Tag,
        TokenKind::Attr,
        TokenKind::AttrValue,
        TokenKind::Comment,
        TokenKind::Text,
    ]
    .into_iter()
    .map(serialized_name)
    .collect();
    assert_eq!(kinds, union_members("TokenKind"));
}
