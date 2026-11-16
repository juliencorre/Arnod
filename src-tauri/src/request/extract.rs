//! Extractions (`docs/plan-mvp-e-tests-etapes.md`, R5 à R7).
//!
//! Module pur : une valeur de la réponse, ou de la requête envoyée, devient
//! une variable d'exécution que les étapes suivantes lisent.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::request::assertions::{ResponseData, value_at};
use crate::request::model::Extraction;

/// Préfixe d'une source lue dans la requête envoyée (R7, R10).
pub const REQUEST_PREFIX: &str = "request.";

/// Valeur extraite à un envoi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedValue {
    pub name: String,
    pub source: String,
    /// Valeur rangée dans la variable ; `None` si la source est absente.
    pub value: Option<String>,
    /// Raison d'une valeur absente, en français.
    pub message: Option<String>,
}

/// Texte d'une valeur JSON : une chaîne telle quelle, le reste en JSON.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Source qu'Arnod sait lire : corps, en-tête, statut ou durée.
fn known(source: &str) -> bool {
    matches!(source, "status" | "responseTime")
        || source.starts_with("headers.")
        || source == "body"
        || source.starts_with("body.")
        || source.starts_with("body[")
}

/// Valeur d'une source dans `data` ; `body` d'un corps qui n'est pas du JSON
/// donne le texte du corps.
fn read(source: &str, data: &ResponseData) -> Option<String> {
    if let Some(value) = value_at(source, data) {
        return Some(text_of(&value));
    }
    (source == "body" && !data.body.is_empty()).then(|| data.body.clone())
}

/// Applique les extractions actives à une réponse et à la requête envoyée
/// (statut et durée sans objet). Une source absente est signalée (R7).
pub fn apply(
    extractions: &[Extraction],
    response: &ResponseData,
    sent: &ResponseData,
) -> Vec<ExtractedValue> {
    extractions
        .iter()
        .filter(|e| e.enabled && !e.name.trim().is_empty())
        .map(|extraction| {
            let source = extraction.source.trim();
            let value = match source.strip_prefix(REQUEST_PREFIX) {
                Some(rest) => read(rest, sent),
                None => read(source, response),
            };
            let message = value.is_none().then(|| {
                if known(source.strip_prefix(REQUEST_PREFIX).unwrap_or(source)) {
                    format!("« {source} » est absent : la variable n'est pas remplie.")
                } else {
                    format!(
                        "Expression « {source} » non reconnue (body.chemin, headers.Nom ou status attendu) : la variable n'est pas remplie."
                    )
                }
            });
            ExtractedValue {
                name: extraction.name.trim().to_owned(),
                source: source.to_owned(),
                value,
                message,
            }
        })
        .collect()
}

/// Range les valeurs extraites dans les variables d'exécution (R5).
pub fn store(values: &[ExtractedValue], into: &mut BTreeMap<String, String>) {
    for extracted in values {
        if let Some(value) = &extracted.value {
            into.insert(extracted.name.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(status: u16, headers: &[(&str, &str)], body: &str) -> ResponseData {
        ResponseData {
            status,
            duration_ms: 12,
            headers: headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: body.to_owned(),
        }
    }

    fn extraction(name: &str, source: &str) -> Extraction {
        Extraction {
            name: name.to_owned(),
            source: source.to_owned(),
            enabled: true,
        }
    }

    #[test]
    fn r7_corps_en_tete_statut_et_requete_envoyee() {
        let response = data(
            201,
            &[("Location", "/clients/7")],
            r#"{"id": "cus_7", "items": [{"qty": 2}]}"#,
        );
        let sent = data(0, &[("X-Ref", "abc")], r#"{"ref": "r-1"}"#);
        let list = [
            extraction("client.id", "body.id"),
            extraction("qty", "body.items[0].qty"),
            extraction("lieu", "headers.location"),
            extraction("code", "status"),
            extraction("ref", "request.body.ref"),
            extraction("xref", "request.headers.X-Ref"),
        ];
        let values: Vec<Option<String>> = apply(&list, &response, &sent)
            .into_iter()
            .map(|v| v.value)
            .collect();
        assert_eq!(
            values,
            [
                Some("cus_7".to_owned()),
                Some("2".to_owned()),
                Some("/clients/7".to_owned()),
                Some("201".to_owned()),
                Some("r-1".to_owned()),
                Some("abc".to_owned()),
            ]
        );
    }

    #[test]
    fn r7_source_absente_signalee_et_rien_range() {
        let response = data(200, &[], "pas du json");
        let mut off = extraction("x", "body.id");
        off.enabled = false;
        let values = apply(
            &[
                extraction("id", "body.id"),
                extraction("texte", "body"),
                off,
            ],
            &response,
            &data(0, &[], ""),
        );
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].value, None);
        assert!(values[0].message.as_deref().unwrap().contains("body.id"));
        assert_eq!(values[1].value.as_deref(), Some("pas du json"));

        let mut vars = BTreeMap::from([("id".to_owned(), "ancien".to_owned())]);
        store(&values, &mut vars);
        assert_eq!(vars["id"], "ancien");
        assert_eq!(vars["texte"], "pas du json");
    }

    #[test]
    fn r7_expression_non_reconnue_signalee_sans_planter() {
        let values = apply(
            &[
                extraction("x", "res.cookies.session"),
                extraction("y", "request.res.z"),
            ],
            &data(200, &[], "{}"),
            &data(0, &[], ""),
        );
        assert!(values.iter().all(|v| v.value.is_none()));
        assert!(
            values[0]
                .message
                .as_deref()
                .unwrap()
                .contains("non reconnue"),
            "{:?}",
            values[0]
        );
        assert!(
            values[1]
                .message
                .as_deref()
                .unwrap()
                .contains("non reconnue")
        );
    }

    #[test]
    fn forme_json() {
        let value = apply(
            &[extraction("id", "body.id")],
            &data(200, &[], r#"{"id":3}"#),
            &data(0, &[], ""),
        );
        assert_eq!(
            serde_json::to_value(&value[0]).unwrap(),
            serde_json::json!({"name": "id", "source": "body.id", "value": "3", "message": null})
        );
    }
}
