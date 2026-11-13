//! Mise en forme d'un corps (R2, R3) : lignes de jetons colorés, chaque valeur JSON portant son
//! chemin canonique (R5) pour que l'interface surligne les résultats du filtre.

use serde::Serialize;
use serde_json::Value;

use super::format::{ResponseFormat, detect};
use super::markup::{self, Item};
use super::path::{self, Part};

/// Au-delà, le corps est montré sans mise en forme (les jetons pèseraient trop).
pub const RENDER_LIMIT: usize = 1024 * 1024;

/// Longueur maximale d'un objet ou d'un tableau de valeurs simples écrit sur une ligne.
const INLINE_WIDTH: usize = 72;

/// Nature d'un jeton, pour sa couleur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    /// Nom de membre JSON.
    Key,
    String,
    Number,
    /// `true`, `false`, `null`.
    Literal,
    /// Ponctuation : accolades, crochets, virgules, chevrons.
    Punct,
    /// Nom de balise.
    Tag,
    /// Nom d'attribut.
    Attr,
    /// Valeur d'attribut.
    AttrValue,
    Comment,
    /// Texte sans mise en forme, indentation comprise.
    Text,
}

/// Morceau de ligne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub text: String,
    pub kind: TokenKind,
    /// Chemin canonique de la valeur JSON (valeur simple, ou accolade et crochet d'un objet ou
    /// d'un tableau).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Corps mis en forme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedBody {
    /// Format appliqué : celui choisi, sinon celui détecté.
    pub format: ResponseFormat,
    /// Format détecté d'après `content-type` et le corps (R1).
    pub detected: ResponseFormat,
    pub lines: Vec<Vec<Token>>,
    /// Pourquoi le corps est montré sans mise en forme, en français.
    pub notice: Option<String>,
}

fn token(text: impl Into<String>, kind: TokenKind) -> Token {
    Token {
        text: text.into(),
        kind,
        path: None,
    }
}

/// Met en forme `body` dans `format`, ou dans le format détecté (R1 à R3).
pub fn render(
    body: &str,
    headers: &[(String, String)],
    format: Option<ResponseFormat>,
) -> RenderedBody {
    let detected = detect(headers, body);
    let format = format.unwrap_or(detected);
    let mut notice = None;
    let lines = if body.is_empty() {
        Vec::new()
    } else if format == ResponseFormat::Text {
        plain(body)
    } else if body.len() > RENDER_LIMIT {
        notice = Some("Corps de plus de 1 Mo : affiché sans mise en forme.".to_owned());
        plain(body)
    } else if format == ResponseFormat::Json {
        match serde_json::from_str::<Value>(body) {
            Ok(value) => json_lines(&value),
            Err(error) => {
                notice = Some(format!(
                    "Ce corps n'est pas du JSON valide (ligne {}, colonne {}) : affiché tel quel.",
                    error.line(),
                    error.column()
                ));
                plain(body)
            }
        }
    } else {
        markup_lines(body, format == ResponseFormat::Html)
    };
    RenderedBody {
        format,
        detected,
        lines,
        notice,
    }
}

/// Une ligne par ligne du texte, sans couleur.
fn plain(body: &str) -> Vec<Vec<Token>> {
    body.lines()
        .map(|line| vec![token(line, TokenKind::Text)])
        .collect()
}

// --- JSON ----------------------------------------------------------------------------------

struct JsonWriter {
    lines: Vec<Vec<Token>>,
    line: Vec<Token>,
}

impl JsonWriter {
    fn push(&mut self, token: Token) {
        self.line.push(token);
    }

    fn newline(&mut self, depth: usize) {
        let done = std::mem::take(&mut self.line);
        self.lines.push(done);
        if depth > 0 {
            self.line.push(token("  ".repeat(depth), TokenKind::Text));
        }
    }

    fn scalar(&mut self, value: &Value, path: &str) {
        let (text, kind) = match value {
            Value::String(text) => (quoted(text), TokenKind::String),
            Value::Number(number) => (number.to_string(), TokenKind::Number),
            Value::Bool(flag) => (flag.to_string(), TokenKind::Literal),
            _ => ("null".to_owned(), TokenKind::Literal),
        };
        self.push(Token {
            text,
            kind,
            path: Some(path.to_owned()),
        });
    }

    fn bracket(&mut self, text: &str, path: &str) {
        self.push(Token {
            text: text.to_owned(),
            kind: TokenKind::Punct,
            path: Some(path.to_owned()),
        });
    }

    fn value(&mut self, value: &Value, path: &str, depth: usize) {
        match value {
            Value::Object(map) => {
                self.bracket("{", path);
                if map.is_empty() {
                    self.bracket("}", path);
                    return;
                }
                let inline = inline_width(value).is_some_and(|w| w + 2 * depth <= INLINE_WIDTH);
                for (index, (key, child)) in map.iter().enumerate() {
                    if inline {
                        self.push(token(if index == 0 { " " } else { ", " }, TokenKind::Punct));
                    } else {
                        if index > 0 {
                            self.push(token(",", TokenKind::Punct));
                        }
                        self.newline(depth + 1);
                    }
                    self.push(token(quoted(key), TokenKind::Key));
                    self.push(token(": ", TokenKind::Punct));
                    let mut child_path = path.to_owned();
                    path::push(&mut child_path, &Part::Name(key));
                    self.value(child, &child_path, depth + 1);
                }
                if inline {
                    self.push(token(" ", TokenKind::Punct));
                } else {
                    self.newline(depth);
                }
                self.bracket("}", path);
            }
            Value::Array(items) => {
                self.bracket("[", path);
                if items.is_empty() {
                    self.bracket("]", path);
                    return;
                }
                let inline = inline_width(value).is_some_and(|w| w + 2 * depth <= INLINE_WIDTH);
                for (index, child) in items.iter().enumerate() {
                    if inline {
                        if index > 0 {
                            self.push(token(", ", TokenKind::Punct));
                        }
                    } else {
                        if index > 0 {
                            self.push(token(",", TokenKind::Punct));
                        }
                        self.newline(depth + 1);
                    }
                    let mut child_path = path.to_owned();
                    path::push(&mut child_path, &Part::Index(index));
                    self.value(child, &child_path, depth + 1);
                }
                if !inline {
                    self.newline(depth);
                }
                self.bracket("]", path);
            }
            scalar => self.scalar(scalar, path),
        }
    }
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// Largeur d'un objet ou d'un tableau écrit sur une ligne, s'il ne contient que des valeurs
/// simples.
fn inline_width(value: &Value) -> Option<usize> {
    let scalar_width = |v: &Value| match v {
        Value::String(text) => Some(quoted(text).chars().count()),
        Value::Number(n) => Some(n.to_string().len()),
        Value::Bool(b) => Some(b.to_string().len()),
        Value::Null => Some(4),
        _ => None,
    };
    match value {
        Value::Object(map) => map.iter().try_fold(4, |width, (key, child)| {
            Some(width + quoted(key).chars().count() + 4 + scalar_width(child)?)
        }),
        Value::Array(items) => items
            .iter()
            .try_fold(2, |width, child| Some(width + 2 + scalar_width(child)?)),
        _ => None,
    }
}

fn json_lines(value: &Value) -> Vec<Vec<Token>> {
    let mut writer = JsonWriter {
        lines: Vec::new(),
        line: Vec::new(),
    };
    writer.value(value, "$", 0);
    writer.newline(0);
    writer.lines
}

// --- XML et HTML ---------------------------------------------------------------------------

/// Texte entre deux balises gardé sur la ligne de sa balise.
const INLINE_TEXT: usize = 80;

fn indent(depth: usize) -> Vec<Token> {
    if depth == 0 {
        Vec::new()
    } else {
        vec![token("  ".repeat(depth), TokenKind::Text)]
    }
}

fn open_tokens(line: &mut Vec<Token>, name: &str, attrs: &[(&str, Option<&str>)], close: bool) {
    line.push(token("<", TokenKind::Punct));
    line.push(token(name, TokenKind::Tag));
    for (attr, value) in attrs {
        line.push(token(" ", TokenKind::Text));
        line.push(token(*attr, TokenKind::Attr));
        if let Some(value) = value {
            line.push(token("=", TokenKind::Punct));
            line.push(token(*value, TokenKind::AttrValue));
        }
    }
    line.push(token(if close { "/>" } else { ">" }, TokenKind::Punct));
}

fn close_tokens(line: &mut Vec<Token>, name: &str) {
    line.push(token("</", TokenKind::Punct));
    line.push(token(name, TokenKind::Tag));
    line.push(token(">", TokenKind::Punct));
}

/// Une balise par ligne, indentée ; un texte court reste sur la ligne de sa balise (R3).
fn markup_lines(body: &str, html: bool) -> Vec<Vec<Token>> {
    let items = markup::tokenize(body, html);
    let mut lines = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < items.len() {
        match &items[i] {
            Item::Open {
                name,
                attrs,
                self_closing,
            } => {
                let mut line = indent(depth);
                let empty = *self_closing || (html && markup::is_void(name));
                open_tokens(&mut line, name, attrs, *self_closing);
                if empty {
                    lines.push(line);
                    i += 1;
                    continue;
                }
                // `<a>texte</a>` ou `<a></a>` sur une ligne.
                let closes = |item: Option<&Item<'_>>| matches!(item, Some(Item::Close { name: close }) if close.eq_ignore_ascii_case(name));
                if closes(items.get(i + 1)) {
                    close_tokens(&mut line, name);
                    lines.push(line);
                    i += 2;
                    continue;
                }
                if let Some(Item::Text(text)) = items.get(i + 1) {
                    let trimmed = text.trim();
                    if closes(items.get(i + 2))
                        && !trimmed.contains('\n')
                        && trimmed.chars().count() <= INLINE_TEXT
                    {
                        line.push(token(trimmed, TokenKind::Text));
                        close_tokens(&mut line, name);
                        lines.push(line);
                        i += 3;
                        continue;
                    }
                }
                lines.push(line);
                depth += 1;
            }
            Item::Close { name } => {
                depth = depth.saturating_sub(1);
                let mut line = indent(depth);
                close_tokens(&mut line, name);
                lines.push(line);
            }
            Item::Text(text) => {
                for part in text.lines().map(str::trim).filter(|p| !p.is_empty()) {
                    let mut line = indent(depth);
                    line.push(token(part, TokenKind::Text));
                    lines.push(line);
                }
            }
            Item::Comment(text) | Item::Declaration(text) => {
                let kind = if matches!(items[i], Item::Comment(_)) {
                    TokenKind::Comment
                } else {
                    TokenKind::Punct
                };
                for part in text.lines().map(str::trim).filter(|p| !p.is_empty()) {
                    let mut line = indent(depth);
                    line.push(token(part, kind));
                    lines.push(line);
                }
            }
        }
        i += 1;
    }
    lines
}
