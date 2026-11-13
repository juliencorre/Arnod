//! Commande cURL d'une requête prête à partir (5.1-b quater, R8 à R10).
//!
//! Les secrets ne sont jamais écrits : à leur place, une variable du shell
//! (`$TOKEN`), marquée dans le texte par [`placeholder`] avant l'écriture.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::CommandError;
use crate::input::{fill, written_names};
use crate::request::model::{AuthKind, Method, Request};
use crate::request::resolve::{Prepared, Variables, prepare};

/// Délimite le nom d'une variable du shell dans une valeur (caractère de contrôle,
/// absent d'un texte saisi).
const MARK: char = '\u{1}';

/// Marque de la variable du shell `name` (`TOKEN`), écrite `$TOKEN`.
pub fn placeholder(name: &str) -> String {
    format!("{MARK}{name}{MARK}")
}

/// Nom de variable du shell d'une variable d'Arnod : `apiKey` → `API_KEY`.
pub fn shell_name(variable: &str) -> String {
    let mut out = String::new();
    let mut previous_lower = false;
    for c in variable.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower {
                out.push('_');
            }
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
            out.push(c.to_ascii_uppercase());
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            previous_lower = false;
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.starts_with(|c: char| c.is_ascii_digit()) || out.is_empty() {
        format!("V_{out}")
    } else {
        out
    }
}

/// Remplace la valeur d'un en-tête `Authorization` écrit en clair (R9) : le
/// jeton d'un `Bearer` devient `$TOKEN`, toute autre valeur `$CREDENTIALS`.
pub fn hide_credentials(headers: &mut [(String, String)]) {
    for (name, value) in headers.iter_mut() {
        if !name.eq_ignore_ascii_case("Authorization") || value.contains(MARK) {
            continue;
        }
        *value = match value.split_once(' ') {
            Some((scheme, _)) if scheme.eq_ignore_ascii_case("Bearer") => {
                format!("{scheme} {}", placeholder("TOKEN"))
            }
            Some((scheme, _)) => format!("{scheme} {}", placeholder("CREDENTIALS")),
            None => placeholder("CREDENTIALS"),
        };
    }
}

/// Argument du shell : entre guillemets simples, ou doubles s'il contient une
/// variable (`"Authorization: Bearer $TOKEN"`).
pub fn quote(arg: &str) -> String {
    if !arg.contains(MARK) {
        return format!("'{}'", arg.replace('\'', "'\\''"));
    }
    let mut out = String::from("\"");
    let mut parts = arg.split(MARK).peekable();
    let mut variable = false;
    while let Some(part) = parts.next() {
        if variable {
            // `${NOM}` si le texte qui suit prolongerait le nom.
            let glued = parts
                .peek()
                .and_then(|next| next.chars().next())
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            if glued {
                out.push_str(&format!("${{{part}}}"));
            } else {
                out.push('$');
                out.push_str(part);
            }
        } else {
            for c in part.chars() {
                if matches!(c, '\\' | '"' | '$' | '`') {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        variable = !variable;
    }
    out.push('"');
    out
}

/// Commande cURL de `prepared`, une option par ligne (R8). `user` : identifiants
/// d'une authentification Basic, passés par `-u` (mot de passe en variable).
pub fn render(prepared: &Prepared, user: Option<&str>) -> String {
    let mut first = String::from("curl");
    match (prepared.method, prepared.body.is_some()) {
        (Method::Get, false) => {}
        (Method::Head, false) => first.push_str(" -I"),
        (method, _) => {
            first.push_str(" -X ");
            first.push_str(method.as_str());
        }
    }
    first.push(' ');
    first.push_str(&quote(&prepared.url));
    let mut lines = vec![first];
    for (name, value) in &prepared.headers {
        lines.push(format!("-H {}", quote(&format!("{name}: {value}"))));
    }
    if let Some(user) = user {
        lines.push(format!("-u {}", quote(user)));
    }
    if let Some(body) = &prepared.body {
        // `--data` lirait un fichier pour une valeur commençant par `@`.
        let option = if body.starts_with('@') {
            "--data-raw"
        } else {
            "--data"
        };
        lines.push(format!("{option} {}", quote(body)));
    }
    lines.join(" \\\n  ")
}

/// Commande cURL de `request` dans l'environnement de `variables` (R8 à R10) :
/// variables partagées résolues ; secrets, jeton, mot de passe et clé d'API
/// remplacés par une variable du shell ; variables inconnues et valeurs
/// dynamiques laissées `{{…}}`.
pub fn command(request: &Request, variables: &Variables) -> Result<String, CommandError> {
    let mut values: BTreeMap<String, String> = variables.values.clone();
    for name in &variables.secrets {
        values.insert(name.clone(), placeholder(&shell_name(name)));
    }
    for name in written_names(request) {
        values
            .entry(name.clone())
            .or_insert_with(|| format!("{{{{{name}}}}}"));
    }
    let mut copy = request.clone();
    if copy.auth.kind == AuthKind::Inherit
        && let Some(inherited) = &copy.inherited.auth
    {
        copy.auth = inherited.auth.clone();
    }
    let mut user = None;
    match copy.auth.kind {
        AuthKind::Bearer => copy.auth.token = placeholder("TOKEN"),
        AuthKind::ApiKey => copy.auth.value = placeholder("API_KEY"),
        AuthKind::Basic => {
            user = Some(format!(
                "{}:{}",
                fill(&copy.auth.username, &values),
                placeholder("PASSWORD")
            ));
            copy.auth.kind = AuthKind::None;
        }
        AuthKind::None | AuthKind::Inherit | AuthKind::Other => {}
    }
    let resolved = Variables {
        values,
        secrets: BTreeSet::new(),
        environment: variables.environment.clone(),
    };
    let mut prepared = prepare(&copy, &resolved)?;
    // Une variable du shell dans un paramètre encodé reste lisible.
    prepared.url = prepared.url.replace("%01", &MARK.to_string());
    hide_credentials(&mut prepared.headers);
    Ok(render(&prepared, user.as_deref()))
}
