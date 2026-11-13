use super::*;
use tempfile::TempDir;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Projet\n",
    )
    .unwrap();
    dir
}

fn names(nodes: &[TreeNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            TreeNode::Folder { name, .. } | TreeNode::Request { name, .. } => name.clone(),
        })
        .collect()
}

#[test]
fn projet_vide_sans_manifeste_ni_environnements() {
    let dir = project();
    fs::create_dir(dir.path().join(ENVIRONMENTS_DIR)).unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    assert!(read(dir.path()).unwrap().is_empty());
}

#[test]
fn dossiers_d_abord_puis_requetes_sans_casse() {
    let dir = project();
    create_folder(dir.path(), "", "zèbre").unwrap();
    create_folder(dir.path(), "", "Alpha").unwrap();
    create_request(dir.path(), "", Some("beta")).unwrap();
    create_request(dir.path(), "", Some("Aaa")).unwrap();
    assert_eq!(
        names(&read(dir.path()).unwrap()),
        ["Alpha", "zèbre", "Aaa", "beta"]
    );
}

#[test]
fn compteur_de_requetes_recursif() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    let b = create_folder(dir.path(), &a, "b").unwrap();
    create_request(dir.path(), &a, Some("r1")).unwrap();
    create_request(dir.path(), &b, Some("r2")).unwrap();
    let tree = read(dir.path()).unwrap();
    let TreeNode::Folder { request_count, .. } = &tree[0] else {
        panic!()
    };
    assert_eq!(*request_count, 2);
}

#[test]
fn noms_uniques_sans_casse_et_valides() {
    let dir = project();
    create_folder(dir.path(), "", "Docs").unwrap();
    assert_eq!(
        create_folder(dir.path(), "", "docs").unwrap_err().kind,
        CommandErrorKind::NameTaken
    );
    assert_eq!(
        create_folder(dir.path(), "", "a/b").unwrap_err().kind,
        CommandErrorKind::InvalidName
    );
    assert_eq!(
        create_folder(dir.path(), "", "environments")
            .unwrap_err()
            .kind,
        CommandErrorKind::NameTaken
    );
    create_request(dir.path(), "", Some("Créer")).unwrap();
    assert_eq!(
        create_request(dir.path(), "", Some("créer"))
            .unwrap_err()
            .kind,
        CommandErrorKind::NameTaken
    );
}

#[test]
fn nom_de_fichier_derive_avec_suffixe() {
    assert_eq!(slug("Créer une commande"), "creer-une-commande");
    assert_eq!(slug("  !!  "), "requete");
    let dir = project();
    let first = create_request(dir.path(), "", Some("Créer")).unwrap();
    let second = create_request(dir.path(), "", Some("Creer !")).unwrap();
    assert_eq!(first, "creer.yml");
    assert_eq!(second, "creer-2.yml");
}

#[test]
fn nouvelle_requete_par_defaut_numerotee() {
    let dir = project();
    create_request(dir.path(), "", None).unwrap();
    create_request(dir.path(), "", None).unwrap();
    let tree = read(dir.path()).unwrap();
    assert_eq!(names(&tree), ["Nouvelle requête", "Nouvelle requête 2"]);
}

#[test]
fn renommer_un_dossier_le_renomme_sur_disque() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    create_request(dir.path(), &a, Some("r")).unwrap();
    let renamed = rename(dir.path(), &a, "A2").unwrap();
    assert_eq!(renamed, "A2");
    assert!(dir.path().join("A2/r.yml").exists());
    assert!(!dir.path().join("a").exists());
}

#[test]
fn renommer_une_requete_suit_le_fichier_derive() {
    let dir = project();
    let path = create_request(dir.path(), "", Some("Ancien")).unwrap();
    let renamed = rename(dir.path(), &path, "Nouveau nom").unwrap();
    assert_eq!(renamed, "nouveau-nom.yml");
    let request = file::read(&dir.path().join(&renamed)).unwrap();
    assert_eq!(request.name, "Nouveau nom");
    // fichier choisi à la main : conservé
    fs::rename(
        dir.path().join(&renamed),
        dir.path().join("mon-fichier.yml"),
    )
    .unwrap();
    assert_eq!(
        rename(dir.path(), "mon-fichier.yml", "Autre").unwrap(),
        "mon-fichier.yml"
    );
}

#[test]
fn deplacer_garde_le_contenu_et_refuse_les_descendants() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    let b = create_folder(dir.path(), &a, "b").unwrap();
    let r = create_request(dir.path(), "", Some("r")).unwrap();
    let before = file::read(&dir.path().join(&r)).unwrap();
    let moved = move_to(dir.path(), &r, &b).unwrap();
    assert_eq!(moved, "a/b/r.yml");
    assert_eq!(file::read(&dir.path().join(&moved)).unwrap(), before);
    assert_eq!(
        move_to(dir.path(), &a, &b).unwrap_err().kind,
        CommandErrorKind::InvalidRequest
    );
    assert_eq!(
        move_to(dir.path(), &a, &a).unwrap_err().kind,
        CommandErrorKind::InvalidRequest
    );
}

#[test]
fn deplacer_refuse_une_collision() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    create_request(dir.path(), &a, Some("r")).unwrap();
    let r = create_request(dir.path(), "", Some("r")).unwrap();
    assert_eq!(
        move_to(dir.path(), &r, &a).unwrap_err().kind,
        CommandErrorKind::NameTaken
    );
}

#[test]
fn dupliquer_garde_le_contenu() {
    let dir = project();
    let r = create_request(dir.path(), "", Some("Lire")).unwrap();
    let copy = duplicate(dir.path(), &r).unwrap();
    let a = file::read(&dir.path().join(&r)).unwrap();
    let b = file::read(&dir.path().join(&copy)).unwrap();
    assert_eq!(b.name, "Lire (copie)");
    assert_ne!(r, copy);
    assert_eq!(a.assertions, b.assertions);
}

#[test]
fn fichier_illisible_liste_sans_bloquer() {
    let dir = project();
    create_request(dir.path(), "", Some("ok")).unwrap();
    fs::write(dir.path().join("casse.yml"), "name: [").unwrap();
    let tree = read(dir.path()).unwrap();
    assert_eq!(tree.len(), 2);
    assert!(
        tree.iter()
            .any(|n| matches!(n, TreeNode::Request { invalid: true, name, .. } if name == "casse"))
    );
}

#[test]
fn suppression_definitive_et_chemin_hors_projet() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    create_request(dir.path(), &a, Some("r")).unwrap();
    delete(dir.path(), &a, true).unwrap();
    assert!(!dir.path().join("a").exists());
    assert_eq!(
        resolve(dir.path(), "../x").unwrap_err().kind,
        CommandErrorKind::InvalidRequest
    );
    assert_eq!(
        delete(dir.path(), "", true).unwrap_err().kind,
        CommandErrorKind::InvalidRequest
    );
}

#[test]
fn requetes_d_un_dossier_dans_l_ordre_de_l_arborescence() {
    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    let b = create_folder(dir.path(), &a, "b").unwrap();
    create_request(dir.path(), &a, Some("z")).unwrap();
    create_request(dir.path(), &b, Some("y")).unwrap();
    create_request(dir.path(), "", Some("racine")).unwrap();
    assert_eq!(
        requests_in(dir.path(), &a).unwrap(),
        ["a/b/y.yml", "a/z.yml"]
    );
    assert_eq!(requests_in(dir.path(), "").unwrap().len(), 3);
}

#[test]
fn nom_de_fichier_jamais_reserve_par_windows() {
    assert_eq!(slug("Con!"), "_con");
    assert_eq!(slug("nul ?"), "_nul");
    assert_eq!(slug("Console"), "console");
    let dir = project();
    assert_eq!(
        create_request(dir.path(), "", Some("Con!")).unwrap(),
        "_con.yml"
    );
}

/// Un lien qui ramène à un dossier parent n'est pas suivi : sans cela la
/// lecture ne finit pas (deux liens doublent l'arbre à chaque niveau).
#[cfg(unix)]
#[test]
fn liens_en_boucle_ignores_liens_externes_suivis() {
    use std::os::unix::fs::symlink;

    let dir = project();
    let a = create_folder(dir.path(), "", "a").unwrap();
    create_request(dir.path(), &a, Some("r1")).unwrap();
    symlink(dir.path().join("a"), dir.path().join("a/boucle")).unwrap();
    symlink(dir.path(), dir.path().join("a/racine")).unwrap();
    // Dossier partagé hors du projet : un lien légitime reste suivi.
    let shared = tempfile::tempdir().unwrap();
    create_request(shared.path(), "", Some("commune")).unwrap();
    symlink(shared.path(), dir.path().join("a/partage")).unwrap();

    let tree = read(dir.path()).unwrap();
    let TreeNode::Folder {
        children,
        request_count,
        ..
    } = &tree[0]
    else {
        panic!()
    };
    assert_eq!(names(children), ["partage", "r1"]);
    assert_eq!(*request_count, 2);
    assert_eq!(
        requests_in(dir.path(), "").unwrap(),
        ["a/partage/commune.yml", "a/r1.yml"]
    );
}

// --- Référentiel de tests (docs/plan-requetes-dans-contrats.md, R1, R5, R6) ---

fn request(name: &str, path: &str, contract: Option<&str>) -> TreeNode {
    TreeNode::Request {
        name: name.to_owned(),
        path: path.to_owned(),
        method: Some(Method::Get),
        assertion_count: 0,
        invalid: false,
        contract: contract.map(str::to_owned),
    }
}

fn folder(path: &str, children: Vec<TreeNode>) -> TreeNode {
    let request_count = children.iter().map(TreeNode::count).sum();
    TreeNode::Folder {
        name: path.rsplit('/').next().unwrap_or(path).to_owned(),
        path: path.to_owned(),
        test: is_test(&children),
        children,
        request_count,
    }
}

#[test]
fn r5_referentiel_sans_les_requetes_liees() {
    let contracts: HashSet<String> = ["contracts/petstore.yaml".to_owned()].into();
    let nodes = vec![
        // Dossier du contrat : seulement des requêtes liées, il disparaît.
        folder(
            "Petstore",
            vec![folder(
                "Petstore/animaux",
                vec![request(
                    "Lister",
                    "Petstore/animaux/lister.yml",
                    Some("contracts/petstore.yaml"),
                )],
            )],
        ),
        // Dossier mixte : il garde ses requêtes libres, avec leur nombre.
        folder(
            "Mixte",
            vec![
                request("Libre", "Mixte/libre.yml", None),
                request("Liée", "Mixte/liee.yml", Some("contracts/petstore.yaml")),
            ],
        ),
        // Dossier vide créé à la main : il reste.
        folder("Vide", vec![]),
        // R6 : contrat absent, la requête reste dans le référentiel.
        request(
            "Orpheline",
            "orpheline.yml",
            Some("contracts/supprime.yaml"),
        ),
        request("google", "google.yml", None),
    ];
    let tests = tests_view(&nodes, &contracts);
    assert_eq!(names(&tests), ["Mixte", "Vide", "Orpheline", "google"]);
    assert!(
        matches!(&tests[0], TreeNode::Folder { request_count: 1, children, .. } if names(children) == ["Libre"])
    );
    assert!(matches!(
        &tests[1],
        TreeNode::Folder {
            request_count: 0,
            ..
        }
    ));
    // Sans contrat présent, tout reste.
    assert_eq!(tests_view(&nodes, &HashSet::new()), nodes);
}

#[test]
fn le_lien_au_contrat_est_lu_dans_l_arborescence() {
    let dir = project();
    let path = create_request(dir.path(), "", Some("liée")).unwrap();
    let absolute = dir.path().join(&path);
    let mut request = file::read(&absolute).unwrap();
    request.contract = Some(crate::request::model::ContractLink {
        file: "contracts/petstore.yaml".to_owned(),
        operation: "GET /pets".to_owned(),
    });
    file::write(&absolute, &request).unwrap();
    assert!(matches!(
        &read(dir.path()).unwrap()[0],
        TreeNode::Request { contract: Some(file), .. } if file == "contracts/petstore.yaml"
    ));
}

// Tests et étapes (docs/plan-mvp-e-tests-etapes.md, R1, R11).

#[test]
fn lot_e_r1_un_dossier_de_requetes_sans_sous_dossier_est_un_test() {
    let dir = project();
    let rangement = create_folder(dir.path(), "", "Clients").unwrap();
    let test = create_folder(dir.path(), &rangement, "Créer puis lire").unwrap();
    create_folder(dir.path(), "", "Vide").unwrap();
    create_request(dir.path(), &test, Some("Lire")).unwrap();
    create_request(dir.path(), &rangement, Some("Libre")).unwrap();
    let tree = read(dir.path()).unwrap();
    let flags: Vec<(String, bool)> = tree
        .iter()
        .filter_map(|n| match n {
            TreeNode::Folder {
                name,
                test,
                children,
                ..
            } => {
                let inner = children.iter().find_map(|c| match c {
                    TreeNode::Folder { test, .. } => Some(*test),
                    TreeNode::Request { .. } => None,
                });
                Some((format!("{name}:{inner:?}"), *test))
            }
            TreeNode::Request { .. } => None,
        })
        .collect();
    // Un dossier qui range un dossier n'est pas un test, même avec une requête ; vide non plus.
    assert_eq!(
        flags,
        [
            ("Clients:Some(true)".to_owned(), false),
            ("Vide:None".to_owned(), false)
        ]
    );
}

#[test]
fn lot_e_r1_les_etapes_suivent_seq_puis_le_nom() {
    let dir = project();
    let test = create_folder(dir.path(), "", "Parcours").unwrap();
    for (name, seq) in [
        ("Alpha", Some(3.0)),
        ("Zulu", Some(1.0)),
        ("Bravo", None),
        ("Mike", Some(2.0)),
    ] {
        let path = create_request(dir.path(), &test, Some(name)).unwrap();
        let file_path = resolve(dir.path(), &path).unwrap();
        let mut request = file::read(&file_path).unwrap();
        request.seq = seq;
        file::write(&file_path, &request).unwrap();
    }
    let view = test_view(dir.path(), &test).unwrap();
    let order: Vec<&str> = view.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(order, ["Zulu", "Mike", "Alpha", "Bravo"]);
    assert!(view.test);
}

#[test]
fn lot_e_r11_nouveau_test_puis_etapes() {
    let dir = project();
    let created = create_test(dir.path(), "").unwrap();
    assert_eq!(created.folder, "Nouveau test");
    assert_eq!(created.step, "Nouveau test/etape-1.yml");
    let again = create_test(dir.path(), "").unwrap();
    assert_eq!(again.folder, "Nouveau test 2");

    // Une étape sans rang, ajoutée à la main : « + Étape » pose les rangs dans l'ordre actuel.
    create_request(dir.path(), &created.folder, Some("Autre")).unwrap();
    let second = add_step(dir.path(), &created.folder).unwrap();
    let view = test_view(dir.path(), &created.folder).unwrap();
    let order: Vec<&str> = view.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(order, ["Étape 1", "Autre", "Étape 3"]);
    assert_eq!(view.steps[2].path, second);
    let seq = |path: &str| file::read(&resolve(dir.path(), path).unwrap()).unwrap().seq;
    assert_eq!(seq(&second), Some(3.0));
    assert_eq!(seq(&view.steps[1].path), Some(2.0));

    // Un dossier qui range des dossiers refuse une étape.
    let error = add_step(dir.path(), "").unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
    let parent = create_folder(dir.path(), "", "Rangement").unwrap();
    create_folder(dir.path(), &parent, "Sous").unwrap();
    let error = add_step(dir.path(), &parent).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
}

#[test]
fn lot_e_variables_lues_par_les_autres_requetes() {
    let dir = project();
    let first = create_request(dir.path(), "", Some("Créer")).unwrap();
    let second = create_request(dir.path(), "", Some("Lire")).unwrap();
    let path = resolve(dir.path(), &second).unwrap();
    let mut request = file::read(&path).unwrap();
    request.url = "{{baseUrl}}/clients/{{ client.id }}".to_owned();
    file::write(&path, &request).unwrap();
    let users = variable_users(dir.path(), &first, &["client.id".to_owned()]).unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].path, second);
    assert_eq!(users[0].variables, ["client.id"]);
    assert!(variable_users(dir.path(), &first, &[]).unwrap().is_empty());
}
