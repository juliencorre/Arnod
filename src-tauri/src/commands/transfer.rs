//! Domaine « copier ou déplacer » une requête (lot J, écrans 4.1-f et 4.1-g) : destinations,
//! « Ce qui change », transfert et « Annuler ».

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::commands::tree::follow_path;
use crate::error::{CommandError, CommandErrorKind};
use crate::layout::windows::Windows;
use crate::state::AppState;
use crate::tree::transfer::{self, Change, Mode, Target, Targets};
use crate::tree::undo::Undo;
use crate::windows::{self, ProjectScope};

/// Requête copiée ou déplacée, et le jeton de « Annuler ».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferDone {
    /// Projet qui la reçoit.
    pub project_id: Uuid,
    pub path: String,
    pub name: String,
    pub undo_token: Uuid,
}

/// Transfert à annuler : projets touchés et fichiers d'avant.
type Pending = (Vec<Uuid>, Undo);

/// Transferts récents, à annuler (« Annuler » de la notification, Ctrl+Z).
fn pending() -> &'static Mutex<VecDeque<(Uuid, Pending)>> {
    static PENDING: std::sync::OnceLock<Mutex<VecDeque<(Uuid, Pending)>>> =
        std::sync::OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Nombre de transferts gardés pour « Annuler ».
const KEPT: usize = 20;

fn changed(window: &WebviewWindow, project: Uuid) {
    for scope in [ProjectScope::Tree, ProjectScope::Contracts] {
        windows::project_changed(window.app_handle(), window.label(), project, scope, None);
    }
}

/// Destinations de la requête `path` de `project` dans le projet `target` (4.1-g).
#[tauri::command]
pub fn transfer_targets(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    target: Uuid,
) -> Result<Targets, CommandError> {
    let from = state.project_ctx(project)?;
    let to = state.project_ctx(target)?;
    transfer::targets(&from.root, &path, &to.root, project == target)
}

/// « Ce qui change » si la requête va à `destination` du projet `target` (4.1-g).
#[tauri::command]
pub fn transfer_preview(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    target: Uuid,
    destination: Target,
    mode: Mode,
) -> Result<Vec<Change>, CommandError> {
    let from = state.project_ctx(project)?;
    let to = state.project_ctx(target)?;
    let name = (project != target)
        .then(|| state.project_name(target))
        .transpose()?;
    transfer::preview(
        &from.root,
        &path,
        &to.root,
        &destination,
        mode,
        name.as_deref(),
    )
}

/// Copie ou déplace la requête `path` de `project` vers `destination` du projet `target`. Ses
/// onglets la suivent (déplacée dans le projet) ou se ferment (déplacée ailleurs).
#[tauri::command]
pub fn transfer_apply(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    target: Uuid,
    destination: Target,
    mode: Mode,
) -> Result<TransferDone, CommandError> {
    let from = state.project_ctx(project)?;
    let to = state.project_ctx(target)?;
    let (done, undo) = transfer::transfer(&from.root, &path, &to.root, &destination, mode)?;
    if mode == Mode::Move && done.path != path {
        if project == target {
            follow_path(&window, project, &path, &done.path);
        } else {
            let _ = windows::apply(window.app_handle(), |w: &mut Windows| {
                w.close_path(project, &path);
                Ok(())
            });
        }
    }
    changed(&window, project);
    if target != project {
        changed(&window, target);
    }
    let undo_token = Uuid::new_v4();
    if let Ok(mut map) = pending().lock() {
        map.push_back((undo_token, (vec![project, target], undo)));
        while map.len() > KEPT {
            map.pop_front();
        }
    }
    Ok(TransferDone {
        project_id: target,
        path: done.path,
        name: done.name,
        undo_token,
    })
}

/// « Annuler » un transfert : les fichiers reviennent comme avant.
#[tauri::command]
pub fn transfer_undo(window: WebviewWindow, undo_token: Uuid) -> Result<(), CommandError> {
    let entry = pending().lock().ok().and_then(|mut map| {
        let index = map.iter().position(|(token, _)| *token == undo_token)?;
        map.remove(index).map(|(_, entry)| entry)
    });
    let Some((projects, undo)) = entry else {
        return Err(CommandError::new(
            CommandErrorKind::UndoExpired,
            "Cette copie ou ce déplacement ne peut plus être annulé.",
        ));
    };
    undo.apply()?;
    for project in projects {
        changed(&window, project);
    }
    Ok(())
}
