//! Règles de la recherche (`docs/plan-v2.md`, R1 à R3), une par test.

use super::*;
use crate::contract::file::{LinkedView, OperationNode, ResourceView};
use crate::request::model::Method;

fn request(name: &str, path: &str, method: Method, url: &str, tags: &[&str]) -> TreeNode {
    TreeNode::Request {
        name: name.into(),
        path: path.into(),
        method: Some(method),
        assertion_count: 0,
        invalid: false,
        contract: None,
        url: url.into(),
        tags: tags.iter().map(|&t| t.to_owned()).collect(),
    }
}

fn folder(name: &str, path: &str, test: bool, children: Vec<TreeNode>) -> TreeNode {
    TreeNode::Folder {
        name: name.into(),
        path: path.into(),
        request_count: children.len(),
        children,
        test,
        tags: Vec::new(),
    }
}

fn operation(method: &str, path: &str, summary: &str, endpoint: Option<&str>) -> OperationNode {
    OperationNode {
        key: format!("{method} {path}"),
        method: method.into(),
        path: path.into(),
        summary: Some(summary.into()),
        supported: true,
        requests: endpoint
            .map(|p| LinkedView {
                path: p.into(),
                name: summary.into(),
                method: Method::Get,
                examples: Vec::new(),
            })
            .into_iter()
            .collect(),
    }
}

fn shop() -> ProjectSource {
    ProjectSource {
        id: Uuid::nil(),
        name: "boutique-tests".into(),
        nodes: vec![folder(
            "paiements",
            "paiements",
            false,
            vec![
                folder(
                    "plusieurs-fois",
                    "paiements/plusieurs-fois",
                    true,
                    vec![request(
                        "Payer en 4 fois",
                        "paiements/plusieurs-fois/payer.yml",
                        Method::Post,
                        "{{baseUrl}}/payments",
                        &["smoke"],
                    )],
                ),
                request(
                    "Payer sur la page de paiement",
                    "paiements/page.yml",
                    Method::Get,
                    "{{baseUrl}}/checkout",
                    &["ihm"],
                ),
                request(
                    "Créer une commande",
                    "paiements/commande.yml",
                    Method::Post,
                    "{{baseUrl}}/orders",
                    &["smoke", "nominal"],
                ),
                request(
                    "Lire la commande",
                    "paiements/lire.yml",
                    Method::Get,
                    "https://api.test/orders/{{orderId}}?x=1",
                    &[],
                ),
            ],
        )],
        contracts: vec![ContractSummary {
            deduced: false,
            path: "contracts/commandes.yaml".into(),
            name: "API Commandes".into(),
            version: "2.4.0".into(),
            operation_count: 3,
            error: None,
            resources: vec![ResourceView {
                path: "/orders".into(),
                operations: vec![
                    operation("post", "/orders", "Créer une commande", Some("c/creer.yml")),
                    operation("get", "/orders/{id}", "Lire une commande", None),
                    operation("patch", "/orders/{id}/status", "Changer le statut", None),
                ],
            }],
            unplaced: Vec::new(),
        }],
        environments: vec!["Recette fonctionnelle".into(), "Local".into()],
    }
}

fn labels(results: &SearchResults) -> Vec<(String, Vec<String>)> {
    results
        .groups
        .iter()
        .map(|g| {
            (
                g.title.clone(),
                g.hits.iter().map(|h| h.label.clone()).collect(),
            )
        })
        .collect()
}

#[test]
fn r1_sans_casse_ni_accents_et_surligne() {
    let results = search(&[shop()], "PAIE");
    assert!(!results.address);
    assert_eq!(
        labels(&results),
        [
            (
                "Tests".to_owned(),
                vec!["Payer sur la page de paiement".to_owned()]
            ),
            ("Dossiers".to_owned(), vec!["paiements".to_owned()]),
        ]
    );
    let hit = &results.groups[0].hits[0];
    // « paie » dans « paiement » : caractères 21 à 25.
    assert_eq!(hit.highlight, [(21, 25)]);
    assert_eq!(hit.detail, "boutique-tests · paiements");
    // Accents ignorés dans les deux sens.
    let results = search(&[shop()], "creer");
    assert_eq!(results.groups[0].hits[0].label, "Créer une commande");
    let results = search(&[shop()], "fonctionnélle");
    assert_eq!(results.groups[0].title, "Environnements");
}

#[test]
fn r1_tous_les_mots_et_l_adresse_comptent() {
    let results = search(&[shop()], "commande lire");
    assert_eq!(
        labels(&results),
        [
            ("Tests".to_owned(), vec!["Lire la commande".to_owned()]),
            ("Endpoints".to_owned(), vec!["Lire une commande".to_owned()]),
        ]
    );
    // Un mot trouvé seulement dans l'adresse : en dernier.
    let results = search(&[shop()], "checkout");
    assert_eq!(
        results.groups[0].hits[0].label,
        "Payer sur la page de paiement"
    );
    assert!(results.groups[0].hits[0].highlight.is_empty());
}

#[test]
fn r1_etiquette_et_methode_filtrent() {
    let results = search(&[shop()], "#smo");
    assert_eq!(
        labels(&results),
        [(
            "Tests".to_owned(),
            vec![
                "Créer une commande".to_owned(),
                "Payer en 4 fois".to_owned()
            ]
        )]
    );
    let results = search(&[shop()], "#smoke payer");
    assert_eq!(labels(&results)[0].1, ["Payer en 4 fois"]);
    let results = search(&[shop()], "POST commande");
    assert_eq!(
        labels(&results),
        [
            ("Tests".to_owned(), vec!["Créer une commande".to_owned()]),
            (
                "Endpoints".to_owned(),
                vec!["Créer une commande".to_owned()]
            ),
        ]
    );
}

#[test]
fn r2_une_adresse_trouve_l_operation_ses_voisines_et_ses_appels() {
    let results = search(&[shop()], "POST /orders");
    assert!(results.address);
    assert_eq!(
        labels(&results),
        [
            (
                "Correspondance exacte".to_owned(),
                vec!["Créer une commande".to_owned()]
            ),
            (
                "Endpoints de /orders".to_owned(),
                vec![
                    "Changer le statut".to_owned(),
                    "Lire une commande".to_owned()
                ]
            ),
            (
                "Tests qui l'appellent".to_owned(),
                vec!["Créer une commande".to_owned()]
            ),
        ]
    );
    // L'opération ouvre son endpoint ; sans endpoint, le contrat.
    let exact = &results.groups[0].hits[0];
    assert_eq!(exact.path.as_deref(), Some("c/creer.yml"));
    assert_eq!(exact.method.as_deref(), Some("POST"));
    assert_eq!(exact.detail, "/orders · API Commandes v2.4.0");
}

#[test]
fn r2_une_url_complete_retrouve_son_operation() {
    let results = search(&[shop()], "https://api.test/v2/orders/42?expand=1");
    assert_eq!(
        labels(&results)[0],
        (
            "Correspondance exacte".to_owned(),
            vec!["Lire une commande".to_owned()]
        )
    );
    // La requête qui appelle /orders/{{orderId}} est trouvée.
    assert!(
        labels(&results)
            .iter()
            .any(|(t, l)| t == "Tests qui l'appellent" && l == &["Lire la commande"])
    );
    // Un modèle saisi tel quel.
    let results = search(&[shop()], "/orders/{id}/status");
    assert_eq!(labels(&results)[0].1, ["Changer le statut"]);
}

#[test]
fn r3_au_plus_huit_par_groupe_dans_un_ordre_stable() {
    let mut project = shop();
    project.nodes = (0..12)
        .map(|i| {
            request(
                &format!("Commande {i:02}"),
                &format!("c{i}.yml"),
                Method::Get,
                "/x",
                &[],
            )
        })
        .collect();
    project.contracts.clear();
    let results = search(&[project], "commande");
    let group = &results.groups[0];
    assert_eq!(group.total, 12);
    assert_eq!(group.hits.len(), PER_GROUP);
    assert_eq!(group.hits[0].label, "Commande 00");
    // Libellé exact d'abord, puis début, puis début d'un mot, puis ailleurs.
    let mut project = shop();
    project.nodes = vec![
        request("Lire la commande", "a.yml", Method::Get, "", &[]),
        request("Commandes", "b.yml", Method::Get, "", &[]),
        request("Commande", "c.yml", Method::Get, "", &[]),
        request("Souscommande", "d.yml", Method::Get, "", &[]),
    ];
    project.contracts.clear();
    let results = search(&[project], "commande");
    assert_eq!(
        labels(&results)[0].1,
        ["Commande", "Commandes", "Lire la commande", "Souscommande"]
    );
}

#[test]
fn une_saisie_vide_ne_rend_rien_et_un_projet_se_trouve() {
    assert!(search(&[shop()], "   ").groups.is_empty());
    let results = search(&[shop()], "boutique");
    assert_eq!(
        labels(&results),
        [("Projets".to_owned(), vec!["boutique-tests".to_owned()])]
    );
    assert_eq!(results.groups[0].hits[0].kind, SearchKind::Project);
}

#[test]
fn chemins_de_requete() {
    assert_eq!(
        request_path("{{baseUrl}}/orders/{{id}}?x=1"),
        "/orders/{{id}}"
    );
    assert_eq!(request_path("https://api.test/v1/x"), "/v1/x");
    assert_eq!(request_path("orders"), "/orders");
    assert!(same_path("/orders/{{id}}", "/orders/42"));
    assert!(same_path("/orders/:id", "/orders/{id}"));
    assert!(!same_path("/orders", "/orders/42"));
    assert!(path_served("/orders/{id}", "/api/v2/orders/42"));
    // Un segment variable saisi ne vaut pas un mot du modèle.
    assert!(!path_served("/orders/{id}", "/orders/{id}/status"));
    assert!(path_served("/orders/{id}", "/orders/{orderId}"));
}
