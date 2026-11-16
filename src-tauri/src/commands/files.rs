//! Domaine « fichiers modifiés hors d'Arnod » (lot I, `docs/plan-mvp-i-fichiers-externes.md`) :
//! conflit entre le brouillon d'une requête et son fichier, réglé élément par élément.

use std::collections::BTreeMap;
use std::time::UNIX_EPOCH;

use tauri::{Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::request::model::Request;
use crate::request::{file, inherit};
use crate::state::AppState;
use crate::tree;
use crate::watch::{self, conflict};
use crate::windows::{self, ProjectScope};

/// Éléments où le fichier de la requête et le brouillon diffèrent (R9, R10). `base` est la
/// version de départ du brouillon (la dernière lue ou enregistrée).
#[tauri::command]
pub fn request_conflict(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    base: Request,
    draft: Request,
) -> Result<conflict::RequestConflict, CommandError> {
    let root = state.project_ctx(project)?.root;
    let absolute = tree::resolve(&root, &path)?;
    let disk = file::read(&absolute)?;
    let modified_at = absolute
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok());
    Ok(conflict::RequestConflict {
        items: conflict::compare(&base, &disk, &draft),
        modified_at,
    })
}

/// Règle le conflit (R11) : le brouillon, avec les éléments choisis « disque » pris du fichier,
/// est écrit une seule fois. Renvoie la requête relue, héritage compris.
#[tauri::command]
pub fn request_conflict_resolve(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    draft: Request,
    choices: BTreeMap<String, conflict::ConflictSide>,
) -> Result<Request, CommandError> {
    let root = state.project_ctx(project)?.root;
    let absolute = tree::resolve(&root, &path)?;
    let disk = file::read(&absolute)?;
    // Cette version du fichier est celle que l'on règle : elle peut être remplacée.
    watch::acknowledge(&absolute);
    let merged = conflict::merge(&disk, &draft, &choices)
        .map_err(|reason| CommandError::new(CommandErrorKind::ChangedOnDisk, reason))?;
    file::write(&absolute, &merged)?;
    windows::project_changed(
        window.app_handle(),
        window.label(),
        project,
        ProjectScope::Request,
        Some(path.as_str()),
    );
    let mut request = file::read(&absolute)?;
    watch::acknowledge(&absolute);
    request.inherited = inherit::resolve(&root, &path);
    Ok(request)
}
