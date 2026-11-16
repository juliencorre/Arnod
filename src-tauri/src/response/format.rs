//! Format d'un corps de réponse (R1) et nom de fichier proposé à l'enregistrement (R8).

use serde::{Deserialize, Serialize};

/// Format d'affichage d'un corps : détecté d'après `content-type`, modifiable dans l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseFormat {
    Json,
    Xml,
    Html,
    Text,
}

impl ResponseFormat {
    /// Extension du fichier enregistré.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::Text => "txt",
        }
    }
}

/// Format d'après l'en-tête `content-type` (R1) ; sans en-tête parlant, d'après le début du
/// corps : `{` ou `[` qui se lit en JSON, `<!doctype html` ou `<html`, puis `<`.
pub fn detect(headers: &[(String, String)], body: &str) -> ResponseFormat {
    let content_type = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.to_ascii_lowercase());
    if let Some(value) = content_type {
        let essence = value.split(';').next().unwrap_or_default().trim();
        if essence.ends_with("/json") || essence.ends_with("+json") {
            return ResponseFormat::Json;
        }
        if essence == "text/html" || essence == "application/xhtml+xml" {
            return ResponseFormat::Html;
        }
        if essence.ends_with("/xml") || essence.ends_with("+xml") {
            return ResponseFormat::Xml;
        }
        if essence.starts_with("text/") && essence != "text/plain" {
            return ResponseFormat::Text;
        }
    }
    sniff(body)
}

fn sniff(body: &str) -> ResponseFormat {
    let start = body.trim_start();
    if (start.starts_with('{') || start.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(body).is_ok()
    {
        return ResponseFormat::Json;
    }
    let head: String = start.chars().take(15).collect::<String>().to_lowercase();
    if head.starts_with("<!doctype html") || head.starts_with("<html") {
        return ResponseFormat::Html;
    }
    if start.starts_with('<') {
        return ResponseFormat::Xml;
    }
    ResponseFormat::Text
}

/// Nom proposé pour enregistrer le corps : le nom de la requête, sans caractère interdit par
/// Windows, suivi de l'extension du format (R8). « reponse » pour un nom vide.
pub fn file_name(name: &str, format: ResponseFormat) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim();
    let stem = if cleaned.is_empty() {
        "reponse"
    } else {
        cleaned
    };
    format!("{stem}.{}", format.extension())
}
