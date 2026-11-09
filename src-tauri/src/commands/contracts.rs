//! Domaine « contrats » : contrats de services d'un projet, requêtes créées
//! depuis leurs opérations et run du contrat (docs/plan-contrats-services.md).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::contract::file::{self, ContractSource, ContractSummary, ContractView, GenerateReport};
use crate::error::{CommandError, CommandErrorKind};
use crate::layout::windows::Windows;
use crate::state::AppState;
use crate::windows::{self, ProjectScope};
use crate::workbench::{RunView, Workbench};

/// Taille maximale d'un contrat lu depuis une adresse (C3).
const MAX_DOWNLOAD: usize = 10 * 1024 * 1024;
/// Délai de lecture d'une adresse.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// Prévient les autres fenêtres ; une portée par partie modifiée.
fn changed(window: &WebviewWindow, project: Uuid, scopes: &[ProjectScope], path: Option<&str>) {
    for scope in scopes {
        windows::project_changed(window.app_handle(), window.label(), project, *scope, path);
    }
}

fn root(state: &AppState, project: Uuid) -> Result<std::path::PathBuf, CommandError> {
    Ok(state.project_ctx(project)?.root)
}

#[tauri::command]
pub fn contract_list(
    state: State<'_, AppState>,
    project: Uuid,
) -> Result<Vec<ContractSummary>, CommandError> {
    Ok(file::list(&root(&state, project)?))
}

#[tauri::command]
pub fn contract_read(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
) -> Result<ContractView, CommandError> {
    file::view(&root(&state, project)?, &path)
}

/// Texte d'un contrat lu depuis une adresse (C3).
async fn download(url: &str) -> Result<String, CommandError> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            "L'adresse du contrat est invalide.",
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "L'adresse du contrat doit commencer par http:// ou https://.",
        ));
    }
    let host = parsed.host_str().unwrap_or_default().to_owned();
    let network = |error: reqwest::Error| {
        if error.is_timeout() {
            CommandError::new(
                CommandErrorKind::Timeout,
                "Le délai est dépassé sans réponse.",
            )
        } else {
            CommandError::new(
                CommandErrorKind::Network,
                format!("Impossible de joindre {host}. Vérifiez l'adresse et la connexion."),
            )
        }
    };
    let client = reqwest::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(network)?;
    let response = client.get(parsed).send().await.map_err(network)?;
    let status = response.status();
    if !status.is_success() {
        return Err(CommandError::new(
            CommandErrorKind::Network,
            format!(
                "L'adresse répond {} : le contrat n'a pas pu être lu.",
                status.as_u16()
            ),
        ));
    }
    let bytes = response.bytes().await.map_err(network)?;
    if bytes.len() > MAX_DOWNLOAD {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Le contrat dépasse 10 Mo.",
        ));
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Le contrat n'est pas un texte UTF-8.",
        )
    })
}

/// Ajoute un contrat collé, lu depuis une adresse ou créé vide (C2 à C4).
#[tauri::command]
pub async fn contract_add(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    source: ContractSource,
) -> Result<ContractView, CommandError> {
    let root = root(&state, project)?;
    let text = match source {
        ContractSource::Text { text } => text,
        ContractSource::Url { url } => download(&url).await?,
        ContractSource::Template { name } => {
            let name = crate::tree::item_name(&name)?;
            file::template(&name)
        }
    };
    let path = file::add(&root, &text)?;
    changed(&window, project, &[ProjectScope::Contracts], Some(&path));
    file::view(&root, &path)
}

/// Importe un fichier choisi dans le sélecteur natif (C1), ou remplace la source
/// du contrat `replace` (C9). `None` si l'utilisateur annule.
///
/// Le plugin dialog n'est appelé que depuis Rust : aucune permission WebView.
#[tauri::command]
pub async fn contract_import_file(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    replace: Option<String>,
) -> Result<Option<ContractView>, CommandError> {
    let root = root(&state, project)?;
    let picked = window
        .dialog()
        .file()
        .set_title(if replace.is_some() {
            "Remplacer le contrat par un fichier"
        } else {
            "Importer un contrat OpenAPI"
        })
        .add_filter("Contrat OpenAPI", &["yaml", "yml", "json"])
        .set_parent(&window)
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let source = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("Le fichier choisi n'est pas accessible : {error}."),
        )
    })?;
    let text = read_file(&source)?;
    let path = match replace {
        Some(path) => {
            file::save(&root, &path, &text)?;
            path
        }
        None => file::add(&root, &text)?,
    };
    changed(&window, project, &[ProjectScope::Contracts], Some(&path));
    file::view(&root, &path).map(Some)
}

fn read_file(path: &Path) -> Result<String, CommandError> {
    std::fs::read_to_string(path)
        .map_err(|error| CommandError::io("Lecture du fichier impossible", path, &error))
}

/// Enregistre la source modifiée d'un contrat, si elle reste lisible (C9, R10).
#[tauri::command]
pub fn contract_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    text: String,
) -> Result<ContractView, CommandError> {
    let root = root(&state, project)?;
    file::save(&root, &path, &text)?;
    changed(&window, project, &[ProjectScope::Contracts], Some(&path));
    file::view(&root, &path)
}

/// Met un contrat à la corbeille et ferme ses onglets partout (C11).
#[tauri::command]
pub fn contract_delete(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    permanent: bool,
) -> Result<(), CommandError> {
    file::delete(&root(&state, project)?, &path, permanent)?;
    let _ = windows::apply(window.app_handle(), |w: &mut Windows| {
        w.close_path(project, &path);
        Ok(())
    });
    changed(&window, project, &[ProjectScope::Contracts], Some(&path));
    Ok(())
}

/// Crée les requêtes des opérations choisies (C6). `environment` reçoit
/// `baseUrl` si elle est vide (R8).
#[tauri::command]
pub fn contract_generate(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    operations: Vec<String>,
    environment: Option<String>,
) -> Result<GenerateReport, CommandError> {
    let report = file::generate(
        &root(&state, project)?,
        &path,
        &operations,
        environment.as_deref(),
    )?;
    let mut scopes = vec![ProjectScope::Tree, ProjectScope::Contracts];
    if report.base_url.is_some() {
        scopes.push(ProjectScope::Environments);
    }
    changed(&window, project, &scopes, Some(&path));
    Ok(report)
}

/// Exécute les requêtes liées au contrat (C10, R12).
#[tauri::command]
pub fn contract_run(
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    path: String,
    environment: String,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    bench.start_contract_run(&ctx, &path, &environment, confirmed)
}
