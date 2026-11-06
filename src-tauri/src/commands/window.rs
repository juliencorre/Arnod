//! Domaine « fenêtre » : titre de la fenêtre principale.
//!
//! L'interface sait ce qui est affiché (workspace, projet, gestion) : elle
//! demande le titre à chaque changement de vue. Appelé depuis Rust, le
//! changement de titre ne demande aucune permission WebView.

use tauri::WebviewWindow;

use crate::error::{CommandError, CommandErrorKind};

/// Nom de l'application, seul titre quand rien n'est ouvert.
pub const APP_TITLE: &str = "Arnod";

/// Titre « Arnod — <sujet> » (ex. « Arnod — Boutique »), ou « Arnod » sans sujet.
pub fn window_title(subject: Option<&str>) -> String {
    match subject.map(str::trim).filter(|subject| !subject.is_empty()) {
        Some(subject) => format!("{APP_TITLE} — {subject}"),
        None => APP_TITLE.to_owned(),
    }
}

/// Met à jour le titre de la fenêtre avec le workspace ou le projet affiché.
#[tauri::command]
pub fn window_set_title(
    window: WebviewWindow,
    subject: Option<String>,
) -> Result<(), CommandError> {
    window
        .set_title(&window_title(subject.as_deref()))
        .map_err(|error| {
            CommandError::new(
                CommandErrorKind::Io,
                format!("Impossible de changer le titre de la fenêtre : {error}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titre_avec_ou_sans_sujet() {
        assert_eq!(window_title(Some("Boutique")), "Arnod — Boutique");
        assert_eq!(
            window_title(Some("  boutique-tests ")),
            "Arnod — boutique-tests"
        );
        assert_eq!(window_title(Some("   ")), "Arnod");
        assert_eq!(window_title(None), "Arnod");
    }
}
