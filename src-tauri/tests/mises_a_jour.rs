//! Lot L (`docs/plan-mvp-l-mises-a-jour.md`) : vérification des paquets par `tauri-plugin-updater`
//! dans une application Tauri simulée (`tauri::test`), avec une vraie paire de clés d'essai.
//!
//! Test d'intégration plutôt que test unitaire : sous Windows, une application Tauri, même
//! simulée, importe des fonctions des Common Controls v6, que le système ne charge qu'avec le
//! manifeste d'application. `build.rs` l'ajoute aux tests d'intégration (`rustc-link-arg-tests`) ;
//! `tauri-build` ne l'ajoute qu'aux exécutables de l'application.

use std::sync::Arc;

use arnod_lib::commands::updates::{UpdatesState, fetch, pubkey, restore};
use arnod_lib::http::Cancel;
use arnod_lib::network::Network;
use arnod_lib::network::proxy::FixedProxySource;
use arnod_lib::secrets::MemorySecretStore;
use arnod_lib::updates::{self, Release};
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::Update;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Paire de clés d'essai, générée pour ces tests seulement (`tauri signer generate`).
const TEST_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEYzRDUzRkUxMkY3NDhGQzMKUldURGozUXY0VC9WOHhKMDZ0cEY1b2JibGwrVGdWTDVYSlJlbTg2YkFCc09iMGZXRUVsZC9MZkkK";
const OTHER_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhDMUJENEY2MDU0MUIwRjYKUldUMnNFRUY5dFFiaktRS1BIYk93YWtxSzh5ZXV0QVRORXQ4UVdkWm05T1ZneTZZNGZieEdYUFUK";
/// `PACKAGE` signé par la clé d'essai pour la version 0.2.0 (`tauri signer sign --app-version`).
const PACKAGE: &[u8] = b"Paquet d essai Arnod 0.2.0\n";
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVURGozUXY0VC9WODdyeEVHblNYbGE3UEtSQklSMFN3c0t1a05NUkVDdnRwakhVdzlMLzgzQzZrYmY2ekY0eTYwU0UyTFhRWmJWWEY1ZFJNK1Y5K1NManR1aytXcDBndEE0PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0ODU5MzkzCWZpbGU6cGFxdWV0LmJpbgl2ZXJzaW9uOjAuMi4wCmtZMGZtTXRXVDZjeWRsTlZRcW9RSTdYSkg1R0JsSk9DdTg0eWpJMk96M2JvNkk0WHpTenlwbUsyL1RLS1hnM1BSL1lJRHpRajVvSVpoNWdTVjNFdkNBPT0K";

/// Application simulée (version 0.1.0) avec le plugin et la clé `pubkey`. Les adresses `http://`
/// des serveurs de test sont admises par le plugin.
fn mock_app(pubkey: &str) -> tauri::App<MockRuntime> {
    let mut context = mock_context(noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".to_owned(),
        serde_json::json!({
            "pubkey": pubkey,
            "requireSignedVersion": true,
            "dangerousInsecureTransportProtocol": true,
        }),
    );
    mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(UpdatesState::default())
        .build(context)
        .unwrap()
}

fn manifest(version: &str, url: &str) -> serde_json::Value {
    let entry = serde_json::json!({ "url": url, "signature": SIGNATURE });
    serde_json::json!({
        "version": version,
        "pub_date": "2026-11-20T10:00:00Z",
        "platforms": {
            "windows-x86_64": entry,
            "linux-x86_64": entry,
            "darwin-aarch64": entry,
            "darwin-x86_64": entry,
        }
    })
}

fn restored(app: &tauri::App<MockRuntime>, version: &str) -> Update {
    restore(
        app.handle(),
        "https://github.com/juliencorre/Arnod/releases/download/v0.2.0/latest.json",
        manifest(version, "https://exemple.fr/paquet"),
    )
    .unwrap()
    .unwrap()
}

#[test]
fn r3_paquet_signe_accepte_modifie_ou_autre_cle_refuse() {
    let app = mock_app(TEST_PUBKEY);
    let update = restored(&app, "0.2.0");
    assert_eq!(update.version, "0.2.0");
    update.verify(PACKAGE).unwrap();

    let mut modified = PACKAGE.to_vec();
    modified[0] = b'p';
    let error = update.verify(&modified).unwrap_err();
    assert!(
        updates::install_message(&error, "0.2.0").contains("ne correspond pas"),
        "{error}"
    );

    let other = mock_app(OTHER_PUBKEY);
    let error = restored(&other, "0.2.0").verify(PACKAGE).unwrap_err();
    assert!(updates::install_message(&error, "0.2.0").contains("rien n'a été installé"));
}

#[test]
fn r3_signature_d_une_autre_version_refusee() {
    let app = mock_app(TEST_PUBKEY);
    // Le manifeste annonce 0.3.0, le paquet est signé pour 0.2.0.
    let error = restored(&app, "0.3.0").verify(PACKAGE).unwrap_err();
    assert!(
        updates::install_message(&error, "0.3.0").contains("signé pour la version 0.2.0"),
        "{error}"
    );
}

#[test]
fn r2_version_pas_plus_recente_ignoree() {
    let app = mock_app(TEST_PUBKEY);
    let none = restore(
        app.handle(),
        "https://exemple.fr/latest.json",
        manifest("0.1.0", "https://exemple.fr/paquet"),
    )
    .unwrap();
    assert!(none.is_none());
}

#[test]
fn r1_cle_publique_lue_dans_la_configuration() {
    assert_eq!(pubkey(mock_app(TEST_PUBKEY).handle()), TEST_PUBKEY);
    assert_eq!(pubkey(mock_app("").handle()), "");
}

#[tokio::test]
async fn mj1_manifeste_et_paquet_telecharges_puis_verifies() {
    let server = MockServer::start().await;
    let package_url = format!("{}/paquet.bin", server.uri());
    Mock::given(method("GET"))
        .and(path("/latest.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest("0.2.0", &package_url)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/paquet.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(PACKAGE))
        .mount(&server)
        .await;
    let release = Release {
        version: "0.2.0".to_owned(),
        prerelease: false,
        highlights: Vec::new(),
        url: format!("{}/tag/v0.2.0", updates::RELEASES_PAGE),
        published: None,
        manifest: Some(format!("{}/latest.json", server.uri())),
    };
    let network = Network::new(
        Arc::new(MemorySecretStore::default()),
        Box::new(FixedProxySource::default()),
    );
    let app = mock_app(TEST_PUBKEY);
    let mut seen = Vec::new();
    let pending = fetch(app.handle(), &network, &release, &Cancel::new(), |r, t| {
        seen.push((r, t));
    })
    .await
    .unwrap();
    assert_eq!(pending.bytes, PACKAGE);
    assert_eq!(
        seen.last(),
        Some(&(PACKAGE.len() as u64, Some(PACKAGE.len() as u64)))
    );

    // Une autre clé : le paquet téléchargé est refusé (R3).
    let other = mock_app(OTHER_PUBKEY);
    let error = fetch(
        other.handle(),
        &network,
        &release,
        &Cancel::new(),
        |_, _| {},
    )
    .await
    .err()
    .unwrap();
    assert!(
        error.message.contains("rien n'a été installé"),
        "{}",
        error.message
    );

    // Une release qui annonce une autre version que son manifeste (R2).
    let mut other_version = release.clone();
    other_version.version = "0.2.1".to_owned();
    let error = fetch(
        app.handle(),
        &network,
        &other_version,
        &Cancel::new(),
        |_, _| {},
    )
    .await
    .err()
    .unwrap();
    assert!(
        error.message.contains("annonce la version 0.2.0"),
        "{}",
        error.message
    );
}
