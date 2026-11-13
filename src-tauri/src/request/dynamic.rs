//! Valeurs dynamiques `{{$nom}}` (`docs/plan-mvp-e-tests-etapes.md`, R9).
//!
//! Module pur : les noms de Bruno, tirés à chaque occurrence. Le hasard vient
//! d'une graine (un UUID v4 à l'envoi, fixe dans les tests) et l'heure est
//! donnée par l'appelant : aucune crate ajoutée.

use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

/// Noms reconnus et leur description, pour l'interface et la documentation ; les plus
/// courants d'abord (ordre de la liste proposée à la frappe de `{{$`, 5.1-h).
pub const NAMES: [(&str, &str); 30] = [
    ("$isoTimestamp", "Date et heure, ISO 8601"),
    ("$timestamp", "Secondes depuis 1970"),
    ("$randomUUID", "Identifiant unique"),
    ("$date", "Date du jour"),
    ("$date:+3d", "Date décalée : +3d, -1d, +2h, +30m"),
    ("$randomInt", "Nombre de 0 à 1 000"),
    ("$randomEmail", "Adresse e-mail"),
    ("$randomFirstName", "Prénom"),
    ("$randomLastName", "Nom de famille"),
    ("$randomFullName", "Prénom et nom"),
    ("$guid", "Identifiant unique"),
    ("$randomFloat", "Nombre décimal de 0 à 1 000"),
    ("$randomBoolean", "true ou false"),
    ("$randomAlphaNumeric", "Lettre ou chiffre"),
    ("$randomHexColor", "Couleur #rrggbb"),
    ("$randomIP", "Adresse IPv4"),
    ("$randomIPV6", "Adresse IPv6"),
    ("$randomMACAddress", "Adresse MAC"),
    ("$randomPassword", "Mot de passe de 15 caractères"),
    ("$randomUserName", "Identifiant d'utilisateur"),
    ("$randomPhoneNumber", "Numéro de téléphone"),
    ("$randomCity", "Ville"),
    ("$randomCountry", "Pays"),
    ("$randomCountryCode", "Code de pays"),
    ("$randomDomainName", "Nom de domaine"),
    ("$randomUrl", "Adresse web"),
    ("$randomWord", "Mot"),
    ("$randomWords", "Quelques mots"),
    ("$randomSemver", "Version x.y.z"),
    ("$randomPrice", "Prix, deux décimales"),
];

/// Valeur dynamique proposée à la saisie (`dynamic_values`, 5.1-h) : nom, description et
/// un exemple tiré maintenant (une autre valeur est tirée à chaque envoi).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicValue {
    pub name: String,
    pub description: String,
    pub example: String,
}

/// Toutes les valeurs dynamiques, dans l'ordre de [`NAMES`], chacune avec un exemple.
pub fn catalog(dice: &mut Dice) -> Vec<DynamicValue> {
    NAMES
        .iter()
        .map(|(name, description)| DynamicValue {
            name: (*name).to_owned(),
            description: (*description).to_owned(),
            example: draw(name, dice).unwrap_or_default(),
        })
        .collect()
}

const FIRST_NAMES: [&str; 12] = [
    "Léa", "Hugo", "Chloé", "Louis", "Emma", "Jules", "Inès", "Gabriel", "Manon", "Arthur",
    "Camille", "Lucas",
];
const LAST_NAMES: [&str; 12] = [
    "Martin", "Bernard", "Dubois", "Thomas", "Robert", "Richard", "Petit", "Durand", "Leroy",
    "Moreau", "Simon", "Laurent",
];
const CITIES: [&str; 10] = [
    "Angers",
    "Nantes",
    "Lyon",
    "Lille",
    "Bordeaux",
    "Rennes",
    "Toulouse",
    "Paris",
    "Tours",
    "Montréal",
];
const COUNTRIES: [(&str, &str); 8] = [
    ("France", "FR"),
    ("Belgique", "BE"),
    ("Suisse", "CH"),
    ("Canada", "CA"),
    ("Espagne", "ES"),
    ("Italie", "IT"),
    ("Allemagne", "DE"),
    ("Portugal", "PT"),
];
const WORDS: [&str; 12] = [
    "colis",
    "panier",
    "facture",
    "client",
    "commande",
    "remise",
    "stock",
    "livraison",
    "article",
    "paiement",
    "retour",
    "avoir",
];
const DOMAINS: [&str; 4] = ["example.com", "example.org", "example.net", "test.fr"];
const ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const PASSWORD: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!#$%&*+-?@";

/// Source de hasard et heure d'un envoi.
#[derive(Debug, Clone)]
pub struct Dice {
    state: u64,
    /// Heure de l'envoi, en millisecondes depuis l'époque Unix.
    pub now_ms: u64,
}

impl Dice {
    /// Hasard et heure de l'envoi.
    pub fn new() -> Self {
        let bytes = Uuid::new_v4().as_u128();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0));
        Self::seeded((bytes as u64) ^ ((bytes >> 64) as u64), now_ms)
    }

    /// Hasard reproductible (tests).
    pub fn seeded(seed: u64, now_ms: u64) -> Self {
        Self {
            state: seed | 1,
            now_ms,
        }
    }

    /// Nombre suivant (xorshift64*).
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Nombre de `0` à `max` compris.
    fn below(&mut self, max: u64) -> u64 {
        self.next() % (max + 1)
    }

    fn pick<'a>(&mut self, list: &[&'a str]) -> &'a str {
        let index = usize::try_from(self.below(list.len() as u64 - 1)).unwrap_or(0);
        list.get(index).copied().unwrap_or_default()
    }

    fn chars(&mut self, alphabet: &[u8], count: usize) -> String {
        (0..count)
            .map(|_| {
                let index = usize::try_from(self.below(alphabet.len() as u64 - 1)).unwrap_or(0);
                char::from(alphabet.get(index).copied().unwrap_or(b'x'))
            })
            .collect()
    }

    fn uuid(&mut self) -> String {
        let high = u128::from(self.next()) << 64;
        let bytes = (high | u128::from(self.next())).to_be_bytes();
        uuid::Builder::from_random_bytes(bytes)
            .into_uuid()
            .to_string()
    }
}

impl Default for Dice {
    fn default() -> Self {
        Self::new()
    }
}

/// Le nom désigne une valeur dynamique (R1 du plan d'ensemble).
pub fn is_dynamic(name: &str) -> bool {
    name.starts_with('$')
}

/// Jours depuis l'époque → (année, mois, jour), calendrier grégorien.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Date et heure ISO 8601 en UTC, à la seconde (`2026-11-12T15:02:11Z`).
pub fn iso(ms: i64) -> String {
    let seconds = ms.div_euclid(1000);
    let (year, month, day) = civil(seconds.div_euclid(86_400));
    let rest = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Décalage `+3d`, `-1d`, `+2h`, `+30m` en millisecondes.
fn offset(text: &str) -> Option<i64> {
    let text = text.trim();
    let (sign, rest) = match text.as_bytes().first()? {
        b'+' => (1, &text[1..]),
        b'-' => (-1, &text[1..]),
        _ => (1, text),
    };
    let unit = rest.chars().last()?;
    let amount: i64 = rest[..rest.len() - unit.len_utf8()].parse().ok()?;
    let ms = match unit {
        'd' => 86_400_000,
        'h' => 3_600_000,
        'm' => 60_000,
        's' => 1000,
        _ => return None,
    };
    Some(sign * amount * ms)
}

/// Valeur tirée pour `name` (avec son `$`), ou `None` s'il est inconnu (R9).
pub fn draw(name: &str, dice: &mut Dice) -> Option<String> {
    let now = i64::try_from(dice.now_ms).unwrap_or(0);
    if let Some(shift) = name.strip_prefix("$date") {
        let shift = match shift.strip_prefix(':') {
            Some(text) => offset(text)?,
            None if shift.is_empty() => 0,
            None => return None,
        };
        return Some(iso(now + shift)[..10].to_owned());
    }
    let value = match name {
        "$guid" | "$randomUUID" => dice.uuid(),
        "$timestamp" => (now / 1000).to_string(),
        "$isoTimestamp" => iso(now),
        "$randomInt" => dice.below(1000).to_string(),
        "$randomFloat" => format!("{}.{:02}", dice.below(999), dice.below(99)),
        "$randomBoolean" => (dice.below(1) == 1).to_string(),
        "$randomAlphaNumeric" => dice.chars(ALPHANUMERIC, 1),
        "$randomHexColor" => format!("#{:06x}", dice.below(0xff_ffff)),
        "$randomIP" => format!(
            "{}.{}.{}.{}",
            dice.below(254) + 1,
            dice.below(255),
            dice.below(255),
            dice.below(254) + 1
        ),
        "$randomIPV6" => (0..8)
            .map(|_| format!("{:x}", dice.below(0xffff)))
            .collect::<Vec<_>>()
            .join(":"),
        "$randomMACAddress" => (0..6)
            .map(|_| format!("{:02x}", dice.below(0xff)))
            .collect::<Vec<_>>()
            .join(":"),
        "$randomPassword" => dice.chars(PASSWORD, 15),
        "$randomFirstName" => dice.pick(&FIRST_NAMES).to_owned(),
        "$randomLastName" => dice.pick(&LAST_NAMES).to_owned(),
        "$randomFullName" => format!("{} {}", dice.pick(&FIRST_NAMES), dice.pick(&LAST_NAMES)),
        "$randomUserName" => format!("{}{}", ascii(dice.pick(&FIRST_NAMES)), dice.below(99)),
        "$randomEmail" => format!(
            "{}.{}{}@{}",
            ascii(dice.pick(&FIRST_NAMES)),
            ascii(dice.pick(&LAST_NAMES)),
            dice.below(99),
            dice.pick(&DOMAINS)
        ),
        "$randomPhoneNumber" => format!(
            "0{} {:02} {:02} {:02} {:02}",
            dice.below(5) + 1,
            dice.below(99),
            dice.below(99),
            dice.below(99),
            dice.below(99)
        ),
        "$randomCity" => dice.pick(&CITIES).to_owned(),
        "$randomCountry" | "$randomCountryCode" => {
            let index = usize::try_from(dice.below(COUNTRIES.len() as u64 - 1)).unwrap_or(0);
            let (country, code) = COUNTRIES.get(index).copied().unwrap_or(("France", "FR"));
            if name == "$randomCountry" {
                country.to_owned()
            } else {
                code.to_owned()
            }
        }
        "$randomDomainName" => format!("{}.example.com", dice.pick(&WORDS)),
        "$randomUrl" => format!("https://{}.example.com", dice.pick(&WORDS)),
        "$randomWord" => dice.pick(&WORDS).to_owned(),
        "$randomWords" => (0..3)
            .map(|_| dice.pick(&WORDS))
            .collect::<Vec<_>>()
            .join(" "),
        "$randomSemver" => format!("{}.{}.{}", dice.below(9), dice.below(20), dice.below(30)),
        "$randomPrice" => format!("{}.{:02}", dice.below(999), dice.below(99)),
        _ => return None,
    };
    Some(value)
}

/// Texte sans accent ni majuscule, pour une adresse e-mail.
fn ascii(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' | 'É' => 'e',
            'à' | 'â' => 'a',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'û' | 'ù' => 'u',
            'ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 12 novembre 2026, 15:02:11 UTC.
    const NOW: u64 = 1_794_495_731_000;

    fn dice() -> Dice {
        Dice::seeded(42, NOW)
    }

    #[test]
    fn r9_dates_et_heures() {
        let mut d = dice();
        assert_eq!(
            draw("$isoTimestamp", &mut d).unwrap(),
            "2026-11-12T15:02:11Z"
        );
        assert_eq!(draw("$timestamp", &mut d).unwrap(), "1794495731");
        assert_eq!(draw("$date", &mut d).unwrap(), "2026-11-12");
        assert_eq!(draw("$date:+3d", &mut d).unwrap(), "2026-11-15");
        assert_eq!(draw("$date:-12d", &mut d).unwrap(), "2026-10-31");
        assert_eq!(draw("$date:+9h", &mut d).unwrap(), "2026-11-13");
        assert_eq!(draw("$date:demain", &mut d), None);
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso(951_782_400_000), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn r9_chaque_tirage_change() {
        let mut d = dice();
        let a = draw("$randomUUID", &mut d).unwrap();
        let b = draw("$guid", &mut d).unwrap();
        assert_ne!(a, b);
        assert_eq!(Uuid::parse_str(&a).unwrap().get_version_num(), 4);
        let n: u64 = draw("$randomInt", &mut d).unwrap().parse().unwrap();
        assert!(n <= 1000);
        let email = draw("$randomEmail", &mut d).unwrap();
        assert!(email.contains('@') && email.is_ascii(), "{email}");
        assert!(draw("$randomHexColor", &mut d).unwrap().starts_with('#'));
        assert_eq!(draw("$randomPassword", &mut d).unwrap().chars().count(), 15);
    }

    #[test]
    fn r9_tous_les_noms_sont_tires_et_un_inconnu_ne_lest_pas() {
        let mut d = dice();
        for (name, _) in NAMES {
            assert!(draw(name, &mut d).is_some_and(|v| !v.is_empty()), "{name}");
        }
        assert_eq!(draw("$hasard", &mut d), None);
        assert!(is_dynamic("$guid") && !is_dynamic("guid"));
    }

    #[test]
    fn la_meme_graine_donne_les_memes_valeurs() {
        let (mut a, mut b) = (dice(), dice());
        assert_eq!(draw("$randomUUID", &mut a), draw("$randomUUID", &mut b));
    }

    #[test]
    fn h_catalogue_des_valeurs_proposees() {
        let list = catalog(&mut dice());
        assert_eq!(list.len(), NAMES.len());
        let first = list.first().unwrap();
        assert_eq!(first.name, "$isoTimestamp");
        assert_eq!(first.example, "2026-11-12T15:02:11Z");
        let shifted = list.iter().find(|v| v.name == "$date:+3d").unwrap();
        assert_eq!(shifted.example, "2026-11-15");
        assert!(list.iter().all(|v| !v.example.is_empty()));
        let json = serde_json::to_value(first).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "name": "$isoTimestamp",
                "description": "Date et heure, ISO 8601",
                "example": "2026-11-12T15:02:11Z"
            })
        );
    }
}
