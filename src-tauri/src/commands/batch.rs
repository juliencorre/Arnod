//! Domaine « sélection » (écran 4.1-a quater) : déplacer, dupliquer, exécuter ou mettre à la
//! corbeille plusieurs éléments de l'arborescence d'un coup, avec « Annuler ».

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::commands::tree::follow_path;
use crate::error::{CommandError, CommandErrorKind};
use crate::layout::windows::Windows;
use crate::state::AppState;
use crate::tree::undo::{Undo, UndoBook};
use crate::tree::{self, batch};
use crate::watch::Renamed;
use crate::windows::{self, ProjectScope};
use crate::workbench::{RunView, Workbench};

/// Résultat d'une action sur la sélection, et le jeton de « Annuler ».
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    /// Chemins d'avant et d'après (absent après une mise à la corbeille).
    pub items: Vec<(String, Option<String>)>,
    pub undo_token: Uuid,
}

/// Actions récentes, à annuler : projet et fichiers d'avant.
static PENDING: Mutex<UndoBook<(Uuid, Undo)>> = Mutex::new(UndoBook::new());

fn changed(window: &WebviewWindow, project: Uuid) {
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Tree,
        None,
    );
}

/// Applique `action` aux éléments `paths` du projet : les onglets suivent un déplacement, ceux
/// d'un élément mis à la corbeille se ferment.
#[tauri::command]
pub fn tree_batch(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    paths: Vec<String>,
    action: batch::BatchAction,
) -> Result<BatchResult, CommandError> {
    let ctx = state.project_ctx(project)?;
    let (done, undo) = batch::apply(&ctx.root, &paths, &action)?;
    let (mut renamed, mut removed) = (Vec::new(), Vec::new());
    for (from, to) in &done.items {
        match to {
            Some(to) if matches!(action, batch::BatchAction::Duplicate) => {
                // La copie d'un test garde son schéma (`docs/plan-v4.md`, R4).
                let _ = crate::flow::store::copy(&ctx.root, from, to);
            }
            Some(to) if matches!(action, batch::BatchAction::Move { .. }) => {
                follow_path(&window, project, from, to);
                renamed.push(Renamed {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            None => {
                bench.remove_status(&ctx, from);
                let _ = windows::apply(window.app_handle(), |w: &mut Windows| {
                    w.close_path(project, from);
                    Ok(())
                });
                removed.push(from.clone());
            }
            Some(_) => {}
        }
    }
    windows::paths_changed(
        window.app_handle(),
        window.label(),
        project,
        renamed,
        removed,
    );
    let undo_token = PENDING
        .lock()
        .map(|mut book| book.push((project, undo)))
        .unwrap_or_else(|_| Uuid::new_v4());
    Ok(BatchResult {
        items: done.items,
        undo_token,
    })
}

/// « Annuler » une action sur la sélection.
#[tauri::command]
pub fn tree_batch_undo(window: WebviewWindow, undo_token: Uuid) -> Result<(), CommandError> {
    let entry = PENDING
        .lock()
        .ok()
        .and_then(|mut book| book.take(undo_token));
    let Some((project, undo)) = entry else {
        return Err(CommandError::new(
            CommandErrorKind::UndoExpired,
            "Cette action ne peut plus être annulée.",
        ));
    };
    undo.apply()?;
    changed(&window, project);
    Ok(())
}

/// Exécute les requêtes de la sélection (requêtes choisies et contenu des dossiers choisis).
#[tauri::command]
pub fn run_selection(
    window: WebviewWindow,
    state: State<'_, AppState>,
    bench: State<'_, Arc<Workbench>>,
    project: Uuid,
    paths: Vec<String>,
    environment: String,
    confirmed: bool,
) -> Result<RunView, CommandError> {
    let ctx = state.project_ctx(project)?;
    let mut only = Vec::new();
    for path in batch::normalize(&paths) {
        if tree::resolve(&ctx.root, &path)?.is_dir() {
            only.extend(tree::requests_in(&ctx.root, &path)?);
        } else {
            only.push(path);
        }
    }
    let run = bench.start_run(&ctx, "", &environment, Some(only), confirmed)?;
    windows::run_started(&window, project, &run);
    Ok(run)
}
