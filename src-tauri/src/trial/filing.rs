//! « Ranger dans un projet » (écran 2.2-d) : les essais d'un contrat déduit de « Non classés »
//! quittent la boîte pour un projet, avec leur contrat (copié tel quel) ou sous un contrat du
//! projet (chaque essai rejoint l'opération qui sert son chemin). Rien n'est perdu : chaque
//! fichier touché est gardé pour « Annuler ».

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::contract::deduce;
use crate::contract::file as contract_file;
use crate::error::{CommandError, CommandErrorKind};
use crate::request::file as request_file;
use crate::request::model::ContractLink;
use crate::tree;

/// Essais rangés : où ils sont allés, ce qui est resté, et de quoi annuler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Filed {
    /// Chemins des essais déplacés : (dans « Non classés », dans le projet).
    pub moved: Vec<(String, String)>,
    /// Essais restés dans « Non classés » : aucune opération du contrat choisi ne les sert.
    pub left: Vec<String>,
    /// Contrat du projet qui les reçoit (`contracts/…`).
    pub contract: String,
    /// Nom du contrat, pour la notification.
    pub contract_name: String,
}

pub use crate::tree::undo::Undo;

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

fn read_text(path: &Path) -> Result<String, CommandError> {
    fs::read_to_string(path).map_err(|e| CommandError::io("Lecture impossible", path, &e))
}

/// Chemin de l'adresse (`https://hôte/a/b?x` → `/a/b`), sans l'origine ni `{{baseUrl}}`.
fn url_path(url: &str) -> String {
    let start = url.find("://").map_or_else(
        || url.find("}}").map_or(0, |i| i + 2),
        |i| url[i + 3..].find('/').map_or(url.len(), |j| i + 3 + j),
    );
    let rest = &url[start..];
    let cut = rest.find(['?', '#']).unwrap_or(rest.len());
    let path = &rest[..cut];
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

/// Adresse servie par le contrat du projet : `{{baseUrl}}` à la place de l'origine.
fn with_base_url(url: &str) -> String {
    let Some(i) = url.find("://") else {
        return url.to_owned();
    };
    let rest = &url[i + 3..];
    let tail = rest.find('/').map_or("", |j| &rest[j..]);
    format!("{{{{baseUrl}}}}{tail}")
}

/// Range les essais du contrat déduit `contract` de « Non classés » (`inbox`) dans le projet
/// `target` : sous une copie du contrat (`into` vide), ou sous le contrat `into` du projet.
pub fn file_into(
    inbox: &Path,
    contract: &str,
    target: &Path,
    into: Option<&str>,
) -> Result<(Filed, Undo), CommandError> {
    let source = tree::resolve(inbox, contract)?;
    let source_text = read_text(&source)?;
    let deduced = contract_file::read(inbox, contract)?;
    if !deduce::is_deduced(&deduced.doc) {
        return Err(invalid(
            "Seul un contrat déduit des essais se range dans un projet.",
        ));
    }
    let essais: Vec<contract_file::Linked> = contract_file::linked(inbox)
        .into_iter()
        .filter(|l| l.link.file == contract)
        .collect();
    let mut undo = Undo::default();

    // Contrat qui reçoit les essais : une copie, ou celui choisi dans le projet.
    let (file, receiving) = match into {
        Some(file) => (file.to_owned(), contract_file::read(target, file)?),
        None => {
            let folder = contract_file::dir(target);
            fs::create_dir_all(&folder)
                .map_err(|e| CommandError::io("Création du dossier impossible", &folder, &e))?;
            let stem = Path::new(contract)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "contrat".to_owned());
            let ext = Path::new(contract)
                .extension()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "yaml".to_owned());
            let name = contract_file::free_name(&folder, &stem, &ext);
            let path = folder.join(&name);
            contract_file::write_text(&path, &source_text)?;
            undo.created(path.clone());
            let relative = contract_file::relative(&path);
            let read = contract_file::read(target, &relative)?;
            (relative, read)
        }
    };
    let copied = into.is_none();
    let folder = contract_file::folder_name(&receiving.title);
    let target_dir = tree::resolve(target, &folder)?;

    let mut moved = Vec::new();
    let mut left = Vec::new();
    for essai in essais {
        let from = tree::resolve(inbox, &essai.path)?;
        let original = read_text(&from)?;
        let mut request = request_file::read(&from)?;
        let operation = if copied {
            Some(essai.link.operation.clone())
        } else {
            let path = url_path(&request.url);
            let method = request.method.as_str().to_uppercase();
            receiving
                .operations
                .iter()
                .find(|op| op.method == method && deduce::path_matches(&op.path, &path))
                .map(|op| op.key.clone())
        };
        let Some(operation) = operation else {
            left.push(essai.path);
            continue;
        };
        request.contract = Some(ContractLink {
            file: file.clone(),
            operation,
        });
        if !copied {
            request.url = with_base_url(&request.url);
        }
        fs::create_dir_all(&target_dir)
            .map_err(|e| CommandError::io("Création du dossier impossible", &target_dir, &e))?;
        let stem = from
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| tree::slug(&request.name));
        let to = target_dir.join(tree::free_file_name(&target_dir, &stem));
        request_file::write(&to, &request)?;
        undo.created(to.clone());
        fs::remove_file(&from)
            .map_err(|e| CommandError::io("Déplacement impossible", &from, &e))?;
        // Suppression faite par Arnod : pas une modification hors d'Arnod (lot I).
        crate::watch::settle(&from);
        undo.restored(from, original);
        let relative = format!(
            "{folder}/{}",
            to.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        moved.push((essai.path, relative));
    }
    if moved.is_empty() {
        undo.apply()?;
        return Err(invalid(format!(
            "Aucune opération de « {} » ne sert ces essais : rien n'a été rangé.",
            receiving.title
        )));
    }
    // Le contrat déduit quitte « Non classés » quand plus aucun essai n'y est lié.
    if left.is_empty() {
        fs::remove_file(&source)
            .map_err(|e| CommandError::io("Retrait du contrat impossible", &source, &e))?;
        crate::watch::settle(&source);
        undo.restored(source, source_text);
    }
    Ok((
        Filed {
            moved,
            left,
            contract: file,
            contract_name: receiving.title,
        },
        undo,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d_chemin_et_base_d_une_adresse() {
        assert_eq!(
            url_path("https://api.boutique.dev/orders/42?x=1"),
            "/orders/42"
        );
        assert_eq!(url_path("https://api.boutique.dev"), "/");
        assert_eq!(url_path("{{baseUrl}}/orders"), "/orders");
        assert_eq!(
            with_base_url("https://api.boutique.dev/orders/{id}?x=1"),
            "{{baseUrl}}/orders/{id}?x=1"
        );
        assert_eq!(with_base_url("{{baseUrl}}/a"), "{{baseUrl}}/a");
    }
}
