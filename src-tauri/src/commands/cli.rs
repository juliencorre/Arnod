//! « Copier la commande » (V2, lot D, LC6, écran 5.3-c) : la commande `arnod run` d'un test ou
//! d'un dossier, avec l'environnement actif, citée pour le terminal du poste. La règle vit dans
//! [`crate::command`].

use tauri::State;
use uuid::Uuid;

use crate::command::{self, Shell};
use crate::error::{CommandError, CommandErrorKind};
use crate::state::AppState;
use crate::tree;

/// Commande d'un dossier ou d'un test du projet (`path` vide : tout le projet).
pub fn command_for(
    root: &std::path::Path,
    path: &str,
    environment: Option<&str>,
    shell: Shell,
) -> Result<String, CommandError> {
    let full = tree::resolve(root, path)?;
    if !full.is_dir() {
        return Err(CommandError::new(
            CommandErrorKind::NotFound,
            "Ce dossier ou ce test n'existe plus.",
        ));
    }
    Ok(command::run_command(path, environment, shell))
}

#[tauri::command]
pub fn cli_command(
    state: State<'_, AppState>,
    project: Uuid,
    path: String,
    environment: Option<String>,
    shell: Option<Shell>,
) -> Result<String, CommandError> {
    let root = state.project_ctx(project)?.root;
    command_for(
        &root,
        &path,
        environment.as_deref(),
        shell.unwrap_or_else(Shell::of_system),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lc6_commande_d_un_dossier_existant() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("commandes/v2")).unwrap();
        assert_eq!(
            command_for(dir.path(), "commandes/v2", Some("Recette"), Shell::Sh).unwrap(),
            "arnod run \"commandes/v2\" --env \"Recette\""
        );
        let error = command_for(dir.path(), "absent", None, Shell::Sh).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::NotFound);
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["kind"], "notFound");
    }
}
