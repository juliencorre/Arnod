//! Aide aux tests : lecture des types miroirs de `src/types/ipc.ts`.
//!
//! Les types échangés sont écrits à la main des deux côtés ; ces tests
//! vérifient que les unions TypeScript concordent avec le Rust.

use std::collections::BTreeSet;

/// Types IPC côté interface, lus à la compilation des tests.
pub const IPC_TS: &str = include_str!("../../src/types/ipc.ts");

/// Chaînes de l'union `export type <name> = "a" | "b";` de `src/types/ipc.ts`.
///
/// Les commentaires de la déclaration ne doivent contenir ni guillemet droit
/// ni point-virgule.
pub fn union_members(name: &str) -> BTreeSet<String> {
    let header = format!("export type {name} =");
    let start = IPC_TS
        .find(&header)
        .unwrap_or_else(|| panic!("`{name}` absent de src/types/ipc.ts"));
    let declaration = &IPC_TS[start + header.len()..];
    let end = declaration
        .find(';')
        .unwrap_or_else(|| panic!("`{name}` sans point-virgule final"));
    declaration[..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Valeur sérialisée en JSON d'une variante d'énumération (`"camelCase"`).
pub fn serialized_name(value: impl serde::Serialize) -> String {
    match serde_json::to_value(value).unwrap() {
        serde_json::Value::String(name) => name,
        other => panic!("chaîne attendue, obtenu {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_une_union_de_chaines() {
        let members = union_members("ProjectStatus");
        assert_eq!(
            members,
            BTreeSet::from(["ok".to_owned(), "missing".to_owned()])
        );
    }
}
