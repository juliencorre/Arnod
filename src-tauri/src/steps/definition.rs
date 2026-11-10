//! Format d'une définition de nature d'étape (`docs/etude-catalogue-etapes.md`, § 2)
//! et sa validation (R1 de `docs/plan-accueil-non-classes.md`).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Version du format de définition lue par cette version d'Arnod.
pub const FORMAT: u32 = 1;

/// Moteurs intégrés connus de ce cœur : identifiant et version d'interface.
const BUILTIN_ENGINES: [(&str, u32); 1] = [("http", 1)];

/// Famille d'une nature : fermée, elle donne la couleur de ses pastilles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Family {
    Calls,
    Data,
    Messages,
    Jobs,
    Interface,
    Code,
    Files,
}

impl Family {
    pub const ALL: [Self; 7] = [
        Self::Calls,
        Self::Data,
        Self::Messages,
        Self::Jobs,
        Self::Interface,
        Self::Code,
        Self::Files,
    ];
}

/// Moteur qui exécute la nature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Engine {
    /// Moteur du cœur Rust, mis à jour avec Arnod.
    Builtin { id: String, abi: u32 },
    /// Module WebAssembly isolé, livré avec la définition (pas encore pris en charge).
    Wasm {
        module: String,
        sha256: String,
        abi: u32,
    },
}

/// Champ principal, saisi dans l'entrée de l'accueil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: String,
}

/// Règle de reconnaissance de ce que l'on colle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Rule {
    /// Adresse d'un des schémas, éventuellement précédée d'une option du
    /// champ `method_field` (`POST https://…`).
    #[serde(rename_all = "camelCase")]
    Url {
        schemes: Vec<String>,
        #[serde(default)]
        method_field: Option<String>,
    },
    /// Premier mot parmi `words`, sans tenir compte de la casse (`SELECT`).
    Keywords { words: Vec<String> },
    /// Texte qui commence par `text` (`kafka://`).
    Prefix { text: String },
    /// Texte qui finit par `text` (`?wsdl`).
    Suffix { text: String },
}

/// Nature d'un champ du formulaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldKind {
    /// Une option parmi `options`.
    Choice,
    /// Une ligne de texte.
    Text,
    /// Plusieurs lignes (corps, requête).
    Multiline,
}

/// Condition d'affichage : le champ `key` vaut l'une des valeurs `values`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleWhen {
    pub key: String,
    #[serde(rename = "in")]
    pub values: Vec<String>,
}

/// Champ déclaratif du formulaire d'une nature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub visible_when: Option<VisibleWhen>,
}

/// Définition d'une nature d'étape : des données, sans code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub format: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub family: Family,
    pub publisher: String,
    pub engine: Engine,
    /// Verbe du bouton : « Envoyer », « Exécuter »…
    pub action: String,
    pub subject: Subject,
    #[serde(default)]
    pub detect: Vec<Rule>,
    #[serde(default)]
    pub form: Vec<Field>,
}

impl Definition {
    pub fn field(&self, key: &str) -> Option<&Field> {
        self.form.iter().find(|field| field.key == key)
    }
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_field_key(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && text.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Version sémantique `x.y.z`, chiffres seulement.
pub fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Lit et valide une définition (R1). L'erreur dit pourquoi elle n'est pas proposée.
pub fn parse(text: &str) -> Result<Definition, String> {
    let definition: Definition =
        serde_json::from_str(text).map_err(|error| format!("définition illisible : {error}"))?;
    validate(&definition)?;
    Ok(definition)
}

/// Règles de validité d'une définition (R1).
pub fn validate(definition: &Definition) -> Result<(), String> {
    if definition.format != FORMAT {
        return Err(format!(
            "format {} non pris en charge (format {FORMAT} attendu)",
            definition.format
        ));
    }
    if !is_identifier(&definition.id) {
        return Err(format!("identifiant « {} » invalide", definition.id));
    }
    if !is_version(&definition.version) {
        return Err(format!("version « {} » invalide", definition.version));
    }
    if definition.name.trim().is_empty() || definition.action.trim().is_empty() {
        return Err("nom ou verbe de l'action vide".to_owned());
    }
    match &definition.engine {
        Engine::Builtin { id, abi } => {
            if !BUILTIN_ENGINES.contains(&(id.as_str(), *abi)) {
                return Err(format!("moteur intégré « {id} » (version {abi}) inconnu"));
            }
        }
        Engine::Wasm { .. } => {
            return Err("moteur WebAssembly pas encore pris en charge".to_owned());
        }
    }
    let mut keys = BTreeSet::new();
    if !is_field_key(&definition.subject.key) {
        return Err(format!(
            "champ principal « {} » invalide",
            definition.subject.key
        ));
    }
    keys.insert(definition.subject.key.as_str());
    for (index, field) in definition.form.iter().enumerate() {
        if !is_field_key(&field.key) || !keys.insert(field.key.as_str()) {
            return Err(format!("champ « {} » invalide ou en double", field.key));
        }
        if field.kind == FieldKind::Choice {
            if field.options.is_empty() {
                return Err(format!("le choix « {} » n'a pas d'option", field.key));
            }
            if let Some(default) = &field.default
                && !field.options.contains(default)
            {
                return Err(format!(
                    "la valeur par défaut de « {} » n'est pas une option",
                    field.key
                ));
            }
        }
        if let Some(condition) = &field.visible_when {
            let target = definition.form[..index]
                .iter()
                .find(|f| f.key == condition.key && f.kind == FieldKind::Choice);
            let Some(target) = target else {
                return Err(format!(
                    "« {} » dépend de « {} », qui n'est pas un choix placé avant lui",
                    field.key, condition.key
                ));
            };
            if condition.values.iter().any(|v| !target.options.contains(v)) {
                return Err(format!(
                    "« {} » dépend d'une valeur inconnue de « {} »",
                    field.key, condition.key
                ));
            }
        }
    }
    for rule in &definition.detect {
        match rule {
            Rule::Url {
                schemes,
                method_field,
            } => {
                if schemes.is_empty() {
                    return Err("règle « url » sans schéma".to_owned());
                }
                if let Some(key) = method_field
                    && definition
                        .field(key)
                        .is_none_or(|f| f.kind != FieldKind::Choice)
                {
                    return Err(format!("règle « url » : « {key} » n'est pas un choix"));
                }
            }
            Rule::Keywords { words } if words.is_empty() => {
                return Err("règle « keywords » sans mot".to_owned());
            }
            Rule::Prefix { text } | Rule::Suffix { text } if text.trim().is_empty() => {
                return Err("règle de préfixe ou de suffixe vide".to_owned());
            }
            _ => {}
        }
    }
    Ok(())
}
