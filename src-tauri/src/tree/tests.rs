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
