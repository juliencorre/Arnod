//! Chemins des valeurs d'un corps JSON (R5) : forme unique `$.items[0].sku`, partagée par la mise
//! en forme et le filtre, et cible de vérification `body.items[0].sku` (R6).

/// Élément d'un chemin : nom de membre ou index de tableau.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part<'a> {
    Name(&'a str),
    Index(usize),
}

/// Nom de membre écrit en notation pointée : lettres ASCII, chiffres et `_`, sans chiffre en tête.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Ajoute un élément à un chemin canonique.
pub fn push(path: &mut String, part: &Part<'_>) {
    match part {
        Part::Name(name) if is_identifier(name) => {
            path.push('.');
            path.push_str(name);
        }
        Part::Name(name) => {
            path.push_str("['");
            for c in name.chars() {
                if c == '\'' || c == '\\' {
                    path.push('\\');
                }
                path.push(c);
            }
            path.push_str("']");
        }
        Part::Index(index) => {
            path.push('[');
            path.push_str(&index.to_string());
            path.push(']');
        }
    }
}

/// Chemin canonique : `$`, puis `.nom`, `['nom spécial']` ou `[index]`.
pub fn canonical<'a>(parts: impl IntoIterator<Item = Part<'a>>) -> String {
    let mut path = String::from("$");
    for part in parts {
        push(&mut path, &part);
    }
    path
}

/// Cible de vérification (`body.items[0].sku`), si chaque nom s'écrit en notation pointée : la
/// cible est relue par les vérifications et écrite `res.body…` dans le fichier (R6).
pub fn assertion_target<'a>(parts: impl IntoIterator<Item = Part<'a>>) -> Option<String> {
    let mut target = String::from("body");
    for part in parts {
        match part {
            Part::Name(name) if is_identifier(name) => {
                target.push('.');
                target.push_str(name);
            }
            Part::Name(_) => return None,
            Part::Index(index) => {
                target.push('[');
                target.push_str(&index.to_string());
                target.push(']');
            }
        }
    }
    Some(target)
}
