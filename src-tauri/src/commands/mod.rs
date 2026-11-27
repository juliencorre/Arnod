//! Commandes Tauri exposées à l'interface, un module par domaine.
//!
//! Chaque commande ajoutée ici doit aussi être :
//! - enregistrée dans `generate_handler!` (`src/lib.rs`) ;
//! - décrite dans `src/types/ipc.ts` et exposée par `src/lib/ipc.ts`.
//!
//! Les commandes sont des façades : la logique vit dans [`crate::catalog`] et
//! [`crate::state`].

pub mod batch;
pub mod catalog;
pub mod contracts;
pub mod environments;
pub mod hello;
pub mod projects;
pub mod requests;
pub mod runs;
pub mod steps;
pub mod transfer;
pub mod tree;
pub mod trial;
pub mod updates;
pub mod window;
pub mod workspaces;
// Lot A - Réponse (docs/plan-mvp-a-reponse.md).
pub mod response;
// Lot B : saisie, texte et cURL.
pub mod input;
// Lot C - Import Bruno.
pub mod import;
// Lot E : tests et étapes.
pub mod test_steps;
// Lot H - Réseau et authentification.
pub mod network;
// Lot I - Fichiers modifiés hors d'Arnod.
pub mod files;
// Lot G - Démarrage et erreurs.
pub mod startup;
// Version 1 (docs/plan-v1.md) : préférences d'interface et historique des envois.
pub mod history;
pub mod interface;
// Version 2, lot A : recherche Ctrl+K.
pub mod search;
// Version 2, lot D : « Copier la commande ».
pub mod cli;
// Version 2, lots E et F : synchronisation Git.
pub mod sync;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::ts_contract::union_members;

    /// Commandes enregistrées dans `generate_handler!` (`src/lib.rs`).
    fn registered_commands() -> BTreeSet<String> {
        let lib = include_str!("../lib.rs");
        let start = lib
            .find("generate_handler![")
            .expect("`generate_handler!` absent de src/lib.rs");
        let block = &lib[start..];
        let block = &block[..block.find(']').unwrap()];
        block
            .split(',')
            .filter_map(|entry| entry.trim().rsplit("::").next())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn commandes_enregistrees_identiques_cote_typescript() {
        let registered = registered_commands();
        assert!(registered.contains("catalog_load"), "{registered:?}");
        assert_eq!(registered, union_members("CommandName"));
    }
}
// Version 4 (docs/plan-v4.md) : tests en schéma, jeux de données, templates.
pub mod datasets;
pub mod flows;
pub mod templates;
