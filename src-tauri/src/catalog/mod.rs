//! Catalogue : « Tous les projets » (le référentiel) et workspaces.
//!
//! Règles R1, R2, R5, R6, R9 et R10 de `docs/plan-projets-workspaces.md`, R3, R4,
//! R7, R8, R11 et R12 révisées par `docs/plan-workspaces-onglets.md` : il n'y a
//! plus de workspace par défaut et un projet peut n'être dans aucun workspace.
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

use self::model::{CatalogFile, CatalogFileV1, Project, Workspace, WorkspaceColor};
use self::snapshot::{CatalogSnapshot, ProjectStatus, ProjectView, WorkspaceView};
use self::undo::UndoAction;
use crate::error::{CommandError, CommandErrorKind};

/// Version courante du format `arnod.json`.
pub const FILE_VERSION: u32 = 2;

/// « Tous les projets » et workspaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    projects: Vec<Project>,
    workspaces: Vec<Workspace>,
    /// Dernier onglet ouvert : un workspace, ou `None` pour « Tous les projets » (R7).
    last_workspace_id: Option<Uuid>,
}

impl Catalog {
    /// Catalogue du premier lancement : aucun projet, aucun workspace, onglet
    /// « Tous les projets » ouvert (R3, R7).
    pub fn first_launch() -> Self {
        Self {
            projects: Vec::new(),
            workspaces: Vec::new(),
            last_workspace_id: None,
        }
    }

    /// Migre un fichier version 1 (R12) : l'ancien workspace par défaut est
    /// retiré s'il contient tous les projets, car il doublait « Tous les
    /// projets » ; sinon il reste un workspace ordinaire.
    pub fn migrate_v1(file: CatalogFileV1) -> CatalogFile {
        let mut workspaces = file.workspaces;
        let redundant = workspaces
            .iter()
            .position(|workspace| workspace.id == file.default_workspace_id)
            .filter(|index| {
                file.projects
                    .iter()
                    .all(|project| workspaces[*index].project_ids.contains(&project.id))
            });
        if let Some(index) = redundant {
            workspaces.remove(index);
        }
        let last_workspace_id = Some(file.last_workspace_id)
            .filter(|id| workspaces.iter().any(|workspace| workspace.id == *id));
        CatalogFile {
            version: FILE_VERSION,
            projects: file.projects,
            workspaces,
            last_workspace_id,
        }
    }

    /// Reconstruit un catalogue lu sur disque.
    ///
    /// Les incohérences sans perte sont réparées : références à des projets
    /// inconnus ou en double, dernier workspace disparu (R7). Un fichier qui ne
    /// décrit pas un catalogue valide (identifiants en double) est refusé, avec
    /// la raison.
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
        let mut workspaces = file.workspaces;
        for workspace in &mut workspaces {
            let mut seen = HashSet::new();
            workspace
                .project_ids
                .retain(|id| project_ids.contains(id) && seen.insert(*id));
        }
        let last_workspace_id = file
            .last_workspace_id
            .filter(|id| workspace_ids.contains(id));

        Ok(Self {
            projects: file.projects,
            workspaces,
            last_workspace_id,
        })
    }

    /// Contenu à écrire dans `arnod.json`.
    pub fn to_file(&self) -> CatalogFile {
        CatalogFile {
            version: FILE_VERSION,
            projects: self.projects.clone(),
            workspaces: self.workspaces.clone(),
            last_workspace_id: self.last_workspace_id,
        }
    }

    /// Dernier onglet ouvert ; `None` pour « Tous les projets ».
    pub fn last_workspace_id(&self) -> Option<Uuid> {
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

    /// Vérifie, sans rien modifier, qu'un projet peut être inscrit, dans
    /// `workspace_id` s'il est donné, avec ce nom et ce dossier (R1, R9, R10).
    /// Renvoie le nom normalisé. À appeler avant de créer le dossier sur disque.
    pub fn check_new_project(
        &self,
        raw_name: &str,
        path: &Path,
        workspace_id: Option<Uuid>,
    ) -> Result<String, CommandError> {
        let name = names::project_name(raw_name)?;
        if let Some(workspace_id) = workspace_id {
            self.workspace(workspace_id)?;
        }
        if let Some(existing) = self
            .projects
            .iter()
            .find(|project| names::same_name(&project.name, &name))
        {
            return Err(CommandError::new(
                CommandErrorKind::NameTaken,
                format!(
                    "Un projet « {} » existe déjà dans « Tous les projets ».",
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

    /// Inscrit un projet dans « Tous les projets » et, si `workspace_id` est
    /// donné, dans ce seul workspace (R1, R4).
    pub fn register_project(
        &mut self,
        id: Uuid,
        raw_name: &str,
        path: PathBuf,
        workspace_id: Option<Uuid>,
    ) -> Result<(), CommandError> {
        // L'identifiant d'abord : réinscrire le même projet (P12) doit dire
        // « déjà inscrit », pas « nom déjà pris ».
        if self.projects.iter().any(|project| project.id == id) {
            return Err(CommandError::new(
                CommandErrorKind::AlreadyRegistered,
                "Ce projet est déjà dans « Tous les projets ».",
            ));
        }
        let name = self.check_new_project(raw_name, &path, workspace_id)?;
        self.projects.push(Project { id, name, path });
        if let Some(workspace_id) = workspace_id {
            self.workspace_mut(workspace_id)?.project_ids.push(id);
        }
        Ok(())
    }

    /// Crée un workspace vide, à la fin des onglets, avec la première couleur
    /// libre de la palette (W5, R11).
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

    /// Mémorise l'onglet ouvert, rouvert au prochain démarrage : un workspace,
    /// ou « Tous les projets » avec `None` (O2, R7).
    pub fn open_workspace(&mut self, id: Option<Uuid>) -> Result<(), CommandError> {
        if let Some(id) = id {
            self.workspace(id)?;
        }
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

    /// Oublie un projet : il quitte le référentiel et tous ses workspaces, ses
    /// fichiers restent sur le disque (P11, R6).
    pub fn forget_project(&mut self, project_id: Uuid) -> Result<UndoAction, CommandError> {
        let index = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
            .ok_or_else(project_not_found)?;
        let project = self.projects.remove(index);
        let memberships = self
            .workspaces
            .iter_mut()
            .filter_map(|workspace| {
                let position = workspace
                    .project_ids
                    .iter()
                    .position(|id| *id == project_id)?;
                workspace.project_ids.remove(position);
                Some((workspace.id, position))
            })
            .collect();
        Ok(UndoAction::RestoreForgottenProject {
            project,
            index,
            memberships,
        })
    }

    /// Supprime un workspace ; ses projets restent dans « Tous les projets » et
    /// dans leurs autres workspaces (O6, R3, R5). S'il était ouvert, « Tous les
    /// projets » devient l'onglet ouvert (R8).
    pub fn delete_workspace(&mut self, id: Uuid) -> Result<UndoAction, CommandError> {
        let index = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == id)
            .ok_or_else(workspace_not_found)?;
        let workspace = self.workspaces.remove(index);
        let was_last = self.last_workspace_id == Some(id);
        if was_last {
            self.last_workspace_id = None;
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
                    self.last_workspace_id = Some(id);
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
            UndoAction::RestoreForgottenProject {
                project,
                index,
                memberships,
            } => self.restore_forgotten(project, index, &memberships)?,
        }
        Ok(())
    }

    /// Réinscrit un projet oublié (annulation de P11), sauf si un autre projet a
    /// pris son nom ou son dossier entre-temps.
    fn restore_forgotten(
        &mut self,
        project: Project,
        index: usize,
        memberships: &[(Uuid, usize)],
    ) -> Result<(), CommandError> {
        if self.project(project.id).is_ok() {
            return Ok(());
        }
        if let Some(existing) = self
            .projects
            .iter()
            .find(|other| names::same_name(&other.name, &project.name))
        {
            return Err(CommandError::new(
                CommandErrorKind::NameTaken,
                format!(
                    "Impossible d'annuler : un autre projet s'appelle déjà « {} ».",
                    existing.name
                ),
            ));
        }
        if let Some(existing) = self
            .projects
            .iter()
            .find(|other| same_path(&other.path, &project.path))
        {
            return Err(CommandError::new(
                CommandErrorKind::AlreadyRegistered,
                format!(
                    "Impossible d'annuler : le projet « {} » occupe déjà ce dossier.",
                    existing.name
                ),
            ));
        }
        let project_id = project.id;
        let index = index.min(self.projects.len());
        self.projects.insert(index, project);
        for (workspace_id, position) in memberships {
            if let Ok(workspace) = self.workspace_mut(*workspace_id) {
                let position = (*position).min(workspace.project_ids.len());
                workspace.project_ids.insert(position, project_id);
            }
        }
        Ok(())
    }

    /// Instantané pour l'interface. `folder_exists` indique si le dossier d'un
    /// projet existe (P10) ; il est injecté pour garder ce module sans disque.
    pub fn snapshot(&self, folder_exists: impl Fn(&Path) -> bool) -> CatalogSnapshot {
        let workspaces = &self.workspaces;
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
            .iter()
            .map(|workspace| WorkspaceView {
                id: workspace.id,
                name: workspace.name.clone(),
                color: workspace.color,
                project_ids: workspace.project_ids.clone(),
            })
            .collect();
        CatalogSnapshot {
            projects,
            workspaces,
            last_workspace_id: self.last_workspace_id,
        }
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
        same_path_ignoring_case(a, b)
    } else {
        a == b
    }
}

/// Compare deux chemins composant par composant, sans tenir compte de la casse :
/// séparateurs répétés, `/` ou `\` et barre finale n'y changent rien.
fn same_path_ignoring_case(a: &Path, b: &Path) -> bool {
    let key = |path: &Path| -> Vec<String> {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    };
    key(a) == key(b)
}

fn project_not_found() -> CommandError {
    CommandError::new(
        CommandErrorKind::NotFound,
        "Ce projet n'est plus dans « Tous les projets ».",
    )
}

fn workspace_not_found() -> CommandError {
    CommandError::new(CommandErrorKind::NotFound, "Ce workspace n'existe plus.")
}

#[cfg(test)]
mod tests;
