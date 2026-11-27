//! Recherche Ctrl+K (écrans 1.5-a et 1.5-b, `docs/plan-v2.md`, lot A). Les règles sont dans
//! [`crate::search`] ; la commande rassemble ce que le cœur sait des projets du workspace et le
//! garde le temps que la palette reste ouverte (`refresh` le relit, à l'ouverture).

use std::sync::Mutex;

use tauri::State;
use uuid::Uuid;

use crate::catalog::snapshot::ProjectStatus;
use crate::contract;
use crate::environment;
use crate::error::{CommandError, CommandErrorKind};
use crate::search::{self, ProjectSource, SearchResults};
use crate::state::AppState;
use crate::tree;

/// Projets lus pour la recherche, par portée (`None` : tous les projets).
#[derive(Default)]
pub struct SearchCache(Mutex<Option<(Option<Uuid>, Vec<ProjectSource>)>>);

/// Ce que le cœur sait des projets de la portée : arborescence, contrats, environnements. Un
/// projet introuvable est ignoré.
fn sources(state: &AppState, workspace: Option<Uuid>) -> Result<Vec<ProjectSource>, CommandError> {
    let snapshot = state.snapshot()?;
    Ok(snapshot
        .projects
        .iter()
        .filter(|p| p.status == ProjectStatus::Ok)
        .filter(|p| workspace.is_none_or(|w| p.workspace_ids.contains(&w)))
        .filter_map(|p| {
            let ctx = state.project_ctx(p.id).ok()?;
            Some(ProjectSource {
                id: p.id,
                name: p.name.clone(),
                nodes: tree::read(&ctx.root).unwrap_or_default(),
                contracts: contract::file::list(&ctx.root),
                environments: environment::list(&ctx.root)
                    .into_iter()
                    .map(|e| e.name)
                    .collect(),
            })
        })
        .collect())
}

/// Résultats de `text` dans les projets du workspace `workspace` (tous si absent), groupés
/// (R1 à R3). `refresh` relit les projets ; sinon, ceux lus à la dernière ouverture servent.
#[tauri::command]
pub fn search_query(
    state: State<'_, AppState>,
    cache: State<'_, SearchCache>,
    workspace: Option<Uuid>,
    text: String,
    refresh: bool,
) -> Result<SearchResults, CommandError> {
    let mut cached = cache
        .0
        .lock()
        .map_err(|_| CommandError::new(CommandErrorKind::Io, "Recherche indisponible."))?;
    let stale = !matches!(cached.as_ref(), Some((scope, _)) if *scope == workspace);
    if refresh || stale {
        *cached = Some((workspace, sources(&state, workspace)?));
    }
    let projects = cached
        .as_ref()
        .map(|(_, p)| p.as_slice())
        .unwrap_or_default();
    Ok(search::search(projects, &text))
}
