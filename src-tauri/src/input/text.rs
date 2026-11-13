//! Une liste de paires éditée en texte (5.1-b quater, R11 à R13) : une ligne par
//! paire, `nom: valeur` pour un en-tête, `nom=valeur` pour un paramètre ; `#`
//! en tête met la ligne de côté (paire désactivée).

use serde::{Deserialize, Serialize};

use crate::request::model::{Pair, ParamKind};

/// Écriture des lignes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PairsFormat {
    /// `Nom: valeur`.
    Header,
    /// `nom=valeur` (`nom: valeur` accepté).
    Query,
}

/// Ligne qui ne se lit pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextError {
    /// Numéro de la ligne, à partir de 1.
    pub line: usize,
    pub message: String,
}

/// Paires lues d'un texte, et les lignes illisibles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairsText {
    pub pairs: Vec<Pair>,
    pub errors: Vec<TextError>,
}

/// Texte d'une liste : une ligne par paire, dans l'ordre (R11).
pub fn to_text(pairs: &[Pair], format: PairsFormat) -> String {
    pairs
        .iter()
        .map(|pair| {
            let line = match format {
                PairsFormat::Header => format!("{}: {}", pair.name, pair.value),
                PairsFormat::Query => format!("{}={}", pair.name, pair.value),
            };
            if pair.enabled {
                line
            } else {
                format!("# {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Nom et valeur d'une ligne, ou la raison de l'échec.
fn split(line: &str, format: PairsFormat) -> Result<(String, String), String> {
    let parts = match format {
        PairsFormat::Header => line.split_once(':'),
        // Le premier des deux séparateurs : `heure: 10:00`, `url=http://…`.
        PairsFormat::Query => line
            .find(['=', ':'])
            .map(|index| (&line[..index], &line[index + 1..])),
    };
    let (name, value) = match (parts, format) {
        (Some(parts), _) => parts,
        // Un paramètre sans valeur (`?debug`) est permis.
        (None, PairsFormat::Query) => (line, ""),
        (None, PairsFormat::Header) => {
            return Err("« : » manquant entre le nom et la valeur".to_owned());
        }
    };
    let name = name.trim();
    if name.is_empty() {
        return Err("nom manquant avant le séparateur".to_owned());
    }
    if format == PairsFormat::Header && name.contains(char::is_whitespace) {
        return Err(format!(
            "« {name} » : un nom d'en-tête ne contient pas d'espace"
        ));
    }
    Ok((name.to_owned(), value.trim().to_owned()))
}

/// Lit `text` (R12). Une ligne vide est sautée, un pseudo-en-tête HTTP/2 collé
/// d'un navigateur (`:authority: …`) aussi. La description d'une paire de
/// `previous` de même nom est gardée (R13).
pub fn from_text(text: &str, format: PairsFormat, previous: &[Pair]) -> PairsText {
    let mut pairs: Vec<Pair> = Vec::new();
    let mut errors = Vec::new();
    let mut used = vec![false; previous.len()];
    for (index, raw) in text.lines().enumerate() {
        let mut line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let enabled = !line.starts_with('#');
        if !enabled {
            line = line.trim_start_matches('#').trim_start();
            if line.is_empty() {
                continue;
            }
        }
        if format == PairsFormat::Header && line.starts_with(':') {
            continue;
        }
        match split(line, format) {
            Ok((name, value)) => {
                let same = |p: &Pair| match format {
                    PairsFormat::Header => p.name.eq_ignore_ascii_case(&name),
                    PairsFormat::Query => p.name == name,
                };
                let description = previous
                    .iter()
                    .enumerate()
                    .find(|(i, p)| !used[*i] && same(p))
                    .map(|(i, p)| {
                        used[i] = true;
                        p.description.clone()
                    })
                    .unwrap_or_default();
                pairs.push(Pair {
                    name,
                    value,
                    enabled,
                    kind: ParamKind::Query,
                    description,
                });
            }
            Err(message) => errors.push(TextError {
                line: index + 1,
                message: format!("Ligne {} : {message}.", index + 1),
            }),
        }
    }
    PairsText { pairs, errors }
}
