//! Diffusion aux fenêtres des changements faits hors d'Arnod (R3 à R6).
//!
//! Pour chaque lot : `arnod://fichiers` d'abord (les fenêtres déplacent les brouillons des
//! requêtes renommées et ferment les onglets des requêtes supprimées), puis les onglets, derniers
//! résultats et statuts suivent les renommages, et enfin `arnod://projet` pour chaque portée
//! touchée, comme l'interconnexion entre fenêtres (origine vide : le cœur en est l'auteur).

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use super::FilesChange;
use super::service::Sink;
use crate::layout::windows::Windows;
use crate::results::ResultBook;
use crate::state::AppState;
use crate::windows::{self, ProjectScope};
use crate::workbench::Workbench;

/// Événement des fichiers modifiés hors d'Arnod (`FilesChange`).
pub const FILES_EVENT: &str = "arnod://fichiers";

/// Diffuse les changements aux fenêtres de l'application.
pub struct AppSink<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AppSink<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    fn scope(&self, change: &FilesChange, scope: ProjectScope, path: Option<&str>) {
        windows::project_changed(&self.app, "", change.project_id, scope, path);
    }
}

impl<R: Runtime> Sink for AppSink<R> {
    fn changed(&self, change: FilesChange) {
        let app = &self.app;
        let project = change.project_id;
        let _ = app.emit(FILES_EVENT, &change);
        let ctx = app.state::<AppState>().project_ctx(project).ok();
        for renamed in &change.renamed {
            let _ = windows::apply(app, |w: &mut Windows| {
                w.rename_path(project, &renamed.from, &renamed.to);
                Ok(())
            });
            if let Ok(mut book) = app.state::<Mutex<ResultBook>>().lock() {
                book.rename(project, &renamed.from, &renamed.to);
            }
            if let Some(ctx) = &ctx {
                app.state::<Arc<Workbench>>()
                    .rename_status(ctx, &renamed.from, &renamed.to);
            }
        }
        if let Some(ctx) = &ctx {
            for path in &change.removed {
                app.state::<Arc<Workbench>>().remove_status(ctx, path);
            }
        }
        if change.contracts {
            self.scope(&change, ProjectScope::Contracts, None);
        }
        if change.environments {
            self.scope(&change, ProjectScope::Environments, None);
        }
        if change.tree {
            self.scope(&change, ProjectScope::Tree, None);
        }
        for path in &change.modified {
            self.scope(&change, ProjectScope::Request, Some(path));
        }
    }
}
