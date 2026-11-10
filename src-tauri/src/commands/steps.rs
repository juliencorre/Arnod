//! Domaine « étapes » : catalogue des natures d'étape et reconnaissance de ce
//! que l'on colle (docs/plan-accueil-non-classes.md, K1, A2).

use crate::steps::{self, Detection, StepCatalog};

/// Natures disponibles sur ce poste (K1).
#[tauri::command]
pub fn steps_catalog() -> StepCatalog {
    steps::catalog()
}

/// Nature reconnue de `input`, ou `null` (A2, R2).
#[tauri::command]
pub fn steps_detect(input: String) -> Option<Detection> {
    steps::detect(&input)
}
