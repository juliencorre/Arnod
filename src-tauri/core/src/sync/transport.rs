//! Transport « smart HTTP » de Git écrit sur [`Network`] (V2, lot E, R9) : libgit2 est compilé
//! sans HTTPS ; ses échanges avec un serveur Git passent par le client du poste (proxy,
//! autorités internes, certificat refusé expliqué), comme tout envoi d'Arnod. Le jeton du compte
//! relié, lu au coffre, part dans l'en-tête `Authorization` ; il n'est jamais écrit dans
//! `.git/config` ni dans l'adresse du dépôt.
//!
//! Les opérations Git sont synchrones : elles tournent sur un fil sans runtime tokio, et chaque
//! échange attend sa réponse sur le runtime du cœur ([`crate::workbench::core_runtime`]).

use std::cell::RefCell;
use std::io::{self, Cursor, Read, Write};
use std::sync::{Arc, Mutex, Once};

use git2::transport::{Service, SmartSubtransport, SmartSubtransportStream, Transport};

use crate::network::Network;
use crate::workbench::core_runtime;

/// Taille maximale d'une réponse (un paquet d'objets) : 1 Go.
const MAX_RESPONSE: u64 = 1024 * 1024 * 1024;

/// Ce qu'un échange emporte : le réseau du poste et l'en-tête d'authentification.
#[derive(Clone)]
pub struct Context {
    pub network: Arc<Network>,
    /// Valeur de l'en-tête `Authorization` (`Basic …`), ou rien pour un dépôt public.
    pub authorization: Option<String>,
}

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
}

/// Exécute `operation` (sur ce fil) avec ce contexte pour ses échanges HTTP.
pub fn with_context<T>(context: Context, operation: impl FnOnce() -> T) -> T {
    register();
    CONTEXT.with(|current| *current.borrow_mut() = Some(context));
    let result = operation();
    CONTEXT.with(|current| *current.borrow_mut() = None);
    result
}

/// Enregistre le transport pour `http` et `https`, une fois pour tout le processus.
fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for prefix in ["https", "http"] {
            // SAFETY : appelé une seule fois, avant toute opération Git qui utilise ce préfixe.
            let _ = unsafe {
                git2::transport::register(prefix, |remote| {
                    let context = CONTEXT.with(|c| c.borrow().clone());
                    Transport::smart(
                        remote,
                        true,
                        Subtransport {
                            context,
                            base: Mutex::new(None),
                        },
                    )
                })
            };
        }
    });
}

struct Subtransport {
    context: Option<Context>,
    base: Mutex<Option<String>>,
}

impl SmartSubtransport for Subtransport {
    fn action(
        &self,
        url: &str,
        action: Service,
    ) -> Result<Box<dyn SmartSubtransportStream>, git2::Error> {
        let mut base = self
            .base
            .lock()
            .map_err(|_| git2::Error::from_str("transport indisponible"))?;
        let base = base
            .get_or_insert_with(|| url.trim_end_matches('/').to_owned())
            .clone();
        let (service, path, post) = match action {
            Service::UploadPackLs => ("upload-pack", "/info/refs?service=git-upload-pack", false),
            Service::UploadPack => ("upload-pack", "/git-upload-pack", true),
            Service::ReceivePackLs => {
                ("receive-pack", "/info/refs?service=git-receive-pack", false)
            }
            Service::ReceivePack => ("receive-pack", "/git-receive-pack", true),
        };
        let context = self.context.clone().ok_or_else(|| {
            git2::Error::from_str("échange Git lancé hors d'une opération d'Arnod")
        })?;
        Ok(Box::new(Stream {
            context,
            url: format!("{base}{path}"),
            service,
            post,
            body: Vec::new(),
            response: None,
        }))
    }

    fn close(&self) -> Result<(), git2::Error> {
        Ok(())
    }
}

struct Stream {
    context: Context,
    url: String,
    service: &'static str,
    post: bool,
    body: Vec<u8>,
    response: Option<Cursor<Vec<u8>>>,
}

fn io_error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

impl Stream {
    fn execute(&mut self) -> io::Result<Vec<u8>> {
        let mut headers = vec![("user-agent".to_owned(), crate::http::USER_AGENT.to_owned())];
        if let Some(authorization) = &self.context.authorization {
            headers.push(("authorization".to_owned(), authorization.clone()));
        }
        let method = if self.post {
            headers.push((
                "content-type".to_owned(),
                format!("application/x-git-{}-request", self.service),
            ));
            headers.push((
                "accept".to_owned(),
                format!("application/x-git-{}-result", self.service),
            ));
            "POST"
        } else {
            "GET"
        };
        let body = self.post.then(|| std::mem::take(&mut self.body));
        let runtime = core_runtime().ok_or_else(|| io_error("runtime indisponible"))?;
        let network = self.context.network.clone();
        let url = self.url.clone();
        let exchange = runtime
            .block_on(async move {
                network
                    .raw(method, &url, &headers, body, MAX_RESPONSE)
                    .await
            })
            .map_err(|error| io_error(error.message))?;
        match exchange.status {
            200 => {}
            401 | 403 => {
                return Err(io_error(
                    "le serveur Git refuse l'accès : connectez le compte du dépôt (jeton absent, \
                     expiré ou sans droit sur ce dépôt)",
                ));
            }
            404 => return Err(io_error("dépôt introuvable sur le serveur Git")),
            status => {
                return Err(io_error(format!(
                    "le serveur Git a répondu {status} à {}",
                    self.url
                )));
            }
        }
        if !self.post {
            let expected = format!("application/x-git-{}-advertisement", self.service);
            let content_type = exchange.header("content-type").unwrap_or_default();
            if !content_type.starts_with(&expected) {
                return Err(io_error(
                    "ce serveur ne parle pas le protocole Git sur HTTP (adresse du dépôt ?)",
                ));
            }
        }
        Ok(exchange.body)
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.response.is_none() {
            let body = self.execute()?;
            self.response = Some(Cursor::new(body));
        }
        self.response
            .as_mut()
            .map_or(Ok(0), |response| response.read(buf))
    }
}

impl Write for Stream {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.response.is_some() {
            return Err(io_error("écriture après la réponse"));
        }
        self.body.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// En-tête `Authorization` d'un jeton : nom d'utilisateur attendu par la forge, puis le jeton.
pub fn basic(user: &str, token: &str) -> String {
    use base64::Engine as _;
    let raw = format!("{user}:{token}");
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(raw)
    )
}
