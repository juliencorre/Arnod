//! Commande `arnod run` d'un test ou d'un dossier (`docs/plan-v2.md`, lot D, LC6, écran 5.3-c) :
//! « Copier la commande » du menu ⋯. Elle se lance depuis le dossier du projet, avec
//! l'environnement actif ; chaque valeur est citée pour le terminal du poste. Module pur.

use serde::Deserialize;

#[cfg(test)]
mod tests;

/// Terminal pour lequel la commande est citée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CliShell"))]
#[serde(rename_all = "camelCase")]
pub enum Shell {
    /// PowerShell (Windows).
    PowerShell,
    /// sh, bash, zsh (Linux).
    Sh,
}

impl Shell {
    /// Terminal habituel du système qui exécute Arnod.
    pub fn of_system() -> Self {
        if cfg!(windows) {
            Self::PowerShell
        } else {
            Self::Sh
        }
    }
}

/// Valeur citée : entre guillemets doubles quand rien ne s'y interprète (« commandes/v2 »),
/// sinon entre apostrophes, la seule forme sans substitution des deux terminaux.
pub fn quote(value: &str, shell: Shell) -> String {
    let special: &[char] = match shell {
        Shell::PowerShell => &['"', '$', '`', '\u{201c}', '\u{201d}', '\u{201e}'],
        Shell::Sh => &['"', '$', '`', '\\', '!'],
    };
    if !value.contains(special) {
        return format!("\"{value}\"");
    }
    match shell {
        // PowerShell : une apostrophe se double (et les apostrophes typographiques aussi).
        Shell::PowerShell => {
            let mut out = String::from("'");
            for c in value.chars() {
                if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}') {
                    out.push(c);
                }
                out.push(c);
            }
            out.push('\'');
            out
        }
        // sh : une apostrophe ferme, s'échappe, puis rouvre.
        Shell::Sh => format!("'{}'", value.replace('\'', "'\\''")),
    }
}

/// `arnod run "<cible>" --env "<environnement>"` ; sans environnement, `--env` est omis.
pub fn run_command(target: &str, environment: Option<&str>, shell: Shell) -> String {
    let mut command = String::from("arnod run");
    let target = target.trim_matches('/');
    if !target.is_empty() {
        command.push(' ');
        command.push_str(&quote(target, shell));
    }
    if let Some(environment) = environment.filter(|e| !e.trim().is_empty()) {
        command.push_str(" --env ");
        command.push_str(&quote(environment, shell));
    }
    command
}
