//! Authentification réglée sur le projet (`docs/plan-mvp-h-reseau-auth.md`, OA1, OA4) :
//! `request.auth` de `opencollection.yml`, et qui l'hérite ou la remplace.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::{CommandError, CommandErrorKind};
use crate::opencollection::collection;
use crate::opencollection::request::{auth_from, auth_value};
use crate::request::model::{Auth, AuthKind};
use crate::request::{file, inherit};
use crate::tree::{self, TreeNode};

/// Ce qui remplace l'authentification du projet : un dossier, une requête, ou les requêtes sans
/// authentification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthOverride {
    /// « dossier « v1-legacy » », « requête « Santé » », ou vide pour « sans authentification ».
    pub label: String,
    /// Chemin du dossier ou de la requête ; vide pour le groupe « sans authentification ».
    pub path: String,
    #[serde(rename = "type")]
    pub kind: AuthKind,
    /// Type d'une authentification `other`.
    pub other: String,
    /// Requêtes concernées.
    pub count: usize,
}

/// Qui utilise l'authentification du projet (OA4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUsage {
    /// Requêtes qui l'héritent.
    pub inheriting: usize,
    pub overrides: Vec<AuthOverride>,
}

/// Authentification du projet ; absente : aucune.
pub fn load(root: &Path) -> Result<Auth, CommandError> {
    let value = collection::read(root)?;
    Ok(auth_from(value.pointer("/request/auth")))
}

/// Écrit l'authentification du projet, validée contre le schéma (R8). « Héritée » n'a pas de
/// sens pour le projet : elle est refusée.
pub fn save(root: &Path, auth: &Auth) -> Result<(), CommandError> {
    if auth.kind == AuthKind::Inherit {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Le projet n'a rien dont hériter : choisissez une authentification ou « Aucune ».",
        ));
    }
    let mut value = collection::read(root)?;
    let previous = value.pointer("/request/auth").cloned();
    let next = auth_value(auth, previous.as_ref());
    let Some(map) = value.as_object_mut() else {
        return Err(CommandError::new(
            CommandErrorKind::NotAProject,
            "La racine du projet est illisible.",
        ));
    };
    let request = map
        .entry("request")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(request) = request.as_object_mut() {
        match next {
            Some(auth) => {
                request.insert("auth".to_owned(), auth);
            }
            None => {
                request.remove("auth");
            }
        }
    }
    if map
        .get("request")
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        map.remove("request");
    }
    collection::write(root, &value)
}

fn requests(nodes: &[TreeNode], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            TreeNode::Folder { children, .. } => requests(children, out),
            TreeNode::Request {
                name,
                path,
                invalid: false,
                ..
            } => out.push((name.clone(), path.clone())),
            TreeNode::Request { .. } => {}
        }
    }
}

/// Qui hérite de l'authentification du projet et qui la remplace (OA4).
pub fn usage(root: &Path) -> AuthUsage {
    let mut found = Vec::new();
    requests(&tree::read(root).unwrap_or_default(), &mut found);
    let mut usage = AuthUsage::default();
    let mut folders: BTreeMap<String, AuthOverride> = BTreeMap::new();
    let mut without = 0usize;
    let collection_source = "collection «";
    for (name, path) in found {
        let Ok(request) = tree::resolve(root, &path).and_then(|p| file::read(&p)) else {
            continue;
        };
        match request.auth.kind {
            AuthKind::Inherit => match inherit::resolve(root, &path).auth {
                Some(inherited) if inherited.source.starts_with(collection_source) => {
                    usage.inheriting += 1;
                }
                Some(inherited) => {
                    let entry =
                        folders
                            .entry(inherited.source.clone())
                            .or_insert_with(|| AuthOverride {
                                label: inherited.source.clone(),
                                path: folder_of(&path),
                                kind: inherited.auth.kind,
                                other: inherited.auth.other.clone(),
                                count: 0,
                            });
                    entry.count += 1;
                }
                None => usage.inheriting += 1,
            },
            AuthKind::None => without += 1,
            kind => usage.overrides.push(AuthOverride {
                label: format!("requête « {name} »"),
                path,
                kind,
                other: request.auth.other.clone(),
                count: 1,
            }),
        }
    }
    let mut all: Vec<AuthOverride> = folders.into_values().collect();
    all.append(&mut usage.overrides);
    if without > 0 {
        all.push(AuthOverride {
            label: String::new(),
            path: String::new(),
            kind: AuthKind::None,
            other: String::new(),
            count: without,
        });
    }
    usage.overrides = all;
    usage
}

fn folder_of(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(folder, _)| folder.to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::request::model::{OAuth2, Request};

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("opencollection.yml"),
            "opencollection: 1.0.0\ninfo:\n  name: Boutique\n",
        )
        .unwrap();
        dir
    }

    fn request(dir: &Path, path: &str, name: &str, kind: AuthKind) {
        let mut r = Request::new(name);
        r.auth.kind = kind;
        let file_path = dir.join(path);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        file::write(&file_path, &r).unwrap();
    }

    #[test]
    fn oa1_authentification_du_projet_ecrite_et_relue() {
        let dir = project();
        assert_eq!(load(dir.path()).unwrap().kind, AuthKind::None);
        let auth = Auth {
            kind: AuthKind::OAuth2,
            oauth2: OAuth2 {
                access_token_url: "{{authUrl}}/oauth/token".into(),
                client_id: "{{oauthClientId}}".into(),
                client_secret: "{{oauth2ClientSecret}}".into(),
                scope: "orders:read".into(),
                ..OAuth2::default()
            },
            ..Auth::default()
        };
        save(dir.path(), &auth).unwrap();
        assert_eq!(load(dir.path()).unwrap(), auth);
        let text = fs::read_to_string(dir.path().join("opencollection.yml")).unwrap();
        assert!(text.contains("flow: client_credentials"), "{text}");
        assert!(!text.contains("s3cr3t"));
        // Aucune : la clé disparaît.
        save(dir.path(), &Auth::default()).unwrap();
        let text = fs::read_to_string(dir.path().join("opencollection.yml")).unwrap();
        assert!(!text.contains("request"), "{text}");
        assert_eq!(
            save(
                dir.path(),
                &Auth {
                    kind: AuthKind::Inherit,
                    ..Auth::default()
                }
            )
            .unwrap_err()
            .kind,
            CommandErrorKind::InvalidRequest
        );
    }

    #[test]
    fn oa4_qui_l_herite_et_qui_la_remplace() {
        let dir = project();
        request(
            dir.path(),
            "commandes/lister.yml",
            "Lister",
            AuthKind::Inherit,
        );
        request(dir.path(), "commandes/lire.yml", "Lire", AuthKind::Inherit);
        request(
            dir.path(),
            "v1-legacy/ancien.yml",
            "Ancien",
            AuthKind::Inherit,
        );
        request(dir.path(), "sante.yml", "Santé", AuthKind::None);
        request(dir.path(), "admin.yml", "Admin", AuthKind::Basic);
        fs::write(
            dir.path().join("v1-legacy/folder.yml"),
            "info:\n  name: v1-legacy\n  type: folder\nrequest:\n  auth:\n    type: basic\n    username: a\n    password: b\n",
        )
        .unwrap();
        save(
            dir.path(),
            &Auth {
                kind: AuthKind::Bearer,
                token: "{{token}}".into(),
                ..Auth::default()
            },
        )
        .unwrap();
        let usage = usage(dir.path());
        assert_eq!(usage.inheriting, 2);
        let labels: Vec<(&str, AuthKind, usize)> = usage
            .overrides
            .iter()
            .map(|o| (o.label.as_str(), o.kind, o.count))
            .collect();
        assert_eq!(
            labels,
            [
                ("dossier « v1-legacy »", AuthKind::Basic, 1),
                ("requête « Admin »", AuthKind::Basic, 1),
                ("", AuthKind::None, 1),
            ]
        );
        // Forme JSON.
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["overrides"][0]["type"], "basic");
        assert_eq!(json["overrides"][0]["path"], "v1-legacy");
    }
}
