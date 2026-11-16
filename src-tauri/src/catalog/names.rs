//! Validation des noms de projets et de workspaces (règles R9 et R10).

use crate::error::{CommandError, CommandErrorKind};

/// Longueur maximale d'un nom, en caractères (R9).
pub const MAX_NAME_CHARS: usize = 64;

/// Caractères interdits dans un nom de dossier sous Windows (R10).
const FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Noms de périphériques réservés par Windows, avec ou sans extension (R10).
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Nom de workspace normalisé (R9) : espaces de bord retirés, 1 à 64 caractères,
/// sans caractère de contrôle.
pub fn workspace_name(raw: &str) -> Result<String, CommandError> {
    common_name(raw)
}

/// Nom de projet normalisé (R9), utilisable tel quel comme nom de dossier sous
/// Windows et Linux (R10).
pub fn project_name(raw: &str) -> Result<String, CommandError> {
    let name = common_name(raw)?;
    if let Some(forbidden) = name.chars().find(|c| FORBIDDEN_CHARS.contains(c)) {
        return Err(invalid(format!(
            "Le caractère « {forbidden} » n'est pas autorisé dans un nom de projet."
        )));
    }
    if name.ends_with('.') {
        return Err(invalid(
            "Un nom de projet ne peut pas se terminer par un point.",
        ));
    }
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    if is_reserved(stem) {
        return Err(invalid(format!(
            "« {stem} » est un nom réservé par Windows : choisissez-en un autre."
        )));
    }
    Ok(name)
}

/// Nom de périphérique réservé par Windows (R10), sans tenir compte de la casse.
pub fn is_reserved(stem: &str) -> bool {
    RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(stem))
}

/// Indique si deux noms sont identiques sans tenir compte de la casse (R9).
pub fn same_name(a: &str, b: &str) -> bool {
    sort_key(a) == sort_key(b)
}

/// Clé de comparaison et de tri d'un nom, insensible à la casse (R9, R11).
pub fn sort_key(name: &str) -> String {
    name.to_lowercase()
}

fn common_name(raw: &str) -> Result<String, CommandError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(invalid("Le nom ne peut pas être vide."));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(invalid(format!(
            "Le nom ne doit pas dépasser {MAX_NAME_CHARS} caractères."
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(invalid(
            "Le nom ne doit pas contenir de caractère de contrôle.",
        ));
    }
    Ok(name.to_owned())
}

fn invalid(message: impl Into<String>) -> CommandError {
    CommandError::new(CommandErrorKind::InvalidName, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid_message(result: Result<String, CommandError>) -> String {
        let error = result.unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidName);
        error.message
    }

    #[test]
    fn retire_les_espaces_de_bord() {
        assert_eq!(workspace_name("  Boutique  ").unwrap(), "Boutique");
        assert_eq!(project_name("\tboutique-tests ").unwrap(), "boutique-tests");
    }

    #[test]
    fn refuse_un_nom_vide() {
        assert_eq!(
            invalid_message(workspace_name("   ")),
            "Le nom ne peut pas être vide."
        );
        assert_eq!(
            invalid_message(project_name("")),
            "Le nom ne peut pas être vide."
        );
    }

    #[test]
    fn limite_a_64_caracteres_et_non_octets() {
        let accents = "é".repeat(MAX_NAME_CHARS);
        assert_eq!(workspace_name(&accents).unwrap(), accents);
        assert_eq!(
            invalid_message(workspace_name(&"a".repeat(MAX_NAME_CHARS + 1))),
            "Le nom ne doit pas dépasser 64 caractères."
        );
    }

    #[test]
    fn refuse_les_caracteres_de_controle() {
        assert_eq!(
            invalid_message(workspace_name("Bou\ntique")),
            "Le nom ne doit pas contenir de caractère de contrôle."
        );
    }

    #[test]
    fn workspace_accepte_les_caracteres_interdits_dans_un_dossier() {
        assert_eq!(
            workspace_name("Facturation & partenaires / v2").unwrap(),
            "Facturation & partenaires / v2"
        );
    }

    #[test]
    fn projet_refuse_les_caracteres_interdits_sous_windows() {
        for forbidden in FORBIDDEN_CHARS {
            let message = invalid_message(project_name(&format!("api{forbidden}v2")));
            assert!(message.contains(forbidden), "{message}");
        }
    }

    #[test]
    fn projet_refuse_un_point_final() {
        assert_eq!(
            invalid_message(project_name("boutique.")),
            "Un nom de projet ne peut pas se terminer par un point."
        );
        assert!(project_name("..").is_err());
    }

    #[test]
    fn projet_refuse_les_noms_reserves_de_windows() {
        for name in ["CON", "nul", "Com1", "lpt9", "aux.tests", "prn .txt"] {
            assert!(project_name(name).is_err(), "{name} devrait être refusé");
        }
        assert_eq!(
            invalid_message(project_name("con")),
            "« con » est un nom réservé par Windows : choisissez-en un autre."
        );
    }

    #[test]
    fn projet_accepte_les_noms_proches_des_noms_reserves() {
        for name in [
            "console",
            "com10",
            "auxiliaire",
            "boutique-tests",
            "API v2",
            ".config",
        ] {
            assert_eq!(project_name(name).unwrap(), name);
        }
    }

    #[test]
    fn compare_sans_tenir_compte_de_la_casse() {
        assert!(same_name("Boutique", "bOUTIQUE"));
        assert!(same_name("Écran", "écran"));
        assert!(!same_name("Boutique", "Boutiques"));
    }
}
