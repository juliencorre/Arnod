//! Forges (V2, lot E, SY1, SY2, Q2) : GitHub (et Enterprise), GitLab (et auto-hébergé), ou une
//! autre adresse Git. Compte, liste et création des dépôts par l'API de la forge, connexion par
//! le navigateur (autorisation « device », RFC 8628) ou jeton personnel ; tout passe par
//! [`Network`] (R9). Le jeton est rangé au coffre par l'appelant, jamais ici.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::state::Account;
use crate::error::{CommandError, CommandErrorKind};
use crate::network::{Exchange, Network};

/// Réponse d'API lue au plus (listes de dépôts) : 16 Mo.
const MAX_API: u64 = 16 * 1024 * 1024;

/// Identifiants des applications OAuth d'Arnod, fournis à la construction (Q2) : sans eux, la
/// connexion se fait par un jeton personnel.
const GITHUB_CLIENT_ID: Option<&str> = option_env!("ARNOD_GITHUB_CLIENT_ID");
const GITLAB_CLIENT_ID: Option<&str> = option_env!("ARNOD_GITLAB_CLIENT_ID");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncForge"))]
#[serde(rename_all = "camelCase")]
pub enum Forge {
    Github,
    Gitlab,
    /// Autre serveur Git (Gitea, Azure…) : jeton saisi, sans API.
    Other,
}

impl Forge {
    /// Nom d'utilisateur attendu par le serveur Git avec un jeton (authentification Basic).
    pub fn git_user(self, login: &str) -> &str {
        match self {
            Self::Github => "x-access-token",
            Self::Gitlab => "oauth2",
            Self::Other => login,
        }
    }

    fn client_id(self) -> Option<&'static str> {
        match self {
            Self::Github => GITHUB_CLIENT_ID,
            Self::Gitlab => GITLAB_CLIENT_ID,
            Self::Other => None,
        }
        .filter(|id| !id.is_empty())
    }

    /// La connexion par le navigateur est possible (application OAuth enregistrée, Q2).
    pub fn browser_login(self) -> bool {
        self.client_id().is_some()
    }
}

/// Adresse de base d'un hôte : `https://hôte`, ou l'adresse donnée avec son schéma (tests).
fn web_base(host: &str) -> String {
    if host.starts_with("http://") || host.starts_with("https://") {
        host.trim_end_matches('/').to_owned()
    } else {
        format!("https://{}", host.trim_end_matches('/'))
    }
}

/// Adresse de l'API d'une forge.
pub fn api_base(forge: Forge, host: &str) -> String {
    match forge {
        Forge::Github if host.eq_ignore_ascii_case("github.com") => {
            "https://api.github.com".to_owned()
        }
        Forge::Github => format!("{}/api/v3", web_base(host)),
        Forge::Gitlab | Forge::Other => format!("{}/api/v4", web_base(host)),
    }
}

/// Dépôt d'un compte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncRepository"))]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    /// `acme/boutique-tests`.
    pub name: String,
    /// Adresse HTTPS du dépôt, sans identifiant.
    pub url: String,
    pub private: bool,
    /// Le dépôt ne contient encore rien.
    pub empty: bool,
}

/// Code à saisir dans le navigateur (connexion « device »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncDeviceCode"))]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    /// Code de l'appareil, gardé par le cœur pour attendre le jeton (jamais montré).
    #[serde(skip)]
    pub device_code: String,
    pub interval_secs: u64,
    pub expires_in_secs: u64,
}

fn network_error(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::Network, message)
}

fn api_headers(token: &str) -> Vec<(String, String)> {
    vec![
        ("authorization".to_owned(), format!("Bearer {token}")),
        ("accept".to_owned(), "application/json".to_owned()),
        ("user-agent".to_owned(), crate::http::USER_AGENT.to_owned()),
    ]
}

fn json_of(exchange: &Exchange, what: &str) -> Result<Value, CommandError> {
    match exchange.status {
        200..=299 => serde_json::from_slice(&exchange.body)
            .map_err(|_| network_error(format!("Réponse illisible de la forge ({what})."))),
        401 => Err(network_error(
            "La forge refuse le jeton (expiré ou révoqué) : reconnectez le compte.",
        )),
        403 => Err(network_error(format!(
            "La forge refuse {what} : le jeton n'a pas les droits nécessaires."
        ))),
        status => Err(network_error(format!(
            "La forge a répondu {status} ({what})."
        ))),
    }
}

/// Compte du jeton (SY1).
pub async fn account(
    network: &Network,
    forge: Forge,
    host: &str,
    token: &str,
    login: Option<&str>,
) -> Result<Account, CommandError> {
    if forge == Forge::Other {
        let login = login
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .ok_or_else(|| {
                CommandError::new(
                    CommandErrorKind::InvalidRequest,
                    "Indiquez le nom d'utilisateur du serveur Git.",
                )
            })?;
        return Ok(Account {
            forge,
            host: host.to_owned(),
            login: login.to_owned(),
            name: login.to_owned(),
            email: format!("{login}@{}", host_name(host)),
        });
    }
    let path = match forge {
        Forge::Github => "/user",
        _ => "/user",
    };
    let url = format!("{}{path}", api_base(forge, host));
    let exchange = network
        .raw("GET", &url, &api_headers(token), None, MAX_API)
        .await?;
    let user = json_of(&exchange, "le compte")?;
    let text = |key: &str| user.get(key).and_then(Value::as_str).map(str::to_owned);
    let login = match forge {
        Forge::Github => text("login"),
        _ => text("username"),
    }
    .ok_or_else(|| network_error("Compte illisible."))?;
    let name = text("name")
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| login.clone());
    let email = text("email")
        .filter(|e| !e.is_empty())
        .unwrap_or_else(|| match forge {
            // Adresse privée de GitHub (Q11).
            Forge::Github => {
                let id = user.get("id").and_then(Value::as_u64).unwrap_or(0);
                format!("{id}+{login}@users.noreply.github.com")
            }
            _ => format!("{login}@users.noreply.{}", host_name(host)),
        });
    Ok(Account {
        forge,
        host: host.to_owned(),
        login,
        name,
        email,
    })
}

fn host_name(host: &str) -> String {
    let base = web_base(host);
    base.split("://")
        .nth(1)
        .unwrap_or(&base)
        .split(['/', ':'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn repository_of(forge: Forge, value: &Value) -> Option<Repository> {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match forge {
        Forge::Github => Some(Repository {
            name: text("full_name")?,
            url: text("clone_url")?,
            private: value
                .get("private")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            empty: value.get("size").and_then(Value::as_u64) == Some(0),
        }),
        _ => Some(Repository {
            name: text("path_with_namespace")?,
            url: text("http_url_to_repo")?,
            private: text("visibility").is_none_or(|v| v != "public"),
            empty: value
                .get("empty_repo")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
    }
}

/// Dépôts du compte, les plus récents d'abord, filtrés par `query` (SY2).
pub async fn repositories(
    network: &Network,
    account: &Account,
    token: &str,
    query: &str,
) -> Result<Vec<Repository>, CommandError> {
    if account.forge == Forge::Other {
        return Ok(Vec::new());
    }
    let path = match account.forge {
        Forge::Github => "/user/repos?per_page=100&sort=updated".to_owned(),
        _ => "/projects?membership=true&simple=true&per_page=100&order_by=last_activity_at"
            .to_owned(),
    };
    let url = format!("{}{path}", api_base(account.forge, &account.host));
    let exchange = network
        .raw("GET", &url, &api_headers(token), None, MAX_API)
        .await?;
    let list = json_of(&exchange, "la liste des dépôts")?;
    let wanted = query.trim().to_lowercase();
    Ok(list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| repository_of(account.forge, value))
        .filter(|repo| wanted.is_empty() || repo.name.to_lowercase().contains(&wanted))
        .collect())
}

/// Crée un dépôt privé et vide au nom du compte (« + Créer un dépôt », SY2).
pub async fn create_repository(
    network: &Network,
    account: &Account,
    token: &str,
    name: &str,
) -> Result<Repository, CommandError> {
    let (path, body) = match account.forge {
        Forge::Github => (
            "/user/repos",
            json!({ "name": name, "private": true, "auto_init": false }),
        ),
        Forge::Gitlab => (
            "/projects",
            json!({ "name": name, "path": name, "visibility": "private" }),
        ),
        Forge::Other => {
            return Err(CommandError::new(
                CommandErrorKind::InvalidRequest,
                "Créez le dépôt sur le serveur Git, puis donnez son adresse.",
            ));
        }
    };
    let url = format!("{}{path}", api_base(account.forge, &account.host));
    let mut headers = api_headers(token);
    headers.push(("content-type".to_owned(), "application/json".to_owned()));
    let exchange = network
        .raw(
            "POST",
            &url,
            &headers,
            Some(body.to_string().into_bytes()),
            MAX_API,
        )
        .await?;
    if exchange.status == 422 || exchange.status == 400 {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("Un dépôt « {name} » existe déjà sur ce compte, ou ce nom est refusé."),
        ));
    }
    let created = json_of(&exchange, "la création du dépôt")?;
    let mut repository = repository_of(account.forge, &created)
        .ok_or_else(|| network_error("Dépôt créé illisible."))?;
    repository.empty = true;
    Ok(repository)
}

fn device_urls(forge: Forge, host: &str) -> (String, String) {
    let web = web_base(host);
    match forge {
        Forge::Github => (
            format!("{web}/login/device/code"),
            format!("{web}/login/oauth/access_token"),
        ),
        _ => (
            format!("{web}/oauth/authorize_device"),
            format!("{web}/oauth/token"),
        ),
    }
}

fn form(pairs: &[(&str, &str)]) -> Vec<u8> {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", utf8_percent_encode(v, NON_ALPHANUMERIC)))
        .collect::<Vec<_>>()
        .join("&")
        .into_bytes()
}

fn form_headers() -> Vec<(String, String)> {
    vec![
        (
            "content-type".to_owned(),
            "application/x-www-form-urlencoded".to_owned(),
        ),
        ("accept".to_owned(), "application/json".to_owned()),
        ("user-agent".to_owned(), crate::http::USER_AGENT.to_owned()),
    ]
}

/// Début de la connexion par le navigateur : le code à saisir et l'adresse à ouvrir.
pub async fn device_start(
    network: &Network,
    forge: Forge,
    host: &str,
) -> Result<DeviceCode, CommandError> {
    let client_id = forge.client_id().ok_or_else(|| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            "La connexion par le navigateur n'est pas encore disponible pour cette forge : \
             utilisez un jeton personnel.",
        )
    })?;
    device_start_as(network, forge, host, client_id).await
}

/// [`device_start`] avec l'identifiant de l'application OAuth donné (tests).
pub async fn device_start_as(
    network: &Network,
    forge: Forge,
    host: &str,
    client_id: &str,
) -> Result<DeviceCode, CommandError> {
    let scope = match forge {
        Forge::Github => "repo",
        _ => "api write_repository",
    };
    let (start, _) = device_urls(forge, host);
    let exchange = network
        .raw(
            "POST",
            &start,
            &form_headers(),
            Some(form(&[("client_id", client_id), ("scope", scope)])),
            MAX_API,
        )
        .await?;
    let value = json_of(&exchange, "la connexion")?;
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(DeviceCode {
        user_code: text("user_code").ok_or_else(|| network_error("Code de connexion absent."))?,
        verification_uri: text("verification_uri")
            .or_else(|| text("verification_url"))
            .ok_or_else(|| network_error("Adresse de connexion absente."))?,
        device_code: text("device_code")
            .ok_or_else(|| network_error("Code de connexion absent."))?,
        interval_secs: value.get("interval").and_then(Value::as_u64).unwrap_or(5),
        expires_in_secs: value
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(900),
    })
}

/// Attente du jeton : `Ok(None)` tant que l'utilisateur n'a pas validé dans le navigateur.
pub async fn device_poll(
    network: &Network,
    forge: Forge,
    host: &str,
    device_code: &str,
) -> Result<Option<String>, CommandError> {
    device_poll_as(
        network,
        forge,
        host,
        device_code,
        forge.client_id().unwrap_or_default(),
    )
    .await
}

/// [`device_poll`] avec l'identifiant de l'application OAuth donné (tests).
pub async fn device_poll_as(
    network: &Network,
    forge: Forge,
    host: &str,
    device_code: &str,
    client_id: &str,
) -> Result<Option<String>, CommandError> {
    let (_, token_url) = device_urls(forge, host);
    let exchange = network
        .raw(
            "POST",
            &token_url,
            &form_headers(),
            Some(form(&[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])),
            MAX_API,
        )
        .await?;
    let value: Value = serde_json::from_slice(&exchange.body).unwrap_or(Value::Null);
    if let Some(token) = value.get("access_token").and_then(Value::as_str) {
        return Ok(Some(token.to_owned()));
    }
    match value.get("error").and_then(Value::as_str) {
        Some("authorization_pending" | "slow_down") => Ok(None),
        Some("access_denied") => Err(CommandError::new(
            CommandErrorKind::Cancelled,
            "Connexion refusée dans le navigateur.",
        )),
        Some("expired_token") => Err(CommandError::new(
            CommandErrorKind::Timeout,
            "Le code de connexion a expiré : recommencez.",
        )),
        _ => Err(network_error(format!(
            "La forge a refusé la connexion (réponse {}).",
            exchange.status
        ))),
    }
}

/// Forge, hôte et nom d'un dépôt d'après son adresse (`https://github.com/acme/x.git`).
pub fn parse_remote(url: &str) -> (Forge, String, String) {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let without_user = without_scheme
        .rsplit_once('@')
        .map_or(without_scheme, |(_, rest)| rest);
    let (host, path) = without_user.split_once('/').unwrap_or((without_user, ""));
    let name = path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_owned();
    let lower = host.to_ascii_lowercase();
    let forge = if lower.contains("github") {
        Forge::Github
    } else if lower.contains("gitlab") {
        Forge::Gitlab
    } else {
        Forge::Other
    };
    (forge, host.to_owned(), name)
}

/// Adresse sans identifiant (R9) : un `user:jeton@` collé est retiré.
pub fn clean_url(url: &str) -> String {
    let url = url.trim();
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let rest = rest
                .split_once('@')
                .filter(|(user, _)| !user.contains('/'))
                .map_or(rest, |(_, host)| host);
            format!("{scheme}://{rest}")
        }
        None => url.to_owned(),
    }
}
