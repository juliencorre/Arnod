//! Erreur commune des commandes Tauri.
//!
//! Toute commande qui peut échouer renvoie `Result<T, CommandError>`. Côté
//! interface, la promesse d'`invoke` est alors rejetée avec l'objet sérialisé
//! `{ "kind": "...", "message": "..." }` : `kind` permet de réagir selon le cas,
//! `message` est un texte en français prêt à afficher.
//!
//! Miroir TypeScript : `CommandError` et `CommandErrorKind` dans
//! `src/types/ipc.ts`. Toute modification ici doit être reportée de l'autre
//! côté ; un test vérifie que les deux listes de natures concordent.

use std::fmt;
use std::io;
use std::path::Path;

use serde::Serialize;

/// Nature d'une erreur de commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorKind {
    /// Nom vide, trop long ou inutilisable comme nom de dossier.
    InvalidName,
    /// Nom déjà utilisé (comparaison sans tenir compte de la casse).
    NameTaken,
    /// Le dossier cible existe déjà et n'est pas vide.
    FolderNotEmpty,
    /// Le projet est déjà dans « Tous les projets ».
    AlreadyRegistered,
    /// Le dossier ne contient pas de manifeste `arnod.yml`.
    NotAProject,
    /// Projet ou workspace introuvable.
    NotFound,
    /// Le jeton d'annulation n'est plus valable.
    UndoExpired,
    /// Erreur d'entrée-sortie : disque, droits, chemin.
    Io,
    /// Le fichier d'état de l'application est illisible.
    CorruptState,
    /// Le fichier d'état a été écrit par une version plus récente d'Arnod.
    NewerVersion,
    /// Une variable `{{nom}}` n'existe pas dans l'environnement actif.
    UnknownVariable,
    /// Fichier de requête ou de dossier invalide, ou opération impossible dessus.
    InvalidRequest,
    /// Échec réseau : hôte introuvable, connexion refusée, certificat.
    Network,
    /// Délai dépassé.
    Timeout,
    /// Exécution annulée par l'utilisateur.
    Cancelled,
    /// Le coffre du système est indisponible.
    SecretStore,
    /// Envoi sur un environnement protégé sans confirmation.
    ProtectedEnvironment,
}

/// Erreur renvoyée par une commande : une nature et un message prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Nature de l'erreur.
    pub kind: CommandErrorKind,
    /// Message en français, destiné à l'utilisateur.
    pub message: String,
    /// Lot H : échec réseau classé (cause, certificat, proxy), absent du JSON s'il est vide
    /// (`docs/plan-mvp-h-reseau-auth.md`, 5.1-i).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Box<crate::network::failure::NetworkFailure>>,
}

impl CommandError {
    /// Construit une erreur de nature `kind` avec son message.
    pub fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            failure: None,
        }
    }

    /// Lot H : joint l'échec réseau classé.
    #[must_use]
    pub fn with_failure(mut self, failure: crate::network::failure::NetworkFailure) -> Self {
        self.failure = Some(Box::new(failure));
        self
    }

    /// Erreur d'entrée-sortie : `action` décrit ce qui a échoué
    /// (« Création du dossier impossible »), `path` le fichier ou dossier visé.
    pub fn io(action: &str, path: &Path, error: &io::Error) -> Self {
        Self::new(
            CommandErrorKind::Io,
            format!("{action} ({}) : {error}", path.display()),
        )
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ts_contract::{serialized_name, union_members};

    /// Toutes les natures, à compléter à chaque ajout de variante.
    const ALL_KINDS: [CommandErrorKind; 17] = [
        CommandErrorKind::InvalidName,
        CommandErrorKind::NameTaken,
        CommandErrorKind::FolderNotEmpty,
        CommandErrorKind::AlreadyRegistered,
        CommandErrorKind::NotAProject,
        CommandErrorKind::NotFound,
        CommandErrorKind::UndoExpired,
        CommandErrorKind::Io,
        CommandErrorKind::CorruptState,
        CommandErrorKind::NewerVersion,
        CommandErrorKind::UnknownVariable,
        CommandErrorKind::InvalidRequest,
        CommandErrorKind::Network,
        CommandErrorKind::Timeout,
        CommandErrorKind::Cancelled,
        CommandErrorKind::SecretStore,
        CommandErrorKind::ProtectedEnvironment,
    ];

    fn serialized_kind(kind: CommandErrorKind) -> String {
        serialized_name(kind)
    }

    #[test]
    fn serialisation_json_en_camel_case() {
        let error = CommandError::new(
            CommandErrorKind::NameTaken,
            "Le nom « Boutique » est déjà utilisé.",
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "kind": "nameTaken",
                "message": "Le nom « Boutique » est déjà utilisé."
            })
        );
    }

    #[test]
    fn natures_en_camel_case() {
        assert_eq!(
            serialized_kind(CommandErrorKind::InvalidName),
            "invalidName"
        );
        assert_eq!(serialized_kind(CommandErrorKind::Io), "io");
        assert_eq!(
            serialized_kind(CommandErrorKind::NotAProject),
            "notAProject"
        );
    }

    #[test]
    fn natures_identiques_cote_typescript() {
        let rust: BTreeSet<String> = ALL_KINDS.into_iter().map(serialized_kind).collect();
        assert_eq!(
            rust.len(),
            ALL_KINDS.len(),
            "nature en double dans ALL_KINDS"
        );
        assert_eq!(rust, union_members("CommandErrorKind"));
    }

    #[test]
    fn erreur_d_entree_sortie_avec_action_et_chemin() {
        let cause = io::Error::new(io::ErrorKind::PermissionDenied, "accès refusé");
        let error = CommandError::io(
            "Création du dossier impossible",
            Path::new("/dev/boutique"),
            &cause,
        );
        assert_eq!(error.kind, CommandErrorKind::Io);
        assert_eq!(
            error.message,
            "Création du dossier impossible (/dev/boutique) : accès refusé"
        );
    }

    #[test]
    fn affichage_du_message() {
        let error = CommandError::new(CommandErrorKind::Io, "Disque plein.");
        assert_eq!(error.to_string(), "Disque plein.");
    }
}
