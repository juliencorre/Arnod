//! Règles du module cURL, cas par cas (`docs/plan-mvp-b-saisie-curl.md`, R4 à R10).

use std::collections::{BTreeMap, BTreeSet};

use super::parse::{self, camel, take_secrets, words};
use super::render::{self, hide_credentials, placeholder, quote, shell_name};
use crate::error::CommandErrorKind;
use crate::request::model::*;
use crate::request::resolve::{Prepared, Variables};

fn header<'a>(request: &'a parse::CurlRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(h, _)| h.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

// --- Lecture des mots (R4) ----------------------------------------------------

#[test]
fn r4_guillemets_simples_doubles_et_echappements() {
    assert_eq!(
        words(r#"curl 'a b' "c \"d\" \$e" f\ g"#).unwrap(),
        ["curl", "a b", "c \"d\" $e", "f g"]
    );
    // Un `\` dans des guillemets doubles reste s'il n'échappe rien.
    assert_eq!(words(r#""a\nb""#).unwrap(), ["a\\nb"]);
    // Guillemets simples : tout est littéral.
    assert_eq!(words(r"'a\b $c'").unwrap(), ["a\\b $c"]);
    // Mots collés : `'a'"b"c` ne fait qu'un mot.
    assert_eq!(words(r#"'a'"b"c"#).unwrap(), ["abc"]);
}

#[test]
fn r4_fin_de_ligne_echappee() {
    let text = "curl -X POST \\\n  'https://api.test/x' \\\r\n  -H 'A: b'";
    assert_eq!(
        words(text).unwrap(),
        ["curl", "-X", "POST", "https://api.test/x", "-H", "A: b"]
    );
}

#[test]
fn r4_chaine_ansi_c_des_navigateurs() {
    assert_eq!(
        words(r#"$'{\"a\":\n1, \x41\u00e9}'"#).unwrap(),
        ["{\"a\":\n1, Aé}"]
    );
}

#[test]
fn r4_guillemet_non_ferme() {
    assert!(words("curl 'abc").unwrap_err().contains("simple"));
    assert!(words("curl \"abc").unwrap_err().contains("double"));
}

// --- Lecture des options (R4) -------------------------------------------------

#[test]
fn r4_commande_de_la_maquette() {
    let request = parse::parse(
        "curl -X POST https://api.boutique.dev/orders -H 'Content-Type: application/json' -H 'Authorization: Bearer eyJhbGci.x' -d '{\"customerId\":\"cus_31\"}' --compressed",
    )
    .unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://api.boutique.dev/orders");
    assert_eq!(
        request.headers,
        [
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("Authorization".to_owned(), "Bearer eyJhbGci.x".to_owned()),
        ]
    );
    assert_eq!(request.body.as_deref(), Some("{\"customerId\":\"cus_31\"}"));
    assert_eq!(request.ignored.len(), 1);
    assert_eq!(request.ignored[0].option, "--compressed");
}

#[test]
fn r4_methode_deduite() {
    assert_eq!(parse::parse("curl https://a.test").unwrap().method, "GET");
    assert_eq!(
        parse::parse("curl https://a.test -d x=1").unwrap().method,
        "POST"
    );
    assert_eq!(
        parse::parse("curl -I https://a.test").unwrap().method,
        "HEAD"
    );
    assert_eq!(
        parse::parse("curl --request patch https://a.test")
            .unwrap()
            .method,
        "PATCH"
    );
    // Forme collée `-XPUT` et option longue `--request=DELETE`.
    assert_eq!(
        parse::parse("curl -XPUT https://a.test").unwrap().method,
        "PUT"
    );
    assert_eq!(
        parse::parse("curl --request=DELETE https://a.test")
            .unwrap()
            .method,
        "DELETE"
    );
}

#[test]
fn r4_donnees_jointes_et_type_par_defaut() {
    let request =
        parse::parse("curl https://a.test -d a=1 --data-raw '@b=2' --data-binary c=3").unwrap();
    assert_eq!(request.body.as_deref(), Some("a=1&@b=2&c=3"));
    // Sans type donné, cURL envoie un formulaire.
    assert_eq!(
        header(&request, "Content-Type"),
        Some("application/x-www-form-urlencoded")
    );
}

#[test]
fn r4_data_urlencode_json_et_get() {
    let request = parse::parse("curl https://a.test --data-urlencode 'q=a b&c'").unwrap();
    assert_eq!(request.body.as_deref(), Some("q=a%20b%26c"));
    let json = parse::parse("curl https://a.test --json '{}'").unwrap();
    assert_eq!(json.body.as_deref(), Some("{}"));
    assert_eq!(header(&json, "Content-Type"), Some("application/json"));
    assert_eq!(header(&json, "Accept"), Some("application/json"));
    let get = parse::parse("curl -G https://a.test/s?x=1 -d q=chat").unwrap();
    assert_eq!(get.method, "GET");
    assert_eq!(get.url, "https://a.test/s?x=1&q=chat");
    assert_eq!(get.body, None);
}

#[test]
fn r4_user_cookie_agent_referer_url() {
    let request = parse::parse(
        "curl --url https://a.test -u ana:s3cret -b 'a=1' --cookie 'b=2' -A agent/1 -e https://ref",
    )
    .unwrap();
    assert_eq!(request.url, "https://a.test");
    assert_eq!(
        header(&request, "Authorization"),
        Some("Basic YW5hOnMzY3JldA==")
    );
    assert_eq!(header(&request, "Cookie"), Some("a=1; b=2"));
    assert_eq!(header(&request, "User-Agent"), Some("agent/1"));
    assert_eq!(header(&request, "Referer"), Some("https://ref"));
}

#[test]
fn r4_schema_ajoute_comme_curl() {
    assert_eq!(
        parse::parse("curl localhost:8080/x").unwrap().url,
        "http://localhost:8080/x"
    );
}

#[test]
fn r4_en_tetes_vides_et_retires() {
    let request = parse::parse("curl https://a.test -H 'X-Vide;' -H 'Accept:' -H 'Bad'").unwrap();
    assert_eq!(request.headers, [("X-Vide".to_owned(), String::new())]);
    assert_eq!(request.ignored.len(), 1);
    assert!(request.ignored[0].reason.contains("Nom: valeur"));
}

#[test]
fn r4_sans_adresse_ou_sans_curl() {
    assert!(
        parse::parse("curl -X POST")
            .unwrap_err()
            .contains("adresse")
    );
    assert!(
        parse::parse("wget https://a.test")
            .unwrap_err()
            .contains("curl")
    );
    assert!(parse::is_curl("curl https://a.test"));
    assert!(!parse::is_curl("curl"));
    assert!(!parse::is_curl("curling https://a.test"));
}

// --- Options sans équivalent (R6) ------------------------------------------------

#[test]
fn r6_options_signalees_une_fois() {
    let request = parse::parse(
        "curl -sSLk https://a.test --compressed --cert c.pem --max-time 5 -x proxy:3128 -o out --frobnicate -F a=b https://autre.test",
    )
    .unwrap();
    let options: Vec<&str> = request.ignored.iter().map(|i| i.option.as_str()).collect();
    assert_eq!(
        options,
        [
            "-k",
            "--compressed",
            "--cert",
            "--max-time",
            "-x",
            "-o",
            "--frobnicate",
            "-F",
            "https://autre.test"
        ]
    );
    // La valeur d'une option signalée n'est pas prise pour l'adresse.
    assert_eq!(request.url, "https://a.test");
    // -s, -S, -L : sans effet sur la requête, sans message.
    let quiet = parse::parse("curl -s -S -L -v -i https://a.test").unwrap();
    assert!(quiet.ignored.is_empty());
}

#[test]
fn r6_fichiers_non_lus() {
    let request = parse::parse("curl https://a.test -d @corps.json -b cookies.txt").unwrap();
    assert_eq!(request.body, None);
    assert_eq!(request.ignored.len(), 2);
}

// --- Secrets (R5) ---------------------------------------------------------------

#[test]
fn r5_jeton_et_identifiants_retires() {
    let mut headers = vec![
        ("Authorization".to_owned(), "Bearer abc.def".to_owned()),
        ("X-Api-Key".to_owned(), "k-123".to_owned()),
        ("Cookie".to_owned(), "session=9".to_owned()),
        ("Content-Type".to_owned(), "application/json".to_owned()),
    ];
    let secrets = take_secrets(&mut headers);
    assert_eq!(headers[0].1, "Bearer {{token}}");
    assert_eq!(headers[1].1, "{{xApiKey}}");
    assert_eq!(headers[2].1, "{{cookie}}");
    assert_eq!(headers[3].1, "application/json");
    let found: Vec<(&str, &str)> = secrets
        .iter()
        .map(|s| (s.name.as_str(), s.value.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("token", "abc.def"),
            ("xApiKey", "k-123"),
            ("cookie", "session=9")
        ]
    );
}

#[test]
fn r5_basic_et_autre_schema() {
    let mut headers = vec![
        ("Authorization".to_owned(), "Basic YTpi".to_owned()),
        ("authorization".to_owned(), "Digest x".to_owned()),
    ];
    let secrets = take_secrets(&mut headers);
    assert_eq!(headers[0].1, "Basic {{basicAuth}}");
    assert_eq!(headers[1].1, "{{authorization}}");
    assert_eq!(secrets.len(), 2);
}

#[test]
fn r5_variable_du_shell_et_variable_existante() {
    let mut headers = vec![
        ("Authorization".to_owned(), "Bearer $TOKEN".to_owned()),
        ("X-Api-Key".to_owned(), "${API_KEY}".to_owned()),
        ("X-Token".to_owned(), "{{monJeton}}".to_owned()),
    ];
    let secrets = take_secrets(&mut headers);
    assert!(secrets.is_empty());
    assert_eq!(headers[0].1, "Bearer {{token}}");
    assert_eq!(headers[1].1, "{{apiKey}}");
    assert_eq!(headers[2].1, "{{monJeton}}");
}

#[test]
fn r5_noms_uniques() {
    let mut headers = vec![
        ("Authorization".to_owned(), "Bearer a".to_owned()),
        ("Authorization".to_owned(), "Bearer b".to_owned()),
    ];
    let secrets = take_secrets(&mut headers);
    assert_eq!(headers[1].1, "Bearer {{token2}}");
    assert_eq!(secrets[1].name, "token2");
}

#[test]
fn noms_en_camel_case() {
    assert_eq!(camel("X-Api-Key"), "xApiKey");
    assert_eq!(camel("API_KEY"), "apiKey");
    assert_eq!(camel("token"), "token");
}

#[test]
fn forme_json_d_un_secret() {
    let mut headers = vec![("Authorization".to_owned(), "Bearer a".to_owned())];
    let secrets = take_secrets(&mut headers);
    assert_eq!(
        serde_json::to_value(&secrets).unwrap(),
        serde_json::json!([{ "name": "token", "value": "a", "header": "Authorization" }])
    );
    let ignored = parse::parse("curl https://a.test -k").unwrap().ignored;
    assert_eq!(
        serde_json::to_value(&ignored).unwrap()[0]["option"],
        serde_json::json!("-k")
    );
}

// --- Écriture (R8 à R10) ----------------------------------------------------------

#[test]
fn r8_guillemets_du_shell() {
    assert_eq!(quote("a b"), "'a b'");
    assert_eq!(quote("l'eau"), r"'l'\''eau'");
    let bearer = format!("Authorization: Bearer {}", placeholder("TOKEN"));
    assert_eq!(quote(&bearer), "\"Authorization: Bearer $TOKEN\"");
    // Texte autour d'une variable : échappé, nom délimité s'il est collé.
    let glued = format!("a\"$`{}b", placeholder("X"));
    assert_eq!(quote(&glued), r#""a\"\$\`${X}b""#);
}

#[test]
fn r8_noms_de_variables_du_shell() {
    assert_eq!(shell_name("token"), "TOKEN");
    assert_eq!(shell_name("apiKey"), "API_KEY");
    assert_eq!(shell_name("x-api-key"), "X_API_KEY");
    assert_eq!(shell_name("2fa"), "V_2FA");
}

fn prepared(method: Method, body: Option<&str>) -> Prepared {
    Prepared {
        method,
        url: "https://recette.boutique.dev/orders".to_owned(),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        body: body.map(str::to_owned),
        timeout_ms: None,
        max_redirects: 5,
        drawn: Vec::new(),
        used: Vec::new(),
    }
}

#[test]
fn r8_une_option_par_ligne() {
    let text = render::render(&prepared(Method::Post, Some("{\"a\":1}")), None);
    assert_eq!(
        text,
        "curl -X POST 'https://recette.boutique.dev/orders' \\\n  -H 'Content-Type: application/json' \\\n  --data '{\"a\":1}'"
    );
    let get = render::render(&prepared(Method::Get, None), None);
    assert!(get.starts_with("curl 'https://"));
    let head = render::render(&prepared(Method::Head, None), None);
    assert!(head.starts_with("curl -I '"));
    let raw = render::render(&prepared(Method::Put, Some("@x")), None);
    assert!(raw.contains("--data-raw '@x'"));
}

#[test]
fn r9_authorization_ecrite_en_clair() {
    let mut headers = vec![
        ("Authorization".to_owned(), "Bearer abc".to_owned()),
        ("X".to_owned(), "Bearer abc".to_owned()),
    ];
    hide_credentials(&mut headers);
    assert_eq!(headers[0].1, format!("Bearer {}", placeholder("TOKEN")));
    assert_eq!(headers[1].1, "Bearer abc");
    let mut basic = vec![("authorization".to_owned(), "Basic YTpi".to_owned())];
    hide_credentials(&mut basic);
    assert_eq!(basic[0].1, format!("Basic {}", placeholder("CREDENTIALS")));
}

fn vars(pairs: &[(&str, &str)], secrets: &[&str]) -> Variables {
    Variables {
        values: pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        secrets: secrets
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>(),
        environment: "Recette".to_owned(),
    }
}

#[test]
fn r10_requete_de_la_maquette() {
    let mut request = Request::new("Créer une commande");
    request.method = Method::Post;
    request.url = "{{baseUrl}}/orders".to_owned();
    request.headers = vec![
        Pair::new("Content-Type", "application/json"),
        Pair::new("X-Client", "arnod-tests"),
        Pair {
            enabled: false,
            ..Pair::new("X-Debug", "true")
        },
    ];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "{{token}}".to_owned(),
        ..Auth::default()
    };
    request.body = Body::raw(BodyKind::Json, "{\"customerId\":\"cus_31\"}");
    let v = vars(
        &[
            ("baseUrl", "https://recette.boutique.dev"),
            ("token", "s3cret"),
        ],
        &["token"],
    );
    let text = render::command(&request, &v).unwrap();
    assert_eq!(
        text,
        "curl -X POST 'https://recette.boutique.dev/orders' \\\n  -H 'Content-Type: application/json' \\\n  -H 'X-Client: arnod-tests' \\\n  -H \"Authorization: Bearer $TOKEN\" \\\n  --data '{\"customerId\":\"cus_31\"}'"
    );
    assert!(!text.contains("s3cret"));
}

#[test]
fn r10_secrets_basic_cle_et_variables_inconnues() {
    let mut request = Request::new("t");
    request.url = "https://a.test/{{chemin}}?t={{$timestamp}}".to_owned();
    request.headers = vec![Pair::new("X-Secret", "{{motDePasse}}")];
    request.auth = Auth {
        kind: AuthKind::Basic,
        username: "{{user}}".to_owned(),
        password: "p4ss".to_owned(),
        ..Auth::default()
    };
    let v = vars(&[("user", "ana"), ("motDePasse", "x")], &["motDePasse"]);
    let text = render::command(&request, &v).unwrap();
    assert!(
        text.contains("'https://a.test/{{chemin}}?t={{$timestamp}}'"),
        "{text}"
    );
    assert!(text.contains("-H \"X-Secret: $MOT_DE_PASSE\""), "{text}");
    assert!(text.contains("-u \"ana:$PASSWORD\""), "{text}");
    assert!(!text.contains("p4ss") && !text.contains("Basic"));

    request.auth = Auth {
        kind: AuthKind::ApiKey,
        name: "key".to_owned(),
        value: "k".to_owned(),
        location: ApiKeyLocation::Query,
        ..Auth::default()
    };
    let text = render::command(&request, &v).unwrap();
    assert!(text.contains("key=$API_KEY"), "{text}");
    assert!(text.starts_with("curl \""), "{text}");
}

#[test]
fn r10_authentification_heritee() {
    let mut request = Request::new("t");
    request.url = "https://a.test".to_owned();
    request.auth.kind = AuthKind::Inherit;
    request.inherited.auth = Some(InheritedAuth {
        auth: Auth {
            kind: AuthKind::Bearer,
            token: "jeton".to_owned(),
            ..Auth::default()
        },
        source: "collection « B »".to_owned(),
    });
    let text = render::command(&request, &vars(&[], &[])).unwrap();
    assert!(text.contains("Bearer $TOKEN"));
    assert!(!text.contains("jeton"));
}

#[test]
fn r10_adresse_sans_schema_refusee() {
    let request = Request::new("t");
    let error = render::command(&request, &vars(&[], &[])).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
}

#[test]
fn r10_aller_retour_curl() {
    // Une commande copiée puis recollée redonne la requête, jeton en variable.
    let mut request = Request::new("t");
    request.method = Method::Put;
    request.url = "https://a.test/x".to_owned();
    request.headers = vec![Pair::new("Accept", "text/plain")];
    request.auth = Auth {
        kind: AuthKind::Bearer,
        token: "abc".to_owned(),
        ..Auth::default()
    };
    request.body = Body::raw(BodyKind::Text, "l'eau");
    let text = render::command(&request, &vars(&[], &[])).unwrap();
    let mut parsed = parse::parse(&text).unwrap();
    assert_eq!(parsed.method, "PUT");
    assert_eq!(parsed.url, "https://a.test/x");
    assert_eq!(parsed.body.as_deref(), Some("l'eau"));
    let secrets = take_secrets(&mut parsed.headers);
    assert!(secrets.is_empty());
    assert_eq!(header(&parsed, "Authorization"), Some("Bearer {{token}}"));
}
