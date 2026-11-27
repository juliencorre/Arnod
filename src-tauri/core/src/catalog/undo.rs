//! Pile d'annulation en mémoire (R6).
//!
//! Retirer un projet d'un workspace, supprimer un workspace ou oublier un
//! projet (ses fichiers restent sur le disque) se fait sans
//! confirmation : l'interface propose « Annuler » dans une notification. La pile
//! ne vit que le temps de la session ; au-delà de [`UndoStack::CAPACITY`]
//! actions, les plus anciennes ne sont plus annulables.

use std::collections::VecDeque;

use uuid::Uuid;

use super::model::{Project, Workspace};
use crate::error::{CommandError, CommandErrorKind};

/// Ce qu'il faut rétablir pour annuler une action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoAction {
    /// Rétablit un workspace supprimé à sa place, et le rouvre s'il était ouvert.
    RestoreWorkspace {
        workspace: Workspace,
        index: usize,
        was_last: bool,
    },
    /// Remet un projet retiré à sa place dans son workspace.
    RestoreProject {
        workspace_id: Uuid,
        project_id: Uuid,
        index: usize,
    },
    /// Réinscrit un projet oublié à sa place dans le référentiel et dans ses
    /// workspaces (`(workspace, position)`).
    RestoreForgottenProject {
        project: Project,
        index: usize,
        memberships: Vec<(Uuid, usize)>,
    },
}

/// Actions annulables de la session, chacune repérée par un jeton.
#[derive(Debug, Default)]
pub struct UndoStack {
    entries: VecDeque<(Uuid, UndoAction)>,
}

impl UndoStack {
    /// Nombre d'actions annulables conservées.
    pub const CAPACITY: usize = 20;

    /// Mémorise une action et renvoie son jeton d'annulation.
    pub fn push(&mut self, action: UndoAction) -> Uuid {
        if self.entries.len() == Self::CAPACITY {
            self.entries.pop_front();
        }
        let token = Uuid::new_v4();
        self.entries.push_back((token, action));
        token
    }

    /// Action associée à un jeton encore valable.
    pub fn get(&self, token: Uuid) -> Result<&UndoAction, CommandError> {
        self.entries
            .iter()
            .find(|(candidate, _)| *candidate == token)
            .map(|(_, action)| action)
            .ok_or_else(|| {
                CommandError::new(
                    CommandErrorKind::UndoExpired,
                    "Cette action ne peut plus être annulée.",
                )
            })
    }

    /// Oublie une action, une fois annulée.
    pub fn remove(&mut self, token: Uuid) {
        self.entries.retain(|(candidate, _)| *candidate != token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(index: usize) -> UndoAction {
        UndoAction::RestoreProject {
            workspace_id: Uuid::nil(),
            project_id: Uuid::nil(),
            index,
        }
    }

    #[test]
    fn retrouve_puis_oublie_une_action() {
        let mut stack = UndoStack::default();
        let token = stack.push(action(3));
        assert_eq!(stack.get(token).unwrap(), &action(3));

        stack.remove(token);

        let error = stack.get(token).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::UndoExpired);
        assert_eq!(error.message, "Cette action ne peut plus être annulée.");
    }

    #[test]
    fn jeton_inconnu_expire() {
        let stack = UndoStack::default();
        assert!(stack.get(Uuid::new_v4()).is_err());
    }

    #[test]
    fn oublie_les_actions_les_plus_anciennes_au_dela_de_la_capacite() {
        let mut stack = UndoStack::default();
        let tokens: Vec<Uuid> = (0..=UndoStack::CAPACITY)
            .map(|index| stack.push(action(index)))
            .collect();

        assert!(stack.get(tokens[0]).is_err());
        assert_eq!(stack.get(tokens[1]).unwrap(), &action(1));
        assert_eq!(
            stack.get(tokens[UndoStack::CAPACITY]).unwrap(),
            &action(UndoStack::CAPACITY)
        );
    }
}
