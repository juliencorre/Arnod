use super::*;
use crate::tree::{ROOT_FILE, create_folder, create_request, requests_in};
use tempfile::TempDir;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Boutique\n",
    )
    .unwrap();
    dir
}

/// Projet : dossiers « commandes » (deux requêtes) et « archives », requête « Santé » à la racine.
fn shop() -> (TempDir, Vec<String>) {
    let dir = project();
    let root = dir.path();
    create_folder(root, "", "commandes").unwrap();
    create_folder(root, "", "archives").unwrap();
    let a = create_request(root, "commandes", Some("Lister")).unwrap();
    let b = create_request(root, "commandes", Some("Créer")).unwrap();
    let c = create_request(root, "", Some("Santé")).unwrap();
    (dir, vec![a, b, c])
}

fn all(root: &Path) -> Vec<String> {
    let mut paths = requests_in(root, "").unwrap();
    paths.sort();
    paths
}

#[test]
fn m1_un_element_dans_un_dossier_choisi_n_est_pris_qu_une_fois() {
    assert_eq!(
        normalize(&[
            "commandes/lister.yml".into(),
            "commandes".into(),
            "sante.yml".into(),
            "commandes".into(),
        ]),
        ["commandes", "sante.yml"]
    );
}

#[test]
fn m2_deplacer_plusieurs_elements_puis_annuler() {
    let (dir, paths) = shop();
    let root = dir.path();
    let before = all(root);
    let selection = [paths[0].clone(), paths[2].clone()];
    let (done, undo) = apply(
        root,
        &selection,
        &BatchAction::Move {
            target: "archives".into(),
        },
    )
    .unwrap();
    assert_eq!(
        done.items,
        [
            (
                "commandes/lister.yml".to_owned(),
                Some("archives/lister.yml".to_owned())
            ),
            (
                "sante.yml".to_owned(),
                Some("archives/sante.yml".to_owned())
            ),
        ]
    );
    undo.apply().unwrap();
    assert_eq!(all(root), before);
}

#[test]
fn m3_dupliquer_une_requete_et_un_dossier_puis_annuler() {
    let (dir, paths) = shop();
    let root = dir.path();
    let before = all(root);
    let (done, undo) = apply(
        root,
        &["commandes".to_owned(), paths[2].clone()],
        &BatchAction::Duplicate,
    )
    .unwrap();
    assert_eq!(
        done.items
            .iter()
            .map(|(_, to)| to.clone().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["commandes (copie)", "sante-copie.yml"]
    );
    assert_eq!(requests_in(root, "commandes (copie)").unwrap().len(), 2);
    undo.apply().unwrap();
    assert_eq!(all(root), before);
    assert!(!root.join("commandes (copie)").exists());
}

#[test]
fn m4_supprimer_puis_annuler_remet_les_fichiers() {
    let (dir, paths) = shop();
    let root = dir.path();
    let before = all(root);
    let (_, undo) = apply(
        root,
        &["commandes".to_owned(), paths[2].clone()],
        &BatchAction::Delete { permanent: true },
    )
    .unwrap();
    assert_eq!(all(root), Vec::<String>::new());
    undo.apply().unwrap();
    assert_eq!(all(root), before);
    assert!(root.join("commandes").is_dir());
}

#[test]
fn m5_un_echec_remet_ce_qui_a_ete_fait() {
    let (dir, paths) = shop();
    let root = dir.path();
    let before = all(root);
    // « commandes » ne peut pas aller dans lui-même : « Santé », déplacée avant, revient.
    let error = apply(
        root,
        &[paths[2].clone(), "commandes".to_owned()],
        &BatchAction::Move {
            target: "commandes".into(),
        },
    )
    .unwrap_err();
    assert_eq!(error.kind, crate::error::CommandErrorKind::InvalidRequest);
    assert_eq!(all(root), before);
}
