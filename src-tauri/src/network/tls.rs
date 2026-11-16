//! Configuration TLS des envois (`docs/plan-mvp-h-reseau-auth.md`, R4) : autorités du système
//! plus les autorités internes, et un vérificateur qui délègue tout à webpki mais garde le
//! certificat refusé, pour expliquer l'échec (5.1-i).
//!
//! La vérification n'est jamais affaiblie : `dangerous()` n'est que le nom de l'API rustls qui
//! permet de fournir son propre vérificateur ; celui-ci renvoie exactement le verdict de webpki.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};

/// Ce que le serveur a présenté de travers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CertificateProblem {
    /// Signé par une autorité inconnue du poste.
    UnknownIssuer,
    /// Expiré.
    Expired,
    /// Pas encore valide.
    NotYetValid,
    /// Délivré pour un autre nom.
    WrongName,
    /// Révoqué.
    Revoked,
    /// Autre refus (usage, signature…).
    Other,
}

/// Certificat refusé lors d'une poignée de main.
#[derive(Debug, Clone)]
pub struct Rejection {
    pub problem: CertificateProblem,
    /// Chaîne présentée par le serveur, certificat du serveur d'abord (DER).
    pub chain: Vec<Vec<u8>>,
    pub at: Instant,
}

/// Certificats refusés, par nom de serveur.
pub type Rejections = Arc<Mutex<HashMap<String, Rejection>>>;

fn problem_of(error: &rustls::Error) -> CertificateProblem {
    match error {
        rustls::Error::InvalidCertificate(cert) => match cert {
            CertificateError::UnknownIssuer => CertificateProblem::UnknownIssuer,
            CertificateError::Expired | CertificateError::ExpiredContext { .. } => {
                CertificateProblem::Expired
            }
            CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
                CertificateProblem::NotYetValid
            }
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
                CertificateProblem::WrongName
            }
            CertificateError::Revoked => CertificateProblem::Revoked,
            _ => CertificateProblem::Other,
        },
        _ => CertificateProblem::Other,
    }
}

/// Nom de serveur, clé des refus.
pub fn server_key(name: &ServerName<'_>) -> String {
    name.to_str().to_ascii_lowercase()
}

/// Vérificateur webpki qui garde le certificat refusé.
#[derive(Debug)]
struct Recording {
    inner: Arc<WebPkiServerVerifier>,
    rejections: Rejections,
}

impl ServerCertVerifier for Recording {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let verdict = self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        );
        if let Err(error) = &verdict
            && let Ok(mut all) = self.rejections.lock()
        {
            let chain = std::iter::once(end_entity)
                .chain(intermediates)
                .map(|c| c.as_ref().to_vec())
                .collect();
            all.insert(
                server_key(server_name),
                Rejection {
                    problem: problem_of(error),
                    chain,
                    at: Instant::now(),
                },
            );
        }
        verdict
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Autorités du système, lues une fois (le magasin Windows est lent à parcourir).
fn system_roots() -> &'static Vec<CertificateDer<'static>> {
    static ROOTS: OnceLock<Vec<CertificateDer<'static>>> = OnceLock::new();
    ROOTS.get_or_init(|| rustls_native_certs::load_native_certs().certs)
}

/// Nombre d'autorités du système reconnues (préférences, 1.6-e).
pub fn system_root_count() -> usize {
    let mut store = RootCertStore::empty();
    store.add_parsable_certificates(system_roots().iter().cloned());
    store.len()
}

/// Autorités des envois : celles du système (sinon les racines Mozilla), plus les internes.
pub fn root_store(with_system: bool, internal: &[Vec<u8>]) -> RootCertStore {
    let mut store = RootCertStore::empty();
    if with_system {
        store.add_parsable_certificates(system_roots().iter().cloned());
    }
    if store.is_empty() {
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    store.add_parsable_certificates(internal.iter().map(|der| CertificateDer::from(der.clone())));
    store
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(ring::default_provider())
}

/// Configuration rustls d'un client : `roots` vérifiées par webpki, refus gardés.
pub fn client_config(
    roots: RootCertStore,
    rejections: Rejections,
) -> Result<ClientConfig, CommandError> {
    let unavailable = |error: rustls::Error| {
        CommandError::new(
            CommandErrorKind::Network,
            format!("Configuration TLS impossible : {error}."),
        )
    };
    let provider = provider();
    let inner = WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .build()
        .map_err(|error| {
            CommandError::new(
                CommandErrorKind::Network,
                format!("Aucune autorité de certification utilisable : {error}."),
            )
        })?;
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(unavailable)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Recording { inner, rejections }))
        .with_no_client_auth();
    Ok(config)
}
