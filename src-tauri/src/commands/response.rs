//! Commandes du corps de la réponse (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis) : façades
//! de [`crate::response`], plus l'enregistrement dans un fichier et l'effacement du résultat.

use std::sync::Mutex;

use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::commands::requests::share;
use crate::error::{CommandError, CommandErrorKind};
use crate::response::{self, FilterResult, HtmlNode, RenderedBody, ResponseFormat, file_name};
use crate::results::{RequestResult, ResultBook};

/// Corps mis en forme dans `format`, ou dans le format détecté (R1 à R3).
#[tauri::command]
pub fn response_render(
    body: String,
    headers: Vec<(String, String)>,
    format: Option<ResponseFormat>,
) -> RenderedBody {
    response::render(&body, &headers, format)
}

/// Aperçu d'un corps HTML, sans script ni ressource (R4).
#[tauri::command]
pub fn response_preview(body: String) -> Vec<HtmlNode> {
    response::preview(&body)
}

/// Filtre JSONPath du corps (R5 à R7).
#[tauri::command]
pub fn response_filter(body: String, expression: String) -> FilterResult {
    response::filter(&body, &expression)
}

/// Enregistre le corps dans un fichier choisi dans le sélecteur natif (R8) ; renvoie le chemin
/// écrit, ou `None` si l'utilisateur annule. Le plugin dialog n'est appelé que depuis Rust.
#[tauri::command]
pub async fn response_save(
    window: WebviewWindow,
    body: String,
    name: String,
    format: ResponseFormat,
    truncated: bool,
) -> Result<Option<String>, CommandError> {
    let picked = window
        .dialog()
        .file()
        .set_title(if truncated {
            "Enregistrer le corps de la réponse (tronqué à 5 Mo)"
        } else {
            "Enregistrer le corps de la réponse"
        })
        .set_file_name(file_name(&name, format))
        .add_filter(
            match format {
                ResponseFormat::Json => "JSON",
                ResponseFormat::Xml => "XML",
                ResponseFormat::Html => "HTML",
                ResponseFormat::Text => "Texte",
            },
            &[format.extension()],
        )
        .set_parent(&window)
        .blocking_save_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let target = picked.into_path().map_err(|error| {
        CommandError::new(
            CommandErrorKind::Io,
            format!("L'emplacement choisi n'est pas accessible : {error}."),
        )
    })?;
    // Écriture bloquante hors du fil asynchrone de la commande.
    let path = target.clone();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&path, body.as_bytes()))
        .await
        .map_err(|error| {
            CommandError::new(
                CommandErrorKind::Io,
                format!("Enregistrement du corps interrompu : {error}."),
            )
        })?
        .map_err(|error| CommandError::io("Enregistrement du corps impossible", &target, &error))?;
    Ok(Some(target.display().to_string()))
}

/// Efface la réponse affichée d'une requête dans toutes les fenêtres (R9).
#[tauri::command]
pub fn response_clear(
    window: WebviewWindow,
    results: State<'_, Mutex<ResultBook>>,
    project: Uuid,
    path: String,
) -> Result<RequestResult, CommandError> {
    let result = results
        .lock()
        .map_err(|_| {
            CommandError::new(
                CommandErrorKind::Io,
                "Les résultats sont indisponibles : redémarrez l'application.",
            )
        })?
        .clear(project, &path);
    share(&window, result.clone());
    Ok(result)
}
