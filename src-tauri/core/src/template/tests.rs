//! Règles des templates (`docs/plan-v4.md`, R16 à R19).

use std::collections::BTreeMap;
use std::fs;

use super::*;
use crate::opencollection::ROOT_FILE;
use crate::request::model::{Body, BodyKind, ContractLink, Pair, Request};

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Boutique\n",
    )
    .unwrap();
    dir
}

/// `commandes/paiements` : propriétés devise (ici) et clientId (héritée), deux requêtes.
fn payments(root: &Path) {
    fs::create_dir_all(root.join("commandes/paiements")).unwrap();
    tree::set_folder_variables(
        root,
        "commandes",
        &[("clientId".into(), "cus_31".into(), String::new())],
    )
    .unwrap();
    tree::set_folder_variables(
        root,
        "commandes/paiements",
        &[
            ("devise".into(), "EUR".into(), String::new()),
            ("montant".into(), "129.90".into(), String::new()),
        ],
    )
    .unwrap();
    let mut pay = Request::new("Créer un paiement");
    pay.url = "{{baseUrl}}/payments?client={{clientId}}".into();
    pay.body = Body::raw(
        BodyKind::Json,
        r#"{"amount": {{montant}}, "currency": "{{devise}}"}"#,
    );
    file::write(&root.join("commandes/paiements/creer.yml"), &pay).unwrap();
    let mut read = Request::new("Lire un paiement");
    read.url = "{{baseUrl}}/payments/{{paymentId}}".into();
    file::write(&root.join("commandes/paiements/lire.yml"), &read).unwrap();
}

#[test]
fn r18_parametres_detectes_proprietes_cochees_environnement_non() {
    let dir = project();
    payments(dir.path());
    let env: BTreeMap<String, String> = [(
        "baseUrl".to_owned(),
        "https://recette.boutique.dev".to_owned(),
    )]
    .into();
    let found = candidates(dir.path(), "commandes/paiements", &env, "Recette").unwrap();
    let names: Vec<(&str, bool)> = found.iter().map(|c| (c.name.as_str(), c.checked)).collect();
    assert_eq!(
        names,
        [
            ("devise", true),
            ("montant", true),
            ("clientId", true),
            ("baseUrl", false)
        ]
    );
    assert_eq!(found[2].origin, "hérité de commandes");
}

#[test]
fn r16_r17_r18_creer_remplacer_utiliser() {
    let dir = project();
    payments(dir.path());
    let params = vec![
        TemplateParam {
            name: "montant".into(),
            default: "129.90".into(),
            description: "Montant TTC".into(),
        },
        TemplateParam {
            name: "clientId".into(),
            default: String::new(),
            description: String::new(),
        },
    ];
    let view = create(
        dir.path(),
        "commandes/paiements",
        "Payer une commande",
        "paiements",
        &params,
        true,
    )
    .unwrap();
    assert_eq!(view.summary.path, "templates/paiements/Payer une commande");
    assert_eq!(view.summary.kind, TemplateKind::Test);
    assert_eq!(view.parameters.len(), 2);
    assert!(view.parameters[1].required());
    // Le dossier d'origine devient une utilisation (R18), sans requête.
    assert_eq!(view.uses.len(), 1);
    assert_eq!(view.uses[0].path, "commandes/paiements");
    assert_eq!(list(dir.path()).len(), 1);
    assert_eq!(categories(dir.path()), ["paiements"]);
    // templates/ n'est pas dans l'arborescence (R20) ; l'utilisation porte son lien.
    let nodes = tree::read(dir.path()).unwrap();
    let Some(TreeNode::Folder { template, .. }) = find(&nodes, "commandes/paiements") else {
        panic!("utilisation absente");
    };
    assert_eq!(template.as_deref(), Some(view.summary.path.as_str()));
    assert_eq!(nodes.len(), 1);
    // Valeurs : héritée de commandes pour clientId, par défaut pour montant (R17).
    let instance = instance(dir.path(), "commandes/paiements").unwrap();
    let origins: Vec<&ValueOrigin> = instance.values.iter().map(|v| &v.origin).collect();
    assert_eq!(
        origins,
        [
            &ValueOrigin::Default,
            &ValueOrigin::Property {
                folder: "commandes".into()
            }
        ]
    );
    let saved = set_values(
        dir.path(),
        "commandes/paiements",
        &[("montant".to_owned(), "59.80".to_owned())].into(),
    )
    .unwrap();
    assert_eq!(saved.values[0].origin, ValueOrigin::Here);
    assert_eq!(saved.values[0].effective, "59.80");
    // Seconde utilisation, dans un autre dossier.
    fs::create_dir_all(dir.path().join("clients")).unwrap();
    let second = use_in(dir.path(), &view.summary.path, "clients", "Premier achat").unwrap();
    assert_eq!(second, "clients/Premier achat");
    assert_eq!(uses(dir.path(), &view.summary.path).len(), 2);
}

#[test]
fn r17_un_parametre_obligatoire_sans_valeur_est_nomme() {
    let dir = project();
    payments(dir.path());
    let params = [TemplateParam {
        name: "clientId".into(),
        default: String::new(),
        description: String::new(),
    }];
    let view = create(
        dir.path(),
        "commandes/paiements",
        "Payer",
        "",
        &params,
        false,
    )
    .unwrap();
    let error = effective_values(
        dir.path(),
        &view.summary.path,
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert!(error.contains("« clientId »"), "{error}");
    let ok = effective_values(
        dir.path(),
        &view.summary.path,
        &BTreeMap::new(),
        &[("clientId".to_owned(), "cus_77".to_owned())].into(),
    )
    .unwrap();
    assert_eq!(ok.get("clientId").map(String::as_str), Some("cus_77"));
}

#[test]
fn r19_transferer_un_endpoint_vers_les_templates() {
    let dir = project();
    let mut request = Request::new("Commande simple");
    request.method = crate::request::model::Method::Post;
    request.url = "{{baseUrl}}/orders".into();
    request.params = vec![Pair::new("lang", "fr")];
    request.body = Body::raw(
        BodyKind::Json,
        r#"{"customerId": "cus_31", "items": [{"sku": "TSH-BLU-M", "qty": 2}], "coupon": "BIENVENUE"}"#,
    );
    request.contract = Some(ContractLink {
        file: "contracts/commandes.yaml".into(),
        operation: "POST /orders".into(),
    });
    fs::create_dir_all(dir.path().join("contrat")).unwrap();
    file::write(&dir.path().join("contrat/simple.yml"), &request).unwrap();
    let found = literals(&request);
    let names: Vec<&str> = found.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["customerId", "sku", "qty", "coupon", "lang"]);
    let view = from_request(
        dir.path(),
        "contrat/simple.yml",
        "Commander un article",
        "commandes",
        &["customerId".into(), "sku".into(), "qty".into()],
    )
    .unwrap();
    assert_eq!(view.parameters.len(), 3);
    assert_eq!(view.parameters[2].default, "2");
    assert!(!dir.path().join("contrat/simple.yml").exists());
    let copied = file::read(
        &dir.path()
            .join("templates/commandes/Commander un article/simple.yml"),
    )
    .unwrap();
    assert_eq!(copied.contract, None);
    let body = copied.body.content();
    assert!(body.contains(r#""customerId": "{{customerId}}""#), "{body}");
    assert!(body.contains(r#""qty": {{qty}}"#), "{body}");
    assert!(body.contains(r#""coupon": "BIENVENUE""#), "{body}");
}
