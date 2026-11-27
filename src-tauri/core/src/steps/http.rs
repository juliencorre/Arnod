//! Moteur intégré `http` : requête d'un essai REST d'après son adresse et les
//! valeurs de son formulaire (R3).

use std::collections::BTreeMap;

use crate::input::{PairsFormat, from_text};
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
    // En-têtes, une ligne `Nom: valeur` chacun (`docs/plan-mvp-b-saisie-curl.md`, R4).
    if let Some(text) = values.get("headers") {
        let read = from_text(text, PairsFormat::Header, &[]);
        if let Some(error) = read.errors.first() {
            let message = error.message.replacen("Ligne", "ligne", 1);
            return Err(format!("en-têtes, {}", message.trim_end_matches('.')));
        }
        request.headers = read.pairs;
    }
    if let Some(body) = values.get("body").filter(|b| !b.trim().is_empty()) {
        let kind = values
            .get("bodyType")
            .map_or(BodyKind::Json, |label| body_kind(label));
        request.body = Body::raw(kind, body.clone());
    }
    Ok(request)
}

/// Copie de `request` où les variables `secrets` sont remplacées par leur
/// valeur, pour l'envoi d'un essai (R5) ; la requête enregistrée garde `{{nom}}`.
pub fn with_secrets(request: &Request, secrets: &BTreeMap<String, String>) -> Request {
    let fill = |text: &str| {
        let mut out = text.to_owned();
        for (name, value) in secrets {
            out = out.replace(&format!("{{{{{name}}}}}"), value);
        }
        out
    };
    let mut sent = request.clone();
    sent.url = fill(&sent.url);
    for pair in sent.headers.iter_mut().chain(sent.params.iter_mut()) {
        pair.value = fill(&pair.value);
    }
    for variant in &mut sent.body.variants {
        variant.content = fill(&variant.content);
    }
    sent
}
