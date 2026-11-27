//! Jeux de données (`docs/plan-v4.md`, lot D, R11, R12) : un tableau CSV du projet, une colonne
//! par variable, une ligne par cas, lié à une requête par une étiquette (`data:` ou
//! `data-file:`).
//!
//! Le CSV est lu et écrit à la main (RFC 4180, séparateur `,`, `;` ou tabulation reconnu à la
//! lecture) : aucune bibliothèque ajoutée. Le tableau d'Arnod est rangé dans `données/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contract::file::write_text;
use crate::error::{CommandError, CommandErrorKind};
use crate::request::file;
use crate::request::model::DataLink;
use crate::tree::{self, DATA_DIR};

#[cfg(test)]
mod tests;

/// Lignes d'un jeu de données au plus.
pub const MAX_CASES: usize = 1_000;

/// Tableau : noms des colonnes (variables), puis une ligne par cas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DataTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Jeu de données d'une requête, pour l'onglet « Données » (4.7-a, 4.7-a bis).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DataSet {
    pub link: DataLink,
    #[serde(flatten)]
    pub table: DataTable,
    /// Fichier illisible : la raison, en français ; le tableau est alors vide.
    pub error: Option<String>,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Séparateur d'un texte : tabulation (copie d'un tableur), point-virgule ou virgule, d'après la
/// première ligne.
fn separator(text: &str) -> char {
    let first = text.lines().next().unwrap_or_default();
    let count = |c: char| first.matches(c).count();
    if count('\t') > 0 {
        '\t'
    } else if count(';') > count(',') {
        ';'
    } else {
        ','
    }
}

/// Lignes d'un texte CSV (RFC 4180) ; une ligne vide est ignorée.
pub fn parse_rows(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let sep = separator(text);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                other => cell.push(other),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => quoted = true,
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                if row.iter().any(|c| !c.trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            c if c == sep => row.push(std::mem::take(&mut cell)),
            other => cell.push(other),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        if row.iter().any(|c| !c.trim().is_empty()) {
            rows.push(row);
        }
    }
    rows
}

/// Tableau d'un texte CSV : la première ligne donne les colonnes (R11).
pub fn parse(text: &str) -> DataTable {
    let mut rows = parse_rows(text);
    if rows.is_empty() {
        return DataTable::default();
    }
    let columns: Vec<String> = rows
        .remove(0)
        .into_iter()
        .map(|c| c.trim().to_owned())
        .collect();
    let width = columns.len();
    for row in &mut rows {
        row.resize(width, String::new());
    }
    rows.truncate(MAX_CASES);
    DataTable { columns, rows }
}

fn quote(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r', ';']) || cell.starts_with(' ') || cell.ends_with(' ') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// Texte CSV d'un tableau, séparé par des virgules, terminé par une fin de ligne.
pub fn render(table: &DataTable) -> String {
    let mut out = String::new();
    let line = |cells: &[String]| cells.iter().map(|c| quote(c)).collect::<Vec<_>>().join(",");
    out.push_str(&line(&table.columns));
    out.push('\n');
    for row in &table.rows {
        out.push_str(&line(row));
        out.push('\n');
    }
    out
}

/// Valeurs de chaque cas (R12) : une ligne vide n'est pas un cas ; une colonne sans nom est
/// ignorée.
pub fn cases(table: &DataTable) -> Vec<BTreeMap<String, String>> {
    table
        .rows
        .iter()
        .filter(|row| row.iter().any(|c| !c.trim().is_empty()))
        .map(|row| {
            table
                .columns
                .iter()
                .zip(row)
                .filter(|(name, _)| !name.trim().is_empty())
                .map(|(name, value)| (name.trim().to_owned(), value.clone()))
                .collect()
        })
        .collect()
}

/// Lit le jeu de données lié : `None` sans lien ; un fichier illisible donne sa raison.
pub fn read(root: &Path, link: &DataLink) -> DataSet {
    let result = tree::resolve(root, &link.file).and_then(|path| {
        fs::read_to_string(&path).map_err(|error| {
            CommandError::io("Lecture du jeu de données impossible", &path, &error)
        })
    });
    match result {
        Ok(text) => DataSet {
            link: link.clone(),
            table: parse(&text),
            error: None,
        },
        Err(error) => DataSet {
            link: link.clone(),
            table: DataTable::default(),
            error: Some(error.message),
        },
    }
}

/// Jeu de données de la requête `path`, s'il y en a un.
pub fn of_request(root: &Path, path: &str) -> Result<Option<DataSet>, CommandError> {
    let request = file::read(&tree::resolve(root, path)?)?;
    Ok(request.data.as_ref().map(|link| read(root, link)))
}

/// Nom de fichier libre dans `données/` d'après `stem`.
fn free_file(root: &Path, stem: &str) -> Result<String, CommandError> {
    let dir = root.join(DATA_DIR);
    fs::create_dir_all(&dir)
        .map_err(|error| CommandError::io("Création du dossier impossible", &dir, &error))?;
    let base = tree::slug(stem);
    let base = if base.is_empty() {
        "donnees".to_owned()
    } else {
        base
    };
    let mut candidate = format!("{base}.csv");
    let mut n = 2;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{n}.csv");
        n += 1;
    }
    Ok(format!("{DATA_DIR}/{candidate}"))
}

/// Enregistre le tableau de la requête `path` (4.7-a) : dans son fichier lié s'il est modifiable,
/// sinon dans un fichier neuf de `données/`, lié à la requête (R11).
pub fn save(root: &Path, path: &str, table: &DataTable) -> Result<DataSet, CommandError> {
    let request_path = tree::resolve(root, path)?;
    let mut request = file::read(&request_path)?;
    if table.columns.iter().all(|c| c.trim().is_empty()) {
        return Err(invalid(
            "Nommez au moins une colonne : chaque colonne est une variable.",
        ));
    }
    if table.rows.len() > MAX_CASES {
        return Err(invalid(format!("{MAX_CASES} cas au plus.")));
    }
    let link = match &request.data {
        Some(link) if link.editable => link.clone(),
        Some(_) => {
            return Err(invalid(
                "Ce jeu de données vient d'un fichier : modifiez-le dans votre tableur, Arnod le relit à chaque envoi.",
            ));
        }
        None => DataLink {
            file: free_file(root, &request.name)?,
            editable: true,
        },
    };
    let target = tree::resolve(root, &link.file)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CommandError::io("Création du dossier impossible", parent, &error))?;
    }
    let text = render(table);
    if fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
        write_text(&target, &text)?;
    }
    if request.data.as_ref() != Some(&link) {
        request.data = Some(link.clone());
        file::write(&request_path, &request)?;
    }
    Ok(read(root, &link))
}

/// Range le fichier CSV `source` du poste dans `données/` et le lie en lecture seule à la
/// requête `path` (4.7-a bis) ; un fichier illisible n'est jamais rangé.
pub fn import(root: &Path, path: &str, source: &Path) -> Result<DataSet, CommandError> {
    let text = fs::read_to_string(source)
        .map_err(|error| CommandError::io("Lecture du fichier impossible", source, &error))?;
    let table = parse(&text);
    if table.columns.is_empty() {
        return Err(invalid(
            "Ce fichier ne contient pas de tableau : sa première ligne donne le nom des variables.",
        ));
    }
    let request_path = tree::resolve(root, path)?;
    let mut request = file::read(&request_path)?;
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| request.name.clone());
    let link = DataLink {
        file: free_file(root, &stem)?,
        editable: false,
    };
    write_text(&tree::resolve(root, &link.file)?, &text)?;
    request.data = Some(link.clone());
    file::write(&request_path, &request)?;
    Ok(read(root, &link))
}

/// Délie le jeu de données de la requête `path` ; le tableau d'Arnod va à la corbeille.
pub fn remove(root: &Path, path: &str) -> Result<(), CommandError> {
    let request_path = tree::resolve(root, path)?;
    let mut request = file::read(&request_path)?;
    let Some(link) = request.data.take() else {
        return Ok(());
    };
    file::write(&request_path, &request)?;
    if link.editable
        && let Ok(target) = tree::resolve(root, &link.file)
        && target.is_file()
    {
        let _ = trash::delete(&target);
        crate::watch::settle(&target);
    }
    Ok(())
}
