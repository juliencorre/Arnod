//! Commandes Tauri exposées à l'interface, un module par domaine.
//!
//! Chaque commande ajoutée ici doit aussi être :
//! - enregistrée dans `generate_handler!` (`src/lib.rs`) ;
//! - décrite dans `src/types/ipc.ts` et exposée par `src/lib/ipc.ts`.
//!
//! Les commandes sont des façades : la logique vit dans [`crate::catalog`] et
//! [`crate::state`].

pub mod catalog;
pub mod hello;
pub mod projects;
pub mod window;
pub mod workspaces;

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
