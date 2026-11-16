//! Réponse attendue reprise d'une réponse reçue (plan de configuration d'un
//! endpoint, R7).

use serde_json::Value;

use crate::request::model::{Example, ExampleBodyKind, ResponseHeader};

/// Type du corps d'après le `Content-Type` ; un corps vide n'a pas de type.
pub fn body_kind(content_type: &str, body: &str) -> ExampleBodyKind {
    if body.is_empty() {
        return ExampleBodyKind::None;
    }
    let media = content_type.to_lowercase();
    if media.contains("json") {
        ExampleBodyKind::Json
    } else if media.contains("xml") {
        ExampleBodyKind::Xml
    } else if media.contains("html") {
        ExampleBodyKind::Html
    } else if media.is_empty() || media.starts_with("text/") {
        ExampleBodyKind::Text
    } else {
        ExampleBodyKind::Binary
    }
}

/// Réponse attendue nommée `name`, reprise d'une réponse reçue (déjà masquée).
/// Un corps JSON est réindenté pour la lecture.
pub fn from_response(
    name: &str,
    status: u16,
    status_text: &str,
    headers: &[(String, String)],
    body: &str,
) -> Example {
    let content_type = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
        .map_or("", |(_, v)| v.as_str());
    let body_kind = body_kind(content_type, body);
    let body = match body_kind {
        ExampleBodyKind::Json => serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| body.to_owned()),
        _ => body.to_owned(),
    };
    Example {
        name: name.to_owned(),
        description: String::new(),
        status: Some(status),
        status_text: status_text.to_owned(),
        headers: headers
            .iter()
            .map(|(name, value)| ResponseHeader {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
        body_kind,
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r7_reponse_json_reindentee() {
        let example = from_response(
            "Reçue",
            201,
            "Created",
            &[(
                "content-type".into(),
                "application/json; charset=utf-8".into(),
            )],
            r#"{"id":"ord_1","total":59.8}"#,
        );
        assert_eq!(example.status, Some(201));
        assert_eq!(example.status_text, "Created");
        assert_eq!(example.body_kind, ExampleBodyKind::Json);
        assert_eq!(
            example.body,
            "{\n  \"id\": \"ord_1\",\n  \"total\": 59.8\n}"
        );
        assert_eq!(example.headers[0].name, "content-type");
    }

    #[test]
    fn r7_type_du_corps() {
        assert_eq!(body_kind("text/html", "<p>"), ExampleBodyKind::Html);
        assert_eq!(body_kind("application/xml", "<a/>"), ExampleBodyKind::Xml);
        assert_eq!(body_kind("", "ok"), ExampleBodyKind::Text);
        assert_eq!(body_kind("image/png", "x"), ExampleBodyKind::Binary);
        assert_eq!(body_kind("application/json", ""), ExampleBodyKind::None);
    }
}
