//! Analyseur des fichiers `.bru` (langage de Bruno, v2), écrit à la main.
//!
//! Un fichier est une suite de blocs `nom {` … `}` (ou `nom [` … `]` pour une
//! liste) ; la fin d'un bloc est une ligne `}` (ou `]`) en première colonne, ce
//! qui suffit pour les blocs imbriqués, dont le contenu est indenté. Trois
//! formes de contenu :
//!
//! - **dictionnaire** (`meta`, `headers`, `params:query`, `assert`…) : lignes
//!   `clé: valeur`, désactivées si elles commencent par `~`, valeur sur
//!   plusieurs lignes entre `'''` ;
//! - **texte** (`body:json`, `script:*`, `tests`, `docs`…) : contenu brut,
//!   désindenté de deux espaces ;
//! - **liste** (`vars:secret [ … ]`) : éléments séparés par des virgules ou des
//!   retours à la ligne.
//!
//! Module pur : aucun accès au disque.

/// Ligne `clé: valeur` d'un dictionnaire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    /// `false` pour une ligne préfixée par `~`.
    pub enabled: bool,
}

/// Élément d'une liste (`vars:secret`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub enabled: bool,
}

/// Contenu d'un bloc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Dict(Vec<Entry>),
    Text(String),
    List(Vec<Item>),
}

/// Bloc `nom { … }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    pub body: Body,
    /// Numéro de la ligne d'ouverture (à partir de 1).
    pub line: usize,
}

/// Fichier `.bru` lu : ses blocs, dans l'ordre.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BruFile {
    pub blocks: Vec<Block>,
}

impl BruFile {
    /// Premier bloc de ce nom.
    pub fn block(&self, name: &str) -> Option<&Block> {
        self.blocks.iter().find(|block| block.name == name)
    }

    /// Lignes du dictionnaire `name` (vide s'il est absent ou n'est pas un dictionnaire).
    pub fn dict(&self, name: &str) -> &[Entry] {
        match self.block(name).map(|block| &block.body) {
            Some(Body::Dict(entries)) => entries,
            _ => &[],
        }
    }

    /// Valeur active de `key` dans le dictionnaire `name`.
    pub fn value(&self, name: &str, key: &str) -> Option<&str> {
        self.dict(name)
            .iter()
            .find(|entry| entry.enabled && entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    /// Texte du bloc `name`, s'il en a un non vide.
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.block(name).map(|block| &block.body) {
            Some(Body::Text(text)) if !text.trim().is_empty() => Some(text),
            _ => None,
        }
    }
}

/// Le bloc `name` a un contenu brut (corps, script, tests, documentation).
pub fn is_text_block(name: &str) -> bool {
    name.starts_with("body:") && !matches!(name, "body:form-urlencoded" | "body:multipart-form")
        || name.starts_with("script:")
        || matches!(name, "tests" | "docs")
}

/// Le bloc `name` est un dictionnaire connu : une ligne illisible y est une erreur.
fn is_known_dict(name: &str) -> bool {
    let base = name.split(':').next().unwrap_or(name);
    matches!(
        base,
        "meta"
            | "get"
            | "post"
            | "put"
            | "patch"
            | "delete"
            | "options"
            | "head"
            | "connect"
            | "trace"
            | "params"
            | "headers"
            | "auth"
            | "vars"
            | "assert"
            | "settings"
            | "body"
    )
}

/// Erreur de lecture, avec sa ligne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ligne {} : {}", self.line, self.message)
    }
}

fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

/// En-tête de bloc : `nom {` ou `nom [`.
fn header(line: &str) -> Option<(&str, char)> {
    let trimmed = line.trim_end();
    let (name, open) = if let Some(name) = trimmed.strip_suffix('{') {
        (name, '{')
    } else {
        (trimmed.strip_suffix('[')?, '[')
    };
    let name = name.trim();
    let valid = !name.is_empty()
        && !line.starts_with(char::is_whitespace)
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.'));
    valid.then_some((name, open))
}

/// Retire jusqu'à `width` espaces en tête.
fn dedent(line: &str, width: usize) -> &str {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    &line[spaces.min(width)..]
}

/// Lit un fichier `.bru`.
pub fn parse(text: &str) -> Result<BruFile, ParseError> {
    let text = text.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.trim().is_empty() {
            index += 1;
            continue;
        }
        let number = index + 1;
        let (name, open) = header(line).ok_or_else(|| {
            error(
                number,
                format!("début de bloc attendu, trouvé « {} »", line.trim()),
            )
        })?;
        let close = if open == '{' { "}" } else { "]" };
        let end = (index + 1..lines.len())
            .find(|&i| lines[i].trim_end() == close)
            .ok_or_else(|| error(number, format!("bloc « {name} » non fermé")))?;
        let content = &lines[index + 1..end];
        let body = if open == '[' {
            Body::List(list(content))
        } else if is_text_block(name) {
            Body::Text(raw(content))
        } else {
            match dict(content, index + 2) {
                Ok(entries) => Body::Dict(entries),
                // Bloc inconnu (exemples de Bruno…) : gardé brut, sans bloquer le fichier.
                Err(_) if !is_known_dict(name) => Body::Text(raw(content)),
                Err(error) => return Err(error),
            }
        };
        blocks.push(Block {
            name: name.to_owned(),
            body,
            line: number,
        });
        index = end + 1;
    }
    Ok(BruFile { blocks })
}

/// Contenu brut d'un bloc : deux espaces d'indentation retirés, lignes vides
/// de fin retirées.
fn raw(content: &[&str]) -> String {
    let mut lines: Vec<&str> = content.iter().map(|line| dedent(line, 2)).collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    lines.join("\n")
}

fn list(content: &[&str]) -> Vec<Item> {
    content
        .iter()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| match item.strip_prefix('~') {
            Some(name) => Item {
                name: name.trim().to_owned(),
                enabled: false,
            },
            None => Item {
                name: item.to_owned(),
                enabled: true,
            },
        })
        .collect()
}

/// Clé et reste d'une ligne `clé: valeur` ; une clé entre guillemets peut
/// contenir `:`.
fn split_entry(line: &str) -> Option<(String, &str)> {
    if let Some(rest) = line.strip_prefix('"') {
        let mut escaped = false;
        for (i, c) in rest.char_indices() {
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => {
                    let key = rest[..i].replace("\\\"", "\"");
                    let after = rest[i + 1..].trim_start();
                    return after.strip_prefix(':').map(|value| (key, value));
                }
                _ => escaped = false,
            }
        }
        return None;
    }
    let colon = line.find(':')?;
    Some((line[..colon].trim().to_owned(), &line[colon + 1..]))
}

fn dict(content: &[&str], first_line: usize) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    let mut index = 0;
    while index < content.len() {
        let number = first_line + index;
        let line = content[index].trim();
        index += 1;
        if line.is_empty() {
            continue;
        }
        let (enabled, line) = match line.strip_prefix('~') {
            Some(rest) => (false, rest.trim_start()),
            None => (true, line),
        };
        let (key, rest) = split_entry(line).ok_or_else(|| {
            error(
                number,
                format!("« clé: valeur » attendu, trouvé « {line} »"),
            )
        })?;
        let rest = rest.trim();
        let value = if rest == "[" {
            // Liste sur plusieurs lignes (`tags: [` … `]`) : éléments séparés par des virgules.
            let start = index;
            let stop = (start..content.len())
                .find(|&i| content[i].trim() == "]")
                .ok_or_else(|| error(number, format!("liste « {key} » non fermée")))?;
            index = stop + 1;
            list(&content[start..stop])
                .into_iter()
                .map(|item| item.name)
                .collect::<Vec<_>>()
                .join(", ")
        } else if rest == "'''" {
            // Valeur sur plusieurs lignes, jusqu'à la ligne `'''`.
            let start = index;
            let stop = (start..content.len())
                .find(|&i| content[i].trim() == "'''")
                .ok_or_else(|| error(number, format!("valeur de « {key} » non fermée (''')")))?;
            index = stop + 1;
            let body = &content[start..stop];
            let width = body
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start_matches(' ').len())
                .min()
                .unwrap_or(0);
            body.iter()
                .map(|l| dedent(l, width))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            rest.to_owned()
        };
        entries.push(Entry {
            key,
            value,
            enabled,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: &str, enabled: bool) -> Entry {
        Entry {
            key: key.into(),
            value: value.into(),
            enabled,
        }
    }

    #[test]
    fn meta_et_methode() {
        let file = parse(
            "meta {\n  name: Lister les clients\n  type: http\n  seq: 2\n}\n\nget {\n  url: {{baseUrl}}/clients?page=1\n  body: none\n  auth: inherit\n}\n",
        )
        .unwrap();
        assert_eq!(file.blocks.len(), 2);
        assert_eq!(file.value("meta", "name"), Some("Lister les clients"));
        assert_eq!(file.value("meta", "seq"), Some("2"));
        // La valeur garde ses « : » et « ? ».
        assert_eq!(file.value("get", "url"), Some("{{baseUrl}}/clients?page=1"));
        assert_eq!(file.block("get").unwrap().line, 7);
    }

    #[test]
    fn lignes_desactivees_par_tilde() {
        let file = parse("params:query {\n  page: 1\n  ~limit: 10\n}\n").unwrap();
        assert_eq!(
            file.dict("params:query"),
            [entry("page", "1", true), entry("limit", "10", false)]
        );
        assert_eq!(file.value("params:query", "limit"), None);
    }

    #[test]
    fn valeur_vide_et_cle_entre_guillemets() {
        let file = parse("headers {\n  \"x:weird\": a\n  scope: \n}\n").unwrap();
        assert_eq!(
            file.dict("headers"),
            [entry("x:weird", "a", true), entry("scope", "", true)]
        );
    }

    #[test]
    fn corps_json_brut_desindente() {
        let file =
            parse("body:json {\n  {\n    \"a\": 1,\n    \"b\": \"}\"\n  }\n}\n\nassert {\n  res.status: eq 200\n}\n")
                .unwrap();
        assert_eq!(
            file.text("body:json"),
            Some("{\n  \"a\": 1,\n  \"b\": \"}\"\n}")
        );
        assert_eq!(file.value("assert", "res.status"), Some("eq 200"));
    }

    #[test]
    fn scripts_tests_et_docs_sont_du_texte() {
        let file = parse(
            "script:pre-request {\n  if (x) {\n    bru.setVar(\"a\", 1);\n  }\n}\n\ntests {\n  test(\"ok\", function () {});\n}\n\ndocs {\n  # Titre\n  \n  texte : avec deux-points\n}\n",
        )
        .unwrap();
        assert_eq!(
            file.text("script:pre-request"),
            Some("if (x) {\n  bru.setVar(\"a\", 1);\n}")
        );
        assert_eq!(file.text("tests"), Some("test(\"ok\", function () {});"));
        assert_eq!(
            file.text("docs"),
            Some("# Titre\n\ntexte : avec deux-points")
        );
    }

    #[test]
    fn valeur_sur_plusieurs_lignes() {
        let file = parse(
            "headers {\n  x-signature: '''\n    ligne 1\n      ligne 2\n  '''\n  x-autre: b\n}\n",
        )
        .unwrap();
        assert_eq!(
            file.dict("headers"),
            [
                entry("x-signature", "ligne 1\n  ligne 2", true),
                entry("x-autre", "b", true)
            ]
        );
    }

    #[test]
    fn liste_des_secrets() {
        let file =
            parse("vars {\n  baseUrl: https://x\n}\nvars:secret [\n  token,\n  ~password\n]\n")
                .unwrap();
        let Some(Body::List(items)) = file.block("vars:secret").map(|b| &b.body) else {
            panic!("liste attendue");
        };
        assert_eq!(
            items,
            &[
                Item {
                    name: "token".into(),
                    enabled: true
                },
                Item {
                    name: "password".into(),
                    enabled: false
                }
            ]
        );
        // Liste sur une ligne.
        let file = parse("vars:secret [a, b]\n");
        assert!(file.is_err(), "une liste s'ouvre seule sur sa ligne");
    }

    #[test]
    fn blocs_imbriques_inconnus() {
        let file = parse(
            "example {\n  name: Réponse 200\n  response {\n    status: 200\n  }\n}\n\nget {\n  url: x\n}\n",
        );
        // Un bloc inconnu qui n'est pas un dictionnaire est gardé brut.
        let file = file.unwrap();
        assert_eq!(file.value("get", "url"), Some("x"));
        assert_eq!(
            file.block("example").map(|b| &b.body),
            Some(&Body::Text(
                "name: Réponse 200\nresponse {\n  status: 200\n}".into()
            ))
        );
        let file = parse(
            "body:file {\n  file: @file(a.png) @contentType(image/png)\n}\n\nget {\n  url: x\n}\n",
        )
        .unwrap();
        assert_eq!(file.value("get", "url"), Some("x"));
    }

    #[test]
    fn etiquettes_en_liste_dans_meta() {
        let file =
            parse("meta {\n  name: A\n  tags: [\n    smoke\n    recette\n  ]\n  seq: 4\n}\n")
                .unwrap();
        assert_eq!(file.value("meta", "tags"), Some("smoke, recette"));
        assert_eq!(file.value("meta", "seq"), Some("4"));
    }

    #[test]
    fn fin_de_ligne_windows_et_bom() {
        let file = parse("\u{feff}meta {\r\n  name: A\r\n}\r\n").unwrap();
        assert_eq!(file.value("meta", "name"), Some("A"));
    }

    #[test]
    fn erreurs_avec_leur_ligne() {
        let error = parse("meta {\n  name: A\n").unwrap_err();
        assert_eq!(error.line, 1);
        assert!(error.message.contains("non fermé"), "{error}");
        let error = parse("meta {\n  sans deux points\n}\n").unwrap_err();
        assert_eq!(error.line, 2);
        let error = parse("n'importe quoi\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "ligne 1 : début de bloc attendu, trouvé « n'importe quoi »"
        );
        let error = parse("headers {\n  a: '''\n  b\n}\n").unwrap_err();
        assert!(error.message.contains("non fermée"), "{error}");
    }
}
