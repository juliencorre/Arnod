//! Modèle d'une requête et format de son fichier YAML (§ 6.1 du plan).
//!
//! Un fichier = une requête, lisible et stable pour Git. Les champs vides sont
//! omis à l'écriture pour garder des diffs courts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Version courante du format d'un fichier de requête.
pub const REQUEST_VERSION: u32 = 1;

/// Délai maximal d'une requête, par défaut (R12).
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Seuil par défaut de la vérification de durée (V5), en millisecondes.
pub const DEFAULT_MAX_DURATION_MS: i64 = 500;

/// Méthode HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    /// Nom HTTP de la méthode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    /// Méthode qui ne modifie rien côté serveur (R8).
    pub fn is_safe(self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Options)
    }
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

/// Ligne nom / valeur : paramètre, en-tête, champ de formulaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    pub name: String,
    #[serde(default)]
    pub value: String,
    /// Case d'activation du tableau (B3).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

/// Type d'authentification (B5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    #[default]
    None,
    Basic,
    Bearer,
    ApiKey,
}

/// Emplacement d'une clé d'API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
}

fn no_location(value: &ApiKeyLocation) -> bool {
    *value == ApiKeyLocation::Header
}

fn is_empty(value: &str) -> bool {
    value.is_empty()
}

/// Authentification : les valeurs acceptent `{{variable}}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    #[serde(rename = "type", default)]
    pub kind: AuthKind,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub username: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub token: String,
    /// Nom de l'en-tête ou du paramètre d'une clé d'API.
    #[serde(default, skip_serializing_if = "is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub value: String,
    #[serde(default, skip_serializing_if = "no_location")]
    pub location: ApiKeyLocation,
}

/// Type de corps (B4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyKind {
    #[default]
    None,
    Json,
    Text,
    Form,
}

/// Corps de la requête.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    #[serde(rename = "type", default)]
    pub kind: BodyKind,
    /// Contenu JSON ou texte.
    #[serde(default, skip_serializing_if = "is_empty")]
    pub content: String,
    /// Champs d'un formulaire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form: Vec<Pair>,
}

/// Valeur attendue d'une vérification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// Opérateur d'une vérification (V2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
    Equals,
    NotEquals,
    Exists,
    Contains,
    LessThan,
    GreaterThan,
    IsNumber,
    IsText,
    /// Statut prévu et corps conforme au schéma du contrat lié (C8).
    MatchesContract,
}

/// Origine d'une vérification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// Calculée d'après une réponse reçue (R10).
    Proposed,
    /// Ajoutée par l'utilisateur.
    Manual,
    /// Vérification de durée ajoutée à la création.
    Default,
}

/// Condition sur la réponse, sans code (§ 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assertion {
    pub id: String,
    /// `status`, `responseTime`, `body.chemin` ou `headers.Nom`.
    pub target: String,
    #[serde(rename = "op")]
    pub operator: Operator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Scalar>,
    pub origin: Origin,
}

/// Opération d'un contrat dont la requête est issue (C7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractLink {
    /// Fichier du contrat, relatif au projet (`contracts/petstore.yaml`).
    pub file: String,
    /// Clé de l'opération : `GET /pets/{petId}`.
    pub operation: String,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

fn is_default_timeout(value: &u64) -> bool {
    *value == DEFAULT_TIMEOUT_MS
}

/// Contenu d'un fichier de requête.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub version: u32,
    pub id: Uuid,
    pub name: String,
    pub method: Method,
    pub url: String,
    #[serde(default)]
    pub params: Vec<Pair>,
    #[serde(default)]
    pub headers: Vec<Pair>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub body: Body,
    #[serde(
        default = "default_timeout",
        skip_serializing_if = "is_default_timeout"
    )]
    pub timeout_ms: u64,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// Opération du contrat liée, le cas échéant (champ ignoré par une version
    /// d'Arnod qui ne le connaît pas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<ContractLink>,
}

impl Request {
    /// Requête neuve : `GET {{baseUrl}}/` avec la vérification de durée par défaut (B1, V5).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: REQUEST_VERSION,
            id: Uuid::new_v4(),
            name: name.into(),
            method: Method::Get,
            url: "{{baseUrl}}/".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: Auth::default(),
            body: Body::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            assertions: vec![Assertion {
                id: "duree".to_owned(),
                target: "responseTime".to_owned(),
                operator: Operator::LessThan,
                value: Some(Scalar::Int(DEFAULT_MAX_DURATION_MS)),
                origin: Origin::Default,
            }],
            contract: None,
        }
    }
}
