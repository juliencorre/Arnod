//! Moteur intégré `http` : requête d'un essai REST d'après son adresse et les
//! valeurs de son formulaire (R3).

use std::collections::BTreeMap;

use crate::request::model::{Body, BodyKind, Method, Request};

use super::detect::is_url;

fn method_of(text: &str) -> Option<Method> {
    serde_json::from_value(serde_json::Value::String(text.to_uppercase())).ok()
}

fn body_kind(label: &str) -> BodyKind {
    match label {
        "XML" => BodyKind::Xml,
        "Texte" => BodyKind::Text,
        _ => BodyKind::Json,
    }
}

/// Requête nommée `name` : méthode, adresse absolue `http(s)://` et corps
/// éventuel. `values` sont déjà normalisées (`values::normalize`).
pub fn request(
    name: &str,
    subject: &str,
    values: &BTreeMap<String, String>,
) -> Result<Request, String> {
    let url = subject.trim();
    if !is_url(url, &["http".to_owned(), "https".to_owned()]) {
        return Err(
            "l'adresse doit commencer par http:// ou https:// et nommer un serveur".to_owned(),
        );
    }
    let method = values
        .get("method")
        .map_or(Some(Method::Get), |m| method_of(m))
        .ok_or_else(|| "méthode inconnue".to_owned())?;
    let mut request = Request::new(name);
    request.method = method;
    request.url = url.to_owned();
    if let Some(body) = values.get("body").filter(|b| !b.trim().is_empty()) {
        let kind = values
            .get("bodyType")
            .map_or(BodyKind::Json, |label| body_kind(label));
        request.body = Body::raw(kind, body.clone());
    }
    Ok(request)
}
