//! Lecture tolérante d'un texte XML ou HTML en balises et textes, pour la mise en forme (R3) et
//! l'aperçu HTML (R4). Ce n'est pas un analyseur validant : un document mal formé est lu au
//! mieux, jamais refusé.

/// Élément lu dans le texte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<'a> {
    /// Balise ouvrante (`self_closing` pour `<br/>`) et ses attributs, valeur brute guillemets
    /// compris.
    Open {
        name: &'a str,
        attrs: Vec<(&'a str, Option<&'a str>)>,
        self_closing: bool,
    },
    /// Balise fermante.
    Close { name: &'a str },
    /// Texte entre deux balises, tel quel.
    Text(&'a str),
    /// Commentaire `<!-- … -->`, délimiteurs compris.
    Comment(&'a str),
    /// `<?xml … ?>`, `<!DOCTYPE …>` ou `<![CDATA[ … ]]>`, délimiteurs compris.
    Declaration(&'a str),
}

/// Éléments HTML sans contenu ni balise fermante.
pub const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Éléments HTML dont le contenu est du texte brut, jusqu'à leur balise fermante.
const RAW_TEXT: [&str; 4] = ["script", "style", "textarea", "title"];

pub fn is_void(name: &str) -> bool {
    VOID.iter().any(|v| v.eq_ignore_ascii_case(name))
}

/// Découpe `src` en éléments ; `html` active les éléments à texte brut (`<script>`, `<style>`).
pub fn tokenize(src: &str, html: bool) -> Vec<Item<'_>> {
    let bytes = src.as_bytes();
    let mut items = Vec::new();
    let mut text_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &src[i..];
        let delimited = |open: &str, close: &str| -> Option<usize> {
            rest.starts_with(open)
                .then(|| rest[open.len()..].find(close))
                .flatten()
                .map(|end| open.len() + end + close.len())
                .or_else(|| rest.starts_with(open).then_some(rest.len()))
        };
        if let Some(len) = delimited("<!--", "-->") {
            flush(&mut items, src, text_start, i);
            items.push(Item::Comment(&src[i..i + len]));
            i += len;
            text_start = i;
            continue;
        }
        if let Some(len) = delimited("<![CDATA[", "]]>")
            .or_else(|| delimited("<?", "?>"))
            .or_else(|| delimited("<!", ">"))
        {
            flush(&mut items, src, text_start, i);
            items.push(Item::Declaration(&src[i..i + len]));
            i += len;
            text_start = i;
            continue;
        }
        let closing = rest.starts_with("</");
        let name_start = i + if closing { 2 } else { 1 };
        let starts_name = bytes
            .get(name_start)
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_' || *b >= 0x80);
        if !starts_name {
            i += 1;
            continue;
        }
        let Some(end) = tag_end(bytes, name_start) else {
            // Balise jamais fermée : le reste est du texte.
            break;
        };
        flush(&mut items, src, text_start, i);
        let inner = &src[name_start..end];
        let name_len = inner
            .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
            .unwrap_or(inner.len());
        let name = &inner[..name_len];
        i = end + 1;
        text_start = i;
        if closing {
            items.push(Item::Close { name });
            continue;
        }
        let self_closing = inner.ends_with('/');
        let attrs_src = inner[name_len..].trim_end_matches('/');
        items.push(Item::Open {
            name,
            attrs: attributes(attrs_src),
            self_closing,
        });
        if html && !self_closing && RAW_TEXT.iter().any(|r| r.eq_ignore_ascii_case(name)) {
            let lower = src[i..].to_ascii_lowercase();
            let close = format!("</{}", name.to_ascii_lowercase());
            let stop = lower.find(&close).map_or(src.len(), |at| i + at);
            flush(&mut items, src, i, stop);
            i = stop;
            text_start = i;
        }
    }
    flush(&mut items, src, text_start, src.len());
    items
}

fn flush<'a>(items: &mut Vec<Item<'a>>, src: &'a str, from: usize, to: usize) {
    if to > from {
        items.push(Item::Text(&src[from..to]));
    }
}

/// Position du `>` qui ferme la balise, hors des valeurs entre guillemets.
fn tag_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (offset, &b) in bytes[from..].iter().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return Some(from + offset),
            None => {}
        }
    }
    None
}

/// Attributs `nom`, `nom=valeur`, `nom="valeur"` ; la valeur garde ses guillemets.
fn attributes(src: &str) -> Vec<(&str, Option<&str>)> {
    let bytes = src.as_bytes();
    let mut attrs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        if i == start {
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        let name = &src[start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            attrs.push((name, None));
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value_start = i;
        if let Some(&q) = bytes.get(i).filter(|b| **b == b'"' || **b == b'\'') {
            i += 1;
            while i < bytes.len() && bytes[i] != q {
                i += 1;
            }
            i = (i + 1).min(bytes.len());
        } else {
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }
        attrs.push((name, Some(&src[value_start..i])));
    }
    attrs
}

/// Décode les entités usuelles (`&amp;`, `&lt;`, `&#233;`, `&#xE9;`…) ; une entité inconnue
/// reste telle quelle.
pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest[1..]
            .find(';')
            .filter(|end| *end <= 10)
            .and_then(|end| {
                let name = &rest[1..=end];
                entity(name).map(|c| (c, end + 2))
            });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code);
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "eacute" => 'é',
        "egrave" => 'è',
        "agrave" => 'à',
        "ccedil" => 'ç',
        "hellip" => '…',
        "laquo" => '«',
        "raquo" => '»',
        "copy" => '©',
        "euro" => '€',
        _ => return None,
    })
}
