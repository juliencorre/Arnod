//! Modèle d'une requête, tel que l'interface l'édite.
//!
//! Un fichier = une requête, au format OpenCollection 1.0.0
//! (docs/plan-opencollection.md) : la correspondance avec ce modèle est dans
//! `crate::opencollection::request`. Ce modèle n'est plus écrit tel quel.

use serde::{Deserialize, Serialize};

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

/// Emplacement d'un paramètre : adresse (`?nom=valeur`) ou chemin (`:nom`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParamKind {
    #[default]
    Query,
    Path,
}

/// Ligne nom / valeur : paramètre, en-tête, champ de formulaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    pub name: String,
    #[serde(default)]
    pub value: String,
    /// Case d'activation du tableau (B3).
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Paramètre de chemin (S8) ; sans objet pour un en-tête ou un champ.
    #[serde(default)]
    pub kind: ParamKind,
}

impl Pair {
    /// Ligne active.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled: true,
            kind: ParamKind::Query,
        }
    }
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
    /// Héritée du dossier ou de la collection (non gérée par Arnod : rien n'est envoyé).
    Inherit,
    /// Autre authentification OpenCollection (digest, OAuth…) : conservée, non envoyée.
    Other,
}

/// Emplacement d'une clé d'API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
}

/// Authentification : les valeurs acceptent `{{variable}}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    #[serde(rename = "type", default)]
    pub kind: AuthKind,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: String,
    /// Nom de l'en-tête ou du paramètre d'une clé d'API.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub location: ApiKeyLocation,
    /// Type d'une authentification `other` (`digest`, `oauth2`…).
    #[serde(default)]
    pub other: String,
}

/// Type de corps (B4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyKind {
    #[default]
    None,
    Json,
    Text,
    Xml,
    Form,
    /// Autre corps OpenCollection (multipart, fichier, variantes…) : conservé, non envoyé.
    Other,
}

/// Corps de la requête.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    #[serde(rename = "type", default)]
    pub kind: BodyKind,
    /// Contenu JSON ou texte.
    #[serde(default)]
    pub content: String,
    /// Champs d'un formulaire.
    #[serde(default)]
    pub form: Vec<Pair>,
    /// Type d'un corps `other` (`multipart-form`, `file`…).
    #[serde(default)]
    pub other: String,
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

/// Opérateur d'une vérification (V2) : ceux d'OpenCollection et de Bruno (R4).
/// Le nom OpenCollection de chacun est donné par `crate::opencollection::request`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
    Equals,
    NotEquals,
    Exists,
    Contains,
    NotContains,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    In,
    NotIn,
    Between,
    StartsWith,
    EndsWith,
    Length,
    Matches,
    NotMatches,
    IsNull,
    IsUndefined,
    IsEmpty,
    IsNotEmpty,
    IsTruthy,
    IsFalsy,
    IsJson,
    IsNumber,
    IsText,
    IsBoolean,
    IsArray,
}

/// Origine d'une vérification, connue pendant la session seulement (non écrite).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// Calculée d'après une réponse reçue (R10).
    Proposed,
    /// Ajoutée par l'utilisateur, ou relue d'un fichier.
    #[default]
    Manual,
    /// Vérification de durée ajoutée à la création.
    Default,
}

/// Condition sur la réponse, sans code (§ 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assertion {
    /// Identifiant de session, recalculé à la lecture (non écrit).
    pub id: String,
    /// `status`, `responseTime`, `body.chemin`, `headers.Nom`, ou une autre
    /// expression OpenCollection gardée telle quelle.
    pub target: String,
    pub operator: Operator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Scalar>,
    #[serde(default)]
    pub origin: Origin,
    /// Une vérification désactivée n'est pas évaluée.
    #[serde(default = "yes")]
    pub enabled: bool,
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

/// Délai par défaut, quand le fichier n'en fixe pas.
pub fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// Requête éditée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
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
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// Opération du contrat liée (étiquette `contract:` du fichier, S7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<ContractLink>,
    /// Parties du fichier conservées sans être gérées par Arnod (scripts,
    /// exemples…), pour le signaler dans l'éditeur. Ignoré à l'enregistrement.
    #[serde(default)]
    pub preserved: Vec<String>,
}

impl Request {
    /// Requête neuve : `GET {{baseUrl}}/` avec la vérification de durée par défaut (B1, V5).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
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
                enabled: true,
            }],
            contract: None,
            preserved: Vec::new(),
        }
    }
}
