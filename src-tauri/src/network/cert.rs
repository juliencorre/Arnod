//! Lecture d'un certificat X.509 pour l'afficher (`docs/plan-mvp-h-reseau-auth.md`, R4) :
//! sujet, émetteur, validité, noms couverts et empreinte SHA-256.
//!
//! Module pur. Seuls les champs montrés sont lus, par un lecteur DER minimal : la vérification
//! elle-même reste celle de rustls (webpki).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandErrorKind};

/// Certificat montré à l'utilisateur, avec son PEM pour l'ajouter aux autorités.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    /// Nom commun du sujet, sinon son organisation, sinon le nom complet.
    pub subject: String,
    /// Nom commun de l'émetteur (même règle).
    pub issuer: String,
    /// Début de validité, `jj/mm/aaaa`.
    pub not_before: String,
    /// Fin de validité, `jj/mm/aaaa`.
    pub not_after: String,
    /// Empreinte SHA-256 en hexadécimal, octets séparés par « : ».
    pub fingerprint: String,
    /// Noms DNS et adresses couverts (extension `subjectAltName`).
    pub names: Vec<String>,
    /// Certificat d'autorité (`basicConstraints` cA).
    pub authority: bool,
    /// Signé par lui-même (sujet et émetteur identiques).
    pub self_signed: bool,
    /// Le certificat en PEM.
    pub pem: String,
}

fn invalid(message: &str) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Élément DER : étiquette, contenu.
struct Tlv<'a> {
    tag: u8,
    content: &'a [u8],
}

/// Lecteur DER séquentiel.
struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn peek_tag(&self) -> Option<u8> {
        self.data.first().copied()
    }

    fn next(&mut self) -> Option<Tlv<'a>> {
        let (&tag, rest) = self.data.split_first()?;
        let (&first, rest) = rest.split_first()?;
        let (len, rest) = if first < 0x80 {
            (usize::from(first), rest)
        } else {
            let count = usize::from(first & 0x7f);
            if count == 0 || count > 4 || rest.len() < count {
                return None;
            }
            let len = rest[..count]
                .iter()
                .fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
            (len, &rest[count..])
        };
        if rest.len() < len {
            return None;
        }
        let (content, rest) = rest.split_at(len);
        self.data = rest;
        Some(Tlv { tag, content })
    }

    fn expect(&mut self, tag: u8) -> Option<Tlv<'a>> {
        let tlv = self.next()?;
        (tlv.tag == tag).then_some(tlv)
    }
}

const SEQUENCE: u8 = 0x30;
const SET: u8 = 0x31;
const OID: u8 = 0x06;
const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
const OID_O: &[u8] = &[0x55, 0x04, 0x0a];
const OID_SAN: &[u8] = &[0x55, 0x1d, 0x11];
const OID_BASIC: &[u8] = &[0x55, 0x1d, 0x13];

/// Nom lisible d'un `Name` : CN, sinon O, sinon les valeurs jointes.
fn name(content: &[u8]) -> String {
    let mut cn = None;
    let mut org = None;
    let mut all = Vec::new();
    let mut rdns = Reader::new(content);
    while let Some(set) = rdns.expect(SET) {
        let mut atvs = Reader::new(set.content);
        while let Some(atv) = atvs.expect(SEQUENCE) {
            let mut parts = Reader::new(atv.content);
            let (Some(oid), Some(value)) = (parts.expect(OID), parts.next()) else {
                continue;
            };
            let text = String::from_utf8_lossy(value.content).into_owned();
            if oid.content == OID_CN {
                cn = Some(text.clone());
            } else if oid.content == OID_O {
                org = Some(text.clone());
            }
            all.push(text);
        }
    }
    cn.or(org).unwrap_or_else(|| all.join(", "))
}

/// Date `jj/mm/aaaa` d'un `UTCTime` (0x17) ou `GeneralizedTime` (0x18).
fn date(tlv: &Tlv<'_>) -> Option<String> {
    let text = std::str::from_utf8(tlv.content).ok()?;
    let (year, rest) = match tlv.tag {
        0x17 => {
            let yy: u32 = text.get(..2)?.parse().ok()?;
            (if yy >= 50 { 1900 + yy } else { 2000 + yy }, text.get(2..)?)
        }
        0x18 => (text.get(..4)?.parse().ok()?, text.get(4..)?),
        _ => return None,
    };
    Some(format!("{}/{}/{year:04}", rest.get(2..4)?, rest.get(..2)?))
}

/// Noms d'un `subjectAltName` : `dNSName` (0x82) et `iPAddress` (0x87).
fn alt_names(value: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut outer = Reader::new(value);
    let Some(seq) = outer.expect(SEQUENCE) else {
        return names;
    };
    let mut items = Reader::new(seq.content);
    while let Some(item) = items.next() {
        match item.tag {
            0x82 => names.push(String::from_utf8_lossy(item.content).into_owned()),
            0x87 if item.content.len() == 4 => names.push(
                item.content
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            _ => {}
        }
    }
    names
}

/// `basicConstraints` : `cA` vrai.
fn is_ca(value: &[u8]) -> bool {
    let mut outer = Reader::new(value);
    let Some(seq) = outer.expect(SEQUENCE) else {
        return false;
    };
    let mut fields = Reader::new(seq.content);
    fields
        .expect(0x01)
        .is_some_and(|b| b.content.first().is_some_and(|v| *v != 0))
}

/// Empreinte SHA-256, `AA:BB:…`.
pub fn fingerprint(der: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, der)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Certificat en PEM.
pub fn to_pem(der: &[u8]) -> String {
    let encoded = STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(chunk));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

/// Lit un certificat DER.
pub fn parse_der(der: &[u8]) -> Result<CertificateInfo, CommandError> {
    let unreadable = || invalid("Ce fichier ne contient pas de certificat lisible.");
    let mut top = Reader::new(der);
    let cert = top.expect(SEQUENCE).ok_or_else(unreadable)?;
    let mut parts = Reader::new(cert.content);
    let tbs = parts.expect(SEQUENCE).ok_or_else(unreadable)?;
    let mut fields = Reader::new(tbs.content);
    if fields.peek_tag() == Some(0xa0) {
        fields.next();
    }
    fields.expect(0x02).ok_or_else(unreadable)?; // numéro de série
    fields.expect(SEQUENCE).ok_or_else(unreadable)?; // algorithme
    let issuer = fields.expect(SEQUENCE).ok_or_else(unreadable)?;
    let validity = fields.expect(SEQUENCE).ok_or_else(unreadable)?;
    let subject = fields.expect(SEQUENCE).ok_or_else(unreadable)?;
    fields.expect(SEQUENCE).ok_or_else(unreadable)?; // clé publique
    let mut names = Vec::new();
    let mut authority = false;
    while !fields.is_empty() {
        let Some(field) = fields.next() else { break };
        if field.tag != 0xa3 {
            continue;
        }
        let mut wrapper = Reader::new(field.content);
        let Some(list) = wrapper.expect(SEQUENCE) else {
            continue;
        };
        let mut extensions = Reader::new(list.content);
        while let Some(extension) = extensions.expect(SEQUENCE) {
            let mut ext = Reader::new(extension.content);
            let Some(oid) = ext.expect(OID) else { continue };
            if ext.peek_tag() == Some(0x01) {
                ext.next();
            }
            let Some(value) = ext.expect(0x04) else {
                continue;
            };
            if oid.content == OID_SAN {
                names = alt_names(value.content);
            } else if oid.content == OID_BASIC {
                authority = is_ca(value.content);
            }
        }
    }
    let mut times = Reader::new(validity.content);
    let not_before = times.next().and_then(|t| date(&t)).unwrap_or_default();
    let not_after = times.next().and_then(|t| date(&t)).unwrap_or_default();
    Ok(CertificateInfo {
        subject: name(subject.content),
        issuer: name(issuer.content),
        not_before,
        not_after,
        fingerprint: fingerprint(der),
        names,
        authority,
        self_signed: issuer.content == subject.content,
        pem: to_pem(der),
    })
}

/// Certificats DER d'un texte PEM, dans l'ordre.
pub fn pem_certificates(text: &str) -> Vec<Vec<u8>> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("-----BEGIN CERTIFICATE-----") {
        let after = &rest[start + "-----BEGIN CERTIFICATE-----".len()..];
        let Some(end) = after.find("-----END CERTIFICATE-----") else {
            break;
        };
        let body: String = after[..end]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if let Ok(der) = STANDARD.decode(body) {
            found.push(der);
        }
        rest = &after[end..];
    }
    found
}

/// Lit le premier certificat d'un PEM (une autorité à ajouter, R4).
pub fn parse_pem(text: &str) -> Result<(Vec<u8>, CertificateInfo), CommandError> {
    let der = pem_certificates(text).into_iter().next().ok_or_else(|| {
        invalid("Ce fichier ne contient pas de certificat PEM (« BEGIN CERTIFICATE »).")
    })?;
    let info = parse_der(&der)?;
    Ok((der, info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::testcert::{Key, Spec, certificate};

    fn authority_spec() -> Spec<'static> {
        Spec {
            organization: Some("Boutique"),
            not_before: "20260102",
            not_after: "20270102",
            ..Spec::authority("Boutique Interne CA")
        }
    }

    #[test]
    fn r4_autorite_lue_sujet_validite_empreinte() {
        let key = Key::generate();
        let der = certificate(&authority_spec(), &key, None);
        let info = parse_der(&der).unwrap();
        assert_eq!(info.subject, "Boutique Interne CA");
        assert_eq!(info.issuer, "Boutique Interne CA");
        assert_eq!(info.not_before, "02/01/2026");
        assert_eq!(info.not_after, "02/01/2027");
        assert!(info.authority);
        assert!(info.self_signed);
        assert_eq!(info.fingerprint.len(), 32 * 3 - 1);
        assert_eq!(info.fingerprint, fingerprint(&der));
        // Le PEM produit se relit à l'identique.
        let (again_der, again) = parse_pem(&info.pem).unwrap();
        assert_eq!(again_der, der);
        assert_eq!(again, info);
    }

    #[test]
    fn certificat_de_serveur_noms_couverts_et_emetteur() {
        let ca_key = Key::generate();
        let ca = authority_spec();
        let key = Key::generate();
        let der = certificate(
            &Spec::server("recette.boutique.local"),
            &key,
            Some((&ca, &ca_key)),
        );
        let info = parse_der(&der).unwrap();
        assert_eq!(info.subject, "recette.boutique.local");
        assert_eq!(info.issuer, "Boutique Interne CA");
        assert_eq!(info.names, ["recette.boutique.local"]);
        assert!(!info.authority);
        assert!(!info.self_signed);
    }

    #[test]
    fn texte_qui_n_est_pas_un_certificat() {
        let error = parse_pem("bonjour").unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        let error = parse_pem("-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n")
            .unwrap_err();
        assert!(
            error.message.contains("certificat lisible"),
            "{}",
            error.message
        );
    }
}
