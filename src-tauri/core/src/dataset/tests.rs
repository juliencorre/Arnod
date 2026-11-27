//! Règles des jeux de données (`docs/plan-v4.md`, R11, R12).

use std::fs;

use super::*;
use crate::opencollection::ROOT_FILE;
use crate::request::model::Request;

#[test]
fn r11_csv_avec_guillemets_et_separateurs() {
    let table = parse("statut;limite;note\npending;10;\"a;b\"\n\npaid;\"1\"\"0\";\n");
    assert_eq!(table.columns, ["statut", "limite", "note"]);
    assert_eq!(
        table.rows,
        [
            vec!["pending".to_owned(), "10".to_owned(), "a;b".to_owned()],
            vec!["paid".to_owned(), "1\"0".to_owned(), String::new()],
        ]
    );
    let again = parse(&render(&table));
    assert_eq!(again, table);
}

#[test]
fn r11_coller_depuis_un_tableur() {
    assert_eq!(
        parse_rows("pending\t10\t200\r\npaid\t10\t200\r\n"),
        [["pending", "10", "200"], ["paid", "10", "200"]].map(|r| r.map(str::to_owned).to_vec())
    );
}

#[test]
fn r12_un_cas_par_ligne_non_vide() {
    let table = DataTable {
        columns: vec!["statut".into(), "".into(), "limite".into()],
        rows: vec![
            vec!["pending".into(), "x".into(), "10".into()],
            vec!["".into(), "".into(), "".into()],
        ],
    };
    let cases = cases(&table);
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].get("statut").map(String::as_str), Some("pending"));
    assert_eq!(cases[0].len(), 2, "une colonne sans nom est ignorée");
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(ROOT_FILE),
        "opencollection: 1.0.0\ninfo:\n  name: Projet\n",
    )
    .unwrap();
    file::write(
        &dir.path().join("lister.yml"),
        &Request::new("Lister les commandes"),
    )
    .unwrap();
    dir
}

#[test]
fn r11_le_tableau_d_arnod_va_dans_donnees_et_se_lie_par_etiquette() {
    let dir = project();
    let table = DataTable {
        columns: vec!["statut".into(), "limite".into()],
        rows: vec![vec!["pending".into(), "10".into()]],
    };
    let set = save(dir.path(), "lister.yml", &table).unwrap();
    assert!(set.link.editable);
    assert!(set.link.file.starts_with("données/"));
    assert_eq!(set.table, table);
    let text = fs::read_to_string(dir.path().join("lister.yml")).unwrap();
    assert!(text.contains("data:données/"), "{text}");
    // L'étiquette n'est pas montrée parmi celles de l'utilisateur (R20).
    let request = file::read(&dir.path().join("lister.yml")).unwrap();
    assert!(request.tags.is_empty());
    assert_eq!(request.data, Some(set.link.clone()));
    // L'arborescence ne montre pas `données/`.
    assert_eq!(tree::read(dir.path()).unwrap().len(), 1);
    remove(dir.path(), "lister.yml").unwrap();
    assert!(
        file::read(&dir.path().join("lister.yml"))
            .unwrap()
            .data
            .is_none()
    );
}

#[test]
fn r11_un_fichier_importe_est_en_lecture_seule() {
    let dir = project();
    let outside = tempfile::tempdir().unwrap();
    let source = outside.path().join("filtres.csv");
    fs::write(&source, "statut,limite\npending,10\npaid,1\n").unwrap();
    let set = import(dir.path(), "lister.yml", &source).unwrap();
    assert!(!set.link.editable);
    assert_eq!(set.table.rows.len(), 2);
    let refused = save(dir.path(), "lister.yml", &set.table);
    assert!(refused.is_err());
}
