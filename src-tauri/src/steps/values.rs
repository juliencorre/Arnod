//! Valeurs d'un essai, vérifiées contre le formulaire de la nature (R3).

use std::collections::BTreeMap;

use super::definition::{Definition, FieldKind};

/// Valeurs retenues : champs visibles seulement, options vérifiées, défauts appliqués.
pub fn normalize(
    definition: &Definition,
    values: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for field in &definition.form {
        if let Some(condition) = &field.visible_when {
            let current = out.get(&condition.key);
            if !current.is_some_and(|value| condition.values.contains(value)) {
                continue;
            }
        }
        let given = values.get(&field.key).map(|v| v.trim_end().to_owned());
        let value = match field.kind {
            FieldKind::Choice => {
                let value = given
                    .filter(|v| !v.is_empty())
                    .or_else(|| field.default.clone())
                    .or_else(|| field.options.first().cloned())
                    .unwrap_or_default();
                // Même option, quelle que soit la casse saisie.
                field
                    .options
                    .iter()
                    .find(|o| o.eq_ignore_ascii_case(&value))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "« {value} » n'est pas une valeur possible de « {} »",
                            field.label
                        )
                    })?
            }
            FieldKind::Text | FieldKind::Multiline => {
                given.or_else(|| field.default.clone()).unwrap_or_default()
            }
        };
        out.insert(field.key.clone(), value);
    }
    Ok(out)
}
