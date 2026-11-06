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
