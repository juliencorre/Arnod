//! Chronologie d'un envoi (lot K, écran 5.1-c) : où le temps est passé, autant que le client
//! HTTP le laisse mesurer. La résolution DNS passe par un résolveur qui se chronomètre ; la
//! connexion (TCP, puis TLS en HTTPS, ou le tunnel d'un proxy) par une couche autour du
//! connecteur de reqwest. Les deux écrivent dans les mesures de l'envoi en cours, portées par une
//! variable locale à la tâche : une connexion reprise du pool ne mesure rien (« réutilisée »).

use std::future::Future;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tower_layer::Layer;
use tower_service::Service;

/// Mesures d'un envoi, en millisecondes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Phases {
    /// Résolution DNS ; `None` si aucune n'a eu lieu (connexion réutilisée, adresse IP).
    pub dns_ms: Option<u64>,
    /// Connexion complète, DNS compris ; `None` si une connexion du pool a servi.
    pub connect_ms: Option<u64>,
}

tokio::task_local! {
    static CURRENT: Arc<Mutex<Phases>>;
}

fn millis(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn current() -> Option<Arc<Mutex<Phases>>> {
    CURRENT.try_with(Arc::clone).ok()
}

/// Exécute `send` en mesurant ses phases.
pub async fn measured<F: Future>(send: F) -> (F::Output, Phases) {
    let phases = Arc::new(Mutex::new(Phases::default()));
    let output = CURRENT.scope(Arc::clone(&phases), send).await;
    let measured = phases.lock().map(|p| *p).unwrap_or_default();
    (output, measured)
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Résolveur du système (getaddrinfo, sur un fil bloquant) qui se chronomètre.
#[derive(Debug, Default)]
pub struct TimedResolver;

impl Resolve for TimedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let phases = current();
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let start = Instant::now();
            let found = tokio::task::spawn_blocking(move || {
                (host.as_str(), 0)
                    .to_socket_addrs()
                    .map(Iterator::collect::<Vec<_>>)
            })
            .await
            .map_err(|e| Box::new(e) as BoxError)?
            .map_err(|e| Box::new(e) as BoxError)?;
            if let Some(phases) = phases
                && let Ok(mut p) = phases.lock()
            {
                p.dns_ms = Some(p.dns_ms.unwrap_or(0) + millis(start));
            }
            Ok(Box::new(found.into_iter()) as Addrs)
        })
    }
}

/// Couche autour du connecteur : chronomètre chaque nouvelle connexion.
#[derive(Debug, Clone, Copy, Default)]
pub struct TimedConnectLayer;

impl<S> Layer<S> for TimedConnectLayer {
    type Service = TimedConnect<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimedConnect(inner)
    }
}

/// Connecteur chronométré.
#[derive(Debug, Clone)]
pub struct TimedConnect<S>(S);

impl<S, R> Service<R> for TimedConnect<S>
where
    S: Service<R>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        let phases = current();
        let connecting = self.0.call(request);
        Box::pin(async move {
            let start = Instant::now();
            let connected = connecting.await;
            if connected.is_ok()
                && let Some(phases) = phases
                && let Ok(mut p) = phases.lock()
            {
                p.connect_ms = Some(millis(start));
            }
            connected
        })
    }
}
