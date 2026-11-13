//! Chemins généralisés d'un contrat déduit (écran 2.2-c) : un segment qui ressemble à un
//! identifiant (`/orders/42/tracking`) devient un paramètre de chemin (`/orders/{id}/tracking`),
//! pour que l'opération serve toutes les valeurs. Proposé, jamais imposé : « Garder tel quel »
//! est retenu dans l'opération (`x-arnod-path-kept`).

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::contract::deduce;
use crate::contract::file as contract_file;
use crate::contract::parse;
use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::yaml;
use crate::request::file as request_file;
use crate::request::model::{ContractLink, Pair, ParamKind};
use crate::tree;

/// Extension d'une opération déduite dont l'utilisateur a gardé le chemin tel quel.
pub const KEPT_MARK: &str = "x-arnod-path-kept";

/// Paramètre de chemin proposé : son nom et la valeur vue dans l'adresse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParam {
    pub name: String,
    pub value: String,
}

/// Chemin généralisé d'un chemin concret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    /// `/orders/{id}/tracking`.
    pub path: String,
    pub params: Vec<PathParam>,
}

/// Proposition montrée au-dessus d'un essai (`trial_generalization`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// Chemin actuel de l'opération : `/orders/42/tracking`.
    pub path: String,
    pub template: Template,
    /// Contrat déduit (`contracts/…`).
    pub contract: String,
}

/// Le segment ressemble-t-il à un identifiant ? Un nombre, un UUID, ou un mot d'au moins six
/// caractères qui mêle lettres et chiffres (`ord_8F2kQ`), mais pas une version (`v2`).
pub fn is_identifier(segment: &str) -> bool {
    if segment.is_empty() || segment.starts_with('{') {
        return false;
    }
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    if is_uuid(segment) {
        return true;
    }
    let allowed = segment
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    let digits = segment.chars().any(|c| c.is_ascii_digit());
    let letters = segment.chars().any(|c| c.is_ascii_alphabetic());
    allowed && digits && letters && segment.len() >= 6
}

fn is_uuid(segment: &str) -> bool {
    let parts: Vec<&str> = segment.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(part, len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Nom du paramètre d'un segment, d'après le segment littéral qui le précède : le dernier
/// s'appelle `id`, les autres `<ressource au singulier>Id` (`/users/7/orders/42` →
/// `{userId}`, `{id}`).
fn param_name(previous: Option<&str>, last: bool, taken: &[String]) -> String {
    let base = if last {
        "id".to_owned()
    } else {
        let word: String = previous
            .unwrap_or("item")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_lowercase();
        let singular = word.strip_suffix('s').unwrap_or(&word);
        let singular = if singular.is_empty() {
            "item"
        } else {
            singular
        };
        format!("{singular}Id")
    };
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}

/// Chemin généralisé de `path`, ou `None` si aucun segment ne ressemble à un identifiant.
pub fn template(path: &str) -> Option<Template> {
    let segments: Vec<&str> = path.split('/').collect();
    let positions: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| is_identifier(s))
        .map(|(i, _)| i)
        .collect();
    let last = *positions.last()?;
    let mut params: Vec<PathParam> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        if positions.contains(&i) {
            let previous = segments[..i]
                .iter()
                .rev()
                .find(|s| !s.is_empty() && !is_identifier(s))
                .copied();
            let taken: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            let name = param_name(previous, i == last, &taken);
            out.push(format!("{{{name}}}"));
            params.push(PathParam {
                name,
                value: (*segment).to_owned(),
            });
        } else {
            out.push((*segment).to_owned());
        }
    }
    Some(Template {
        path: out.join("/"),
        params,
    })
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Méthode et chemin d'une clé d'opération (`GET /orders/42`).
fn split_key(key: &str) -> Option<(String, String)> {
    let (method, path) = key.split_once(' ')?;
    Some((method.to_lowercase(), path.to_owned()))
}

/// Contrat déduit lié à la requête `path`, son document et la clé de l'opération.
fn linked_deduced(root: &Path, path: &str) -> Result<Option<(ContractLink, Value)>, CommandError> {
    let request = request_file::read(&tree::resolve(root, path)?)?;
    let Some(link) = request.contract else {
        return Ok(None);
    };
    let Ok(file) = tree::resolve(root, &link.file) else {
        return Ok(None);
    };
    let Ok(text) = fs::read_to_string(&file) else {
        return Ok(None);
    };
    let Ok(doc) = yaml::parse(&text) else {
        return Ok(None);
    };
    Ok(deduce::is_deduced(&doc).then_some((link, doc)))
}

/// Proposition de chemin généralisé pour l'essai `path`, s'il y en a une : requête liée à une
/// opération d'un contrat déduit, dont un segment ressemble à un identifiant, et que
/// l'utilisateur n'a pas gardée telle quelle.
pub fn suggestion(root: &Path, path: &str) -> Result<Option<Suggestion>, CommandError> {
    let Some((link, doc)) = linked_deduced(root, path)? else {
        return Ok(None);
    };
    let Some((method, op_path)) = split_key(&link.operation) else {
        return Ok(None);
    };
    let kept = doc
        .get("paths")
        .and_then(|p| p.get(&op_path))
        .and_then(|item| item.get(&method))
        .and_then(|op| op.get(KEPT_MARK))
        .and_then(Value::as_bool)
        == Some(true);
    if kept {
        return Ok(None);
    }
    Ok(template(&op_path).map(|template| Suggestion {
        path: op_path,
        template,
        contract: link.file,
    }))
}

fn write_contract(root: &Path, file: &str, doc: &Value) -> Result<(), CommandError> {
    let text = yaml::render(doc)?;
    parse::load(&text).map_err(|reason| {
        invalid(format!(
            "Le contrat déduit ne serait pas lisible : {reason}"
        ))
    })?;
    contract_file::write_text(&tree::resolve(root, file)?, &text)
}

/// Opération `method` de `path` dans le document, à modifier.
fn operation_mut<'a>(
    doc: &'a mut Value,
    path: &str,
    method: &str,
) -> Option<&'a mut Map<String, Value>> {
    doc.get_mut("paths")?
        .get_mut(path)?
        .get_mut(method)?
        .as_object_mut()
}

/// « Garder tel quel » : la proposition n'est plus faite pour cette opération.
pub fn keep(root: &Path, path: &str) -> Result<(), CommandError> {
    let Some((link, mut doc)) = linked_deduced(root, path)? else {
        return Err(invalid("Cet essai n'est pas lié à un contrat déduit."));
    };
    let (method, op_path) = split_key(&link.operation)
        .ok_or_else(|| invalid("L'opération de cet essai est illisible."))?;
    let operation = operation_mut(&mut doc, &op_path, &method)
        .ok_or_else(|| invalid("L'opération de cet essai n'existe plus dans son contrat."))?;
    operation.insert(KEPT_MARK.to_owned(), Value::Bool(true));
    write_contract(root, &link.file, &doc)
}

/// Schéma d'un paramètre d'après sa valeur : entier pour un nombre, sinon texte.
fn param_schema(value: &str) -> Value {
    if value.chars().all(|c| c.is_ascii_digit()) {
        json!({ "type": "integer" })
    } else {
        json!({ "type": "string" })
    }
}

/// Remplace dans l'adresse `url` le chemin concret `from` par `to`, en gardant l'origine, la
/// requête et le fragment.
fn replace_path(url: &str, from: &str, to: &str) -> String {
    // Le chemin suit l'origine (`https://hôte`) ou `{{baseUrl}}` ; il est cherché après elle.
    let start = url.find("://").map_or(0, |i| i + 3);
    let rest = &url[start..];
    let Some(slash) = rest.find('/') else {
        return url.to_owned();
    };
    let head = &url[..start + slash];
    let tail = &rest[slash..];
    let cut = tail.find(['?', '#']).unwrap_or(tail.len());
    if &tail[..cut] == from {
        format!("{head}{to}{}", &tail[cut..])
    } else {
        url.to_owned()
    }
}

/// « Généraliser » (2.2-c) : l'opération passe au chemin généralisé (fusionnée avec une
/// opération déjà présente à ce chemin), avec ses paramètres de chemin ; chaque requête liée
/// suit, son adresse prend `{nom}` et la valeur va dans ses paramètres de chemin. Renvoie les
/// requêtes modifiées.
pub fn generalize(root: &Path, path: &str) -> Result<Vec<String>, CommandError> {
    let Some(suggestion) = suggestion(root, path)? else {
        return Err(invalid("Le chemin de cet essai n'a rien à généraliser."));
    };
    let Some((link, mut doc)) = linked_deduced(root, path)? else {
        return Err(invalid("Cet essai n'est pas lié à un contrat déduit."));
    };
    let (method, old) = split_key(&link.operation)
        .ok_or_else(|| invalid("L'opération de cet essai est illisible."))?;
    let new = suggestion.template.path.clone();
    let new_key = format!("{} {new}", method.to_uppercase());

    // L'opération quitte l'ancien chemin…
    let paths = doc
        .get_mut("paths")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid("Le contrat déduit n'a pas de chemins."))?;
    let mut operation = paths
        .get_mut(&old)
        .and_then(Value::as_object_mut)
        .and_then(|item| item.remove(&method))
        .and_then(|op| op.as_object().cloned())
        .ok_or_else(|| invalid("L'opération de cet essai n'existe plus dans son contrat."))?;
    if paths
        .get(&old)
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        paths.remove(&old);
    }
    // … prend ses paramètres de chemin…
    let mut parameters: Vec<Value> = operation
        .get("parameters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for param in &suggestion.template.params {
        parameters.push(json!({
            "name": param.name,
            "in": "path",
            "required": true,
            "schema": param_schema(&param.value),
            "example": param.value,
        }));
    }
    operation.insert("parameters".to_owned(), Value::Array(parameters));
    operation.insert("summary".to_owned(), Value::String(new_key.clone()));
    // … et rejoint le chemin généralisé : une opération déjà là garde ses réponses et reçoit
    // les statuts qu'elle n'a pas.
    let item = paths
        .entry(new.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    let item = item
        .as_object_mut()
        .ok_or_else(|| invalid("Le chemin généralisé est illisible dans le contrat."))?;
    match item.get_mut(&method).and_then(Value::as_object_mut) {
        Some(existing) => {
            let incoming = operation
                .get("responses")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(responses) = existing
                .entry("responses")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
            {
                for (status, response) in incoming {
                    responses.entry(status).or_insert(response);
                }
            }
        }
        None => {
            if let Some(id) = operation.get_mut("operationId") {
                *id = Value::String(operation_id(&method, &new));
            }
            item.insert(method.clone(), Value::Object(operation));
        }
    }
    write_contract(root, &link.file, &doc)?;

    // Les requêtes liées à l'ancienne opération suivent.
    let mut changed = Vec::new();
    for linked in contract_file::linked(root) {
        if linked.link.file != link.file || linked.link.operation != link.operation {
            continue;
        }
        let file = tree::resolve(root, &linked.path)?;
        let mut request = request_file::read(&file)?;
        request.url = replace_path(&request.url, &old, &new);
        request.contract = Some(ContractLink {
            file: link.file.clone(),
            operation: new_key.clone(),
        });
        for param in &suggestion.template.params {
            request
                .params
                .retain(|p| !(p.kind == ParamKind::Path && p.name == param.name));
            request
                .params
                .push(Pair::path(param.name.clone(), param.value.clone()));
        }
        crate::request::model::normalize_path_params(&mut request);
        request_file::write(&file, &request)?;
        changed.push(linked.path);
    }
    Ok(changed)
}

/// Identifiant d'opération d'un chemin généralisé : `get_orders_id_tracking`.
fn operation_id(method: &str, path: &str) -> String {
    let mut words = vec![method.to_lowercase()];
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        let word: String = segment
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let word = word.trim_matches('_').to_owned();
        if !word.is_empty() {
            words.push(word);
        }
    }
    words.join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_segments_qui_ressemblent_a_un_identifiant() {
        let yes = [
            "42",
            "ord_8F2kQ",
            "7c1e2b04-5d3a-4f6e-9b21-0c8d4e7a91f3",
            "cus_31",
        ];
        let no = ["orders", "v2", "{id}", "", "tracking", "a1"];
        assert!(yes.iter().all(|s| is_identifier(s)), "{yes:?}");
        assert!(no.iter().all(|s| !is_identifier(s)), "{no:?}");
    }

    #[test]
    fn c_chemin_generalise_et_noms_des_parametres() {
        assert_eq!(
            template("/orders/42/tracking"),
            Some(Template {
                path: "/orders/{id}/tracking".to_owned(),
                params: vec![PathParam {
                    name: "id".to_owned(),
                    value: "42".to_owned()
                }],
            })
        );
        let nested = template("/users/7/orders/ord_8F2kQ").unwrap();
        assert_eq!(nested.path, "/users/{userId}/orders/{id}");
        assert_eq!(
            nested
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["userId", "id"]
        );
        assert_eq!(template("/products"), None);
        assert_eq!(template("/v2/products"), None);
    }

    #[test]
    fn c_adresse_au_chemin_generalise() {
        assert_eq!(
            replace_path(
                "https://api.boutique.dev/orders/42/tracking?full=1",
                "/orders/42/tracking",
                "/orders/{id}/tracking"
            ),
            "https://api.boutique.dev/orders/{id}/tracking?full=1"
        );
        assert_eq!(
            replace_path("{{baseUrl}}/orders/42", "/orders/42", "/orders/{id}"),
            "{{baseUrl}}/orders/{id}"
        );
        assert_eq!(
            replace_path("https://h/autre", "/orders/42", "/orders/{id}"),
            "https://h/autre"
        );
    }
}
