//! Synchronisation d'un projet avec son équipe (V2, lots E et F, 7.1-a à 7.1-e) : façades des
//! règles de [`crate::sync`]. Les opérations Git tournent sur un fil à part (libgit2 est
//! synchrone, ses échanges passent par le runtime du cœur) ; chaque changement est diffusé à
//! toutes les fenêtres par `arnod://synchro`, et une récupération qui écrit des fichiers par les
//! événements des fichiers modifiés (les onglets ouverts se rechargent, SY7).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::secrets::{SecretStore, SystemSecretStore};
use crate::state::AppState;
use crate::sync::forge::{self, DeviceCode, Forge, Repository};
use crate::sync::merge::Choice;
use crate::sync::state::{self as sync_state, Account};
use crate::sync::{self, JournalEntry, Pulled, SyncBook, SyncConflicts, SyncCtx, SyncView};
use crate::watch::FilesChange;
use crate::watch::service::Sink;
use crate::workbench::{ProjectCtx, Workbench};

/// Événement de l'état de la synchronisation d'un projet (`SyncView`).
pub const SYNC_EVENT: &str = "arnod://synchro";

/// Connexions par le navigateur en cours : code de l'appareil, par forge et hôte.
#[derive(Default)]
pub struct SyncLogins(Mutex<HashMap<String, (DeviceCode, bool)>>);

/// Secrets à renseigner après une récupération (7.1-e, SY8, R15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MissingSecrets {
    pub environment: String,
    pub names: Vec<String>,
}

/// Ce que la connexion propose pour chaque forge (Q2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncLoginOptions {
    pub github_browser: bool,
    pub gitlab_browser: bool,
}

fn ctx_of(state: &AppState, project: Uuid) -> Result<(ProjectCtx, String), CommandError> {
    let ctx = state.project_ctx(project)?;
    let name = crate::opencollection::collection::read(&ctx.root)
        .ok()
        .and_then(|root| crate::opencollection::collection::name(&root))
        .unwrap_or_else(|| {
            ctx.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    Ok((ctx, name))
}

/// Lance `work` sur un fil sans runtime tokio et attend son issue.
async fn detached<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || std::thread::spawn(work).join())
        .await
        .map_err(|_| CommandError::new(CommandErrorKind::Io, "Opération interrompue."))?
        .map_err(|_| CommandError::new(CommandErrorKind::Io, "Opération interrompue."))
}

/// Exécute une opération de synchronisation sur le projet.
async fn with_sync<T: Send + 'static>(
    app: &AppHandle,
    project: Uuid,
    work: impl FnOnce(&SyncCtx<'_>, &SyncBook) -> T + Send + 'static,
) -> Result<T, CommandError> {
    let state = app.state::<AppState>();
    let (ctx, name) = ctx_of(&state, project)?;
    let network = app.state::<Arc<Workbench>>().network().clone();
    let app = app.clone();
    detached(move || {
        let book = app.state::<Arc<SyncBook>>();
        let sync_ctx = SyncCtx {
            project,
            name: &name,
            root: &ctx.root,
            data_dir: &ctx.data_dir,
            network,
            secrets: Arc::new(SystemSecretStore),
        };
        work(&sync_ctx, &book)
    })
    .await
}

fn broadcast(app: &AppHandle, view: &SyncView) {
    let _ = app.emit(SYNC_EVENT, view);
}

/// Fichiers écrits par une récupération : diffusés comme des fichiers modifiés hors d'Arnod,
/// pour que les fenêtres rechargent arborescence, environnements, contrats et onglets (SY7).
fn files_changed(app: &AppHandle, project: Uuid, pulled: &Pulled) {
    if pulled.written.is_empty() && pulled.removed.is_empty() {
        return;
    }
    let all = pulled.written.iter().chain(&pulled.removed);
    let change = FilesChange {
        project_id: project,
        added: Vec::new(),
        modified: pulled
            .written
            .iter()
            .filter(|p| sync::merge::is_request(p))
            .cloned()
            .collect(),
        removed: pulled.removed.clone(),
        renamed: Vec::new(),
        tree: true,
        environments: all
            .clone()
            .any(|p| p.starts_with("environments/") || p == "opencollection.yml"),
        contracts: all.clone().any(|p| p.starts_with("contracts/")),
    };
    crate::watch::app::AppSink::new(app.clone()).changed(change);
}

#[tauri::command]
pub async fn sync_status(app: AppHandle, project: Uuid) -> Result<SyncView, CommandError> {
    with_sync(&app, project, sync::view).await
}

/// Vérifie ce qui arrive (au démarrage, toutes les 5 minutes, au retour sur la fenêtre, Q6).
#[tauri::command]
pub async fn sync_check(app: AppHandle, project: Uuid) -> Result<SyncView, CommandError> {
    let view = with_sync(&app, project, sync::check).await?;
    broadcast(&app, &view);
    Ok(view)
}

#[tauri::command]
pub fn sync_login_options() -> SyncLoginOptions {
    SyncLoginOptions {
        github_browser: Forge::Github.browser_login(),
        gitlab_browser: Forge::Gitlab.browser_login(),
    }
}

fn login_key(forge: Forge, host: &str) -> String {
    format!("{forge:?}:{}", host.to_ascii_lowercase())
}

/// Connexion par le navigateur (SY1) : ouvre la page de la forge et rend le code à saisir.
#[tauri::command]
pub async fn sync_connect_start(
    app: AppHandle,
    logins: State<'_, SyncLogins>,
    forge: Forge,
    host: String,
) -> Result<DeviceCode, CommandError> {
    let network = app.state::<Arc<Workbench>>().network().clone();
    let code = forge::device_start(&network, forge, &host).await?;
    if let Ok(mut all) = logins.0.lock() {
        all.insert(login_key(forge, &host), (code.clone(), false));
    }
    let _ = crate::startup::open_external(&code.verification_uri);
    Ok(code)
}

/// Attend la validation dans le navigateur, range le jeton au coffre et rend le compte.
#[tauri::command]
pub async fn sync_connect_wait(
    app: AppHandle,
    logins: State<'_, SyncLogins>,
    forge: Forge,
    host: String,
) -> Result<Account, CommandError> {
    let key = login_key(forge, &host);
    let (code, _) = logins
        .0
        .lock()
        .ok()
        .and_then(|all| all.get(&key).cloned())
        .ok_or_else(|| {
            CommandError::new(CommandErrorKind::NotFound, "Aucune connexion en cours.")
        })?;
    let network = app.state::<Arc<Workbench>>().network().clone();
    let deadline = std::time::Instant::now() + Duration::from_secs(code.expires_in_secs);
    let mut interval = code.interval_secs.max(1);
    loop {
        let cancelled = logins
            .0
            .lock()
            .ok()
            .and_then(|all| all.get(&key).map(|(_, cancelled)| *cancelled))
            .unwrap_or(true);
        if cancelled {
            return Err(CommandError::new(
                CommandErrorKind::Cancelled,
                "Connexion annulée.",
            ));
        }
        if std::time::Instant::now() > deadline {
            return Err(CommandError::new(
                CommandErrorKind::Timeout,
                "Le code de connexion a expiré : recommencez.",
            ));
        }
        tokio_sleep(Duration::from_secs(interval)).await;
        match forge::device_poll(&network, forge, &host, &code.device_code).await {
            Ok(Some(token)) => {
                if let Ok(mut all) = logins.0.lock() {
                    all.remove(&key);
                }
                return store_account(&network, forge, &host, &token, None).await;
            }
            Ok(None) => {}
            Err(error) if error.kind == CommandErrorKind::Network => interval += 1,
            Err(error) => return Err(error),
        }
    }
}

async fn tokio_sleep(duration: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
}

#[tauri::command]
pub fn sync_connect_cancel(logins: State<'_, SyncLogins>, forge: Forge, host: String) {
    if let Ok(mut all) = logins.0.lock()
        && let Some(entry) = all.get_mut(&login_key(forge, &host))
    {
        entry.1 = true;
    }
}

async fn store_account(
    network: &crate::network::Network,
    forge: Forge,
    host: &str,
    token: &str,
    login: Option<&str>,
) -> Result<Account, CommandError> {
    let account = forge::account(network, forge, host, token, login).await?;
    SystemSecretStore.set(&sync_state::token_key(host, &account.login), token)?;
    Ok(account)
}

/// Jeton personnel saisi (SY1, autre URL Git, ou repli sans connexion par le navigateur).
#[tauri::command]
pub async fn sync_token(
    app: AppHandle,
    forge: Forge,
    host: String,
    token: String,
    login: Option<String>,
) -> Result<Account, CommandError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Collez le jeton du compte.",
        ));
    }
    let network = app.state::<Arc<Workbench>>().network().clone();
    store_account(&network, forge, host.trim(), token, login.as_deref()).await
}

fn token_of(account: &Account) -> Result<String, CommandError> {
    SystemSecretStore
        .get(&sync_state::token_key(&account.host, &account.login))?
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorKind::NotFound,
                "Le jeton de ce compte n'est plus dans le coffre : reconnectez-le.",
            )
        })
}

#[tauri::command]
pub async fn sync_repositories(
    app: AppHandle,
    account: Account,
    query: String,
) -> Result<Vec<Repository>, CommandError> {
    let network = app.state::<Arc<Workbench>>().network().clone();
    forge::repositories(&network, &account, &token_of(&account)?, &query).await
}

#[tauri::command]
pub async fn sync_create_repository(
    app: AppHandle,
    account: Account,
    name: String,
) -> Result<Repository, CommandError> {
    let network = app.state::<Arc<Workbench>>().network().clone();
    forge::create_repository(&network, &account, &token_of(&account)?, name.trim()).await
}

/// « Relier et envoyer » (7.1-a).
#[tauri::command]
pub async fn sync_link(
    app: AppHandle,
    project: Uuid,
    url: String,
    branch: String,
    account: Option<Account>,
) -> Result<SyncView, CommandError> {
    let view = with_sync(&app, project, move |ctx, book| {
        sync::link(ctx, book, &url, &branch, account)
    })
    .await??;
    broadcast(&app, &view);
    Ok(view)
}

/// Récupérer, récupérer puis envoyer, ou envoyer seulement (7.1-b, SY6) : `items` choisis parmi
/// « Mes modifications », avec `message`.
#[tauri::command]
pub async fn sync_pull_push(
    app: AppHandle,
    project: Uuid,
    items: Vec<String>,
    message: String,
    pull: bool,
    push: bool,
) -> Result<SyncView, CommandError> {
    let outcome = with_sync(&app, project, move |ctx, book| {
        if push {
            sync::send(ctx, book, &items, &message, pull)
        } else {
            sync::pull(ctx, book).map(|pulled| (sync::view(ctx, book), pulled))
        }
    })
    .await?;
    let (view, pulled) = match outcome {
        Ok(done) => done,
        Err(error) => {
            // Un conflit ou un refus : la barre d'état suit quand même.
            if let Ok(view) = with_sync(&app, project, sync::view).await {
                broadcast(&app, &view);
            }
            return Err(error);
        }
    };
    files_changed(&app, project, &pulled);
    broadcast(&app, &view);
    Ok(view)
}

#[tauri::command]
pub async fn sync_conflicts(app: AppHandle, project: Uuid) -> Result<SyncConflicts, CommandError> {
    with_sync(&app, project, sync::conflicts).await
}

/// « Valider la résolution » (7.1-c, CF3) : chemin → élément → version gardée.
#[tauri::command]
pub async fn sync_resolve(
    app: AppHandle,
    project: Uuid,
    choices: BTreeMap<String, BTreeMap<String, Choice>>,
) -> Result<SyncView, CommandError> {
    let (pulled, view) = with_sync(&app, project, move |ctx, book| {
        sync::resolve(ctx, book, &choices).map(|pulled| (pulled, sync::view(ctx, book)))
    })
    .await??;
    files_changed(&app, project, &pulled);
    broadcast(&app, &view);
    Ok(view)
}

#[tauri::command]
pub async fn sync_abandon(app: AppHandle, project: Uuid) -> Result<SyncView, CommandError> {
    let view = with_sync(&app, project, |ctx, book| {
        sync::abandon(ctx, book);
        sync::view(ctx, book)
    })
    .await?;
    broadcast(&app, &view);
    Ok(view)
}

/// Historique d'un test, d'une requête ou d'un dossier, en clair (7.1-d, CF5).
#[tauri::command]
pub async fn sync_journal(
    app: AppHandle,
    project: Uuid,
    path: String,
) -> Result<Vec<JournalEntry>, CommandError> {
    with_sync(&app, project, move |ctx, _| sync::journal(ctx, &path)).await
}

/// Variables locales déclarées sans valeur sur ce poste, par environnement (7.1-e, R15) ; un
/// environnement remis « à plus tard » n'est plus signalé tant que rien de neuf ne manque.
#[tauri::command]
pub fn sync_missing_secrets(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
) -> Result<Vec<MissingSecrets>, CommandError> {
    let ctx = state.project_ctx(project)?;
    let later = sync_state::load(&ctx.data_dir, project).secrets_later;
    let mut out = Vec::new();
    for environment in crate::environment::list(&ctx.root) {
        let Ok((variables, _)) = bench.variables(&ctx, &environment.name) else {
            continue;
        };
        let names: Vec<String> = environment
            .locals
            .iter()
            .filter(|name| !variables.values.contains_key(*name))
            .cloned()
            .collect();
        let postponed = later
            .get(&environment.name)
            .is_some_and(|known| names.iter().all(|n| known.contains(n)));
        if !names.is_empty() && !postponed {
            out.push(MissingSecrets {
                environment: environment.name,
                names,
            });
        }
    }
    Ok(out)
}

/// « Plus tard » (7.1-e) : l'environnement n'est plus signalé pour ces noms.
#[tauri::command]
pub fn sync_secrets_later(
    state: State<'_, AppState>,
    project: Uuid,
    environment: String,
    names: Vec<String>,
) -> Result<(), CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut sync = sync_state::load(&ctx.data_dir, project);
    sync.secrets_later.insert(environment, names);
    sync_state::save(&ctx.data_dir, project, &sync)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forme_json_des_secrets_manquants_et_des_options() {
        let missing = MissingSecrets {
            environment: "Recette".into(),
            names: vec!["token".into()],
        };
        assert_eq!(
            serde_json::to_value(missing).unwrap(),
            serde_json::json!({ "environment": "Recette", "names": ["token"] })
        );
        assert_eq!(
            serde_json::to_value(sync_login_options()).unwrap()["githubBrowser"],
            serde_json::json!(Forge::Github.browser_login())
        );
    }
}
