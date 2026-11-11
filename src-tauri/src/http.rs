//! Envoi d'une requête HTTP et mesure de la réponse (E1 à E6).

use std::error::Error as StdError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use serde::Serialize;
use tokio::sync::Notify;

use crate::error::{CommandError, CommandErrorKind};
use crate::request::assertions::ResponseData;
use crate::request::resolve::Prepared;

/// Taille maximale du corps conservé (5 Mo) ; au-delà il est tronqué.
pub const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

/// Réponse reçue, prête à être affichée.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub truncated: bool,
    pub size_bytes: usize,
    /// Attente du premier octet, en ms.
    pub wait_ms: u64,
    /// Réception du corps, en ms.
    pub receive_ms: u64,
    pub total_ms: u64,
}

impl HttpResponse {
    pub fn data(&self) -> ResponseData {
        ResponseData {
            status: self.status,
            duration_ms: self.total_ms,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}

/// Annulation d'un envoi ou d'un run (E5). Elle est mémorisée : un envoi qui
/// commence après elle est annulé aussi, alors qu'un simple `Notify` ne
/// réveille que les envois déjà en attente.
#[derive(Debug, Default)]
pub struct Cancel {
    cancelled: AtomicBool,
    notify: Notify,
}

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Annule les envois en cours et ceux à venir.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Rend la main dès que l'annulation est demandée, même avant l'appel.
    pub async fn cancelled(&self) {
        let notified = self.notify.notified();
        tokio::pin!(notified);
        // Inscrit avant de relire le drapeau : une annulation entre les deux n'est pas perdue.
        notified.as_mut().enable();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

fn error(kind: CommandErrorKind, message: impl Into<String>) -> CommandError {
    CommandError::new(kind, message)
}

/// Cause première d'une erreur : le dernier maillon de sa chaîne de sources
/// (connexion refusée, nom introuvable, certificat refusé…).
fn root_cause(error: &(dyn StdError + 'static)) -> String {
    let mut cause = error;
    while let Some(source) = cause.source() {
        cause = source;
    }
    cause.to_string()
}

fn network(error: &reqwest::Error, url: &str) -> CommandError {
    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned());
    if error.is_timeout() {
        return self::error(
            CommandErrorKind::Timeout,
            "Le délai est dépassé sans réponse.",
        );
    }
    if error.is_connect() {
        // La cause distingue un serveur arrêté d'un nom introuvable ou d'un certificat refusé.
        return self::error(
            CommandErrorKind::Network,
            format!(
                "Impossible de joindre {host} ({}). Vérifiez l'adresse et la connexion.",
                root_cause(error)
            ),
        );
    }
    if error.is_builder() {
        return self::error(
            CommandErrorKind::InvalidRequest,
            "L'adresse de la requête est invalide.",
        );
    }
    if error.is_redirect() {
        return self::error(
            CommandErrorKind::Network,
            "Trop de redirections : augmentez la limite dans les réglages de la requête.",
        );
    }
    self::error(
        CommandErrorKind::Network,
        format!("La requête a échoué : {}.", root_cause(error)),
    )
}

/// En-têtes vérifiés avant l'envoi : un nom ou une valeur refusés par HTTP
/// sont nommés, au lieu d'une « adresse invalide ».
fn checked_headers(
    headers: &[(String, String)],
) -> Result<Vec<(HeaderName, HeaderValue)>, CommandError> {
    headers
        .iter()
        .map(|(name, value)| {
            let header = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                error(
                    CommandErrorKind::InvalidRequest,
                    format!(
                        "L'en-tête « {name} » a un nom invalide : ni espace, ni accent, ni « : »."
                    ),
                )
            })?;
            let value = HeaderValue::from_str(value).map_err(|_| {
                error(
                    CommandErrorKind::InvalidRequest,
                    format!(
                        "La valeur de l'en-tête « {name} » est invalide : ni retour à la ligne, ni caractère de contrôle."
                    ),
                )
            })?;
            Ok((header, value))
        })
        .collect()
}

/// Retire la fin d'un caractère UTF-8 coupé par la troncature.
fn trim_cut_character(bytes: &mut Vec<u8>) {
    if let Err(error) = std::str::from_utf8(bytes)
        && error.error_len().is_none()
    {
        bytes.truncate(error.valid_up_to());
    }
}

/// Envoie la requête ; `cancel` l'interrompt (E5), même demandé avant l'envoi.
pub async fn send(prepared: &Prepared, cancel: &Cancel) -> Result<HttpResponse, CommandError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(error(CommandErrorKind::Cancelled, "Envoi annulé.")),
        result = run(prepared) => result,
    }
}

async fn run(prepared: &Prepared) -> Result<HttpResponse, CommandError> {
    // Redirections suivies au plus (R3) ; 0 : aucune.
    let policy = match prepared.max_redirects {
        0 => Policy::none(),
        max => Policy::limited(max as usize),
    };
    let client = reqwest::Client::builder()
        .redirect(policy)
        .build()
        .map_err(|e| {
            error(
                CommandErrorKind::Network,
                format!("Client HTTP indisponible : {e}."),
            )
        })?;
    let method = reqwest::Method::from_bytes(prepared.method.as_str().as_bytes())
        .map_err(|_| error(CommandErrorKind::InvalidRequest, "Méthode invalide."))?;
    let headers = checked_headers(&prepared.headers)?;
    let mut builder = client.request(method, &prepared.url);
    if let Some(ms) = prepared.timeout_ms {
        builder = builder.timeout(Duration::from_millis(ms));
    }
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = &prepared.body {
        builder = builder.body(body.clone());
    }
    let started = Instant::now();
    let mut response = builder
        .send()
        .await
        .map_err(|e| network(&e, &prepared.url))?;
    let wait = started.elapsed();
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(n, v)| {
            (
                n.as_str().to_owned(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect();
    let mut bytes: Vec<u8> = Vec::new();
    let mut size = 0usize;
    let mut truncated = false;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| network(&e, &prepared.url))?
    {
        size += chunk.len();
        if bytes.len() < MAX_BODY_BYTES {
            let room = MAX_BODY_BYTES - bytes.len();
            bytes.extend_from_slice(&chunk[..chunk.len().min(room)]);
        }
        if size > MAX_BODY_BYTES {
            truncated = true;
        }
    }
    if truncated {
        trim_cut_character(&mut bytes);
    }
    let total = started.elapsed();
    let millis = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_owned(),
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
        truncated,
        size_bytes: size,
        wait_ms: millis(wait),
        receive_ms: millis(total.saturating_sub(wait)),
        total_ms: millis(total),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::model::Method;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn prepared(url: String) -> Prepared {
        Prepared {
            method: Method::Get,
            url,
            headers: vec![("x-test".into(), "oui".into())],
            body: None,
            timeout_ms: Some(2000),
            max_redirects: 5,
        }
    }

    #[tokio::test]
    async fn recoit_statut_en_tetes_et_corps() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a"))
            .and(header("x-test", "oui"))
            .respond_with(ResponseTemplate::new(201).set_body_string("{\"ok\":true}"))
            .mount(&server)
            .await;
        let response = send(&prepared(format!("{}/a", server.uri())), &Cancel::new())
            .await
            .unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.body, "{\"ok\":true}");
        assert!(!response.truncated);
        assert!(response.total_ms >= response.wait_ms);
    }

    #[tokio::test]
    async fn tronque_au_dela_de_cinq_mo() {
        let server = MockServer::start().await;
        Mock::given(path("/gros"))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(vec![b'a'; MAX_BODY_BYTES + 10]),
            )
            .mount(&server)
            .await;
        let response = send(&prepared(format!("{}/gros", server.uri())), &Cancel::new())
            .await
            .unwrap();
        assert!(response.truncated);
        assert_eq!(response.body.len(), MAX_BODY_BYTES);
        assert_eq!(response.size_bytes, MAX_BODY_BYTES + 10);
    }

    #[tokio::test]
    async fn delai_depasse() {
        let server = MockServer::start().await;
        Mock::given(path("/lent"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(600)))
            .mount(&server)
            .await;
        let mut p = prepared(format!("{}/lent", server.uri()));
        p.timeout_ms = Some(100);
        let error = send(&p, &Cancel::new()).await.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Timeout);
    }

    #[tokio::test]
    async fn r3_redirections_suivies_ou_non() {
        let server = MockServer::start().await;
        Mock::given(path("/ancien"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/nouveau"))
            .mount(&server)
            .await;
        Mock::given(path("/nouveau"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let mut p = prepared(format!("{}/ancien", server.uri()));
        let followed = send(&p, &Cancel::new()).await.unwrap();
        assert_eq!(followed.status, 200);
        p.max_redirects = 0;
        let kept = send(&p, &Cancel::new()).await.unwrap();
        assert_eq!(kept.status, 302);
    }

    #[tokio::test]
    async fn r5_sans_delai() {
        let server = MockServer::start().await;
        Mock::given(path("/lent"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(300)))
            .mount(&server)
            .await;
        let mut p = prepared(format!("{}/lent", server.uri()));
        p.timeout_ms = None;
        assert_eq!(send(&p, &Cancel::new()).await.unwrap().status, 200);
    }

    #[tokio::test]
    async fn serveur_injoignable() {
        let mut request = prepared("http://127.0.0.1:1/".into());
        // Sous Windows, un refus de connexion n'est constaté qu'après plusieurs essais
        // (environ 2 s) : le délai du test doit les laisser aboutir.
        request.timeout_ms = Some(15_000);
        let error = send(&request, &Cancel::new()).await.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Network);
        assert!(error.message.contains("127.0.0.1"));
    }

    #[tokio::test]
    async fn annulation() {
        let server = MockServer::start().await;
        Mock::given(path("/lent"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;
        let cancel = std::sync::Arc::new(Cancel::new());
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            c.cancel();
        });
        let mut p = prepared(format!("{}/lent", server.uri()));
        p.timeout_ms = Some(10_000);
        let error = send(&p, &cancel).await.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Cancelled);
    }

    #[tokio::test]
    async fn annulation_demandee_avant_l_envoi_rien_ne_part() {
        let server = MockServer::start().await;
        Mock::given(path("/a"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let cancel = Cancel::new();
        cancel.cancel();
        let error = send(&prepared(format!("{}/a", server.uri())), &cancel)
            .await
            .unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::Cancelled);
        // `expect(0)` : le serveur n'a rien reçu.
        server.verify().await;
    }

    #[tokio::test]
    async fn en_tete_invalide_nomme_dans_le_message() {
        let mut p = prepared("http://127.0.0.1:1/".into());
        p.headers = vec![("Content Type".into(), "application/json".into())];
        let error = send(&p, &Cancel::new()).await.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        assert!(
            error.message.contains("« Content Type »"),
            "{}",
            error.message
        );

        p.headers = vec![("X-Trace".into(), "a\nb".into())];
        let error = send(&p, &Cancel::new()).await.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        assert!(
            error.message.contains("valeur de l'en-tête « X-Trace »"),
            "{}",
            error.message
        );
    }

    #[tokio::test]
    async fn troncature_sans_caractere_coupe() {
        let server = MockServer::start().await;
        // Un octet puis des « é » (deux octets) : la limite tombe au milieu d'un caractère.
        let body = format!("a{}", "é".repeat(MAX_BODY_BYTES / 2 + 8));
        Mock::given(path("/accents"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        let response = send(
            &prepared(format!("{}/accents", server.uri())),
            &Cancel::new(),
        )
        .await
        .unwrap();
        assert!(response.truncated);
        assert!(response.body.len() <= MAX_BODY_BYTES);
        assert!(response.body.ends_with('é'), "fin : caractère entier");
        assert!(!response.body.contains('\u{fffd}'));
    }

    #[tokio::test]
    async fn echec_de_connexion_donne_sa_cause() {
        // Serveur qui ne parle pas TLS : la poignée de main https échoue.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            use std::io::Write;
            for mut socket in listener.incoming().flatten() {
                let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
            }
        });
        let tls = send(
            &prepared(format!("https://127.0.0.1:{port}/")),
            &Cancel::new(),
        )
        .await
        .unwrap_err();
        let mut closed = prepared("http://127.0.0.1:1/".into());
        closed.timeout_ms = Some(15_000);
        let refused = send(&closed, &Cancel::new()).await.unwrap_err();
        assert_eq!(tls.kind, CommandErrorKind::Network);
        assert_eq!(refused.kind, CommandErrorKind::Network);
        // Les deux pannes ne se confondent plus : chacune donne sa cause.
        assert_ne!(tls.message, refused.message);
        assert!(
            refused.message.contains("127.0.0.1 ("),
            "{}",
            refused.message
        );
    }

    #[test]
    fn cause_premiere_d_une_chaine_d_erreurs() {
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("erreur d'envoi")
            }
        }
        impl StdError for Outer {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                Some(&self.0)
            }
        }
        let error = Outer(std::io::Error::other("certificat refusé"));
        assert_eq!(root_cause(&error), "certificat refusé");
    }
}
