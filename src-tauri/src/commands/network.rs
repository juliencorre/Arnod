//! Lot H - Réseau et authentification (`docs/plan-mvp-h-reseau-auth.md`) : préférences réseau du
//! poste (1.6-e), autorités internes (5.1-i), authentification du projet et jetons OAuth 2
//! (3.4-a), cookies gardés par environnement (3.4-g).

use std::sync::Arc;

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::http::Cancel;
use crate::input::added::fill;
use crate::network::cert::{self, CertificateInfo};
use crate::network::cookies::CookieJarView;
use crate::network::oauth::TokenStatus;
use crate::network::project_auth::{self, AuthUsage};
use crate::network::{NetworkPrefsInput, NetworkView, Session};
use crate::request::model::{Auth, AuthKind, Request};
use crate::request::resolve;
use crate::state::AppState;
use crate::windows::{self, ProjectScope};
use crate::workbench::{ProjectCtx, Workbench};

/// Nom de la variable locale secrète qui garde le secret client OAuth 2 (R10).
pub const CLIENT_SECRET_VARIABLE: &str = "oauth2ClientSecret";

#[tauri::command]
pub fn network_load(bench: State<'_, Arc<Workbench>>) -> NetworkView {
    bench.network().view()
}

#[tauri::command]
pub fn network_save(
    bench: State<'_, Arc<Workbench>>,
    prefs: NetworkPrefsInput,
) -> Result<NetworkView, CommandError> {
    bench.network().save(prefs)
}

/// Choisit un fichier PEM et le lit, sans l'ajouter : l'interface montre sujet, validité et
/// empreinte avant « Faire confiance » (R4). `None` : choix annulé.
#[tauri::command]
pub async fn network_pick_authority(
    window: WebviewWindow,
) -> Result<Option<CertificateInfo>, CommandError> {
    let picked = window
        .dialog()
        .file()
        .set_title("Ajouter une autorité de certification")
        .add_filter("Certificat", &["pem", "crt", "cer"])
        .set_parent(&window)
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le fichier choisi n'est pas accessible : {error}."),
        )
    })?;
    let read = path.clone();
    let text = tauri::async_runtime::spawn_blocking(move || std::fs::read(&read))
        .await
        .map_err(|error| CommandError::new(CommandErrorKind::Io, error.to_string()))?
        .map_err(|error| CommandError::io("Lecture impossible", &path, &error))?;
    let text = String::from_utf8_lossy(&text);
    let info = if text.contains("BEGIN CERTIFICATE") {
        cert::parse_pem(&text)?.1
    } else {
        // Certificat DER (.cer de Windows).
        let der =
            std::fs::read(&path).map_err(|e| CommandError::io("Lecture impossible", &path, &e))?;
        cert::parse_der(&der)?
    };
    Ok(Some(info))
}

#[tauri::command]
pub fn network_add_authority(
    bench: State<'_, Arc<Workbench>>,
    pem: String,
) -> Result<NetworkView, CommandError> {
    bench.network().add_authority(&pem)
}

#[tauri::command]
pub fn network_remove_authority(
    bench: State<'_, Arc<Workbench>>,
    fingerprint: String,
) -> Result<NetworkView, CommandError> {
    bench.network().remove_authority(&fingerprint)
}

/// Authentification du projet vue par les réglages (3.4-a).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAuthView {
    pub auth: Auth,
    pub environment: String,
    /// Jeton de l'environnement, sans sa valeur.
    pub token: Option<TokenStatus>,
    /// Un secret client est au coffre pour cet environnement.
    pub has_client_secret: bool,
    pub usage: AuthUsage,
}

fn has_secret(bench: &Workbench, ctx: &ProjectCtx, env: &str) -> bool {
    bench.variables(ctx, env).is_ok_and(|(v, _)| {
        v.values
            .get(CLIENT_SECRET_VARIABLE)
            .is_some_and(|s| !s.is_empty())
    })
}

fn auth_view(
    bench: &Workbench,
    ctx: &ProjectCtx,
    env: &str,
) -> Result<ProjectAuthView, CommandError> {
    Ok(ProjectAuthView {
        auth: project_auth::load(&ctx.root)?,
        environment: env.to_owned(),
        token: bench.network().token_status(ctx.id, env),
        has_client_secret: has_secret(bench, ctx, env),
        usage: project_auth::usage(&ctx.root),
    })
}

#[tauri::command]
pub async fn project_auth_load(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
) -> Result<ProjectAuthView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let bench = bench.inner().clone();
    tauri::async_runtime::spawn_blocking(move || auth_view(&bench, &ctx, &environment))
        .await
        .map_err(|error| CommandError::new(CommandErrorKind::Io, error.to_string()))?
}

/// Range le secret client au coffre, variable locale secrète de l'environnement (R10).
fn store_client_secret(
    bench: &Workbench,
    ctx: &ProjectCtx,
    env: &str,
    value: &str,
) -> Result<(), CommandError> {
    let mut found = environment::list(&ctx.root)
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(env))
        .ok_or_else(|| {
            CommandError::new(
                CommandErrorKind::NotFound,
                format!("L'environnement « {env} » n'existe pas."),
            )
        })?;
    if !found.locals.iter().any(|n| n == CLIENT_SECRET_VARIABLE) {
        found.locals.push(CLIENT_SECRET_VARIABLE.to_owned());
        let name = found.name.clone();
        bench.save_environment(ctx, &found, Some(&name))?;
    }
    bench.set_local(ctx, &found.name, CLIENT_SECRET_VARIABLE, value, true)
}

/// Enregistre l'authentification du projet ; un secret client saisi va au coffre de
/// l'environnement et le fichier n'en garde que la référence (R10).
#[tauri::command]
pub fn project_auth_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    auth: Auth,
    client_secret: Option<String>,
) -> Result<ProjectAuthView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut auth = auth;
    if auth.kind == AuthKind::OAuth2 {
        if let Some(secret) = client_secret.as_deref().filter(|s| !s.is_empty()) {
            store_client_secret(&bench, &ctx, &environment, secret)?;
            auth.oauth2.client_secret = format!("{{{{{CLIENT_SECRET_VARIABLE}}}}}");
            windows::project_changed(
                window.app_handle(),
                window.label(),
                project,
                ProjectScope::Environments,
                None,
            );
        }
        // Une valeur en clair n'est jamais écrite dans le projet.
        if !auth.oauth2.client_secret.trim().is_empty() && !auth.oauth2.client_secret.contains("{{")
        {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                "Le secret client ne s'écrit pas dans le projet : saisissez-le dans le champ prévu, il ira au coffre du poste.",
            ));
        }
    }
    project_auth::save(&ctx.root, &auth)?;
    bench.network().forget_tokens(ctx.id, &environment);
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Tree,
        None,
    );
    auth_view(&bench, &ctx, &environment)
}

/// « Renouveler maintenant » : jeton redemandé pour l'environnement (OA3).
#[tauri::command]
pub async fn oauth2_token_renew(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
) -> Result<ProjectAuthView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let auth = project_auth::load(&ctx.root)?;
    if auth.kind != AuthKind::OAuth2 {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Le projet n'a pas d'authentification OAuth 2.",
        ));
    }
    let (variables, env) = bench.variables(&ctx, &environment)?;
    let mut request = Request::new("jeton");
    request.url = "http://jeton.invalid/".to_owned();
    request.auth = auth;
    let prepared = resolve::prepare(&request, &variables)?;
    let Some(grant) = prepared.oauth2 else {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Le projet n'a pas d'authentification OAuth 2.",
        ));
    };
    let session = Session {
        project,
        environment: env.name.clone(),
    };
    bench
        .network()
        .token(Some(&session), &grant, true, &Cancel::new())
        .await
        .map_err(|mut e| {
            e.message = variables.mask(&e.message);
            e
        })?;
    auth_view(&bench, &ctx, &env.name)
}

#[tauri::command]
pub fn oauth2_token_forget(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
) -> Result<ProjectAuthView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.network().forget_tokens(project, &environment);
    auth_view(&bench, &ctx, &environment)
}

/// Hôte de l'adresse `url` (variables non secrètes de l'environnement remplacées).
fn host_of(bench: &Workbench, ctx: &ProjectCtx, env: &str, url: Option<&str>) -> Option<String> {
    let url = url?;
    let mut values: std::collections::BTreeMap<String, String> = environment::list(&ctx.root)
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(env))
        .map(|e| e.variables.into_iter().map(|v| (v.name, v.value)).collect())
        .unwrap_or_default();
    values.extend(bench.runtime_variables(ctx.id));
    reqwest::Url::parse(fill(url, &values).trim())
        .ok()?
        .host_str()
        .map(str::to_ascii_lowercase)
}

fn session(project: Uuid, environment: &str) -> Session {
    Session {
        project,
        environment: environment.to_owned(),
    }
}

/// Cookies gardés de l'environnement ; avec `url`, ceux qui partiraient vers son hôte (CK2).
#[tauri::command]
pub fn cookies_list(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    url: Option<String>,
) -> Result<CookieJarView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let host = host_of(&bench, &ctx, &environment, url.as_deref());
    Ok(bench
        .network()
        .cookies(&session(project, &environment), host.as_deref()))
}

/// « Vider » : oublie les cookies d'un domaine, ou tous (CK3).
#[tauri::command]
pub fn cookies_clear(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    domain: Option<String>,
    url: Option<String>,
) -> Result<CookieJarView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let s = session(project, &environment);
    bench.network().clear_cookies(&s, domain.as_deref());
    let host = host_of(&bench, &ctx, &environment, url.as_deref());
    Ok(bench.network().cookies(&s, host.as_deref()))
}

/// « Renvoyer les cookies reçus de ce domaine » (CK4).
#[tauri::command]
pub fn cookies_set_sending(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    environment: String,
    domain: String,
    enabled: bool,
    url: Option<String>,
) -> Result<CookieJarView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let s = session(project, &environment);
    bench.network().set_cookie_sending(&s, &domain, enabled);
    let host = host_of(&bench, &ctx, &environment, url.as_deref());
    Ok(bench.network().cookies(&s, host.as_deref()))
}
