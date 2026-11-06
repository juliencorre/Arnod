//! Catalogue : référentiel des projets et workspaces (règles R1 à R11 du plan).
//!
//! Module pur : ni Tauri, ni disque. Chaque méthode applique une règle et
//! renvoie une [`CommandError`] prête à afficher si elle est enfreinte.
//! L'écriture sur disque et la création des dossiers relèvent de
//! [`crate::storage`], [`crate::manifest`] et [`crate::state`].

pub mod model;
pub mod names;
pub mod snapshot;
pub mod undo;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use self::model::{CatalogFile, Project, Workspace, WorkspaceColor};
use self::snapshot::{CatalogSnapshot, ProjectStatus, ProjectView, WorkspaceView};
use self::undo::UndoAction;
use crate::error::{CommandError, CommandErrorKind};

/// Nom du workspace créé au premier lancement (W1).
pub const DEFAULT_WORKSPACE_NAME: &str = "Mes projets";

/// Version courante du format `arnod.json`.
pub const FILE_VERSION: u32 = 1;

/// Référentiel des projets et workspaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    projects: Vec<Project>,
    workspaces: Vec<Workspace>,
    default_workspace_id: Uuid,
    last_workspace_id: Uuid,
}

impl Catalog {
    /// Catalogue du premier lancement : « Mes projets », gris, par défaut et ouvert (W1).
    pub fn first_launch() -> Self {
        let id = Uuid::new_v4();
        Self {
            projects: Vec::new(),
            workspaces: vec![Workspace {
                id,
                name: DEFAULT_WORKSPACE_NAME.to_owned(),
                color: WorkspaceColor::Gray,
                project_ids: Vec::new(),
            }],
            default_workspace_id: id,
            last_workspace_id: id,
        }
    }

    /// Reconstruit un catalogue lu sur disque.
    ///
    /// Les incohérences sans perte sont réparées : références à des projets
    /// inconnus ou en double, dernier workspace disparu (R7). Un fichier qui ne
    /// décrit pas un catalogue valide (identifiants en double, workspace par
    /// défaut absent) est refusé, avec la raison.
    pub fn from_file(file: CatalogFile) -> Result<Self, String> {
        let mut project_ids = HashSet::new();
        for project in &file.projects {
            if !project_ids.insert(project.id) {
                return Err(format!("projet {} en double", project.id));
            }
        }
        let mut workspace_ids = HashSet::new();
        for workspace in &file.workspaces {
            if !workspace_ids.insert(workspace.id) {
                return Err(format!("workspace {} en double", workspace.id));
            }
        }
        if !workspace_ids.contains(&file.default_workspace_id) {
            return Err("workspace par défaut absent".to_owned());
        }

        let mut workspaces = file.workspaces;
        for workspace in &mut workspaces {
            let mut seen = HashSet::new();
            workspace
                .project_ids
                .retain(|id| project_ids.contains(id) && seen.insert(*id));
        }
        let last_workspace_id = if workspace_ids.contains(&file.last_workspace_id) {
            file.last_workspace_id
        } else {
            file.default_workspace_id
        };

        Ok(Self {
            projects: file.projects,
            workspaces,
            default_workspace_id: file.default_workspace_id,
            last_workspace_id,
        })
    }

    /// Contenu à écrire dans `arnod.json`.
    pub fn to_file(&self) -> CatalogFile {
        CatalogFile {
            version: FILE_VERSION,
            projects: self.projects.clone(),
            workspaces: self.workspaces.clone(),
            default_workspace_id: self.default_workspace_id,
            last_workspace_id: self.last_workspace_id,
        }
    }

    pub fn default_workspace_id(&self) -> Uuid {
        self.default_workspace_id
    }

    pub fn last_workspace_id(&self) -> Uuid {
        self.last_workspace_id
    }

    /// Projet du référentiel.
    pub fn project(&self, id: Uuid) -> Result<&Project, CommandError> {
        self.projects
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(project_not_found)
    }

    /// Workspace existant.
    pub fn workspace(&self, id: Uuid) -> Result<&Workspace, CommandError> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == id)
            .ok_or_else(workspace_not_found)
    }

    fn workspace_mut(&mut self, id: Uuid) -> Result<&mut Workspace, CommandError> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
            .ok_or_else(workspace_not_found)
    }

    /// Vérifie, sans rien modifier, qu'un projet peut être inscrit dans
    /// `workspace_id` avec ce nom et ce dossier (R1, R9, R10). Renvoie le nom
    /// normalisé. À appeler avant de créer le dossier sur disque.
    pub fn check_new_project(
        &self,
        raw_name: &str,
        path: &Path,
        workspace_id: Uuid,
    ) -> Result<String, CommandError> {
        let name = names::project_name(raw_name)?;
        self.workspace(workspace_id)?;
        if let Some(existing) = self
            .projects
            .iter()
            .find(|project| names::same_name(&project.name, &name))
        {
            return Err(CommandError::new(
                CommandErrorKind::NameTaken,
                format!(
                    "Un projet « {} » existe déjà dans le référentiel.",
                    existing.name
                ),
            ));
        }
        if let Some(existing) = self
            .projects
            .iter()
            .find(|project| same_path(&project.path, path))
        {
            return Err(CommandError::new(
                CommandErrorKind::AlreadyRegistered,
                format!("Ce dossier est déjà celui du projet « {} ».", existing.name),
            ));
        }
        Ok(name)
    }

    /// Inscrit un projet au référentiel et l'ajoute au workspace par défaut ainsi
    /// qu'à `workspace_id` s'il est différent (R1, R4).
    pub fn register_project(
        &mut self,
        id: Uuid,
        raw_name: &str,
        path: PathBuf,
        workspace_id: Uuid,
    ) -> Result<(), CommandError> {
        let name = self.check_new_project(raw_name, &path, workspace_id)?;
        if self.projects.iter().any(|project| project.id == id) {
            return Err(CommandError::new(
                CommandErrorKind::AlreadyRegistered,
                "Ce projet est déjà inscrit au référentiel.",
            ));
        }
        self.projects.push(Project { id, name, path });
        for target in [self.default_workspace_id, workspace_id] {
            let workspace = self.workspace_mut(target)?;
            if !workspace.project_ids.contains(&id) {
                workspace.project_ids.push(id);
            }
        }
        Ok(())
    }

    /// Crée un workspace vide, avec la première couleur libre de la palette (W5).
    pub fn create_workspace(&mut self, raw_name: &str) -> Result<Uuid, CommandError> {
        let name = names::workspace_name(raw_name)?;
        self.ensure_workspace_name_free(&name, None)?;
        let id = Uuid::new_v4();
        let color = self.next_color();
        self.workspaces.push(Workspace {
            id,
            name,
            color,
            project_ids: Vec::new(),
        });
        Ok(id)
    }

    /// Renomme un workspace (W7, R9).
    pub fn rename_workspace(&mut self, id: Uuid, raw_name: &str) -> Result<(), CommandError> {
        let name = names::workspace_name(raw_name)?;
        self.ensure_workspace_name_free(&name, Some(id))?;
        self.workspace_mut(id)?.name = name;
        Ok(())
    }

    /// Change la couleur d'un workspace (W8).
    pub fn set_workspace_color(
        &mut self,
        id: Uuid,
        color: WorkspaceColor,
    ) -> Result<(), CommandError> {
        self.workspace_mut(id)?.color = color;
        Ok(())
    }

    /// Transfère le rôle de workspace par défaut (W9, R3).
    pub fn set_default_workspace(&mut self, id: Uuid) -> Result<(), CommandError> {
        self.workspace(id)?;
        self.default_workspace_id = id;
        Ok(())
    }

    /// Mémorise le workspace ouvert, rouvert au prochain démarrage (W2, R7).
    pub fn open_workspace(&mut self, id: Uuid) -> Result<(), CommandError> {
        self.workspace(id)?;
        self.last_workspace_id = id;
        Ok(())
    }

    /// Ajoute des projets du référentiel à un workspace ; ceux qui y sont déjà
    /// sont ignorés (W6, P6). Rien n'est ajouté si un projet est inconnu.
    pub fn add_projects(
        &mut self,
        workspace_id: Uuid,
        project_ids: &[Uuid],
    ) -> Result<(), CommandError> {
        for id in project_ids {
            self.project(*id)?;
        }
        let workspace = self.workspace_mut(workspace_id)?;
        for id in project_ids {
            if !workspace.project_ids.contains(id) {
                workspace.project_ids.push(*id);
            }
        }
        Ok(())
    }

    /// Retire un projet d'un workspace ; le projet reste au référentiel (P7, R5).
    pub fn remove_project(
        &mut self,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<UndoAction, CommandError> {
        let workspace = self.workspace_mut(workspace_id)?;
        let index = workspace
            .project_ids
            .iter()
            .position(|id| *id == project_id)
            .ok_or_else(|| {
                CommandError::new(
                    CommandErrorKind::NotFound,
                    "Ce projet n'est pas dans ce workspace.",
                )
            })?;
        workspace.project_ids.remove(index);
        Ok(UndoAction::RestoreProject {
            workspace_id,
            project_id,
            index,
        })
    }

    /// Supprime un workspace ; ses projets restent au référentiel et dans leurs
    /// autres workspaces (W11, R3, R5). S'il était ouvert, le workspace par
    /// défaut devient le workspace ouvert (R8).
    pub fn delete_workspace(&mut self, id: Uuid) -> Result<UndoAction, CommandError> {
        if id == self.default_workspace_id {
            return Err(CommandError::new(
                CommandErrorKind::DefaultWorkspace,
                "Le workspace par défaut ne peut pas être supprimé : définissez d'abord un autre workspace par défaut.",
            ));
        }
        let index = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == id)
            .ok_or_else(workspace_not_found)?;
        let workspace = self.workspaces.remove(index);
        let was_last = self.last_workspace_id == id;
        if was_last {
            self.last_workspace_id = self.default_workspace_id;
        }
        Ok(UndoAction::RestoreWorkspace {
            workspace,
            index,
            was_last,
        })
    }

    /// Annule une action de la pile d'annulation (R6). Sans effet si elle est
    /// déjà annulée.
    pub fn undo(&mut self, action: UndoAction) -> Result<(), CommandError> {
        match action {
            UndoAction::RestoreWorkspace {
                mut workspace,
                index,
                was_last,
            } => {
                if self.workspace(workspace.id).is_ok() {
                    return Ok(());
                }
                self.ensure_workspace_name_free(&workspace.name, None)
                    .map_err(|_| {
                        CommandError::new(
                            CommandErrorKind::NameTaken,
                            format!(
                                "Impossible d'annuler : un autre workspace s'appelle déjà « {} ».",
                                workspace.name
                            ),
                        )
                    })?;
                workspace
                    .project_ids
                    .retain(|id| self.projects.iter().any(|project| project.id == *id));
                let id = workspace.id;
                let index = index.min(self.workspaces.len());
                self.workspaces.insert(index, workspace);
                if was_last {
                    self.last_workspace_id = id;
                }
            }
            UndoAction::RestoreProject {
                workspace_id,
                project_id,
                index,
            } => {
                self.project(project_id)?;
                let workspace = self.workspace_mut(workspace_id)?;
                if !workspace.project_ids.contains(&project_id) {
                    let index = index.min(workspace.project_ids.len());
                    workspace.project_ids.insert(index, project_id);
                }
            }
        }
        Ok(())
    }

    /// Instantané pour l'interface. `folder_exists` indique si le dossier d'un
    /// projet existe (P10) ; il est injecté pour garder ce module sans disque.
    pub fn snapshot(&self, folder_exists: impl Fn(&Path) -> bool) -> CatalogSnapshot {
        let workspaces = self.sorted_workspaces();
        let projects = self
            .projects
            .iter()
            .map(|project| ProjectView {
                id: project.id,
                name: project.name.clone(),
                path: project.path.display().to_string(),
                status: if folder_exists(&project.path) {
                    ProjectStatus::Ok
                } else {
                    ProjectStatus::Missing
                },
                workspace_ids: workspaces
                    .iter()
                    .filter(|workspace| workspace.project_ids.contains(&project.id))
                    .map(|workspace| workspace.id)
                    .collect(),
            })
            .collect();
        let workspaces = workspaces
            .into_iter()
            .map(|workspace| WorkspaceView {
                id: workspace.id,
                name: workspace.name.clone(),
                color: workspace.color,
                is_default: workspace.id == self.default_workspace_id,
                project_ids: workspace.project_ids.clone(),
            })
            .collect();
        CatalogSnapshot {
            projects,
            workspaces,
            default_workspace_id: self.default_workspace_id,
            last_workspace_id: self.last_workspace_id,
        }
    }

    /// Workspace par défaut d'abord, puis les autres par nom (R11).
    fn sorted_workspaces(&self) -> Vec<&Workspace> {
        let mut sorted: Vec<&Workspace> = self.workspaces.iter().collect();
        sorted.sort_by_cached_key(|workspace| {
            (
                workspace.id != self.default_workspace_id,
                names::sort_key(&workspace.name),
            )
        });
        sorted
    }

    fn ensure_workspace_name_free(
        &self,
        name: &str,
        except: Option<Uuid>,
    ) -> Result<(), CommandError> {
        match self.workspaces.iter().find(|workspace| {
            Some(workspace.id) != except && names::same_name(&workspace.name, name)
        }) {
            Some(existing) => Err(CommandError::new(
                CommandErrorKind::NameTaken,
                format!("Un workspace « {} » existe déjà.", existing.name),
            )),
            None => Ok(()),
        }
    }

    /// Première couleur qu'aucun workspace n'utilise, sinon rotation dans la palette.
    fn next_color(&self) -> WorkspaceColor {
        let order = WorkspaceColor::ASSIGNMENT_ORDER;
        order
            .into_iter()
            .find(|color| !self.workspaces.iter().any(|w| w.color == *color))
            .unwrap_or(order[self.workspaces.len() % order.len()])
    }
}

/// Deux chemins désignent-ils le même dossier ? Sous Windows, la casse est ignorée.
fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

fn project_not_found() -> CommandError {
    CommandError::new(
        CommandErrorKind::NotFound,
        "Ce projet n'existe plus dans le référentiel.",
    )
}

fn workspace_not_found() -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, "Ce workspace n'existe plus.")
}

#[cfg(test)]
mod tests;
