//! Aperçu d'un corps HTML (R4) : un arbre réduit à des balises de mise en page sûres, sans
//! attribut. Rien ne s'exécute ni ne se charge : scripts, styles, cadres et objets disparaissent
//! avec leur contenu, une image devient son texte de remplacement, une balise inconnue laisse son
//! contenu.

use serde::Serialize;

use super::markup::{self, Item};

/// Balises gardées dans l'aperçu, sans leurs attributs.
pub const ALLOWED: [&str; 37] = [
    "a",
    "abbr",
    "article",
    "b",
    "blockquote",
    "br",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "small",
    "span",
    "strong",
    "table",
    "td",
    "th",
    "tr",
    "ul",
];

/// Balises retirées avec tout leur contenu.
const DROPPED: [&str; 14] = [
    "script", "style", "head", "template", "iframe", "frame", "object", "embed", "noscript", "svg",
    "math", "canvas", "select", "textarea",
];

/// Nombre maximal de nœuds de l'aperçu.
pub const MAX_NODES: usize = 5000;

/// Nœud de l'aperçu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HtmlNode {
    Text {
        text: String,
    },
    Element {
        tag: String,
        children: Vec<HtmlNode>,
    },
}

struct Frame {
    /// Nom lu dans le texte, en minuscules.
    name: String,
    /// Balise gardée, ou `None` : le contenu rejoint le parent.
    tag: Option<&'static str>,
    children: Vec<HtmlNode>,
}

fn allowed(name: &str) -> Option<&'static str> {
    ALLOWED.iter().copied().find(|tag| *tag == name)
}

/// Arbre de l'aperçu d'un document HTML (R4).
pub fn preview(body: &str) -> Vec<HtmlNode> {
    let items = markup::tokenize(body, true);
    let mut stack = vec![Frame {
        name: String::new(),
        tag: None,
        children: Vec::new(),
    }];
    let mut count = 0usize;
    let mut skip: Option<(String, usize)> = None;
    for item in &items {
        if count >= MAX_NODES {
            break;
        }
        // Contenu d'une balise retirée : ignoré jusqu'à sa fermeture.
        if let Some((name, nested)) = skip.as_mut() {
            match item {
                Item::Open {
                    name: open,
                    self_closing: false,
                    ..
                } if open.eq_ignore_ascii_case(name) => *nested += 1,
                Item::Close { name: close } if close.eq_ignore_ascii_case(name) => {
                    if *nested == 0 {
                        skip = None;
                    } else {
                        *nested -= 1;
                    }
                }
                _ => {}
            }
            continue;
        }
        match item {
            Item::Open {
                name,
                attrs,
                self_closing,
            } => {
                let lower = name.to_ascii_lowercase();
                if DROPPED.contains(&lower.as_str()) {
                    if !self_closing {
                        skip = Some((lower, 0));
                    }
                    continue;
                }
                if lower == "img" {
                    let alt = attrs
                        .iter()
                        .find(|(attr, _)| attr.eq_ignore_ascii_case("alt"))
                        .and_then(|(_, value)| *value)
                        .map(|v| markup::decode_entities(v.trim_matches(['"', '\''])))
                        .filter(|alt| !alt.trim().is_empty());
                    let text =
                        alt.map_or_else(|| "[image]".to_owned(), |alt| format!("[image : {alt}]"));
                    push(&mut stack, HtmlNode::Text { text });
                    count += 1;
                    continue;
                }
                let tag = allowed(&lower);
                if *self_closing || markup::is_void(&lower) {
                    if let Some(tag) = tag {
                        push(
                            &mut stack,
                            HtmlNode::Element {
                                tag: tag.to_owned(),
                                children: Vec::new(),
                            },
                        );
                        count += 1;
                    }
                    continue;
                }
                stack.push(Frame {
                    name: lower,
                    tag,
                    children: Vec::new(),
                });
            }
            Item::Close { name } => {
                let lower = name.to_ascii_lowercase();
                let Some(position) = stack.iter().rposition(|f| f.name == lower) else {
                    continue;
                };
                if position == 0 {
                    continue;
                }
                while stack.len() > position {
                    close(&mut stack, &mut count);
                }
            }
            Item::Text(text) => {
                let text = markup::decode_entities(text);
                if !text.trim().is_empty() {
                    push(&mut stack, HtmlNode::Text { text });
                    count += 1;
                }
            }
            Item::Comment(_) | Item::Declaration(_) => {}
        }
    }
    while stack.len() > 1 {
        close(&mut stack, &mut count);
    }
    let mut root = stack.pop().map(|f| f.children).unwrap_or_default();
    if count >= MAX_NODES {
        root.push(HtmlNode::Text {
            text: "… aperçu limité au début du document.".to_owned(),
        });
    }
    root
}

fn push(stack: &mut [Frame], node: HtmlNode) {
    if let Some(top) = stack.last_mut() {
        top.children.push(node);
    }
}

/// Ferme la balise du sommet : gardée, elle devient un nœud ; sinon son contenu rejoint le parent.
fn close(stack: &mut Vec<Frame>, count: &mut usize) {
    let Some(frame) = stack.pop() else { return };
    let Some(parent) = stack.last_mut() else {
        return;
    };
    match frame.tag {
        Some(tag) => {
            parent.children.push(HtmlNode::Element {
                tag: tag.to_owned(),
                children: frame.children,
            });
            *count += 1;
        }
        None => parent.children.extend(frame.children),
    }
}
