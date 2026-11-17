//! Sortie d'Arnod sans perdre un brouillon (plan qualité, R6, IN3).
//!
//! Fermer la fenêtre principale ou « Fichier › Quitter » ne quitte plus aussitôt : le cœur
//! demande à chaque fenêtre d'enregistrer (`arnod://quitter`), puis quitte quand toutes ont
//! répondu oui. Une fenêtre qui garde un brouillon non enregistré répond non : la sortie est
//! abandonnée et la fenêtre dit pourquoi. Une fenêtre qui ne répond pas (vue bloquée) ne retient
//! pas Arnod au-delà de [`ANSWER_DELAY`].
//!
//! Ce module est pur : il tient la demande en cours ; `windows.rs` émet, attend et quitte.

use std::collections::BTreeSet;
use std::time::Duration;

/// Délai laissé aux fenêtres pour enregistrer avant de quitter quand même.
pub const ANSWER_DELAY: Duration = Duration::from_secs(10);

/// Suite d'une réponse ou d'une demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// D'autres fenêtres doivent encore répondre.
    Wait,
    /// Toutes ont enregistré : quitter.
    Exit,
    /// Une fenêtre garde un brouillon : la sortie est abandonnée.
    Cancelled,
    /// Aucune demande en cours (réponse tardive, déjà abandonnée) : rien à faire.
    Ignored,
}

/// Demande de sortie en cours : les fenêtres qui n'ont pas encore répondu.
#[derive(Debug, Default)]
pub struct QuitGate {
    pending: Option<BTreeSet<String>>,
    /// Numéro de la demande, pour qu'un délai écoulé ne vise que la sienne.
    round: u64,
}

impl QuitGate {
    /// Nouvelle demande auprès de `windows` ; `None` si une demande est déjà en cours (rien à
    /// émettre), sinon son numéro.
    pub fn start(&mut self, windows: impl IntoIterator<Item = String>) -> Option<u64> {
        if self.pending.is_some() {
            return None;
        }
        self.round += 1;
        self.pending = Some(windows.into_iter().collect());
        Some(self.round)
    }

    /// Réponse de la fenêtre `label` : `saved` vaut faux si un brouillon reste.
    pub fn answer(&mut self, label: &str, saved: bool) -> Step {
        let Some(pending) = self.pending.as_mut() else {
            return Step::Ignored;
        };
        if !saved {
            self.pending = None;
            return Step::Cancelled;
        }
        pending.remove(label);
        if pending.is_empty() {
            self.pending = None;
            Step::Exit
        } else {
            Step::Wait
        }
    }

    /// Délai écoulé pour la demande `round` : quitter si elle attend encore (fenêtre muette).
    pub fn expire(&mut self, round: u64) -> Step {
        if self.round != round || self.pending.is_none() {
            return Step::Ignored;
        }
        self.pending = None;
        Step::Exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn quitte_quand_toutes_les_fenetres_ont_enregistre() {
        let mut gate = QuitGate::default();
        assert_eq!(gate.start(labels(&["main", "fenetre-1"])), Some(1));
        assert_eq!(gate.answer("main", true), Step::Wait);
        assert_eq!(gate.answer("fenetre-1", true), Step::Exit);
        assert_eq!(gate.answer("fenetre-1", true), Step::Ignored);
    }

    #[test]
    fn un_brouillon_non_enregistre_abandonne_la_sortie() {
        let mut gate = QuitGate::default();
        let round = gate.start(labels(&["main", "fenetre-1"])).unwrap();
        assert_eq!(gate.answer("fenetre-1", false), Step::Cancelled);
        // Plus rien en cours : la réponse suivante et le délai sont sans effet.
        assert_eq!(gate.answer("main", true), Step::Ignored);
        assert_eq!(gate.expire(round), Step::Ignored);
        // Une nouvelle demande repart de zéro.
        assert_eq!(gate.start(labels(&["main"])), Some(round + 1));
    }

    #[test]
    fn une_seule_demande_a_la_fois() {
        let mut gate = QuitGate::default();
        assert!(gate.start(labels(&["main"])).is_some());
        assert_eq!(gate.start(labels(&["main"])), None);
    }

    #[test]
    fn une_fenetre_muette_ne_retient_pas_arnod() {
        let mut gate = QuitGate::default();
        let first = gate.start(labels(&["main", "fenetre-1"])).unwrap();
        assert_eq!(gate.answer("main", true), Step::Wait);
        assert_eq!(gate.expire(first), Step::Exit);
        // Le délai d'une demande abandonnée ne vise pas la suivante.
        let second = gate.start(labels(&["main"])).unwrap();
        assert_eq!(gate.expire(first), Step::Ignored);
        assert_eq!(gate.answer("main", true), Step::Exit);
        assert_eq!(gate.expire(second), Step::Ignored);
    }
}
