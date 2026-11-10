//! Derniers résultats des requêtes envoyées (`docs/plan-resultat-detachable.md`, R5, R6).
//!
//! Module pur : le cœur garde, pour la session, le dernier envoi de chaque
//! requête (en cours, réponse ou erreur) afin que toutes les fenêtres le
//! montrent, dont la fenêtre de résultat.

use std::collections::VecDeque;

use serde::Serialize;
use uuid::Uuid;

use crate::error::{CommandError, CommandErrorKind};
use crate::workbench::SendOutcome;

/// Nombre de résultats gardés : les corps de réponse pèsent jusqu'à 5 Mo (R5).
pub const MAX_RESULTS: usize = 50;

/// Dernier envoi d'une requête, diffusé par `arnod://resultat`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestResult {
    pub project_id: Uuid,
    pub path: String,
    /// Un envoi est en cours.
    pub sending: bool,
    /// Dernière réponse reçue.
    pub outcome: Option<SendOutcome>,
    /// Message du dernier échec (réseau, délai, variable inconnue…).
    pub error: Option<String>,
    /// Réception de la réponse, en millisecondes depuis l'époque Unix.
    pub received_at: Option<u64>,
}

/// Résultat diffusé aux fenêtres, avec la fenêtre d'origine qui l'a déjà appliqué.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultChange {
    pub result: RequestResult,
    pub origin: String,
}

/// Résultats, du plus récent au plus ancien.
#[derive(Debug, Default)]
pub struct ResultBook {
    entries: VecDeque<RequestResult>,
}

impl ResultBook {
    pub fn get(&self, project_id: Uuid, path: &str) -> Option<&RequestResult> {
        self.entries
            .iter()
            .find(|e| e.project_id == project_id && e.path == path)
    }

    fn take(&mut self, project_id: Uuid, path: &str) -> Option<RequestResult> {
        let index = self
            .entries
            .iter()
            .position(|e| e.project_id == project_id && e.path == path)?;
        self.entries.remove(index)
    }

    fn put(&mut self, result: RequestResult) -> RequestResult {
        self.entries.push_front(result.clone());
        self.entries.truncate(MAX_RESULTS);
        result
    }

    /// Début d'un envoi : la réponse précédente reste affichée en attendant.
    pub fn start(&mut self, project_id: Uuid, path: &str) -> RequestResult {
        let mut result = self.take(project_id, path).unwrap_or(RequestResult {
            project_id,
            path: path.to_owned(),
            sending: false,
            outcome: None,
            error: None,
            received_at: None,
        });
        result.sending = true;
        result.error = None;
        self.put(result)
    }

    /// Fin d'un envoi. Annulé : la réponse précédente reste (R6).
    pub fn finish(
        &mut self,
        project_id: Uuid,
        path: &str,
        sent: &Result<SendOutcome, CommandError>,
        now_ms: u64,
    ) -> RequestResult {
        let mut result = self.start(project_id, path);
        result.sending = false;
        match sent {
            Ok(outcome) => {
                result.outcome = Some(outcome.clone());
                result.received_at = Some(now_ms);
            }
            Err(error) if error.kind == CommandErrorKind::Cancelled => {}
            Err(error) => {
                result.outcome = None;
                result.received_at = None;
                result.error = Some(error.message.clone());
            }
        }
        self.take(project_id, path);
        self.put(result)
    }

    /// Une requête renommée ou déplacée garde son résultat.
    pub fn rename(&mut self, project_id: Uuid, from: &str, to: &str) {
        for entry in &mut self.entries {
            if entry.project_id == project_id && entry.path == from {
                to.clone_into(&mut entry.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpResponse;

    const P: Uuid = Uuid::from_u128(7);

    fn outcome(status: u16) -> SendOutcome {
        SendOutcome {
            response: HttpResponse {
                status,
                status_text: "OK".into(),
                headers: Vec::new(),
                body: "{}".into(),
                size_bytes: 2,
                truncated: false,
                wait_ms: 1,
                receive_ms: 1,
                total_ms: 2,
            },
            assertions: Vec::new(),
            passed: true,
            slow: false,
        }
    }

    #[test]
    fn un_envoi_reussi_garde_la_reponse() {
        let mut book = ResultBook::default();
        let started = book.start(P, "a.yml");
        assert!(started.sending);
        assert!(started.outcome.is_none());

        let done = book.finish(P, "a.yml", &Ok(outcome(201)), 1_000);

        assert!(!done.sending);
        assert_eq!(done.outcome.map(|o| o.response.status), Some(201));
        assert_eq!(done.received_at, Some(1_000));
        assert_eq!(book.get(P, "a.yml").map(|r| r.sending), Some(false));
    }

    #[test]
    fn la_reponse_precedente_reste_pendant_l_envoi_et_apres_une_annulation() {
        let mut book = ResultBook::default();
        book.finish(P, "a.yml", &Ok(outcome(200)), 1);
        let sending = book.start(P, "a.yml");
        assert!(sending.sending);
        assert!(sending.outcome.is_some());

        let cancelled = CommandError::new(CommandErrorKind::Cancelled, "Annulé.");
        let after = book.finish(P, "a.yml", &Err(cancelled), 2);

        assert!(!after.sending);
        assert_eq!(after.outcome.map(|o| o.response.status), Some(200));
        assert_eq!(after.received_at, Some(1));
    }

    #[test]
    fn un_echec_remplace_la_reponse_par_son_message() {
        let mut book = ResultBook::default();
        book.finish(P, "a.yml", &Ok(outcome(200)), 1);
        let failed = CommandError::new(CommandErrorKind::Network, "Hôte introuvable.");

        let after = book.finish(P, "a.yml", &Err(failed), 2);

        assert!(after.outcome.is_none());
        assert_eq!(after.error.as_deref(), Some("Hôte introuvable."));
        assert!(book.start(P, "a.yml").error.is_none());
    }

    #[test]
    fn garde_au_plus_cinquante_resultats_les_plus_recents() {
        let mut book = ResultBook::default();
        for i in 0..=MAX_RESULTS {
            book.finish(P, &format!("{i}.yml"), &Ok(outcome(200)), 1);
        }
        assert!(book.get(P, "0.yml").is_none());
        assert!(book.get(P, &format!("{MAX_RESULTS}.yml")).is_some());
        assert_eq!(book.entries.len(), MAX_RESULTS);
    }

    #[test]
    fn une_requete_renommee_garde_son_resultat() {
        let mut book = ResultBook::default();
        book.finish(P, "a.yml", &Ok(outcome(200)), 1);

        book.rename(P, "a.yml", "dossier/b.yml");

        assert!(book.get(P, "a.yml").is_none());
        assert!(book.get(P, "dossier/b.yml").is_some());
    }

    #[test]
    fn forme_json_du_resultat() {
        let mut book = ResultBook::default();
        let result = book.start(P, "a.yml");
        let json = serde_json::to_value(ResultChange {
            result,
            origin: "main".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "result": {
                    "projectId": P.to_string(),
                    "path": "a.yml",
                    "sending": true,
                    "outcome": null,
                    "error": null,
                    "receivedAt": null
                },
                "origin": "main"
            })
        );
    }
}
