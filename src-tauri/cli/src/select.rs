//! Ce que `arnod run` joue (LC1, LC3, LC5) : un dossier, un test, une requête, tout le projet,
//! ou les tests qui portent une étiquette (R4 : étiquettes effectives, héritées des dossiers).

use arnod_core::tree::TreeNode;

use crate::suggest::closest;

/// Nature de la cible, pour l'en-tête de la sortie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Project,
    Folder,
    Test,
    Request,
    Tags,
}

/// Run à lancer : dossier exécuté et, s'il le faut, les seules requêtes à jouer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub kind: Kind,
    /// Dossier exécuté (`""` : tout le projet).
    pub folder: String,
    pub only: Option<Vec<String>>,
    /// Ce qui est lancé, en clair : « Dossier commandes/v2 », « Étiquette smoke ».
    pub title: String,
}

/// Comparaison d'une étiquette : sans casse ni accents (comme le filtre du référentiel, R1).
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'û' | 'ü' | 'ú' | 'ù' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'œ' => 'o',
            c => c,
        })
        .collect()
}

/// Chemin saisi, ramené à la forme de l'arborescence (`commandes/v2`).
fn normalize(target: &str) -> String {
    let path = target.trim().replace('\\', "/");
    let path = path.trim_start_matches("./").trim_matches('/');
    path.to_owned()
}

fn walk<'a>(nodes: &'a [TreeNode], visit: &mut impl FnMut(&'a TreeNode)) {
    for node in nodes {
        visit(node);
        if let TreeNode::Folder { children, .. } = node {
            walk(children, visit);
        }
    }
}

fn find_folder<'a>(nodes: &'a [TreeNode], wanted: &str) -> Option<&'a TreeNode> {
    let mut exact = None;
    let mut loose = None;
    walk(nodes, &mut |node| {
        if let TreeNode::Folder { path, .. } = node {
            if path == wanted {
                exact = exact.or(Some(node));
            } else if path.eq_ignore_ascii_case(wanted) {
                loose = loose.or(Some(node));
            }
        }
    });
    exact.or(loose)
}

fn find_request<'a>(nodes: &'a [TreeNode], wanted: &str) -> Option<&'a TreeNode> {
    let with_ext = format!("{wanted}.yml");
    let mut found = None;
    walk(nodes, &mut |node| {
        if let TreeNode::Request { path, invalid, .. } = node
            && !invalid
            && (path == wanted || *path == with_ext)
        {
            found = found.or(Some(node));
        }
    });
    found
}

/// Chemins d'un nœud : ses requêtes lisibles, dans l'ordre.
fn requests(node: &TreeNode, out: &mut Vec<String>) {
    match node {
        TreeNode::Folder { children, .. } => {
            for child in children {
                requests(child, out);
            }
        }
        TreeNode::Request {
            path,
            invalid: false,
            ..
        } => out.push(path.clone()),
        TreeNode::Request { .. } => {}
    }
}

fn has_tag(tags: &[String], wanted: &[String]) -> bool {
    tags.iter()
        .any(|tag| wanted.iter().any(|w| fold(tag) == fold(w)))
}

/// Requêtes à jouer pour des étiquettes : chaque test dont une étape porte l'étiquette (toutes
/// ses étapes), chaque requête hors test qui la porte.
fn tagged(nodes: &[TreeNode], wanted: &[String], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            TreeNode::Folder {
                children,
                test: true,
                ..
            } => {
                let any = children.iter().any(|child| {
                    matches!(child, TreeNode::Request { tags, invalid: false, .. } if has_tag(tags, wanted))
                });
                if any {
                    requests(node, out);
                }
            }
            TreeNode::Folder { children, .. } => tagged(children, wanted, out),
            TreeNode::Request {
                path,
                tags,
                invalid: false,
                ..
            } if has_tag(tags, wanted) => out.push(path.clone()),
            TreeNode::Request { .. } => {}
        }
    }
}

/// Toutes les cibles possibles, pour proposer la plus proche (LC5).
fn candidates(nodes: &[TreeNode]) -> Vec<String> {
    let mut out = Vec::new();
    walk(nodes, &mut |node| match node {
        TreeNode::Folder { path, .. } => out.push(path.clone()),
        TreeNode::Request { path, .. } => {
            out.push(path.strip_suffix(".yml").unwrap_or(path).to_owned());
        }
    });
    out
}

fn tags_title(tags: &[String]) -> String {
    let label = if tags.len() > 1 {
        "Étiquettes"
    } else {
        "Étiquette"
    };
    format!("{label} {}", tags.join(", "))
}

/// Run de la cible et des étiquettes demandées.
pub fn resolve(nodes: &[TreeNode], target: Option<&str>, tags: &[String]) -> Result<Plan, String> {
    let target = target.map(normalize).filter(|t| !t.is_empty());
    let scope: Option<&TreeNode> = match &target {
        None => None,
        Some(wanted) => Some(
            find_folder(nodes, wanted)
                .or_else(|| find_request(nodes, wanted))
                .ok_or_else(|| {
                    let all = candidates(nodes);
                    let mut message = format!("La cible « {wanted} » n'existe pas dans ce projet.");
                    if let Some(near) = closest(wanted, all.iter().map(String::as_str)) {
                        message.push_str(&format!(" Vouliez-vous dire « {near} » ?"));
                    }
                    message
                })?,
        ),
    };
    if !tags.is_empty() {
        let mut only = Vec::new();
        match scope {
            None => tagged(nodes, tags, &mut only),
            Some(node @ TreeNode::Folder { test: true, .. }) => {
                tagged(std::slice::from_ref(node), tags, &mut only);
            }
            Some(TreeNode::Folder { children, .. }) => tagged(children, tags, &mut only),
            Some(node @ TreeNode::Request { .. }) => {
                tagged(std::slice::from_ref(node), tags, &mut only);
            }
        }
        if only.is_empty() {
            return Err(format!(
                "Aucun test ne porte l'étiquette {}{}.",
                tags.iter()
                    .map(|t| format!("« {t} »"))
                    .collect::<Vec<_>>()
                    .join(" ni "),
                target
                    .as_ref()
                    .map(|t| format!(" dans « {t} »"))
                    .unwrap_or_default()
            ));
        }
        let folder = match scope {
            Some(TreeNode::Folder { path, .. }) => path.clone(),
            _ => String::new(),
        };
        let mut title = tags_title(tags);
        if let Some(target) = &target {
            title.push_str(&format!(" dans {target}"));
        }
        return Ok(Plan {
            kind: Kind::Tags,
            folder,
            only: Some(only),
            title,
        });
    }
    Ok(match scope {
        None => Plan {
            kind: Kind::Project,
            folder: String::new(),
            only: None,
            title: "Projet entier".to_owned(),
        },
        Some(TreeNode::Folder { path, test, .. }) => Plan {
            kind: if *test { Kind::Test } else { Kind::Folder },
            folder: path.clone(),
            only: None,
            title: format!("{} {path}", if *test { "Test" } else { "Dossier" }),
        },
        Some(TreeNode::Request { path, name, .. }) => Plan {
            kind: Kind::Request,
            folder: path
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_owned())
                .unwrap_or_default(),
            only: Some(vec![path.clone()]),
            title: format!("Requête {name}"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str, tags: &[&str]) -> TreeNode {
        TreeNode::Request {
            name: path
                .rsplit('/')
                .next()
                .unwrap()
                .trim_end_matches(".yml")
                .to_owned(),
            path: path.to_owned(),
            method: None,
            assertion_count: 0,
            invalid: false,
            contract: None,
            url: String::new(),
            tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    fn folder(path: &str, test: bool, children: Vec<TreeNode>) -> TreeNode {
        TreeNode::Folder {
            name: path.rsplit('/').next().unwrap().to_owned(),
            path: path.to_owned(),
            request_count: children.len(),
            children,
            test,
            tags: Vec::new(),
        }
    }

    fn tree() -> Vec<TreeNode> {
        vec![folder(
            "commandes",
            false,
            vec![
                folder(
                    "commandes/v2",
                    false,
                    vec![
                        request("commandes/v2/lister.yml", &["smoke"]),
                        folder(
                            "commandes/v2/parcours",
                            true,
                            vec![
                                request("commandes/v2/parcours/a.yml", &[]),
                                request("commandes/v2/parcours/b.yml", &["Smoke", "lent"]),
                            ],
                        ),
                    ],
                ),
                request("commandes/sante.yml", &["santé"]),
            ],
        )]
    }

    #[test]
    fn lc1_dossier_test_requete_ou_projet() {
        let nodes = tree();
        let plan = resolve(&nodes, Some("./commandes/v2/"), &[]).unwrap();
        assert_eq!(
            (
                plan.kind,
                plan.folder.as_str(),
                plan.only,
                plan.title.as_str()
            ),
            (Kind::Folder, "commandes/v2", None, "Dossier commandes/v2")
        );
        let plan = resolve(&nodes, Some("commandes\\v2\\parcours"), &[]).unwrap();
        assert_eq!(
            (plan.kind, plan.title.as_str()),
            (Kind::Test, "Test commandes/v2/parcours")
        );
        let plan = resolve(&nodes, Some("commandes/v2/lister"), &[]).unwrap();
        assert_eq!(
            (plan.kind, plan.folder.as_str(), plan.only),
            (
                Kind::Request,
                "commandes/v2",
                Some(vec!["commandes/v2/lister.yml".to_owned()])
            )
        );
        let plan = resolve(&nodes, None, &[]).unwrap();
        assert_eq!((plan.kind, plan.folder.as_str()), (Kind::Project, ""));
        assert_eq!(
            resolve(&nodes, Some("COMMANDES/V2"), &[]).unwrap().folder,
            "commandes/v2"
        );
    }

    #[test]
    fn lc3_etiquettes_cumulees_tests_entiers() {
        let nodes = tree();
        let plan = resolve(&nodes, None, &["smoke".into()]).unwrap();
        assert_eq!(plan.title, "Étiquette smoke");
        assert_eq!(
            plan.only.unwrap(),
            [
                "commandes/v2/lister.yml",
                "commandes/v2/parcours/a.yml",
                "commandes/v2/parcours/b.yml"
            ]
        );
        let plan = resolve(&nodes, Some("commandes"), &["sante".into(), "lent".into()]).unwrap();
        assert_eq!(plan.title, "Étiquettes sante, lent dans commandes");
        assert_eq!(plan.folder, "commandes");
        assert_eq!(
            plan.only.unwrap(),
            [
                "commandes/v2/parcours/a.yml",
                "commandes/v2/parcours/b.yml",
                "commandes/sante.yml"
            ]
        );
        assert_eq!(
            resolve(&nodes, None, &["absente".into()]),
            Err("Aucun test ne porte l'étiquette « absente ».".into())
        );
    }

    #[test]
    fn lc5_une_cible_inconnue_propose_la_plus_proche() {
        assert_eq!(
            resolve(&tree(), Some("commande/v2"), &[]),
            Err("La cible « commande/v2 » n'existe pas dans ce projet. Vouliez-vous dire « commandes/v2 » ?".into())
        );
        assert_eq!(
            resolve(&tree(), Some("clients"), &[]),
            Err("La cible « clients » n'existe pas dans ce projet.".into())
        );
    }
}
