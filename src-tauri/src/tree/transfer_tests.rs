use super::*;
use crate::request::model::{Assertion, Operator, Origin, Pair, Scalar};
use crate::tree::{ROOT_FILE, add_step, create_folder, create_request, create_test};
use tempfile::TempDir;

const SHOP: &str = r#"openapi: 3.0.3
info:
  title: API Commandes
  version: 2.4.0
paths:
  /orders:
    get:
      tags: [commandes]
      responses:
        "200": { description: ok }
    post:
      tags: [commandes]
      responses:
        "201": { description: créée }
  /orders/{id}:
    get:
      responses:
        "200": { description: ok }
"#;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Boutique\n",
    )
    .unwrap();
    dir
}

/// Requête `name` dans `folder`, avec en-tête, vérification et variables.
fn order(root: &Path, folder: &str, name: &str) -> String {
    let path = create_request(root, folder, Some(name)).unwrap();
    let file_path = resolve(root, &path).unwrap();
    let mut request = file::read(&file_path).unwrap();
    request.method = crate::request::model::Method::Post;
    request.url = "{{baseUrl}}/orders/{{orderId}}".to_owned();
    request
        .headers
        .push(Pair::new("Content-Type", "application/json"));
    request.assertions.push(Assertion {
        id: "a1".to_owned(),
        target: "status".to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Int(201)),
        origin: Origin::Manual,
        enabled: true,
    });
    request.tags.push("smoke".to_owned());
    file::write(&file_path, &request).unwrap();
    path
}

fn get(root: &Path, path: &str) -> Request {
    file::read(&resolve(root, path).unwrap()).unwrap()
}

fn link(root: &Path, path: &str, contract: &str, operation: &str) {
    let file_path = resolve(root, path).unwrap();
    let mut request = file::read(&file_path).unwrap();
    request.contract = Some(ContractLink {
        file: contract.to_owned(),
        operation: operation.to_owned(),
    });
    file::write(&file_path, &request).unwrap();
}

fn seqs(root: &Path, test: &str) -> Vec<(String, Option<f64>)> {
    test_view(root, test)
        .unwrap()
        .steps
        .iter()
        .map(|s| (s.name.clone(), get(root, &s.path).seq))
        .collect()
}

#[test]
fn j1_copie_dans_un_dossier_tout_le_contenu_suit_l_original_reste() {
    let dir = project();
    let root = dir.path();
    create_folder(root, "", "commandes").unwrap();
    let path = order(root, "", "Créer une commande");
    let (done, _) = transfer(
        root,
        &path,
        root,
        &Target::Folder {
            folder: "commandes".to_owned(),
        },
        Mode::Copy,
    )
    .unwrap();
    let copy = get(root, &done.path);
    assert_eq!(
        (
            done.path.as_str(),
            copy.name.as_str(),
            copy.headers.len(),
            copy.assertions.len(),
            copy.tags.clone(),
            resolve(root, &path).unwrap().exists()
        ),
        (
            "commandes/creer-une-commande.yml",
            "Créer une commande",
            1,
            2,
            vec!["smoke".to_owned()],
            true
        )
    );
}

#[test]
fn j2_copie_a_cote_de_l_original_prend_copie_dans_son_nom() {
    let dir = project();
    let root = dir.path();
    let path = order(root, "", "Créer");
    let target = Target::Folder {
        folder: String::new(),
    };
    let (first, _) = transfer(root, &path, root, &target, Mode::Copy).unwrap();
    let (second, _) = transfer(root, &path, root, &target, Mode::Copy).unwrap();
    assert_eq!(
        [first.name, second.name],
        ["Créer (copie)", "Créer (copie) 2"]
    );
}

#[test]
fn j3_deplacement_dans_un_test_a_une_place_renumerote_et_s_annule() {
    let dir = project();
    let root = dir.path();
    let created = create_test(root, "").unwrap();
    add_step(root, &created.folder).unwrap();
    let path = order(root, "", "Sans coupon");
    let before = seqs(root, &created.folder);
    let (done, undo) = transfer(
        root,
        &path,
        root,
        &Target::Step {
            test: created.folder.clone(),
            position: 1,
        },
        Mode::Move,
    )
    .unwrap();
    assert_eq!(
        seqs(root, &created.folder),
        [
            ("Étape 1".to_owned(), Some(1.0)),
            ("Sans coupon".to_owned(), Some(2.0)),
            ("Étape 2".to_owned(), Some(3.0)),
        ]
    );
    assert!(!resolve(root, &path).unwrap().exists());
    undo.apply().unwrap();
    assert_eq!(
        (
            seqs(root, &created.folder),
            resolve(root, &path).unwrap().exists(),
            resolve(root, &done.path).unwrap().exists()
        ),
        (before, true, false)
    );
}

#[test]
fn j4_deplacement_dans_son_propre_test_change_seulement_l_ordre() {
    let dir = project();
    let root = dir.path();
    let created = create_test(root, "").unwrap();
    let second = add_step(root, &created.folder).unwrap();
    let (done, _) = transfer(
        root,
        &second,
        root,
        &Target::Step {
            test: created.folder.clone(),
            position: 0,
        },
        Mode::Move,
    )
    .unwrap();
    assert_eq!(done.path, second);
    assert_eq!(
        seqs(root, &created.folder),
        [
            ("Étape 2".to_owned(), Some(1.0)),
            ("Étape 1".to_owned(), Some(2.0)),
        ]
    );
}

#[test]
fn j5_vers_une_operation_le_lien_est_pose_et_le_fichier_range_sous_le_contrat() {
    let dir = project();
    let root = dir.path();
    let contract = contract_file::add(root, SHOP).unwrap();
    let path = order(root, "", "Sans coupon");
    let (done, _) = transfer(
        root,
        &path,
        root,
        &Target::Operation {
            contract: contract.clone(),
            operation: "POST /orders".to_owned(),
        },
        Mode::Move,
    )
    .unwrap();
    let moved = get(root, &done.path);
    assert_eq!(
        (done.path.as_str(), moved.contract.map(|l| l.operation)),
        (
            "API Commandes/commandes/sans-coupon.yml",
            Some("POST /orders".to_owned())
        )
    );
}

#[test]
fn j6_hors_d_une_operation_le_lien_est_retire() {
    let dir = project();
    let root = dir.path();
    let contract = contract_file::add(root, SHOP).unwrap();
    let path = order(root, "", "Commande simple");
    link(root, &path, &contract, "POST /orders");
    let created = create_test(root, "").unwrap();
    let (done, _) = transfer(
        root,
        &path,
        root,
        &Target::Step {
            test: created.folder,
            position: 9,
        },
        Mode::Copy,
    )
    .unwrap();
    let copy = get(root, &done.path);
    assert_eq!((copy.contract, copy.seq), (None, Some(2.0)));
    assert!(get(root, &path).contract.is_some());
}

#[test]
fn j7_vers_un_autre_projet() {
    let source = project();
    let other = project();
    let path = order(source.path(), "", "Créer");
    let (done, _) = transfer(
        source.path(),
        &path,
        other.path(),
        &Target::Folder {
            folder: String::new(),
        },
        Mode::Move,
    )
    .unwrap();
    assert_eq!(done.path, "creer.yml");
    assert_eq!(get(other.path(), &done.path).headers.len(), 1);
    assert!(!resolve(source.path(), &path).unwrap().exists());
}

#[test]
fn j8_destinations_operations_de_meme_methode_dossiers_et_tests() {
    let dir = project();
    let root = dir.path();
    let contract = contract_file::add(root, SHOP).unwrap();
    create_folder(root, "", "commandes").unwrap();
    let created = create_test(root, "commandes").unwrap();
    let path = order(root, "commandes", "Créer");
    let found = targets(root, &path, root, true).unwrap();
    assert_eq!(
        found
            .operations
            .iter()
            .map(|o| (
                o.contract.as_str(),
                o.contract_name.as_str(),
                o.operation.as_str()
            ))
            .collect::<Vec<_>>(),
        [(contract.as_str(), "API Commandes v2.4.0", "POST /orders")]
    );
    assert_eq!(
        found
            .folders
            .iter()
            .map(|f| (f.path.as_str(), f.current))
            .collect::<Vec<_>>(),
        [("", false), ("commandes", true)]
    );
    assert_eq!(
        found
            .tests
            .iter()
            .map(|t| (t.path.as_str(), t.steps.clone()))
            .collect::<Vec<_>>(),
        [(created.folder.as_str(), vec!["Étape 1".to_owned()])]
    );
}

#[test]
fn j9_ce_qui_change_etape_lien_variables_et_copie() {
    let dir = project();
    let root = dir.path();
    let contract = contract_file::add(root, SHOP).unwrap();
    let path = order(root, "", "Commande simple");
    link(root, &path, &contract, "POST /orders");
    let created = create_test(root, "").unwrap();
    let changes = preview(
        root,
        &path,
        root,
        &Target::Step {
            test: created.folder,
            position: 0,
        },
        Mode::Copy,
        None,
    )
    .unwrap();
    let titles: Vec<(&str, bool)> = changes
        .iter()
        .map(|c| (c.title.as_str(), c.warning))
        .collect();
    assert_eq!(
        titles,
        [
            ("Devient l'étape 1 de « Nouveau test »", false),
            ("N'est plus liée à POST /orders", true),
            ("Authentification", false),
            ("Variables", true),
            ("Suivent la requête", false),
            ("Copie", false),
        ]
    );
    assert_eq!(changes[0].detail, "L'actuelle étape 1 devient la 2.");
    assert_eq!(
        changes[3].detail,
        "{{baseUrl}} et {{orderId}} inconnues dans la destination."
    );
    assert_eq!(
        changes[4].detail,
        "En-têtes, 2 vérifications et étiquettes. Les secrets restent au coffre : ils ne sont jamais copiés."
    );
    assert_eq!(
        changes[5].detail,
        "« Commande simple » reste aussi sous POST /orders."
    );
}

#[test]
fn j10_reste_liee_a_son_operation() {
    let dir = project();
    let root = dir.path();
    let contract = contract_file::add(root, SHOP).unwrap();
    let path = order(root, "", "Commande simple");
    link(root, &path, &contract, "POST /orders");
    let changes = preview(
        root,
        &path,
        root,
        &Target::Operation {
            contract,
            operation: "POST /orders".to_owned(),
        },
        Mode::Copy,
        None,
    )
    .unwrap();
    assert_eq!(changes[0].title, "Reste liée à POST /orders");
    assert_eq!(
        changes[0].detail,
        "API Commandes v2.4.0 : conformité vérifiée à chaque envoi."
    );
}
