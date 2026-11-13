//! Échec réseau expliqué (`docs/plan-mvp-h-reseau-auth.md`, R5 à R7, écran 5.1-i) : la cause en
//! clair, ce qui n'est pas parti et de quoi régler le poste.

use std::error::Error as StdError;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};
use crate::network::cert::{self, CertificateInfo};
use crate::network::tls::{CertificateProblem, Rejection};

/// Cause d'un échec réseau (R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureCause {
    /// Certificat du serveur refusé.
    Certificate,
    /// Proxy injoignable.
    Proxy,
    /// Le proxy demande une identification (407) ou refuse le tunnel.
    ProxyAuth,
    /// Nom introuvable (DNS).
    Dns,
    /// Serveur injoignable (connexion refusée, délai de connexion).
    Unreachable,
    /// Délai de réponse dépassé.
    Timeout,
    /// Connexion sécurisée impossible pour une autre raison.
    Tls,
    /// Autre panne réseau.
    Other,
}

/// Certificat refusé : le problème, le certificat du serveur et l'autorité à laquelle faire
/// confiance si le serveur l'a présentée (R7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateFailure {
    pub problem: CertificateProblem,
    pub server: CertificateInfo,
    /// Autorité présentée, à laquelle « Faire confiance » ; `None` si elle n'a pas été envoyée.
    pub authority: Option<CertificateInfo>,
}

/// Échec d'un envoi, classé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFailure {
    pub cause: FailureCause,
    /// Titre : la cause en clair.
    pub title: String,
    /// Explication, avec ce qui n'est pas parti.
    pub explanation: String,
    /// La requête n'a pas atteint le serveur (R6).
    pub not_sent: bool,
    /// Hôte et port visés.
    pub host: String,
    pub port: u16,
    /// Proxy utilisé, s'il y en a un.
    pub proxy: Option<String>,
    /// Délai de connexion du poste, en ms.
    pub connect_timeout_ms: u64,
    /// Durée jusqu'à l'échec, en ms.
    pub elapsed_ms: u64,
    pub certificate: Option<CertificateFailure>,
}

/// Ce qu'on sait de l'envoi qui a échoué.
#[derive(Debug, Clone)]
pub struct Attempt<'a> {
    pub url: &'a str,
    pub proxy: Option<String>,
    pub connect_timeout_ms: u64,
    pub elapsed_ms: u64,
    pub rejection: Option<Rejection>,
}

/// Textes de la chaîne des causes, erreurs d'entrée-sortie ouvertes comprises.
fn chain_texts(error: &(dyn StdError + 'static)) -> Vec<String> {
    let mut texts = Vec::new();
    let mut current: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(cause) = current {
        texts.push(cause.to_string().to_lowercase());
        if let Some(io) = cause.downcast_ref::<std::io::Error>()
            && let Some(inner) = io.get_ref()
        {
            texts.push(inner.to_string().to_lowercase());
            if inner.downcast_ref::<rustls::Error>().is_some() {
                texts.push("tls:".to_owned());
            }
        }
        if cause.downcast_ref::<rustls::Error>().is_some() {
            texts.push("tls:".to_owned());
        }
        current = cause.source();
    }
    texts
}

/// Cause première lisible : le dernier maillon de la chaîne.
fn root_cause(error: &(dyn StdError + 'static)) -> String {
    let mut cause = error;
    while let Some(source) = cause.source() {
        cause = source;
    }
    cause.to_string()
}

fn host_port(url: &str) -> (String, u16) {
    reqwest::Url::parse(url).map_or_else(
        |_| (url.to_owned(), 0),
        |u| {
            (
                u.host_str().unwrap_or_default().to_owned(),
                u.port_or_known_default().unwrap_or(0),
            )
        },
    )
}

fn seconds(ms: u64) -> String {
    let s = ms / 1000;
    if ms.is_multiple_of(1000) {
        format!("{s} s")
    } else {
        format!("{:.1} s", ms as f64 / 1000.0).replace('.', ",")
    }
}

/// Le certificat refusé : serveur et autorité à proposer (R7).
fn certificate_failure(rejection: &Rejection) -> Option<CertificateFailure> {
    let server = cert::parse_der(rejection.chain.first()?).ok()?;
    // Un certificat auto-signé refusé l'est faute d'autorité, même quand webpki bute d'abord sur
    // une racine du poste au même nom (algorithme de clé différent).
    let problem = if server.self_signed && rejection.problem == CertificateProblem::Other {
        CertificateProblem::UnknownIssuer
    } else {
        rejection.problem
    };
    let authority = if problem != CertificateProblem::UnknownIssuer {
        None
    } else if server.self_signed {
        Some(server.clone())
    } else if rejection.chain.len() > 1 {
        // La dernière autorité présentée : une racine, ou une intermédiaire dont la racine
        // manque ; webpki accepte l'une comme l'autre en ancre.
        rejection
            .chain
            .last()
            .and_then(|der| cert::parse_der(der).ok())
    } else {
        None
    };
    Some(CertificateFailure {
        problem,
        server,
        authority,
    })
}

const NOT_SENT: &str = "La requête n'est pas partie : rien n'a été écrit sur le serveur.";

/// Échec réseau classé et message prêt à afficher (R5, R6).
pub fn classify(error: &reqwest::Error, attempt: &Attempt<'_>) -> CommandError {
    let (host, port) = host_port(attempt.url);
    let texts = chain_texts(error);
    let has = |needle: &str| texts.iter().any(|t| t.contains(needle));
    let via = attempt
        .proxy
        .as_deref()
        .map(|p| format!(" par le proxy {p}"))
        .unwrap_or_default();
    let mut failure = NetworkFailure {
        cause: FailureCause::Other,
        title: String::new(),
        explanation: String::new(),
        not_sent: true,
        host: host.clone(),
        port,
        proxy: attempt.proxy.clone(),
        connect_timeout_ms: attempt.connect_timeout_ms,
        elapsed_ms: attempt.elapsed_ms,
        certificate: None,
    };
    let mut kind = CommandErrorKind::Network;
    if let Some(certificate) = attempt.rejection.as_ref().and_then(certificate_failure) {
        failure.cause = FailureCause::Certificate;
        let issuer = certificate.server.issuer.clone();
        match certificate.problem {
            CertificateProblem::UnknownIssuer => {
                failure.title = format!("Le certificat de {host} n'est pas reconnu");
                failure.explanation = format!(
                    "Il est signé par « {issuer} », une autorité que ni le système ni Arnod ne connaissent. {NOT_SENT}"
                );
            }
            CertificateProblem::Expired => {
                failure.title = format!("Le certificat de {host} a expiré");
                failure.explanation = format!(
                    "Il était valide du {} au {}. {NOT_SENT}",
                    certificate.server.not_before, certificate.server.not_after
                );
            }
            CertificateProblem::NotYetValid => {
                failure.title = format!("Le certificat de {host} n'est pas encore valide");
                failure.explanation = format!(
                    "Il sera valide à partir du {}. Vérifiez aussi l'heure du poste. {NOT_SENT}",
                    certificate.server.not_before
                );
            }
            CertificateProblem::WrongName => {
                failure.title = format!("Le certificat présenté n'est pas celui de {host}");
                let names = if certificate.server.names.is_empty() {
                    certificate.server.subject.clone()
                } else {
                    certificate.server.names.join(", ")
                };
                failure.explanation = format!("Il couvre seulement : {names}. {NOT_SENT}");
            }
            CertificateProblem::Revoked => {
                failure.title = format!("Le certificat de {host} est révoqué");
                failure.explanation = format!("Son autorité « {issuer} » l'a retiré. {NOT_SENT}");
            }
            CertificateProblem::Other => {
                failure.title = format!("Le certificat de {host} est refusé");
                failure.explanation = format!("{}. {NOT_SENT}", root_cause(error));
            }
        }
        failure.certificate = Some(certificate);
    } else if has("proxy auth") || has("407") {
        failure.cause = FailureCause::ProxyAuth;
        failure.title = "Le proxy demande une identification (407)".to_owned();
        failure.explanation = format!(
            "Le proxy{} refuse la connexion sans identifiant. {NOT_SENT}",
            attempt
                .proxy
                .as_deref()
                .map(|p| format!(" {p}"))
                .unwrap_or_default()
        );
    } else if has("unsuccessful tunnel") || (has("tunnel") && attempt.proxy.is_some()) {
        failure.cause = FailureCause::ProxyAuth;
        failure.title = format!("Le proxy refuse d'ouvrir la connexion vers {host}");
        failure.explanation = format!(
            "Le proxy{} a répondu par un refus ({}). {NOT_SENT}",
            attempt
                .proxy
                .as_deref()
                .map(|p| format!(" {p}"))
                .unwrap_or_default(),
            root_cause(error)
        );
    } else if error.is_connect() && attempt.proxy.is_some() {
        failure.cause = FailureCause::Proxy;
        let proxy = attempt.proxy.clone().unwrap_or_default();
        failure.title = format!("Le proxy {proxy} est injoignable");
        failure.explanation = format!(
            "Arnod n'a pas pu s'y connecter ({}). {NOT_SENT}",
            root_cause(error)
        );
    } else if has("dns error") || has("failed to lookup") || has("name or service not known") {
        failure.cause = FailureCause::Dns;
        failure.title = format!("Le nom {host} est introuvable");
        failure.explanation = format!(
            "Le DNS du poste ne connaît pas ce nom{via}. Vérifiez l'adresse ou l'environnement. {NOT_SENT}"
        );
    } else if error.is_timeout() && error.is_connect() {
        failure.cause = FailureCause::Unreachable;
        kind = CommandErrorKind::Timeout;
        failure.title = format!("Le serveur {host}:{port} ne répond pas");
        failure.explanation = format!(
            "La connexion n'a pas abouti en {}. {NOT_SENT}",
            seconds(attempt.connect_timeout_ms)
        );
    } else if error.is_timeout() {
        failure.cause = FailureCause::Timeout;
        kind = CommandErrorKind::Timeout;
        failure.not_sent = false;
        failure.title = "Le délai est dépassé sans réponse".to_owned();
        failure.explanation = format!(
            "La requête est partie vers {host} mais la réponse n'est pas arrivée en {} : elle a pu être traitée par le serveur.",
            seconds(attempt.elapsed_ms)
        );
    } else if has("tls:") {
        failure.cause = FailureCause::Tls;
        failure.title = format!("La connexion sécurisée avec {host} a échoué");
        failure.explanation = format!("{}. {NOT_SENT}", root_cause(error));
    } else if error.is_connect() {
        failure.cause = FailureCause::Unreachable;
        failure.title = format!("Le serveur {host}:{port} est injoignable");
        failure.explanation = format!(
            "La connexion a été refusée ({}). Vérifiez l'adresse, le port et que le serveur est démarré. {NOT_SENT}",
            root_cause(error)
        );
    } else if error.is_builder() {
        return CommandError::new(
            CommandErrorKind::InvalidRequest,
            "L'adresse de la requête est invalide.",
        );
    } else if error.is_redirect() {
        return CommandError::new(
            CommandErrorKind::Network,
            "Trop de redirections : augmentez la limite dans les réglages de la requête.",
        );
    } else {
        failure.not_sent = false;
        failure.title = format!("La requête a échoué : {}", root_cause(error));
        failure.explanation = String::new();
    }
    let message = if failure.explanation.is_empty() {
        format!("{}.", failure.title)
    } else {
        format!("{}. {}", failure.title, failure.explanation)
    };
    CommandError::new(kind, message).with_failure(failure)
}

/// Le proxy a répondu 407 à une requête http (sans tunnel) : même échec que pour https.
pub fn proxy_auth_response(
    url: &str,
    proxy: &str,
    connect_timeout_ms: u64,
    elapsed_ms: u64,
) -> CommandError {
    let (host, port) = host_port(url);
    let failure = NetworkFailure {
        cause: FailureCause::ProxyAuth,
        title: "Le proxy demande une identification (407)".to_owned(),
        explanation: format!("Le proxy {proxy} refuse la connexion sans identifiant. {NOT_SENT}"),
        not_sent: true,
        host,
        port,
        proxy: Some(proxy.to_owned()),
        connect_timeout_ms,
        elapsed_ms,
        certificate: None,
    };
    CommandError::new(
        CommandErrorKind::Network,
        format!("{}. {}", failure.title, failure.explanation),
    )
    .with_failure(failure)
}
