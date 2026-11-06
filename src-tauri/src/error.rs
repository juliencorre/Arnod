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
    /// Le projet est déjà inscrit au référentiel.
    AlreadyRegistered,
    /// Le dossier ne contient pas de manifeste `arnod.yml`.
    NotAProject,
    /// Projet ou workspace introuvable.
    NotFound,
    /// Action interdite sur le workspace par défaut.
    DefaultWorkspace,
    /// Le jeton d'annulation n'est plus valable.
    UndoExpired,
    /// Erreur d'entrée-sortie : disque, droits, chemin.
    Io,
    /// Le fichier d'état de l'application est illisible.
    CorruptState,
    /// Le fichier d'état a été écrit par une version plus récente d'Arnod.
    NewerVersion,
}

/// Erreur renvoyée par une commande : une nature et un message prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Nature de l'erreur.
    pub kind: CommandErrorKind,
    /// Message en français, destiné à l'utilisateur.
    pub message: String,
}

impl CommandError {
    /// Construit une erreur de nature `kind` avec son message.
    pub fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
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

    /// Toutes les natures, à compléter à chaque ajout de variante.
    const ALL_KINDS: [CommandErrorKind; 11] = [
        CommandErrorKind::InvalidName,
        CommandErrorKind::NameTaken,
        CommandErrorKind::FolderNotEmpty,
        CommandErrorKind::AlreadyRegistered,
        CommandErrorKind::NotAProject,
        CommandErrorKind::NotFound,
        CommandErrorKind::DefaultWorkspace,
        CommandErrorKind::UndoExpired,
        CommandErrorKind::Io,
        CommandErrorKind::CorruptState,
        CommandErrorKind::NewerVersion,
    ];

    /// Miroir TypeScript des types IPC, lu à la compilation des tests.
    const IPC_TS: &str = include_str!("../../src/types/ipc.ts");

    fn serialized_kind(kind: CommandErrorKind) -> String {
        match serde_json::to_value(kind).unwrap() {
            serde_json::Value::String(name) => name,
            other => panic!("nature sérialisée en {other}, chaîne attendue"),
        }
    }

    /// Natures déclarées dans `export type CommandErrorKind = ...;` côté TypeScript.
    fn typescript_kinds() -> BTreeSet<String> {
        let start = IPC_TS
            .find("export type CommandErrorKind =")
            .expect("`CommandErrorKind` absent de src/types/ipc.ts");
        let declaration = &IPC_TS[start..];
        let declaration = &declaration[..declaration.find(';').unwrap()];
        declaration
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
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
        assert_eq!(rust, typescript_kinds());
    }

    #[test]
    fn affichage_du_message() {
        let error = CommandError::new(CommandErrorKind::Io, "Disque plein.");
        assert_eq!(error.to_string(), "Disque plein.");
    }
}
