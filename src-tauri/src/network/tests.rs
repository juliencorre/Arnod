//! Envois réels sur des serveurs locaux : chaque cause d'échec (R5 à R7), jetons OAuth 2 (R9),
//! cookies (R12) et préférences (R1).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::failure::FailureCause;
use super::proxy::{FixedProxySource, ProxyMode, SystemProxy};
use super::testcert::{Key, Spec, certificate};
use super::tls::CertificateProblem;
use super::*;
use crate::request::model::{Method, TokenPlacement};
use crate::secrets::MemorySecretStore;

fn network() -> Network {
    Network::new(
        Arc::new(MemorySecretStore::default()),
        Box::new(FixedProxySource(SystemProxy::default())),
    )
}

fn prepared(url: impl Into<String>) -> Prepared {
    Prepared {
        method: Method::Get,
        url: url.into(),
        headers: Vec::new(),
        body: None,
        timeout_ms: Some(5_000),
        max_redirects: 0,
        drawn: Vec::new(),
        used: Vec::new(),
        oauth2: None,
        hidden: Vec::new(),
    }
}

fn session() -> Session {
    Session {
        project: Uuid::nil(),
        environment: "Recette".into(),
    }
}

async fn fail(net: &Network, url: &str) -> CommandError {
    net.send(&mut prepared(url), None, &Cancel::new())
        .await
        .unwrap_err()
}

// ------------------------------------------------------------------ TLS

struct Ca {
    spec: Spec<'static>,
    key: Key,
    der: Vec<u8>,
}

fn ca(name: &'static str) -> Ca {
    let spec = Spec::authority(name);
    let key = Key::generate();
    let der = certificate(&spec, &key, None);
    Ca { spec, key, der }
}

fn leaf(ca: &Ca, name: &'static str, expired: bool) -> (CertificateDer<'static>, Key) {
    let mut spec = Spec::server(name);
    if expired {
        spec.not_before = "20200101";
        spec.not_after = "20210101";
    }
    let key = Key::generate();
    let der = certificate(&spec, &key, Some((&ca.spec, &ca.key)));
    (CertificateDer::from(der), key)
}

/// Serveur https minimal qui répond 200 « ok » avec la chaîne `chain`.
fn tls_server(chain: Vec<CertificateDer<'static>>, key: &Key) -> u16 {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.pkcs8.clone()));
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let config = config.clone();
            std::thread::spawn(move || {
                let Ok(conn) = rustls::ServerConnection::new(config) else {
                    return;
                };
                let mut tls = rustls::StreamOwned::new(conn, stream);
                let mut buf = [0u8; 4096];
                if matches!(tls.read(&mut buf), Ok(n) if n > 0) {
                    let _ = tls.write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
                    );
                    let _ = tls.flush();
                }
            });
        }
    });
    port
}

#[tokio::test]
async fn r7_autorite_inconnue_puis_ajoutee() {
    let authority = ca("Boutique Interne CA");
    let (cert, key) = leaf(&authority, "localhost", false);
    let port = tls_server(
        vec![cert, CertificateDer::from(authority.der.clone())],
        &key,
    );
    let url = format!("https://localhost:{port}/orders");
    let net = network();

    let error = fail(&net, &url).await;
    assert_eq!(error.kind, CommandErrorKind::Network);
    let failure = error.failure.clone().expect("échec classé");
    assert_eq!(failure.cause, FailureCause::Certificate);
    assert!(failure.not_sent);
    assert_eq!(failure.host, "localhost");
    let certificate = failure.certificate.clone().unwrap();
    assert_eq!(certificate.problem, CertificateProblem::UnknownIssuer);
    assert_eq!(certificate.server.issuer, "Boutique Interne CA");
    let candidate = certificate.authority.expect("autorité présentée");
    assert_eq!(candidate.subject, "Boutique Interne CA");
    assert!(
        error.message.contains("n'est pas reconnu"),
        "{}",
        error.message
    );
    // Forme JSON de l'erreur : `failure` en camelCase.
    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["failure"]["cause"], "certificate");
    assert_eq!(json["failure"]["certificate"]["problem"], "unknownIssuer");
    assert_eq!(json["failure"]["notSent"], true);

    // Faire confiance : le client est reconstruit et l'envoi aboutit (R1, R4).
    let view = net.add_authority(&candidate.pem).unwrap();
    assert_eq!(view.authorities.len(), 1);
    // Une deuxième fois : pas de doublon.
    assert_eq!(
        net.add_authority(&candidate.pem).unwrap().authorities.len(),
        1
    );
    let response = net
        .send(&mut prepared(&url), None, &Cancel::new())
        .await
        .unwrap();
    assert_eq!(response.status, 200);

    // Retirée : refusée de nouveau.
    net.remove_authority(&candidate.fingerprint).unwrap();
    assert_eq!(
        fail(&net, &url).await.failure.unwrap().cause,
        FailureCause::Certificate
    );
    assert_eq!(
        net.remove_authority(&candidate.fingerprint)
            .unwrap_err()
            .kind,
        CommandErrorKind::NotFound
    );
}

#[tokio::test]
async fn r7_certificat_expire_ou_pour_un_autre_nom_sans_confiance_proposee() {
    let authority = ca("Interne");
    let net = network();
    net.add_authority(&cert::to_pem(&authority.der)).unwrap();

    let (expired, key) = leaf(&authority, "localhost", true);
    let port = tls_server(vec![expired], &key);
    let failure = fail(&net, &format!("https://localhost:{port}/"))
        .await
        .failure
        .unwrap();
    let certificate = failure.certificate.unwrap();
    assert_eq!(certificate.problem, CertificateProblem::Expired);
    assert!(certificate.authority.is_none());
    assert!(
        failure.explanation.contains("01/01/2021"),
        "{}",
        failure.explanation
    );

    let (other, key) = leaf(&authority, "autre.boutique.local", false);
    let port = tls_server(vec![other], &key);
    let failure = fail(&net, &format!("https://localhost:{port}/"))
        .await
        .failure
        .unwrap();
    assert_eq!(
        failure.certificate.unwrap().problem,
        CertificateProblem::WrongName
    );
    assert!(failure.explanation.contains("autre.boutique.local"));
}

#[tokio::test]
async fn r7_certificat_auto_signe_est_sa_propre_autorite() {
    let key = Key::generate();
    let der = certificate(&Spec::server("localhost"), &key, None);
    let port = tls_server(vec![CertificateDer::from(der)], &key);
    let failure = fail(&network(), &format!("https://localhost:{port}/"))
        .await
        .failure
        .unwrap();
    let certificate = failure.certificate.unwrap();
    assert_eq!(
        certificate.authority.map(|a| a.fingerprint),
        Some(certificate.server.fingerprint)
    );
}

// ------------------------------------------------------------------ proxy, nom, serveur

/// Faux proxy qui répond 407 à tout.
fn proxy_407() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut socket in listener.incoming().flatten() {
            let mut buf = [0u8; 2048];
            let _ = socket.read(&mut buf);
            let _ = socket.write_all(
                b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"corp\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    port
}

fn with_manual_proxy(net: &Network, url: &str) {
    let mut proxy = net.prefs().proxy;
    proxy.mode = ProxyMode::Manual;
    proxy.manual.url = url.to_owned();
    net.save(NetworkPrefsInput {
        proxy,
        proxy_password: None,
        connect_timeout_ms: 2_000,
    })
    .unwrap();
}

#[tokio::test]
async fn r5_proxy_qui_demande_une_identification() {
    let net = network();
    with_manual_proxy(&net, &format!("127.0.0.1:{}", proxy_407()));
    for url in ["https://api.boutique.test/", "http://api.boutique.test/"] {
        let failure = fail(&net, url).await.failure.unwrap();
        assert_eq!(failure.cause, FailureCause::ProxyAuth, "{url}");
        assert!(failure.proxy.is_some());
        assert!(failure.not_sent);
    }
}

#[tokio::test]
async fn r5_proxy_injoignable_et_exception() {
    let net = network();
    with_manual_proxy(&net, "127.0.0.1:1");
    let failure = fail(&net, "http://api.boutique.test/")
        .await
        .failure
        .unwrap();
    assert_eq!(failure.cause, FailureCause::Proxy);
    assert_eq!(failure.proxy.as_deref(), Some("http://127.0.0.1:1"));
    assert!(failure.title.contains("127.0.0.1:1"));
}

#[tokio::test]
async fn r5_nom_introuvable() {
    let failure = fail(&network(), "http://nexiste-pas.invalid/")
        .await
        .failure
        .unwrap();
    assert_eq!(failure.cause, FailureCause::Dns);
    assert!(failure.title.contains("nexiste-pas.invalid"));
}

#[tokio::test]
async fn r5_serveur_injoignable() {
    let error = fail(&network(), "http://127.0.0.1:1/").await;
    let failure = error.failure.unwrap();
    assert_eq!(failure.cause, FailureCause::Unreachable);
    assert_eq!(failure.port, 1);
    assert!(failure.not_sent);
}

#[tokio::test]
async fn r6_delai_depasse_la_requete_a_pu_arriver() {
    let server = MockServer::start().await;
    Mock::given(path("/lent"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(800)))
        .mount(&server)
        .await;
    let mut p = prepared(format!("{}/lent", server.uri()));
    p.timeout_ms = Some(100);
    let error = network()
        .send(&mut p, None, &Cancel::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::Timeout);
    let failure = error.failure.unwrap();
    assert_eq!(failure.cause, FailureCause::Timeout);
    assert!(!failure.not_sent);
}

#[test]
fn r1_preferences_ecrites_et_relues() {
    let dir = tempfile::tempdir().unwrap();
    let net = network();
    net.attach(dir.path());
    with_manual_proxy(&net, "proxy.corp:8080");
    let again = network();
    again.attach(dir.path());
    assert_eq!(again.prefs().proxy.manual.url, "proxy.corp:8080");
    assert_eq!(again.prefs().connect_timeout_ms, 2_000);
    let proxy = again.effective_proxy();
    assert_eq!(
        proxy.for_url("https", "api.test").as_deref(),
        Some("http://proxy.corp:8080")
    );
    // Adresse illisible refusée.
    let mut bad = again.prefs().proxy;
    bad.manual.url = "http://a b".into();
    let error = again
        .save(NetworkPrefsInput {
            proxy: bad,
            proxy_password: None,
            connect_timeout_ms: 10,
        })
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
}

#[test]
fn mot_de_passe_du_proxy_au_coffre() {
    let net = network();
    let mut proxy = net.prefs().proxy;
    proxy.mode = ProxyMode::Manual;
    proxy.manual.url = "p:8080".into();
    proxy.manual.username = "julien".into();
    let view = net
        .save(NetworkPrefsInput {
            proxy: proxy.clone(),
            proxy_password: Some("motdepasse".into()),
            connect_timeout_ms: 10_000,
        })
        .unwrap();
    assert!(view.has_proxy_password);
    assert_eq!(
        net.effective_proxy().password.as_deref(),
        Some("motdepasse")
    );
    // Rien du mot de passe dans la vue sérialisée.
    assert!(!serde_json::to_string(&view).unwrap().contains("motdepasse"));
    let view = net
        .save(NetworkPrefsInput {
            proxy,
            proxy_password: Some(String::new()),
            connect_timeout_ms: 10_000,
        })
        .unwrap();
    assert!(!view.has_proxy_password);
}

// ------------------------------------------------------------------ OAuth 2

fn grant(server: &MockServer) -> OAuth2Grant {
    OAuth2Grant {
        token_url: format!("{}/oauth/token", server.uri()),
        client_id: "boutique".into(),
        client_secret: "s3cr3t".into(),
        scope: "orders:read".into(),
        credentials: CredentialsPlacement::BasicAuthHeader,
        placement: TokenPlacement::Header,
        token_name: "Authorization".into(),
    }
}

#[tokio::test]
async fn r9_jeton_obtenu_une_fois_puis_renouvele() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=client_credentials"))
        .and(body_string_contains("scope=orders%3Aread"))
        // `boutique:s3cr3t` en Base64.
        .and(header("authorization", "Basic Ym91dGlxdWU6czNjcjN0"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"abc","expires_in":3600}"#),
        )
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(path("/orders"))
        .and(header("authorization", "Bearer abc"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&server)
        .await;
    let net = network();
    let s = session();
    for _ in 0..2 {
        let mut p = prepared(format!("{}/orders", server.uri()));
        p.oauth2 = Some(grant(&server));
        let response = net.send(&mut p, Some(&s), &Cancel::new()).await;
        let response = response.unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(response.status, 200);
        assert_eq!(p.hidden, ["abc"], "le jeton est à masquer");
    }
    let status = net.token_status(s.project, "Recette").unwrap();
    assert!(status.expires_at.unwrap() > status.obtained_at);
    // « Renouveler maintenant » : redemandé.
    net.token(Some(&s), &grant(&server), true, &Cancel::new())
        .await
        .unwrap();
    net.forget_tokens(s.project, "Recette");
    assert!(net.token_status(s.project, "Recette").is_none());
    server.verify().await;
}

#[tokio::test]
async fn r9_refus_du_serveur_de_jetons() {
    let server = MockServer::start().await;
    Mock::given(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string(r#"{"error":"invalid_client"}"#))
        .mount(&server)
        .await;
    Mock::given(path("/orders"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let mut p = prepared(format!("{}/orders", server.uri()));
    p.oauth2 = Some(grant(&server));
    let error = network()
        .send(&mut p, Some(&session()), &Cancel::new())
        .await
        .unwrap_err();
    assert!(
        error.message.contains("invalid_client"),
        "{}",
        error.message
    );
    assert!(!error.message.contains("s3cr3t"));
    server.verify().await;
}

// ------------------------------------------------------------------ cookies

#[tokio::test]
async fn r12_cookie_garde_et_renvoye_par_environnement() {
    let server = MockServer::start().await;
    Mock::given(path("/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=xyz; Path=/; HttpOnly; Max-Age=600"),
        )
        .mount(&server)
        .await;
    Mock::given(path("/orders"))
        .and(header("cookie", "session=xyz"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/orders"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let net = network();
    net.attach(dir.path());
    let recette = session();
    let mut login = prepared(format!("{}/login", server.uri()));
    login.method = Method::Post;
    net.send(&mut login, Some(&recette), &Cancel::new())
        .await
        .unwrap();
    let mut orders = prepared(format!("{}/orders", server.uri()));
    let response = net
        .send(&mut orders, Some(&recette), &Cancel::new())
        .await
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(orders.hidden, ["xyz"], "HttpOnly : masqué dans « Envoyé »");
    // Autre environnement : rien.
    let local = Session {
        environment: "Local".into(),
        ..recette.clone()
    };
    let other = net
        .send(
            &mut prepared(format!("{}/orders", server.uri())),
            Some(&local),
            &Cancel::new(),
        )
        .await
        .unwrap();
    assert_eq!(other.status, 401);
    // Liste : d'où il vient, valeur masquée ; persistant, relu par un autre réseau.
    let view = net.cookies(&recette, Some("127.0.0.1"));
    assert_eq!(view.domains[0].cookies[0].received_from, "POST /login");
    assert_eq!(view.domains[0].cookies[0].value, "••••••••");
    let again = network();
    again.attach(dir.path());
    assert_eq!(again.cookies(&recette, None).domains.len(), 1);
    // Vider.
    net.clear_cookies(&recette, None);
    assert!(net.cookies(&recette, None).domains.is_empty());
    server.verify().await;
}

#[test]
fn unions_typescript_concordantes() {
    use super::proxy::ProxySource;
    use crate::request::model::{CredentialsPlacement as C, TokenPlacement as T};
    use crate::ts_contract::{serialized_name, union_members};
    use std::collections::BTreeSet;

    fn names<V: serde::Serialize>(values: impl IntoIterator<Item = V>) -> BTreeSet<String> {
        values.into_iter().map(serialized_name).collect()
    }
    use FailureCause as F;
    assert_eq!(
        union_members("FailureCause"),
        names([
            F::Certificate,
            F::Proxy,
            F::ProxyAuth,
            F::Dns,
            F::Unreachable,
            F::Timeout,
            F::Tls,
            F::Other
        ])
    );
    use CertificateProblem as P;
    assert_eq!(
        union_members("CertificateProblem"),
        names([
            P::UnknownIssuer,
            P::Expired,
            P::NotYetValid,
            P::WrongName,
            P::Revoked,
            P::Other
        ])
    );
    assert_eq!(
        union_members("ProxyMode"),
        names([ProxyMode::System, ProxyMode::None, ProxyMode::Manual])
    );
    assert_eq!(
        union_members("ProxySource"),
        names([ProxySource::Environment, ProxySource::Windows])
    );
    assert_eq!(
        union_members("CredentialsPlacement"),
        names([C::BasicAuthHeader, C::Body])
    );
    assert_eq!(
        union_members("TokenPlacement"),
        names([T::Header, T::Query])
    );
}
