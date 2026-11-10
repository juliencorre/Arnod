//! Essais sans projet (`docs/plan-accueil-non-classes.md`, R6 à R11) : dans
//! « Non classés », le contrat déduit du serveur est créé ou enrichi, et
//! l'essai devient la requête « Essai N » liée à son opération.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::contract::deduce::{self, Exchange};
use crate::contract::file as contract_file;
use crate::contract::parse;
use crate::error::{CommandError, CommandErrorKind};
use crate::http::HttpResponse;
use crate::opencollection::yaml;
use crate::request::assertions;
use crate::request::example;
use crate::request::file as request_file;
use crate::request::model::{ContractLink, Request};
use crate::tree::{self, TreeNode};

/// Préfixe du nom des essais : « Essai 1 », « Essai 2 »…
pub const TRIAL_PREFIX: &str = "Essai";

/// Essai enregistré : la requête et son contrat.
#[derive(Debug, Clone)]
pub struct Recorded {
    /// Chemin de la requête, relatif au projet.
    pub path: String,
    /// Chemin du contrat déduit (`contracts/…`).
    pub contract: String,
    pub request: Request,
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidRequest, message)
}

/// Prochain numéro libre parmi les requêtes « Essai N » du projet (R11).
pub fn next_number(nodes: &[TreeNode]) -> u32 {
    fn visit(nodes: &[TreeNode], max: &mut u32) {
        for node in nodes {
            match node {
                TreeNode::Folder { children, .. } => visit(children, max),
                TreeNode::Request { name, .. } => {
                    let number = name
                        .strip_prefix(TRIAL_PREFIX)
                        .map(str::trim)
                        .and_then(|rest| rest.parse::<u32>().ok());
                    if let Some(number) = number {
                        *max = (*max).max(number);
                    }
                }
            }
        }
    }
    let mut max = 0;
    visit(nodes, &mut max);
    max + 1
}

fn read_document(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).map_err(|error| error.to_string())
    } else {
        yaml::parse(trimmed)
    }
}

/// Contrat déduit qui sert l'origine de `url`, ou le fichier libre où en créer
/// un (R6, R9). Un contrat déduit illisible à son nom refuse l'essai (R10).
fn contract_for(root: &Path, url: &reqwest::Url) -> Result<(PathBuf, Value), CommandError> {
    let folder = contract_file::dir(root);
    let base = tree::slug(&deduce::title(url));
    if let Ok(entries) = fs::read_dir(&folder) {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && contract_file::is_contract_file(path))
            .collect();
        files.sort();
        for file in files {
            match read_document(&file) {
                Ok(doc) if deduce::is_deduced(&doc) && deduce::serves(&doc, url) => {
                    return Ok((file, doc));
                }
                Ok(_) => {}
                Err(reason) => {
                    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
                    if stem.as_deref() == Some(base.as_str()) {
                        return Err(invalid(format!(
                            "Le contrat déduit {} est illisible ({reason}) : corrigez-le ou supprimez-le, puis réessayez.",
                            contract_file::relative(&file)
                        )));
                    }
                }
            }
        }
    }
    let name = contract_file::free_name(&folder, &base, "yaml");
    Ok((folder.join(name), deduce::new_document(url)))
}

/// Enregistre l'essai `request` et sa réponse dans le projet `root` (R6 à R11).
pub fn record(
    root: &Path,
    mut request: Request,
    response: &HttpResponse,
) -> Result<Recorded, CommandError> {
    let url = reqwest::Url::parse(request.url.trim())
        .map_err(|_| invalid("L'adresse de l'essai est invalide."))?;

    // Contrat déduit, créé ou enrichi, vérifié lisible avant d'être écrit.
    let (contract_path, mut doc) = contract_for(root, &url)?;
    let key = deduce::add_exchange(
        &mut doc,
        &Exchange {
            method: request.method.as_str(),
            url: &url,
            status: response.status,
            status_text: &response.status_text,
            headers: &response.headers,
            body: &response.body,
        },
    );
    let text = yaml::render(&doc)?;
    parse::load(&text).map_err(|reason| {
        invalid(format!(
            "Le contrat déduit ne serait pas lisible : {reason}"
        ))
    })?;
    let folder = contract_file::dir(root);
    fs::create_dir_all(&folder)
        .map_err(|error| CommandError::io("Création du dossier impossible", &folder, &error))?;
    contract_file::write_text(&contract_path, &text)?;
    let contract = contract_file::relative(&contract_path);

    // Requête « Essai N » dans le dossier du serveur, liée à l'opération.
    let number = tree::read(root).map_or(1, |nodes| next_number(&nodes));
    request.name = format!("{TRIAL_PREFIX} {number}");
    request.contract = Some(ContractLink {
        file: contract.clone(),
        operation: key,
    });
    let data = response.data();
    request.assertions.extend(assertions::propose(&data));
    request.examples = vec![example::from_response(
        format!("{} {}", response.status, response.status_text).trim(),
        response.status,
        &response.status_text,
        &response.headers,
        &response.body,
    )];
    let relative_dir = contract_file::folder_name(&deduce::title(&url));
    let target = tree::resolve(root, &relative_dir)?;
    fs::create_dir_all(&target)
        .map_err(|error| CommandError::io("Création du dossier impossible", &target, &error))?;
    let file = target.join(tree::free_file_name(&target, &tree::slug(&request.name)));
    request_file::write(&file, &request)?;
    let path = format!(
        "{relative_dir}/{}",
        file.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    Ok(Recorded {
        path,
        contract,
        request,
    })
}

#[cfg(test)]
mod tests;
