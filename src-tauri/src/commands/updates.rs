//! Domaine « mises à jour » (écran 1.6-g) : préférences du poste, vérification des versions
//! publiées et ouverture de la page d'une release (lot L allégé) ; téléchargement, vérification
//! de la signature et installation (lot L, `docs/plan-mvp-l-mises-a-jour.md`).
//!
//! Arnod lit le manifeste `latest.json` et télécharge le paquet par `network::Network` (proxy,
//! autorités internes, R4) ; le plugin `tauri-plugin-updater` reprend le manifeste, vérifie le
//! paquet (R3) et l'installe (R7). L'état vit dans [`UpdatesState`] et se diffuse à toutes les
//! fenêtres par `arnod://mise-a-jour` (R8).

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, Runtime, State, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{CommandError, CommandErrorKind};
use crate::http::Cancel;
use crate::network::{Network, now_ms};
use crate::request::model::Method;
use crate::request::resolve::Prepared;
use crate::state::AppState;
use crate::updates::{self, InstallState, Release, UpdatePrefs, UpdatesView};
use crate::windows::{WindowsState, events};
use crate::workbench::Workbench;

/// Paquet téléchargé et vérifié, prêt à installer.
struct Pending {
    update: Update,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct Session {
    /// Résultat de la dernière vérification : `None` tant qu'il n'y en a pas eu.
    found: Option<Option<Release>>,
    install: InstallState,
    pending: Option<Pending>,
    /// Annulation du téléchargement en cours (R5).
    cancel: Option<Arc<Cancel>>,
}

impl Session {
    /// Oublie le paquet et arrête le téléchargement en cours (R5).
    fn forget_install(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
        }
        self.pending = None;
        self.install = InstallState::Idle;
    }
}

/// État des mises à jour de la session, confié à Tauri (`app.manage`).
#[derive(Default)]
pub struct UpdatesState(Mutex<Session>);

impl UpdatesState {
    fn lock(&self) -> Result<MutexGuard<'_, Session>, CommandError> {
        self.0.lock().map_err(|_| {
            CommandError::new(
                CommandErrorKind::Io,
                "L'état des mises à jour est indisponible : redémarrez l'application.",
            )
        })
    }
}

/// Clé publique embarquée (`plugins.updater.pubkey` de `tauri.conf.json`).
fn pubkey<R: Runtime>(app: &AppHandle<R>) -> String {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Ce poste est installé par un paquet que le plugin remplace (R1) : l'installeur NSIS sous
/// Windows, l'AppImage lancée comme telle sous Linux.
fn platform_supported<R: Runtime>(app: &AppHandle<R>) -> bool {
    #[cfg(target_os = "linux")]
    {
        app.env().appimage.is_some()
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        true
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = app;
        false
    }
}

fn view<R: Runtime>(app: &AppHandle<R>, session: &Session, prefs: UpdatePrefs) -> UpdatesView {
    let latest = session.found.clone().flatten();
    UpdatesView {
        current: env!("CARGO_PKG_VERSION").to_owned(),
        prefs,
        checked: session.found.is_some(),
        installable: latest
            .as_ref()
            .is_some_and(|r| updates::installable(r, &pubkey(app), platform_supported(app))),
        latest,
        install: session.install.clone(),
    }
}

/// Vue courante, lue sous le verrou de la session.
fn current_view<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    updates: &UpdatesState,
) -> Result<UpdatesView, CommandError> {
    let prefs = updates::load(state.data_dir());
    Ok(view(app, &*updates.lock()?, prefs))
}

/// Diffuse la vue à toutes les fenêtres (R8).
fn broadcast<R: Runtime>(app: &AppHandle<R>, view: &UpdatesView) {
    let _ = app.emit(events::UPDATES, view);
}

fn refused(error: &tauri_plugin_updater::Error, version: &str) -> CommandError {
    CommandError::new(
        CommandErrorKind::InvalidRequest,
        updates::install_message(error, version),
    )
}

/// Préférences et dernière version trouvée.
#[tauri::command]
pub fn updates_load<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    updates: State<'_, UpdatesState>,
) -> Result<UpdatesView, CommandError> {
    current_view(&app, &state, &updates)
}

/// Enregistre les préférences (vérification automatique, canal) ; un autre canal oublie la
/// version trouvée et son paquet (R5).
#[tauri::command]
pub fn updates_save<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    updates: State<'_, UpdatesState>,
    automatic: bool,
    channel: updates::Channel,
) -> Result<UpdatesView, CommandError> {
    let mut prefs = updates::load(state.data_dir());
    let mut session = updates.lock()?;
    if prefs.channel != channel {
        session.found = None;
        session.forget_install();
    }
    prefs.automatic = automatic;
    prefs.channel = channel;
    updates::save(state.data_dir(), &prefs)?;
    Ok(view(&app, &session, prefs))
}

/// Cherche une version plus récente. `automatic` : seulement si la vérification est due (au
/// démarrage) ; sinon la vue est renvoyée telle quelle. Une autre version trouvée oublie le
/// paquet de la précédente (R5).
#[tauri::command]
pub async fn updates_check<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    updates: State<'_, UpdatesState>,
    automatic: bool,
) -> Result<UpdatesView, CommandError> {
    let data_dir = state.data_dir().to_path_buf();
    let mut prefs = updates::load(&data_dir);
    if automatic && !updates::due(&prefs, now_ms()) {
        return Ok(view(&app, &*updates.lock()?, prefs));
    }
    let mut request = Prepared {
        method: Method::Get,
        url: updates::RELEASES_API.to_owned(),
        headers: vec![
            (
                "Accept".to_owned(),
                "application/vnd.github+json".to_owned(),
            ),
            (
                "User-Agent".to_owned(),
                format!("Arnod/{}", env!("CARGO_PKG_VERSION")),
            ),
        ],
        body: None,
        timeout_ms: Some(15_000),
        max_redirects: 5,
        drawn: Vec::new(),
        used: Vec::new(),
        oauth2: None,
        hidden: Vec::new(),
    };
    let response = bench
        .network()
        .send(&mut request, None, &Cancel::new())
        .await?;
    let releases = updates::read_releases(response.status, &response.body)?;
    let found = updates::pick(&releases, prefs.channel, env!("CARGO_PKG_VERSION"));
    prefs.last_check = Some(now_ms());
    updates::save(&data_dir, &prefs)?;
    let mut session = updates.lock()?;
    let known = session.found.clone().flatten().map(|r| r.version);
    if known != found.as_ref().map(|r| r.version.clone()) {
        session.forget_install();
    }
    session.found = Some(found);
    let next = view(&app, &session, prefs);
    drop(session);
    broadcast(&app, &next);
    Ok(next)
}

/// Ouvre la page d'une release dans le navigateur du système.
#[tauri::command]
pub fn updates_open(url: String) -> Result<(), CommandError> {
    if !updates::openable(&url) {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Seules les pages des versions d'Arnod s'ouvrent ici.",
        ));
    }
    crate::startup::open_external(&url).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le navigateur n'a pas pu s'ouvrir ({error}). Adresse : {url}"),
        )
    })
}

/// Reprend le manifeste lu par Arnod dans le plugin : `None` si la version n'est pas plus récente
/// que celle installée. Le plugin exige une adresse : celle du manifeste, jamais interrogée.
fn restore<R: Runtime>(
    app: &AppHandle<R>,
    manifest_url: &str,
    manifest: serde_json::Value,
) -> Result<Option<Update>, tauri_plugin_updater::Error> {
    let url = Url::parse(manifest_url)?;
    app.updater_builder()
        .endpoints(vec![url])?
        .build()?
        .restore_update(manifest)
}

/// Lit le manifeste de `release` (R2), télécharge son paquet (R4) et le vérifie (R3).
async fn fetch<R: Runtime>(
    app: &AppHandle<R>,
    network: &Network,
    release: &Release,
    cancel: &Cancel,
    progress: impl FnMut(u64, Option<u64>) + Send,
) -> Result<Pending, CommandError> {
    let version = release.version.as_str();
    let manifest_url = release.manifest.as_deref().ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "Arnod {version} ne s'installe pas depuis Arnod : passez par la page de la version."
            ),
        )
    })?;
    let stall = Duration::from_secs(updates::STALL_SECS);
    let body = network
        .download(
            manifest_url,
            updates::MAX_MANIFEST_BYTES,
            stall,
            cancel,
            |_, _| {},
        )
        .await?;
    let manifest = updates::check_manifest(&body, version)?;
    let update = restore(app, manifest_url, manifest)
        .map_err(|error| refused(&error, version))?
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorKind::InvalidRequest,
                format!("Arnod {version} n'est pas plus récente que la version installée."),
            )
        })?;
    let bytes = network
        .download(
            update.download_url.as_str(),
            updates::MAX_PACKAGE_BYTES,
            stall,
            cancel,
            progress,
        )
        .await?;
    update
        .verify(&bytes)
        .map_err(|error| refused(&error, version))?;
    Ok(Pending { update, bytes })
}

/// Télécharge et vérifie la version trouvée (MJ1) ; la progression est diffusée (R8). Un
/// téléchargement en cours ou un paquet déjà prêt pour cette version renvoient la vue.
#[tauri::command]
pub async fn updates_download<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    updates: State<'_, UpdatesState>,
) -> Result<UpdatesView, CommandError> {
    let prefs = updates::load(state.data_dir());
    let (release, cancel, started) = {
        let mut session = updates.lock()?;
        let current = view(&app, &session, prefs.clone());
        let Some(release) = current.latest.clone() else {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                "Aucune version plus récente n'a été trouvée : vérifiez d'abord.",
            ));
        };
        if !current.installable {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                format!(
                    "Arnod {} ne s'installe pas depuis Arnod : passez par la page de la version.",
                    release.version
                ),
            ));
        }
        match &session.install {
            InstallState::Downloading { .. } => return Ok(current),
            InstallState::Ready { version, .. } if *version == release.version => {
                return Ok(current);
            }
            _ => {}
        }
        let cancel = Arc::new(Cancel::new());
        session.forget_install();
        session.cancel = Some(cancel.clone());
        session.install = InstallState::Downloading {
            received: 0,
            total: None,
        };
        let started = view(&app, &session, prefs.clone());
        (release, cancel, started)
    };
    broadcast(&app, &started);

    let ours = |session: &Session| {
        session
            .cancel
            .as_ref()
            .is_some_and(|c| Arc::ptr_eq(c, &cancel))
    };
    let mut shown = 0u64;
    let progress = |received: u64, total: Option<u64>| {
        if !updates::progress_due(shown, received, total) {
            return;
        }
        shown = received;
        let Ok(mut session) = updates.lock() else {
            return;
        };
        if !ours(&session) {
            return;
        }
        session.install = InstallState::Downloading { received, total };
        let next = view(&app, &session, prefs.clone());
        drop(session);
        broadcast(&app, &next);
    };
    let result = fetch(&app, bench.network(), &release, &cancel, progress).await;

    let mut session = updates.lock()?;
    let outcome = if ours(&session) {
        session.cancel = None;
        match result {
            Ok(pending) => {
                session.install = InstallState::Ready {
                    version: release.version.clone(),
                    on_exit: false,
                };
                session.pending = Some(pending);
                Ok(())
            }
            Err(error) => {
                session.install = if error.kind == CommandErrorKind::Cancelled {
                    InstallState::Idle
                } else {
                    InstallState::Failed {
                        message: error.message.clone(),
                    }
                };
                Err(error)
            }
        }
    } else {
        // Annulé, ou canal changé pendant le téléchargement : l'état est déjà remis à zéro.
        result.map(|_| ())
    };
    let next = view(&app, &session, prefs);
    drop(session);
    broadcast(&app, &next);
    outcome.map(|()| next)
}

/// Arrête le téléchargement en cours et oublie le paquet téléchargé (R5).
#[tauri::command]
pub fn updates_cancel<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    updates: State<'_, UpdatesState>,
) -> Result<UpdatesView, CommandError> {
    updates.lock()?.forget_install();
    let next = current_view(&app, &state, &updates)?;
    broadcast(&app, &next);
    Ok(next)
}

/// Installe le paquet vérifié. `now` (MJ2) : installe et relance Arnod, sans revenir (Windows :
/// l'installeur NSIS passif ferme Arnod puis le relance ; Linux : l'AppImage est remplacée puis
/// Arnod redémarre). Sinon (MJ3) : installation à la fermeture d'Arnod (R9). L'interface
/// enregistre ses brouillons avant l'appel (R6).
#[tauri::command]
pub async fn updates_install<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    updates: State<'_, UpdatesState>,
    now: bool,
) -> Result<UpdatesView, CommandError> {
    let prefs = updates::load(state.data_dir());
    let mut session = updates.lock()?;
    let version = match &session.install {
        InstallState::Ready { version, .. } if session.pending.is_some() => version.clone(),
        _ => {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                "Aucune version téléchargée à installer : lancez d'abord l'installation.",
            ));
        }
    };
    if !now {
        session.install = InstallState::Ready {
            version,
            on_exit: true,
        };
        let next = view(&app, &session, prefs);
        drop(session);
        broadcast(&app, &next);
        return Ok(next);
    }
    let Some(Pending { update, bytes }) = session.pending.take() else {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Aucune version téléchargée à installer.",
        ));
    };
    drop(session);
    // Positions des fenêtres détachées, pour la restauration (F13).
    app.state::<WindowsState>().flush();
    // Windows : lance l'installeur et quitte, sans revenir en cas de succès.
    match update.install(&bytes) {
        Ok(()) => app.restart(),
        Err(error) => {
            let message = updates::install_message(&error, &version);
            let mut session = updates.lock()?;
            session.install = InstallState::Failed {
                message: message.clone(),
            };
            let next = view(&app, &session, prefs);
            drop(session);
            broadcast(&app, &next);
            Err(CommandError::new(CommandErrorKind::Io, message))
        }
    }
}

/// Sortie d'Arnod (R9, `RunEvent::Exit`) : installe le paquet mis de côté pour « Au prochain
/// lancement », sans relancer Arnod. Sous Windows, l'installeur prend la suite et Arnod se ferme.
pub fn install_on_exit<R: Runtime>(app: &AppHandle<R>) {
    let Some(updates) = app.try_state::<UpdatesState>() else {
        return;
    };
    let pending = {
        let Ok(mut session) = updates.0.lock() else {
            return;
        };
        if !matches!(session.install, InstallState::Ready { on_exit: true, .. }) {
            return;
        }
        session.pending.take()
    };
    if let Some(Pending { update, bytes }) = pending {
        let _ = update.restart_after_install(false).install(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

    /// Paire de clés d'essai, générée pour ces tests seulement (`tauri signer generate`).
    const TEST_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEYzRDUzRkUxMkY3NDhGQzMKUldURGozUXY0VC9WOHhKMDZ0cEY1b2JibGwrVGdWTDVYSlJlbTg2YkFCc09iMGZXRUVsZC9MZkkK";
    const OTHER_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhDMUJENEY2MDU0MUIwRjYKUldUMnNFRUY5dFFiaktRS1BIYk93YWtxSzh5ZXV0QVRORXQ4UVdkWm05T1ZneTZZNGZieEdYUFUK";
    /// `PACKAGE` signé par la clé d'essai pour la version 0.2.0 (`tauri signer sign --app-version`).
    const PACKAGE: &[u8] = b"Paquet d essai Arnod 0.2.0\n";
    const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVURGozUXY0VC9WODdyeEVHblNYbGE3UEtSQklSMFN3c0t1a05NUkVDdnRwakhVdzlMLzgzQzZrYmY2ekY0eTYwU0UyTFhRWmJWWEY1ZFJNK1Y5K1NManR1aytXcDBndEE0PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0ODU5MzkzCWZpbGU6cGFxdWV0LmJpbgl2ZXJzaW9uOjAuMi4wCmtZMGZtTXRXVDZjeWRsTlZRcW9RSTdYSkg1R0JsSk9DdTg0eWpJMk96M2JvNkk0WHpTenlwbUsyL1RLS1hnM1BSL1lJRHpRajVvSVpoNWdTVjNFdkNBPT0K";

    /// Application simulée (version 0.1.0) avec le plugin et la clé `pubkey`. Les adresses
    /// `http://` des serveurs de test sont admises par le plugin.
    fn app(pubkey: &str) -> tauri::App<MockRuntime> {
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
        let app = app(TEST_PUBKEY);
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

        let other = app_with_other_key();
        let error = restored(&other, "0.2.0").verify(PACKAGE).unwrap_err();
        assert!(updates::install_message(&error, "0.2.0").contains("rien n'a été installé"));
    }

    fn app_with_other_key() -> tauri::App<MockRuntime> {
        app(OTHER_PUBKEY)
    }

    #[test]
    fn r3_signature_d_une_autre_version_refusee() {
        let app = app(TEST_PUBKEY);
        // Le manifeste annonce 0.3.0, le paquet est signé pour 0.2.0.
        let error = restored(&app, "0.3.0").verify(PACKAGE).unwrap_err();
        assert!(
            updates::install_message(&error, "0.3.0").contains("signé pour la version 0.2.0"),
            "{error}"
        );
    }

    #[test]
    fn r2_version_pas_plus_recente_ignoree() {
        let app = app(TEST_PUBKEY);
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
        assert_eq!(pubkey(app(TEST_PUBKEY).handle()), TEST_PUBKEY);
        assert_eq!(pubkey(app("").handle()), "");
    }

    #[tokio::test]
    async fn mj1_manifeste_et_paquet_telecharges_puis_verifies() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

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
            Arc::new(crate::secrets::MemorySecretStore::default()),
            Box::new(crate::network::proxy::FixedProxySource::default()),
        );
        let app = app(TEST_PUBKEY);
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
        let other = app_with_other_key();
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

    #[test]
    fn r5_annuler_oublie_le_paquet() {
        let mut session = Session::default();
        let cancel = Arc::new(Cancel::new());
        session.cancel = Some(cancel.clone());
        session.install = InstallState::Downloading {
            received: 1,
            total: None,
        };
        session.forget_install();
        assert!(cancel.is_cancelled());
        assert_eq!(session.install, InstallState::Idle);
        assert!(session.cancel.is_none() && session.pending.is_none());
    }
}
