//! Certificats de test (ECDSA P-256), construits en DER avec `ring` déjà présent : autorité,
//! certificat de serveur, expiré ou pour un autre nom, auto-signé. Aucune clé n'est versionnée.

use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};

/// Clé de test et son PKCS#8.
pub struct Key {
    pub pair: EcdsaKeyPair,
    pub pkcs8: Vec<u8>,
}

impl Key {
    pub fn generate() -> Self {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .unwrap()
            .as_ref()
            .to_vec();
        let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &pkcs8, &rng).unwrap();
        Self { pair, pkcs8 }
    }
}

/// Description d'un certificat à produire.
pub struct Spec<'a> {
    pub subject: &'a str,
    pub organization: Option<&'a str>,
    pub dns: Vec<&'a str>,
    pub authority: bool,
    /// `AAAAMMJJ`.
    pub not_before: &'a str,
    pub not_after: &'a str,
}

impl<'a> Spec<'a> {
    pub fn authority(subject: &'a str) -> Self {
        Self {
            subject,
            organization: None,
            dns: Vec::new(),
            authority: true,
            not_before: "20260102",
            not_after: "20460102",
        }
    }

    pub fn server(name: &'a str) -> Self {
        Self {
            subject: name,
            organization: None,
            dns: vec![name],
            authority: false,
            not_before: "20260102",
            not_after: "20460102",
        }
    }
}

fn len(n: usize) -> Vec<u8> {
    if n < 0x80 {
        vec![u8::try_from(n).unwrap()]
    } else {
        let bytes: Vec<u8> = n
            .to_be_bytes()
            .into_iter()
            .skip_while(|b| *b == 0)
            .collect();
        let mut out = vec![0x80 | u8::try_from(bytes.len()).unwrap()];
        out.extend(bytes);
        out
    }
}

fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(len(content.len()));
    out.extend_from_slice(content);
    out
}

fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    tlv(0x30, &parts.concat())
}

fn oid(bytes: &[u8]) -> Vec<u8> {
    tlv(0x06, bytes)
}

const ECDSA_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
const EC_PUBLIC_KEY: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const P256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];

fn name(spec: &Spec<'_>) -> Vec<u8> {
    let attribute =
        |id: &[u8], value: &str| tlv(0x31, &seq(&[oid(id), tlv(0x0c, value.as_bytes())]));
    let mut rdns = Vec::new();
    if let Some(org) = spec.organization {
        rdns.push(attribute(&[0x55, 0x04, 0x0a], org));
    }
    rdns.push(attribute(&[0x55, 0x04, 0x03], spec.subject));
    seq(&rdns)
}

/// `AAAAMMJJ` en UTCTime (avant 2050) ou GeneralizedTime.
fn time(date: &str) -> Vec<u8> {
    let year: u32 = date[..4].parse().unwrap();
    if year < 2050 {
        tlv(0x17, format!("{}000000Z", &date[2..]).as_bytes())
    } else {
        tlv(0x18, format!("{date}000000Z").as_bytes())
    }
}

/// Certificat DER de `spec`, signé par `issuer` (sujet et clé), ou par lui-même.
pub fn certificate(spec: &Spec<'_>, key: &Key, issuer: Option<(&Spec<'_>, &Key)>) -> Vec<u8> {
    let algorithm = seq(&[oid(ECDSA_SHA256)]);
    let mut public = vec![0u8];
    public.extend_from_slice(key.pair.public_key().as_ref());
    let spki = seq(&[seq(&[oid(EC_PUBLIC_KEY), oid(P256)]), tlv(0x03, &public)]);
    let mut extensions = Vec::new();
    if spec.authority {
        let value = seq(&[tlv(0x01, &[0xff])]);
        extensions.push(seq(&[
            oid(&[0x55, 0x1d, 0x13]),
            tlv(0x01, &[0xff]),
            tlv(0x04, &value),
        ]));
    }
    if !spec.dns.is_empty() {
        let names: Vec<Vec<u8>> = spec.dns.iter().map(|d| tlv(0x82, d.as_bytes())).collect();
        extensions.push(seq(&[oid(&[0x55, 0x1d, 0x11]), tlv(0x04, &seq(&names))]));
    }
    let (issuer_spec, signer) = issuer.unwrap_or((spec, key));
    let serial = {
        let mut bytes = [0u8; 8];
        ring::rand::SecureRandom::fill(&SystemRandom::new(), &mut bytes).unwrap();
        bytes[0] &= 0x7f;
        bytes[0] |= 0x01;
        tlv(0x02, &bytes)
    };
    let tbs = seq(&[
        tlv(0xa0, &tlv(0x02, &[2])),
        serial,
        algorithm.clone(),
        name(issuer_spec),
        seq(&[time(spec.not_before), time(spec.not_after)]),
        name(spec),
        spki,
        tlv(0xa3, &seq(&extensions)),
    ]);
    let signature = signer.pair.sign(&SystemRandom::new(), &tbs).unwrap();
    let mut bits = vec![0u8];
    bits.extend_from_slice(signature.as_ref());
    seq(&[tbs, algorithm, tlv(0x03, &bits)])
}
