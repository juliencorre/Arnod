//! Réseau du poste (`docs/plan-mvp-h-reseau-auth.md`, lot H) : préférences (proxy, autorités,
//! délai de connexion), client HTTP reconstruit quand elles changent, échecs expliqués, jetons
//! OAuth 2 et cookies gardés par environnement.
//!
//! [`Network`] est tenu par l'établi (`Workbench::network`) et confié à Tauri pour les commandes
//! des préférences, des jetons et des cookies.

pub mod cert;
pub mod cookies;
pub mod failure;
pub mod oauth;
pub mod prefs;
pub mod project_auth;
pub mod proxy;
pub mod tls;

#[cfg(test)]
mod testcert;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::Url;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::http::{self, Cancel, Failed, HttpResponse, USER_AGENT};
use crate::request::model::{CredentialsPlacement, Method};
use crate::request::resolve::{OAuth2Grant, Prepared};
use crate::secrets::SecretStore;

use self::cert::CertificateInfo;
use self::cookies::{CookieJarView, ProjectJars};
use self::failure::Attempt;
use self::oauth::{Token, TokenCache, TokenKey, TokenStatus};
use self::prefs::{NetworkPrefs, PROXY_SECRET_KEY, ProxyPrefs, StoredAuthority};
use self::proxy::{EffectiveProxy, ProxyMode, SystemProxy, SystemProxySource};
use self::tls::Rejections;

/// Bornes du délai de connexion réglable (1 s à 5 min).
const CONNECT_TIMEOUT_RANGE: (u64, u64) = (1_000, 300_000);

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}

/// Où part un envoi : projet et environnement, pour les jetons et les cookies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub project: Uuid,
    pub environment: String,
}

/// Autorité interne, telle que montrée dans les préférences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityView {
    #[serde(flatten)]
    pub info: CertificateInfo,
    pub added_at: u64,
}

/// Préférences réseau vues par l'interface (1.6-e).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkView {
    pub proxy: ProxyPrefs,
    /// Un mot de passe du proxy est au coffre (jamais sa valeur).
    pub has_proxy_password: bool,
    /// Proxy du système détecté.
    pub system: SystemProxy,
    pub connect_timeout_ms: u64,
    /// Nombre d'autorités du système reconnues.
    pub system_authorities: usize,
    /// Autorités internes ajoutées.
    pub authorities: Vec<AuthorityView>,
}

/// Préférences modifiées par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPrefsInput {
    pub proxy: ProxyPrefs,
    /// `None` : inchangé ; vide : retiré du coffre.
    #[serde(default)]
    pub proxy_password: Option<String>,
    pub connect_timeout_ms: u64,
}

/// Client construit pour des préférences données.
struct Built {
    tls: rustls::ClientConfig,
    proxy: EffectiveProxy,
    connect_timeout_ms: u64,
    clients: HashMap<u32, reqwest::Client>,
}

/// Réseau du poste.
pub struct Network {
    data_dir: OnceLock<PathBuf>,
    secrets: Arc<dyn SecretStore>,
    system: Box<dyn SystemProxySource>,
    prefs: RwLock<NetworkPrefs>,
    built: Mutex<Option<Built>>,
    rejections: Rejections,
    tokens: Mutex<TokenCache>,
    jars: Mutex<HashMap<Uuid, ProjectJars>>,
}

fn busy() -> CommandError {
    CommandError::new(
        CommandErrorKind::Io,
        "L'état du réseau est indisponible : redémarrez l'application.",
    )
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

impl Network {
    pub fn new(secrets: Arc<dyn SecretStore>, system: Box<dyn SystemProxySource>) -> Self {
        Self {
            data_dir: OnceLock::new(),
            secrets,
            system,
            prefs: RwLock::new(NetworkPrefs::default()),
            built: Mutex::new(None),
            rejections: Arc::default(),
            tokens: Mutex::new(TokenCache::default()),
            jars: Mutex::new(HashMap::new()),
        }
    }

    /// Branche le dossier de données : préférences lues, cookies persistants écrits (R1).
    pub fn attach(&self, data_dir: &Path) {
        let _ = self.data_dir.set(data_dir.to_path_buf());
        if let Ok(mut current) = self.prefs.write() {
            *current = prefs::load(data_dir);
        }
        self.invalidate();
    }

    fn invalidate(&self) {
        if let Ok(mut built) = self.built.lock() {
            *built = None;
        }
    }

    pub fn prefs(&self) -> NetworkPrefs {
        self.prefs.read().map(|p| p.clone()).unwrap_or_default()
    }

    /// Modifie les préférences, les écrit et reconstruit le client (R1).
    fn update(&self, edit: impl FnOnce(&mut NetworkPrefs)) -> Result<(), CommandError> {
        let mut next = self.prefs();
        edit(&mut next);
        if let Some(dir) = self.data_dir.get() {
            prefs::save(dir, &next)?;
        }
        *self.prefs.write().map_err(|_| busy())? = next;
        self.invalidate();
        Ok(())
    }

    fn proxy_password(&self) -> Option<String> {
        self.secrets.get(PROXY_SECRET_KEY).ok().flatten()
    }

    /// Proxy effectif des envois (R2).
    pub fn effective_proxy(&self) -> EffectiveProxy {
        let prefs = self.prefs();
        match prefs.proxy.mode {
            ProxyMode::None => EffectiveProxy::default(),
            ProxyMode::System => {
                let system = self.system.detect();
                EffectiveProxy {
                    http: system.http,
                    https: system.https,
                    no_proxy: system.no_proxy,
                    ..EffectiveProxy::default()
                }
            }
            ProxyMode::Manual => {
                let manual = prefs.proxy.manual;
                let url = Some(manual.url.trim().to_owned()).filter(|u| !u.is_empty());
                EffectiveProxy {
                    http: url.clone(),
                    https: url,
                    no_proxy: manual.no_proxy,
                    password: (!manual.username.trim().is_empty())
                        .then(|| self.proxy_password())
                        .flatten(),
                    username: manual.username.trim().to_owned(),
                }
            }
        }
    }

    /// Vue des préférences (1.6-e).
    pub fn view(&self) -> NetworkView {
        let prefs = self.prefs();
        NetworkView {
            has_proxy_password: self.proxy_password().is_some_and(|p| !p.is_empty()),
            system: self.system.detect(),
            connect_timeout_ms: prefs.connect_timeout_ms,
            system_authorities: tls::system_root_count(),
            authorities: prefs
                .authorities
                .iter()
                .filter_map(|a| {
                    cert::parse_pem(&a.pem).ok().map(|(_, info)| AuthorityView {
                        info,
                        added_at: a.added_at,
                    })
                })
                .collect(),
            proxy: prefs.proxy,
        }
    }

    /// Enregistre le proxy et le délai de connexion (PR3, PR4).
    pub fn save(&self, input: NetworkPrefsInput) -> Result<NetworkView, CommandError> {
        let url = input.proxy.manual.url.trim();
        if !url.is_empty() && Url::parse(&proxy::normalize(url)).is_err() {
            return Err(invalid(format!(
                "L'adresse du proxy « {url} » est invalide : écrivez hôte:port, par exemple proxy.entreprise.local:8080."
            )));
        }
        match input.proxy_password.as_deref() {
            Some("") => self.secrets.delete(PROXY_SECRET_KEY)?,
            Some(password) => self.secrets.set(PROXY_SECRET_KEY, password)?,
            None => {}
        }
        let timeout = input
            .connect_timeout_ms
            .clamp(CONNECT_TIMEOUT_RANGE.0, CONNECT_TIMEOUT_RANGE.1);
        self.update(|prefs| {
            prefs.proxy = input.proxy;
            prefs.connect_timeout_ms = timeout;
        })?;
        Ok(self.view())
    }

    /// Ajoute une autorité interne (R4, PR5, EC1).
    pub fn add_authority(&self, pem: &str) -> Result<NetworkView, CommandError> {
        let (der, info) = cert::parse_pem(pem)?;
        let known =
            self.prefs().authorities.iter().any(|a| {
                cert::parse_pem(&a.pem).is_ok_and(|(_, i)| i.fingerprint == info.fingerprint)
            });
        if !known {
            let pem = cert::to_pem(&der);
            self.update(|prefs| {
                prefs.authorities.push(StoredAuthority {
                    pem,
                    added_at: now_ms(),
                });
            })?;
        }
        Ok(self.view())
    }

    /// Retire une autorité interne (PR6).
    pub fn remove_authority(&self, fingerprint: &str) -> Result<NetworkView, CommandError> {
        let before = self.prefs().authorities.len();
        self.update(|prefs| {
            prefs.authorities.retain(|a| {
                cert::parse_pem(&a.pem).is_ok_and(|(_, i)| i.fingerprint != fingerprint)
            });
        })?;
        if self.prefs().authorities.len() == before {
            return Err(CommandError::new(
                CommandErrorKind::NotFound,
                "Cette autorité n'est plus dans les préférences.",
            ));
        }
        Ok(self.view())
    }

    /// Client des envois pour `max_redirects`, avec le proxy et le délai de connexion.
    fn client(
        &self,
        max_redirects: u32,
    ) -> Result<(reqwest::Client, EffectiveProxy, u64), CommandError> {
        let mut built = self.built.lock().map_err(|_| busy())?;
        if built.is_none() {
            let prefs = self.prefs();
            let internal: Vec<Vec<u8>> = prefs
                .authorities
                .iter()
                .filter_map(|a| cert::parse_pem(&a.pem).ok().map(|(der, _)| der))
                .collect();
            let roots = tls::root_store(true, &internal);
            *built = Some(Built {
                tls: tls::client_config(roots, self.rejections.clone())?,
                proxy: self.effective_proxy(),
                connect_timeout_ms: prefs.connect_timeout_ms,
                clients: HashMap::new(),
            });
        }
        let Some(built) = built.as_mut() else {
            return Err(busy());
        };
        if let Some(client) = built.clients.get(&max_redirects) {
            return Ok((
                client.clone(),
                built.proxy.clone(),
                built.connect_timeout_ms,
            ));
        }
        let policy = match max_redirects {
            0 => Policy::none(),
            max => Policy::limited(max as usize),
        };
        let mut builder = reqwest::Client::builder()
            .redirect(policy)
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_millis(built.connect_timeout_ms))
            .use_preconfigured_tls(built.tls.clone());
        if built.proxy.is_direct() {
            builder = builder.no_proxy();
        } else {
            let effective = built.proxy.clone();
            let mut proxy = reqwest::Proxy::custom(move |url| {
                effective
                    .for_url(url.scheme(), url.host_str().unwrap_or_default())
                    .and_then(|p| Url::parse(&p).ok())
            });
            if !built.proxy.username.is_empty() {
                proxy = proxy.basic_auth(
                    &built.proxy.username,
                    built.proxy.password.as_deref().unwrap_or_default(),
                );
            }
            builder = builder.proxy(proxy);
        }
        let client = builder.build().map_err(|e| {
            CommandError::new(
                CommandErrorKind::Network,
                format!("Client HTTP indisponible : {e}."),
            )
        })?;
        built.clients.insert(max_redirects, client.clone());
        Ok((client, built.proxy.clone(), built.connect_timeout_ms))
    }

    /// Échange `prepared`, échec expliqué (R5 à R7).
    async fn exchange(
        &self,
        prepared: &Prepared,
        cancel: &Cancel,
    ) -> Result<HttpResponse, CommandError> {
        let (client, effective, connect_timeout_ms) = self.client(prepared.max_redirects)?;
        let url = Url::parse(&prepared.url).ok();
        let host = url
            .as_ref()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
            .unwrap_or_default();
        let used_proxy = url
            .as_ref()
            .and_then(|u| effective.for_url(u.scheme(), &host));
        let started = Instant::now();
        match http::send_with(&client, prepared, cancel).await {
            Ok(response) => {
                if response.status == 407
                    && let Some(proxy) = &used_proxy
                {
                    return Err(failure::proxy_auth_response(
                        &prepared.url,
                        proxy,
                        connect_timeout_ms,
                        response.total_ms,
                    ));
                }
                Ok(response)
            }
            Err(Failed::Command(error)) => Err(error),
            Err(Failed::Http(error, elapsed_ms)) => {
                let rejection = self
                    .rejections
                    .lock()
                    .ok()
                    .and_then(|mut all| all.remove(&host))
                    .filter(|r| r.at >= started);
                Err(failure::classify(
                    &error,
                    &Attempt {
                        url: &prepared.url,
                        proxy: used_proxy,
                        connect_timeout_ms,
                        elapsed_ms,
                        rejection,
                    },
                ))
            }
        }
    }

    /// Envoie `prepared` : jeton OAuth 2 et cookies ajoutés, cookies reçus gardés (R9, R12).
    /// Les valeurs ajoutées à masquer vont dans `prepared.hidden`.
    pub async fn send(
        &self,
        prepared: &mut Prepared,
        session: Option<&Session>,
        cancel: &Cancel,
    ) -> Result<HttpResponse, CommandError> {
        if let Some(grant) = prepared.oauth2.clone() {
            let token = self.token(session, &grant, false, cancel).await?;
            oauth::apply(prepared, &grant, &token.access_token);
        }
        let url = Url::parse(&prepared.url).ok();
        if let (Some(session), Some(url)) = (session, url.as_ref())
            && !prepared
                .headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("cookie"))
        {
            let cookie = self.with_jar(session, |jar| jar.header_for(url, now_ms()));
            if let Some((header, hidden)) = cookie.flatten() {
                prepared.headers.push(("Cookie".to_owned(), header));
                prepared.hidden.extend(hidden);
            }
        }
        let response = self.exchange(prepared, cancel).await?;
        if let (Some(session), Some(url)) = (session, url.as_ref()) {
            let set_cookies: Vec<&str> = response
                .headers
                .iter()
                .filter(|(n, _)| n.eq_ignore_ascii_case("set-cookie"))
                .map(|(_, v)| v.as_str())
                .collect();
            if !set_cookies.is_empty() {
                let origin = format!("{} {}", prepared.method.as_str(), url.path());
                self.change_jar(session, |jar| {
                    jar.store(url, &set_cookies, &origin, now_ms());
                });
            }
        }
        Ok(response)
    }

    // ------------------------------------------------------------ jetons OAuth 2

    /// Jeton de `grant` : celui gardé s'il est frais, sinon demandé (R9).
    pub async fn token(
        &self,
        session: Option<&Session>,
        grant: &OAuth2Grant,
        force: bool,
        cancel: &Cancel,
    ) -> Result<Token, CommandError> {
        let key = TokenKey::of(
            session.map_or(Uuid::nil(), |s| s.project),
            session.map_or("", |s| s.environment.as_str()),
            grant,
        );
        let now = now_ms();
        if !force
            && let Some(token) = self
                .tokens
                .lock()
                .ok()
                .and_then(|cache| cache.fresh(&key, now).cloned())
        {
            return Ok(token);
        }
        let token = self.fetch_token(grant, cancel).await?;
        if let Ok(mut cache) = self.tokens.lock() {
            cache.put(key, token.clone());
        }
        Ok(token)
    }

    async fn fetch_token(
        &self,
        grant: &OAuth2Grant,
        cancel: &Cancel,
    ) -> Result<Token, CommandError> {
        if grant.token_url.is_empty() {
            return Err(invalid(
                "Renseignez l'URL du jeton OAuth 2 dans les réglages du projet.",
            ));
        }
        let in_body = grant.credentials == CredentialsPlacement::Body;
        let encode = oauth::url_encode;
        let body = oauth::form(grant, in_body)
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let mut headers = vec![
            (
                "Content-Type".to_owned(),
                "application/x-www-form-urlencoded".to_owned(),
            ),
            ("Accept".to_owned(), "application/json".to_owned()),
        ];
        if !in_body {
            use base64::Engine;
            let credentials = format!(
                "{}:{}",
                encode(&grant.client_id),
                encode(&grant.client_secret)
            );
            headers.push((
                "Authorization".to_owned(),
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode(credentials)
                ),
            ));
        }
        let request = Prepared {
            method: Method::Post,
            url: grant.token_url.clone(),
            headers,
            body: Some(body),
            timeout_ms: Some(30_000),
            max_redirects: 0,
            drawn: Vec::new(),
            used: Vec::new(),
            oauth2: None,
            hidden: Vec::new(),
        };
        let response = self.exchange(&request, cancel).await?;
        oauth::parse_response(response.status, &response.body, now_ms()).map_err(|reason| {
            CommandError::new(
                CommandErrorKind::Network,
                format!(
                    "Le jeton OAuth 2 n'a pas été obtenu de {} : {reason}. Vérifiez l'identifiant, le secret et la portée dans les réglages du projet.",
                    grant.token_url
                ),
            )
        })
    }

    /// État du jeton d'un environnement (OA3).
    pub fn token_status(&self, project: Uuid, environment: &str) -> Option<TokenStatus> {
        self.tokens
            .lock()
            .ok()
            .and_then(|cache| cache.status(project, environment))
    }

    /// Oublie les jetons d'un environnement (OA3).
    pub fn forget_tokens(&self, project: Uuid, environment: &str) {
        if let Ok(mut cache) = self.tokens.lock() {
            cache.forget(project, environment);
        }
    }

    // ------------------------------------------------------------ cookies

    fn with_jar<T>(&self, session: &Session, read: impl FnOnce(&cookies::Jar) -> T) -> Option<T> {
        let mut all = self.jars.lock().ok()?;
        let jars = self.project_jars(&mut all, session.project);
        Some(read(jars.entry(session.environment.clone()).or_default()))
    }

    fn project_jars<'a>(
        &self,
        all: &'a mut HashMap<Uuid, ProjectJars>,
        project: Uuid,
    ) -> &'a mut ProjectJars {
        all.entry(project).or_insert_with(|| {
            self.data_dir
                .get()
                .map(|dir| cookies::load(dir, project, now_ms()))
                .unwrap_or_default()
        })
    }

    /// Modifie les cookies d'un environnement et écrit les persistants.
    fn change_jar(&self, session: &Session, change: impl FnOnce(&mut cookies::Jar)) {
        let Ok(mut all) = self.jars.lock() else {
            return;
        };
        let jars = self.project_jars(&mut all, session.project);
        let jar = jars.entry(session.environment.clone()).or_default();
        change(jar);
        jar.purge(now_ms());
        if let Some(dir) = self.data_dir.get() {
            let _ = cookies::save(dir, session.project, jars);
        }
    }

    /// Cookies gardés d'un environnement, ceux qui couvrent `host` (CK2).
    pub fn cookies(&self, session: &Session, host: Option<&str>) -> CookieJarView {
        self.with_jar(session, |jar| {
            jar.view(&session.environment, host, now_ms())
        })
        .unwrap_or_else(|| cookies::Jar::default().view(&session.environment, host, now_ms()))
    }

    /// Vide les cookies d'un domaine, ou tous (CK3).
    pub fn clear_cookies(&self, session: &Session, domain: Option<&str>) {
        self.change_jar(session, |jar| jar.clear(domain));
    }

    /// Renvoie ou non les cookies d'un domaine (CK4).
    pub fn set_cookie_sending(&self, session: &Session, domain: &str, enabled: bool) {
        self.change_jar(session, |jar| jar.set_sending(domain, enabled));
    }
}
