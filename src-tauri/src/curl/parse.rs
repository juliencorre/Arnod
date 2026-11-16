//! Lecture d'une commande cURL collée (2.1-a sexies, R4 à R7).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;

/// Caractères conservés tels quels par `--data-urlencode`.
const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Option de la commande sans équivalent dans Arnod, signalée (R6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ignored {
    /// Option telle qu'écrite (`--compressed`, `-k`).
    pub option: String,
    /// Raison, en minuscules, prête à suivre « est ignorée : ».
    pub reason: String,
}

/// Requête lue d'une commande cURL.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CurlRequest {
    /// Méthode en majuscules (`GET` si rien ne la donne).
    pub method: String,
    /// Adresse ; `http://` ajouté si le schéma manque, comme cURL.
    pub url: String,
    /// En-têtes dans l'ordre de la commande, `-u`, `-b`, `-A`, `-e` compris.
    pub headers: Vec<(String, String)>,
    /// Corps (`-d`, `--data-*`, `--json`), parties jointes par `&`.
    pub body: Option<String>,
    pub ignored: Vec<Ignored>,
}

/// Valeur d'un en-tête collé qui ne doit pas rester dans le texte (R5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    /// Nom de la variable locale qui la remplace (`token`).
    pub name: String,
    pub value: String,
    /// En-tête d'où elle vient (`Authorization`).
    pub header: String,
}

/// Découpe `text` en mots, comme un shell POSIX : espaces, guillemets simples,
/// doubles et `$'…'`, `\` d'échappement et de fin de ligne.
pub fn words(text: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            '\\' => match chars.next() {
                // Fin de ligne échappée : la commande continue (R4).
                Some('\n') => {}
                Some('\r') => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                Some(other) => {
                    current.push(other);
                    in_word = true;
                }
                None => {}
            },
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => current.push(ch),
                        None => return Err("guillemet simple non fermé".to_owned()),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(ch @ ('"' | '\\' | '$' | '`')) => current.push(ch),
                            Some('\n') => {}
                            Some(ch) => {
                                current.push('\\');
                                current.push(ch);
                            }
                            None => return Err("guillemet double non fermé".to_owned()),
                        },
                        Some(ch) => current.push(ch),
                        None => return Err("guillemet double non fermé".to_owned()),
                    }
                }
            }
            '$' if chars.peek() == Some(&'\'') => {
                chars.next();
                in_word = true;
                ansi_c(&mut chars, &mut current)?;
            }
            _ => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}

/// Chaîne `$'…'` de bash (« Copier en cURL » des navigateurs).
fn ansi_c(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    out: &mut String,
) -> Result<(), String> {
    let hex = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>, max: usize| {
        let mut digits = String::new();
        while digits.len() < max {
            match chars.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    digits.push(*c);
                    chars.next();
                }
                _ => break,
            }
        }
        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
    };
    loop {
        match chars.next() {
            Some('\'') => return Ok(()),
            Some('\\') => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('x') => out.extend(hex(chars, 2)),
                Some('u') => out.extend(hex(chars, 4)),
                Some(ch @ ('\\' | '\'' | '"' | '?')) => out.push(ch),
                Some(ch) => {
                    out.push('\\');
                    out.push(ch);
                }
                None => return Err("chaîne $'…' non fermée".to_owned()),
            },
            Some(ch) => out.push(ch),
            None => return Err("chaîne $'…' non fermée".to_owned()),
        }
    }
}

/// Ce que fait une option de cURL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opt {
    Method,
    Header,
    Data,
    DataRaw,
    DataUrlencode,
    Json,
    User,
    Cookie,
    Url,
    UserAgent,
    Referer,
    Get,
    Head,
    /// Sans effet sur la requête (`-s`, `-v`, `-L`…).
    Quiet,
    /// Sans effet, avec une valeur (`--max-redirs 5`).
    QuietValue,
    /// Signalée, sans valeur.
    Note(&'static str),
    /// Signalée, avec une valeur.
    NoteValue(&'static str),
}

impl Opt {
    fn takes_value(self) -> bool {
        !matches!(self, Self::Get | Self::Head | Self::Quiet | Self::Note(_))
    }
}

const COMPRESSED: &str = "Arnod reçoit la réponse sans compression, il n'y a rien à régler";
const INSECURE: &str = "Arnod vérifie toujours le certificat du serveur";
const CERTIFICATE: &str = "les certificats se règlent sur le poste, pas dans la requête";
const PROXY: &str = "le proxy se règle sur le poste, pas dans la requête";
const TIMEOUT: &str = "le délai se règle dans les réglages de la requête";
const OUTPUT: &str = "la réponse s'affiche dans Arnod";
const RETRY: &str = "Arnod ne renvoie pas une requête de lui-même";
const MULTIPART: &str = "le corps multipart n'est pas repris : ajoutez-le dans l'onglet Corps";

fn short(c: char) -> Option<Opt> {
    Some(match c {
        'X' => Opt::Method,
        'H' => Opt::Header,
        'd' => Opt::Data,
        'u' => Opt::User,
        'b' => Opt::Cookie,
        'A' => Opt::UserAgent,
        'e' => Opt::Referer,
        'G' => Opt::Get,
        'I' => Opt::Head,
        's' | 'S' | 'v' | 'i' | 'L' | 'f' | 'g' | 'N' => Opt::Quiet,
        'k' => Opt::Note(INSECURE),
        'E' => Opt::NoteValue(CERTIFICATE),
        'x' => Opt::NoteValue(PROXY),
        'm' => Opt::NoteValue(TIMEOUT),
        'o' => Opt::NoteValue(OUTPUT),
        'F' => Opt::NoteValue(MULTIPART),
        _ => return None,
    })
}

fn long(name: &str) -> Option<Opt> {
    Some(match name {
        "request" => Opt::Method,
        "header" => Opt::Header,
        "data" | "data-ascii" | "data-binary" => Opt::Data,
        "data-raw" => Opt::DataRaw,
        "data-urlencode" => Opt::DataUrlencode,
        "json" => Opt::Json,
        "user" => Opt::User,
        "cookie" => Opt::Cookie,
        "url" => Opt::Url,
        "user-agent" => Opt::UserAgent,
        "referer" => Opt::Referer,
        "get" => Opt::Get,
        "head" => Opt::Head,
        "silent"
        | "show-error"
        | "verbose"
        | "include"
        | "location"
        | "location-trusted"
        | "fail"
        | "globoff"
        | "no-buffer"
        | "http1.1"
        | "http2"
        | "http2-prior-knowledge" => Opt::Quiet,
        "max-redirs" => Opt::QuietValue,
        "compressed" => Opt::Note(COMPRESSED),
        "insecure" => Opt::Note(INSECURE),
        "cert" | "key" | "cacert" | "capath" | "cert-type" | "key-type" => {
            Opt::NoteValue(CERTIFICATE)
        }
        "proxy" | "proxy-user" => Opt::NoteValue(PROXY),
        "max-time" | "connect-timeout" => Opt::NoteValue(TIMEOUT),
        "output" => Opt::NoteValue(OUTPUT),
        "retry" => Opt::NoteValue(RETRY),
        "form" | "form-string" => Opt::NoteValue(MULTIPART),
        _ => return None,
    })
}

/// État de la lecture.
#[derive(Default)]
struct Reader {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<(String, String)>,
    data: Vec<String>,
    get: bool,
    head: bool,
    ignored: Vec<Ignored>,
}

fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(h, _)| h.eq_ignore_ascii_case(name))
}

impl Reader {
    fn ignore(&mut self, option: &str, reason: &str) {
        if !self.ignored.iter().any(|i| i.option == option) {
            self.ignored.push(Ignored {
                option: option.to_owned(),
                reason: reason.to_owned(),
            });
        }
    }

    fn header(&mut self, option: &str, line: &str) {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim();
            // `-H 'Nom:'` retire un en-tête de cURL : rien à reprendre.
            if !name.is_empty() && !value.trim().is_empty() {
                self.headers
                    .push((name.to_owned(), value.trim().to_owned()));
            }
        } else if let Some(name) = line.trim().strip_suffix(';') {
            // `-H 'Nom;'` : en-tête vide.
            self.headers.push((name.trim().to_owned(), String::new()));
        } else {
            self.ignore(
                &format!("{option} {line}"),
                "un en-tête s'écrit « Nom: valeur »",
            );
        }
    }

    fn apply(&mut self, opt: Opt, option: &str, value: &str) {
        match opt {
            Opt::Method => self.method = Some(value.trim().to_uppercase()),
            Opt::Header => self.header(option, value),
            Opt::Data | Opt::DataRaw => {
                if opt == Opt::Data && value.starts_with('@') {
                    self.ignore(
                        &format!("{option} {value}"),
                        "le contenu d'un fichier n'est pas lu : collez-le dans le corps",
                    );
                } else {
                    self.data.push(value.to_owned());
                }
            }
            Opt::DataUrlencode => self.urlencode(option, value),
            Opt::Json => {
                self.data.push(value.to_owned());
                for name in ["Content-Type", "Accept"] {
                    if !has_header(&self.headers, name) {
                        self.headers
                            .push((name.to_owned(), "application/json".to_owned()));
                    }
                }
            }
            Opt::User => {
                let credentials = if value.contains(':') {
                    value.to_owned()
                } else {
                    format!("{value}:")
                };
                self.headers.push((
                    "Authorization".to_owned(),
                    format!("Basic {}", STANDARD.encode(credentials)),
                ));
            }
            Opt::Cookie => {
                if value.contains('=') {
                    match self
                        .headers
                        .iter_mut()
                        .find(|(h, _)| h.eq_ignore_ascii_case("Cookie"))
                    {
                        Some((_, existing)) => {
                            existing.push_str("; ");
                            existing.push_str(value);
                        }
                        None => self.headers.push(("Cookie".to_owned(), value.to_owned())),
                    }
                } else {
                    self.ignore(
                        &format!("{option} {value}"),
                        "un fichier de cookies n'est pas lu",
                    );
                }
            }
            Opt::Url => self.address(value),
            Opt::UserAgent => self
                .headers
                .push(("User-Agent".to_owned(), value.to_owned())),
            Opt::Referer => self.headers.push(("Referer".to_owned(), value.to_owned())),
            Opt::Get => self.get = true,
            Opt::Head => self.head = true,
            Opt::Quiet | Opt::QuietValue => {}
            Opt::Note(reason) | Opt::NoteValue(reason) => self.ignore(option, reason),
        }
    }

    fn urlencode(&mut self, option: &str, value: &str) {
        let encode = |text: &str| utf8_percent_encode(text, UNRESERVED).to_string();
        if let Some((name, content)) = value.split_once('=') {
            self.data.push(if name.is_empty() {
                encode(content)
            } else {
                format!("{name}={}", encode(content))
            });
        } else if value.contains('@') {
            self.ignore(
                &format!("{option} {value}"),
                "le contenu d'un fichier n'est pas lu : collez-le dans le corps",
            );
        } else {
            self.data.push(encode(value));
        }
    }

    fn address(&mut self, value: &str) {
        if self.url.is_some() {
            self.ignore(value, "une requête n'a qu'une adresse");
        } else {
            self.url = Some(value.trim().to_owned());
        }
    }
}

/// `true` si `input` est une commande cURL (`curl …`).
pub fn is_curl(input: &str) -> bool {
    let mut parts = input.split_whitespace();
    matches!(parts.next(), Some("curl" | "curl.exe")) && parts.next().is_some()
}

/// Lit la commande cURL `input` (R4) ; erreur si ce n'en est pas une ou si
/// elle ne donne aucune adresse.
pub fn parse(input: &str) -> Result<CurlRequest, String> {
    let words = words(input)?;
    let mut iter = words.into_iter();
    match iter.next().as_deref() {
        Some("curl" | "curl.exe") => {}
        _ => return Err("la commande doit commencer par curl".to_owned()),
    }
    let mut reader = Reader::default();
    let mut options_done = false;
    while let Some(word) = iter.next() {
        if options_done || !word.starts_with('-') || word == "-" {
            reader.address(&word);
            continue;
        }
        if word == "--" {
            options_done = true;
            continue;
        }
        if let Some(name) = word.strip_prefix("--") {
            let (name, attached) = match name.split_once('=') {
                Some((n, v)) => (n, Some(v.to_owned())),
                None => (name, None),
            };
            let option = format!("--{name}");
            match long(name) {
                Some(opt) if opt.takes_value() => {
                    let value = attached.or_else(|| iter.next()).unwrap_or_default();
                    reader.apply(opt, &option, &value);
                }
                Some(opt) => reader.apply(opt, &option, ""),
                None => reader.ignore(&option, "option sans équivalent dans Arnod"),
            }
            continue;
        }
        // Options courtes, éventuellement groupées (`-sSL`, `-XPOST`).
        let letters: Vec<char> = word.chars().skip(1).collect();
        for (index, letter) in letters.iter().enumerate() {
            let option = format!("-{letter}");
            match short(*letter) {
                Some(opt) if opt.takes_value() => {
                    let rest: String = letters[index + 1..].iter().collect();
                    let value = if rest.is_empty() {
                        iter.next().unwrap_or_default()
                    } else {
                        rest
                    };
                    reader.apply(opt, &option, &value);
                    break;
                }
                Some(opt) => reader.apply(opt, &option, ""),
                None => reader.ignore(&option, "option sans équivalent dans Arnod"),
            }
        }
    }

    let mut url = reader
        .url
        .take()
        .ok_or_else(|| "la commande ne donne aucune adresse".to_owned())?;
    if !url.contains("://") {
        url = format!("http://{url}");
    }
    let data = (!reader.data.is_empty()).then(|| reader.data.join("&"));
    let mut body = None;
    if let Some(data) = data {
        if reader.get {
            // `-G` : les données partent dans l'adresse.
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&data);
        } else {
            // Ce que cURL envoie sans type donné.
            if !has_header(&reader.headers, "Content-Type") {
                reader.headers.push((
                    "Content-Type".to_owned(),
                    "application/x-www-form-urlencoded".to_owned(),
                ));
            }
            body = Some(data);
        }
    }
    let method = reader.method.clone().unwrap_or_else(|| {
        if reader.head {
            "HEAD".to_owned()
        } else if body.is_some() {
            "POST".to_owned()
        } else {
            "GET".to_owned()
        }
    });
    Ok(CurlRequest {
        method,
        url,
        headers: reader.headers,
        body,
        ignored: reader.ignored,
    })
}

/// `X-Api-Key` → `xApiKey`, `API_KEY` → `apiKey`.
pub fn camel(text: &str) -> String {
    let mut out = String::new();
    for (index, part) in text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .enumerate()
    {
        let lower = part.to_ascii_lowercase();
        if index == 0 {
            out.push_str(&lower);
        } else {
            let mut chars = lower.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

/// `$TOKEN` ou `${TOKEN}` : variable du shell, reprise en `{{token}}`.
fn shell_reference(value: &str) -> Option<String> {
    let name = value.strip_prefix('$')?;
    let name = name
        .strip_prefix('{')
        .and_then(|n| n.strip_suffix('}'))
        .unwrap_or(name);
    let valid = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then(|| format!("{{{{{}}}}}", camel(name)))
}

/// En-tête dont la valeur est un secret, et le nom de la variable proposée.
fn secret_name(header: &str) -> Option<String> {
    let lower = header.to_ascii_lowercase();
    if lower == "cookie" {
        return Some("cookie".to_owned());
    }
    let compact: String = lower.chars().filter(char::is_ascii_alphanumeric).collect();
    ["apikey", "token", "secret", "password"]
        .iter()
        .any(|word| compact.contains(word))
        .then(|| camel(header))
}

/// Retire les secrets des en-têtes (R5) : chacun est remplacé par `{{nom}}` et
/// rendu à part. Une variable du shell (`$TOKEN`) devient `{{token}}`, sans
/// secret ; une valeur qui est déjà une variable `{{…}}` est laissée.
pub fn take_secrets(headers: &mut [(String, String)]) -> Vec<Secret> {
    let mut secrets: Vec<Secret> = Vec::new();
    let unique = |base: &str, secrets: &[Secret]| {
        let mut name = base.to_owned();
        let mut n = 2;
        while secrets.iter().any(|s| s.name == name) {
            name = format!("{base}{n}");
            n += 1;
        }
        name
    };
    for (header, value) in headers.iter_mut() {
        if value.contains("{{") || value.trim().is_empty() {
            continue;
        }
        let (prefix, secret, base) = if header.eq_ignore_ascii_case("Authorization") {
            match value.split_once(' ') {
                Some((scheme, rest)) if scheme.eq_ignore_ascii_case("Bearer") => (
                    format!("{scheme} "),
                    rest.trim().to_owned(),
                    "token".to_owned(),
                ),
                Some((scheme, rest)) if scheme.eq_ignore_ascii_case("Basic") => (
                    format!("{scheme} "),
                    rest.trim().to_owned(),
                    "basicAuth".to_owned(),
                ),
                _ => (String::new(), value.clone(), "authorization".to_owned()),
            }
        } else if let Some(base) = secret_name(header) {
            (String::new(), value.clone(), base)
        } else {
            continue;
        };
        if let Some(reference) = shell_reference(&secret) {
            *value = format!("{prefix}{reference}");
            continue;
        }
        let name = unique(&base, &secrets);
        *value = format!("{prefix}{{{{{name}}}}}");
        secrets.push(Secret {
            name,
            value: secret,
            header: header.clone(),
        });
    }
    secrets
}
