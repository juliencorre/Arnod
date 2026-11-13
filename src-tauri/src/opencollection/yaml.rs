//! Lecture et écriture YAML des fichiers OpenCollection.
//!
//! Les textes sur plusieurs lignes (corps, scripts, docs) sont écrits en blocs
//! littéraux (`|-`), comme Bruno, pour des diffs lisibles.

use std::fs;
use std::io;
use std::path::Path;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::Value;

use crate::error::{CommandError, CommandErrorKind};

/// Valeur YAML lue d'un texte.
pub fn parse(text: &str) -> Result<Value, String> {
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_saphyr::from_str::<Value>(text).map_err(|error| format!("YAML illisible : {error}"))
}

/// Valeur YAML d'un fichier ; `NotFound` s'il n'existe pas.
pub fn read(path: &Path) -> Result<Value, CommandError> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CommandError::new(
                CommandErrorKind::NotFound,
                format!("Le fichier {} n'existe plus.", path.display()),
            )
        } else {
            CommandError::io("Lecture impossible", path, &error)
        }
    })?;
    parse(&text).map_err(|reason| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!("Le fichier {} est illisible : {reason}", path.display()),
        )
    })
}

/// Valeur JSON sérialisée en YAML, blocs littéraux pour les textes multilignes.
struct Yaml<'a>(&'a Value);

impl Serialize for Yaml<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::String(text) if text.contains('\n') => {
                serde_saphyr::LitStr(text).serialize(serializer)
            }
            Value::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&Yaml(item))?;
                }
                seq.end()
            }
            Value::Object(map) => {
                let mut out = serializer.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    out.serialize_entry(key, &Yaml(value))?;
                }
                out.end()
            }
            other => other.serialize(serializer),
        }
    }
}

/// Texte YAML d'une valeur.
pub fn render(value: &Value) -> Result<String, CommandError> {
    serde_saphyr::to_string(&Yaml(value)).map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Écriture YAML impossible : {error}"),
        )
    })
}

/// Écrit une valeur de façon atomique ; rien n'est réécrit si le texte n'a pas
/// changé (Git).
pub fn write(path: &Path, value: &Value) -> Result<(), CommandError> {
    let text = render(value)?;
    if fs::read_to_string(path).is_ok_and(|current| current == text) {
        crate::watch::record_write(path, text.as_bytes());
        return Ok(());
    }
    let tmp = path.with_extension("yml.tmp");
    fs::write(&tmp, &text)
        .map_err(|error| CommandError::io("Écriture impossible", &tmp, &error))?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        CommandError::io("Écriture impossible", path, &error)
    })?;
    // Écriture d'Arnod : son écho n'est pas un changement fait hors d'Arnod (lot I, R2).
    crate::watch::record_write(path, text.as_bytes());
    Ok(())
}
