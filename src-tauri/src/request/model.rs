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
    /// Description de la ligne (EP2, EP4).
    #[serde(default)]
    pub description: String,
}

impl Pair {
    /// Ligne active.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled: true,
            kind: ParamKind::Query,
            description: String::new(),
        }
    }

    /// Paramètre de chemin actif.
    pub fn path(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: ParamKind::Path,
            ..Self::new(name, value)
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
    /// Lot H : OAuth 2 par identifiants client (`docs/plan-mvp-h-reseau-auth.md`, R8).
    #[serde(rename = "oauth2")]
    OAuth2,
}

/// Où le client présente ses identifiants au serveur de jetons (OAuth 2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialsPlacement {
    /// En-tête `Authorization: Basic`.
    #[default]
    BasicAuthHeader,
    /// Champs `client_id` et `client_secret` du formulaire.
    Body,
}

/// Où le jeton OAuth 2 part avec la requête (R11).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenPlacement {
    #[default]
    Header,
    Query,
}

/// OAuth 2 par identifiants client (flux `client_credentials` du schéma OpenCollection, R8) ;
/// les valeurs acceptent `{{variable}}`, le secret n'est jamais écrit en clair (R10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2 {
    #[serde(default)]
    pub access_token_url: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub credentials: CredentialsPlacement,
    #[serde(default)]
    pub token_placement: TokenPlacement,
    /// Nom de l'en-tête (`Authorization`) ou du paramètre (`access_token`).
    #[serde(default = "default_token_name")]
    pub token_name: String,
}

fn default_token_name() -> String {
    "Authorization".to_owned()
}

impl Default for OAuth2 {
    fn default() -> Self {
        Self {
            access_token_url: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            scope: String::new(),
            credentials: CredentialsPlacement::default(),
            token_placement: TokenPlacement::default(),
            token_name: default_token_name(),
        }
    }
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
    /// Lot H : réglages OAuth 2 (`type: oauth2`).
    #[serde(default)]
    pub oauth2: OAuth2,
}

/// Type de corps (B4) ; commun à toutes les variantes (Q3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyKind {
    #[default]
    None,
    Json,
    Text,
    Xml,
    Form,
    /// Autre corps OpenCollection (multipart, fichier…) : conservé, non envoyé.
    Other,
}

/// Variante d'un corps (EP3) : un titre vide pour un corps simple.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyVariant {
    #[serde(default)]
    pub title: String,
    /// Contenu JSON, XML ou texte.
    #[serde(default)]
    pub content: String,
    /// Champs d'un formulaire.
    #[serde(default)]
    pub form: Vec<Pair>,
}

/// Corps de la requête : une ou plusieurs variantes, la sélectionnée part (R2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    #[serde(rename = "type", default)]
    pub kind: BodyKind,
    #[serde(default)]
    pub variants: Vec<BodyVariant>,
    /// Index de la variante sélectionnée.
    #[serde(default)]
    pub selected: usize,
    /// Type d'un corps `other` (`multipart-form`, `file`…).
    #[serde(default)]
    pub other: String,
}

impl Body {
    /// Corps simple JSON, XML ou texte.
    pub fn raw(kind: BodyKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            variants: vec![BodyVariant {
                content: content.into(),
                ..BodyVariant::default()
            }],
            ..Self::default()
        }
    }

    /// Formulaire simple.
    pub fn form(fields: Vec<Pair>) -> Self {
        Self {
            kind: BodyKind::Form,
            variants: vec![BodyVariant {
                form: fields,
                ..BodyVariant::default()
            }],
            ..Self::default()
        }
    }

    /// Variante qui part à l'envoi (R2).
    pub fn current(&self) -> Option<&BodyVariant> {
        self.variants
            .get(self.selected)
            .or_else(|| self.variants.first())
    }

    /// Contenu de la variante sélectionnée.
    pub fn content(&self) -> &str {
        self.current().map_or("", |v| v.content.as_str())
    }

    /// Champs de la variante sélectionnée.
    pub fn fields(&self) -> &[Pair] {
        self.current().map_or(&[], |v| v.form.as_slice())
    }
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

/// Extraction (`docs/plan-mvp-e-tests-etapes.md`, R7) : une valeur de la
/// réponse, ou de la requête envoyée, rangée dans une variable d'exécution.
/// Écrite en action OpenCollection `set-variable` (R2 du plan d'ensemble).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extraction {
    /// Nom de la variable (`client.id`).
    pub name: String,
    /// `body.id`, `headers.Location`, `status`, ou `request.body.ref` pour
    /// une valeur envoyée ; une autre expression est gardée telle quelle.
    pub source: String,
    /// Une extraction désactivée n'est pas appliquée.
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

/// Nombre de redirections suivies au plus, par défaut (R3).
pub const DEFAULT_MAX_REDIRECTS: u32 = 5;

/// Réglages d'envoi propres à la requête (EP7) ; `None` : hérité (R3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Délai en millisecondes ; 0 : aucun délai (R5).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub follow_redirects: Option<bool>,
    #[serde(default)]
    pub max_redirects: Option<u32>,
    #[serde(default)]
    pub encode_url: Option<bool>,
}

/// Réglages effectifs, après héritage (R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSettings {
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub encode_url: bool,
}

impl Default for ResolvedSettings {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            follow_redirects: true,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            encode_url: true,
        }
    }
}

impl Settings {
    /// Réglages effectifs : les siens, sinon ceux de `inherited`.
    pub fn resolve(&self, inherited: &ResolvedSettings) -> ResolvedSettings {
        ResolvedSettings {
            timeout_ms: self.timeout_ms.unwrap_or(inherited.timeout_ms),
            follow_redirects: self.follow_redirects.unwrap_or(inherited.follow_redirects),
            max_redirects: self.max_redirects.unwrap_or(inherited.max_redirects),
            encode_url: self.encode_url.unwrap_or(inherited.encode_url),
        }
    }
}

/// Type du corps d'une réponse attendue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExampleBodyKind {
    /// Pas de corps.
    #[default]
    None,
    Json,
    Text,
    Xml,
    Html,
    Binary,
}

/// En-tête d'une réponse attendue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseHeader {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

/// Réponse attendue (EP8) : un exemple OpenCollection complet (R6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub headers: Vec<ResponseHeader>,
    #[serde(default)]
    pub body_kind: ExampleBodyKind,
    #[serde(default)]
    pub body: String,
}

/// Authentification reprise d'un dossier ou de la collection (R4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InheritedAuth {
    pub auth: Auth,
    /// « dossier « Animalerie » » ou « collection ».
    pub source: String,
}

/// En-tête ajouté par un dossier ou la collection (R4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InheritedHeader {
    pub name: String,
    pub value: String,
    pub source: String,
}

/// Ce qu'une requête hérite de ses dossiers et de la collection : calculé à
/// la lecture pour l'éditeur, jamais écrit (R3, R4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inherited {
    #[serde(default)]
    pub auth: Option<InheritedAuth>,
    #[serde(default)]
    pub headers: Vec<InheritedHeader>,
    #[serde(default)]
    pub settings: ResolvedSettings,
}

/// Requête éditée : un endpoint (docs/plan-configuration-endpoint.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub name: String,
    /// Description courte (`info.description`, EP1).
    #[serde(default)]
    pub description: String,
    /// Étiquettes, sans celle du lien au contrat (EP1, R9).
    #[serde(default)]
    pub tags: Vec<String>,
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
    #[serde(default)]
    pub settings: Settings,
    /// Documentation en Markdown (EP6).
    #[serde(default)]
    pub docs: String,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// Extractions, appliquées après la réponse (R2 du plan d'ensemble).
    #[serde(default)]
    pub extractions: Vec<Extraction>,
    /// Rang de l'étape dans son dossier (`info.seq`), s'il est écrit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<f64>,
    /// Réponses attendues (EP8).
    #[serde(default)]
    pub examples: Vec<Example>,
    /// Opération du contrat liée (étiquette `contract:` du fichier, S7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<ContractLink>,
    /// Parties du fichier conservées sans être gérées par Arnod (scripts,
    /// variables…), pour le signaler dans l'éditeur. Ignoré à l'enregistrement.
    #[serde(default)]
    pub preserved: Vec<String>,
    /// Héritage des dossiers et de la collection, pour l'éditeur. Ignoré à
    /// l'enregistrement.
    #[serde(default)]
    pub inherited: Inherited,
}

impl Request {
    /// Requête neuve : `GET {{baseUrl}}/` avec la vérification de durée par défaut (B1, V5).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            tags: Vec::new(),
            method: Method::Get,
            url: "{{baseUrl}}/".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: Auth::default(),
            body: Body::default(),
            settings: Settings::default(),
            docs: String::new(),
            assertions: vec![Assertion {
                id: "duree".to_owned(),
                target: "responseTime".to_owned(),
                operator: Operator::LessThan,
                value: Some(Scalar::Int(DEFAULT_MAX_DURATION_MS)),
                origin: Origin::Default,
                enabled: true,
            }],
            extractions: Vec::new(),
            seq: None,
            examples: Vec::new(),
            contract: None,
            preserved: Vec::new(),
            inherited: Inherited::default(),
        }
    }
}

/// Noms des paramètres de chemin d'une adresse : segments `:nom` ou `{nom}`,
/// dans l'ordre, sans doublon (R1). `{{variable}}` n'en est pas un.
pub fn path_param_names(url: &str) -> Vec<String> {
    // Seule la partie chemin compte : ni la requête (`?…`), ni le fragment.
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    for segment in path.split('/') {
        let name = if let Some(rest) = segment.strip_prefix(':') {
            rest
        } else if let Some(rest) = segment
            .strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .filter(|r| !r.starts_with('{'))
        {
            rest
        } else {
            continue;
        };
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
        if valid && !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    names
}

/// Aligne les paramètres de chemin sur l'adresse (R1) : ceux qui n'y figurent
/// plus sont retirés, ceux qui manquent ajoutés vides, dans l'ordre de l'adresse.
pub fn normalize_path_params(request: &mut Request) {
    let names = path_param_names(&request.url);
    let (path, query): (Vec<Pair>, Vec<Pair>) = std::mem::take(&mut request.params)
        .into_iter()
        .partition(|p| p.kind == ParamKind::Path);
    let mut params: Vec<Pair> = names
        .iter()
        .map(|name| {
            path.iter()
                .find(|p| &p.name == name)
                .cloned()
                .unwrap_or_else(|| Pair::path(name.clone(), ""))
        })
        .collect();
    params.extend(query);
    request.params = params;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1_noms_des_parametres_de_chemin() {
        assert_eq!(
            path_param_names("{{baseUrl}}/orders/:id/lines/{lineId}?x=:y#:z"),
            ["id", "lineId"]
        );
        assert_eq!(path_param_names("{{baseUrl}}/a/{{b}}/:id/:id"), ["id"]);
        assert!(path_param_names("https://h/a:b/:").is_empty());
    }

    #[test]
    fn r1_parametres_de_chemin_alignes_sur_l_adresse() {
        let mut request = Request::new("x");
        request.url = "{{baseUrl}}/pets/:petId/{photo}".to_owned();
        request.params = vec![
            Pair::new("page", "1"),
            Pair::path("ancien", "a"),
            Pair::path("photo", "p1"),
        ];
        normalize_path_params(&mut request);
        let summary: Vec<(&str, &str, ParamKind)> = request
            .params
            .iter()
            .map(|p| (p.name.as_str(), p.value.as_str(), p.kind))
            .collect();
        assert_eq!(
            summary,
            [
                ("petId", "", ParamKind::Path),
                ("photo", "p1", ParamKind::Path),
                ("page", "1", ParamKind::Query),
            ]
        );
    }
}
