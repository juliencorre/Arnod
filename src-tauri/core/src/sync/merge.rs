//! Fusion à trois versions, élément par élément (V2, lots E et F, R12, R13). Module pur.
//!
//! Pour chaque fichier changé des deux côtés depuis la version commune : ce qu'un seul côté a
//! changé est pris sans question ; seul un élément changé des deux côtés, différemment, est un
//! conflit. Une requête se découpe en éléments comme le conflit du lot I
//! ([`crate::watch::conflict`]) : champs simples et lignes des listes. Les autres fichiers
//! (environnements, contrats, réglages) se gardent en entier, d'un côté ou de l'autre (Q8).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::opencollection::request::into_value;
use crate::opencollection::yaml;
use crate::request::file;
use crate::request::model::Request;
use crate::watch::conflict::{self, ConflictSide};

/// Clé de l'élément « fichier entier ».
pub const WHOLE: &str = "*";

/// Version gardée pour un élément en conflit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncChoice"))]
#[serde(rename_all = "camelCase")]
pub enum Choice {
    /// Ma version (« Vous »).
    Ours,
    /// Celle du collègue.
    Theirs,
    /// Les deux lignes (une ligne de liste seulement : « Garder les deux assertions »).
    Both,
}

/// Élément changé des deux côtés, différemment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncConflictElement"))]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// Clé stable (`url`, `assertions:body.total#0`, `*` pour le fichier entier).
    pub key: String,
    /// Libellé en français (« Assertion body.total »).
    pub label: String,
    /// Ma version, prête à afficher (`None` : absente ou supprimée).
    pub ours: Option<String>,
    /// Celle du collègue.
    pub theirs: Option<String>,
    /// « Garder les deux » est possible (ligne d'une liste).
    pub both: bool,
}

/// Issue de la fusion d'un fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Contenu fusionné (`None` : le fichier est supprimé).
    Clean(Option<Vec<u8>>),
    /// Des éléments attendent un choix ; `auto` liste ce qui a été pris sans question.
    Conflict {
        elements: Vec<Element>,
        auto: Vec<Auto>,
    },
}

/// Élément pris d'un seul côté, sans question (CF2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SyncMergedElement"))]
#[serde(rename_all = "camelCase")]
pub struct Auto {
    pub label: String,
    /// `true` : la version du collègue (lui seul l'a changé) ; `false` : la mienne.
    pub theirs: bool,
}

fn is_list(key: &str) -> bool {
    ["params:", "headers:", "assertions:", "extractions:"]
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

/// Fichier de requête (et pas `folder.yml`, la racine, un environnement ou un contrat).
pub fn is_request(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    (lower.ends_with(".yml") || lower.ends_with(".yaml"))
        && name != "folder.yml"
        && name != "opencollection.yml"
        && !lower.starts_with("environments/")
        && !lower.starts_with("contracts/")
}

fn parse(path: &str, bytes: &[u8]) -> Option<(Request, serde_json::Value)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let request = file::parse(text, Path::new(path)).ok()?;
    let value = yaml::parse(text).ok()?;
    Some((request, value))
}

fn whole(base: Option<&[u8]>, ours: Option<&[u8]>, theirs: Option<&[u8]>) -> Element {
    let shown = |side: Option<&[u8]>| {
        side.map(|bytes| {
            if base.is_none() {
                "ajouté".to_owned()
            } else if Some(bytes) == base {
                "inchangé".to_owned()
            } else {
                "modifié".to_owned()
            }
        })
        .or_else(|| Some("supprimé".to_owned()))
    };
    Element {
        key: WHOLE.to_owned(),
        label: "Fichier entier".to_owned(),
        ours: shown(ours),
        theirs: shown(theirs),
        both: false,
    }
}

/// Fusionne un fichier ; `choices` règle les éléments en conflit (lot F, CF1, CF3).
pub fn merge_file(
    path: &str,
    base: Option<&[u8]>,
    ours: Option<&[u8]>,
    theirs: Option<&[u8]>,
    choices: &BTreeMap<String, Choice>,
) -> Merged {
    if ours == theirs || theirs == base {
        return Merged::Clean(ours.map(<[u8]>::to_vec));
    }
    if ours == base {
        return Merged::Clean(theirs.map(<[u8]>::to_vec));
    }
    let parsed = if is_request(path) {
        match (base, ours, theirs) {
            (Some(b), Some(o), Some(t)) => parse(path, b).zip(parse(path, o)).zip(parse(path, t)),
            _ => None,
        }
    } else {
        None
    };
    let Some((((base_req, _), (ours_req, ours_value)), (theirs_req, _))) = parsed else {
        // Fichier gardé en entier, d'un côté ou de l'autre (Q8).
        return match choices.get(WHOLE) {
            Some(Choice::Theirs) => Merged::Clean(theirs.map(<[u8]>::to_vec)),
            Some(Choice::Ours | Choice::Both) => Merged::Clean(ours.map(<[u8]>::to_vec)),
            None => Merged::Conflict {
                elements: vec![whole(base, ours, theirs)],
                auto: Vec::new(),
            },
        };
    };
    // « disque » = le collègue, « brouillon » = moi.
    let items = conflict::compare(&base_req, &theirs_req, &ours_req);
    let mut sides: BTreeMap<String, ConflictSide> = BTreeMap::new();
    let mut both: BTreeSet<String> = BTreeSet::new();
    let mut elements = Vec::new();
    let mut auto = Vec::new();
    for item in items {
        if item.identical {
            continue;
        }
        match item.suggested {
            Some(side) => {
                auto.push(Auto {
                    label: item.label.clone(),
                    theirs: side == ConflictSide::Disk,
                });
                sides.insert(item.key.clone(), side);
            }
            None => match choices.get(&item.key) {
                Some(Choice::Ours) => {
                    sides.insert(item.key.clone(), ConflictSide::Draft);
                }
                Some(Choice::Theirs) => {
                    sides.insert(item.key.clone(), ConflictSide::Disk);
                }
                Some(Choice::Both) if is_list(&item.key) => {
                    both.insert(item.key.clone());
                }
                _ => elements.push(Element {
                    both: is_list(&item.key),
                    key: item.key,
                    label: item.label,
                    ours: item.draft,
                    theirs: item.disk,
                }),
            },
        }
    }
    if !elements.is_empty() {
        return Merged::Conflict { elements, auto };
    }
    let Ok(merged) = conflict::merge_keeping(&theirs_req, &ours_req, &sides, &both) else {
        return Merged::Conflict {
            elements: vec![whole(base, ours, theirs)],
            auto,
        };
    };
    // Identique à une version (tout pris d'un côté) : ses octets, sans mise en forme nouvelle,
    // pour qu'elle ne paraisse pas modifiée.
    let same = |other: &Request| conflict::compare(&merged, &merged, other).is_empty();
    if same(&theirs_req) {
        return Merged::Clean(theirs.map(<[u8]>::to_vec));
    }
    if same(&ours_req) {
        return Merged::Clean(ours.map(<[u8]>::to_vec));
    }
    match yaml::render(&into_value(&merged, Some(&ours_value))) {
        Ok(text) => Merged::Clean(Some(text.into_bytes())),
        Err(_) => Merged::Conflict {
            elements: vec![whole(base, ours, theirs)],
            auto,
        },
    }
}

/// Éléments pris sans question pour un fichier fusionné proprement (CF2).
pub fn auto_merged(
    path: &str,
    base: Option<&[u8]>,
    ours: Option<&[u8]>,
    theirs: Option<&[u8]>,
) -> Vec<Auto> {
    match merge_file(path, base, ours, theirs, &BTreeMap::new()) {
        Merged::Conflict { auto, .. } => auto,
        Merged::Clean(_) => {
            let (Some(b), Some(o), Some(t)) = (base, ours, theirs) else {
                return Vec::new();
            };
            let (Some((b, _)), Some((o, _)), Some((t, _))) =
                (parse(path, b), parse(path, o), parse(path, t))
            else {
                return Vec::new();
            };
            conflict::compare(&b, &t, &o)
                .into_iter()
                .filter(|item| !item.identical)
                .filter_map(|item| {
                    item.suggested.map(|side| Auto {
                        label: item.label,
                        theirs: side == ConflictSide::Disk,
                    })
                })
                .collect()
        }
    }
}
