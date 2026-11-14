//! Domaine « mises à jour » (lot L allégé, écran 1.6-g) : préférences du poste, vérification
//! des versions publiées et ouverture de la page d'une release. Rien n'est installé.

use std::sync::{Arc, Mutex, OnceLock};

use tauri::State;

use crate::error::{CommandError, CommandErrorKind};
use crate::http::Cancel;
use crate::network::now_ms;
use crate::request::model::Method;
use crate::request::resolve::Prepared;
use crate::state::AppState;
use crate::updates::{self, Release, UpdatePrefs, UpdatesView};
use crate::workbench::Workbench;

/// Résultat de la dernière vérification de la session : `None` tant qu'il n'y en a pas eu.
fn last() -> &'static Mutex<Option<Option<Release>>> {
    static LAST: OnceLock<Mutex<Option<Option<Release>>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

fn view(prefs: UpdatePrefs) -> UpdatesView {
    let found = last().lock().ok().and_then(|l| l.clone());
    UpdatesView {
        current: env!("CARGO_PKG_VERSION").to_owned(),
        prefs,
        checked: found.is_some(),
        latest: found.flatten(),
    }
}

/// Préférences et dernière version trouvée.
#[tauri::command]
pub fn updates_load(state: State<'_, AppState>) -> UpdatesView {
    view(updates::load(state.data_dir()))
}

/// Enregistre les préférences (vérification automatique, canal) ; un autre canal oublie la
/// version trouvée.
#[tauri::command]
pub fn updates_save(
    state: State<'_, AppState>,
    automatic: bool,
    channel: updates::Channel,
) -> Result<UpdatesView, CommandError> {
    let mut prefs = updates::load(state.data_dir());
    if prefs.channel != channel
        && let Ok(mut found) = last().lock()
    {
        *found = None;
    }
    prefs.automatic = automatic;
    prefs.channel = channel;
    updates::save(state.data_dir(), &prefs)?;
    Ok(view(prefs))
}

/// Cherche une version plus récente. `automatic` : seulement si la vérification est due (au
/// démarrage) ; sinon la vue est renvoyée telle quelle.
#[tauri::command]
pub async fn updates_check(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    automatic: bool,
) -> Result<UpdatesView, CommandError> {
    let data_dir = state.data_dir().to_path_buf();
    let mut prefs = updates::load(&data_dir);
    if automatic && !updates::due(&prefs, now_ms()) {
        return Ok(view(prefs));
    }
    let request = Prepared {
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
    let mut request = request;
    let response = bench
        .network()
        .send(&mut request, None, &Cancel::new())
        .await?;
    let releases = updates::read_releases(response.status, &response.body)?;
    let found = updates::pick(&releases, prefs.channel, env!("CARGO_PKG_VERSION"));
    if let Ok(mut last) = last().lock() {
        *last = Some(found);
    }
    prefs.last_check = Some(now_ms());
    updates::save(&data_dir, &prefs)?;
    Ok(view(prefs))
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
