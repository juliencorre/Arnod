//! Règles de l'historique des envois (`docs/plan-v1.md`, H1 à H6), une par test.

use super::*;
use crate::http::{HttpResponse, Timeline};

fn check(id: &str, passed: bool) -> AssertionResult {
    AssertionResult {
        id: id.into(),
        target: id.into(),
        passed,
        message: None,
        actual: None,
        ignored: false,
    }
}

fn entry(at: u64, status: u16, total_ms: u64, body: &str) -> HistoryEntry {
    HistoryEntry {
        id: Uuid::new_v4(),
        at,
        environment: Some("Local".into()),
        status,
        status_text: if status < 400 {
            "Created".into()
        } else {
            "Bad Request".into()
        },
        total_ms,
        size_bytes: body.len(),
        passed: status < 400,
        assertions: vec![check("status", status < 400), check("body.id", true)],
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("Location".into(), "/orders/1".into()),
            ("Date".into(), format!("jour {at}")),
        ],
        body: body.into(),
        truncated: false,
        sent: None,
    }
}

fn outcome(body: &str) -> SendOutcome {
    SendOutcome {
        response: HttpResponse {
            status: 201,
            status_text: "Created".into(),
            headers: vec![("Location".into(), "/orders/1".into())],
            body: body.into(),
            truncated: false,
            size_bytes: body.len(),
            wait_ms: 10,
            receive_ms: 1,
            total_ms: 42,
            timeline: Timeline::default(),
        },
        assertions: vec![check("status", true)],
        passed: true,
        slow: false,
        sent: None,
    }
}

#[test]
fn h1_un_envoi_garde_sa_reponse_et_coupe_un_long_corps() {
    let kept = entry_of(&outcome(r#"{"id":"ord_1"}"#), 1000);
    assert_eq!(kept.status, 201);
    assert_eq!(kept.total_ms, 42);
    assert_eq!(kept.at, 1000);
    assert!(kept.passed);
    assert!(!kept.truncated);
    assert_eq!(kept.body, r#"{"id":"ord_1"}"#);

    let long = "é".repeat(MAX_BODY);
    let kept = entry_of(&outcome(&long), 1000);
    assert!(kept.truncated);
    assert!(kept.body.len() <= MAX_BODY);
    assert!(kept.body.chars().all(|c| c == 'é'));
}

#[test]
fn h1_vingt_envois_au_plus_par_requete_le_plus_recent_d_abord() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    for at in 0..25 {
        record(
            dir.path(),
            project,
            "Clients/lister.yml",
            entry(at, 201, 10, "{}"),
        )
        .unwrap();
    }
    let kept = entries(dir.path(), project, "Clients/lister.yml");
    assert_eq!(kept.len(), MAX_ENTRIES);
    assert_eq!(kept[0].at, 24);
    assert_eq!(kept[19].at, 5);
    // Rien dans le projet : le fichier est dans le dossier de données du poste.
    let file = file(dir.path(), project, "Clients/lister.yml");
    assert!(file.starts_with(local::project_dir(dir.path(), project)));
    assert_eq!(
        file.file_name().unwrap().to_str().unwrap(),
        "Clients%2Flister%2Eyml.json"
    );
    assert!(entries(dir.path(), project, "autre.yml").is_empty());
}

#[test]
fn h2_chaque_envoi_se_compare_au_precedent() {
    let older = entry(1, 201, 38, r#"{"id":1}"#);
    let mut newer = entry(2, 201, 42, r#"{"id":1}"#);
    newer.headers.retain(|(n, _)| n != "Location");
    let view = view_of("a.yml", vec![newer, older]);
    assert_eq!(view.items.len(), 2);
    assert_eq!(view.max, 20);
    assert_eq!(view.items[0].previous_at, Some(1));
    assert_eq!(
        view.items[0].changes,
        ["l'en-tête Location a disparu", "4 ms de plus"]
    );
    // Le plus ancien n'a rien à quoi se comparer.
    assert_eq!(view.items[1].previous_at, None);
    assert!(view.items[1].changes.is_empty());
}

#[test]
fn h3_le_resume_dit_statut_corps_en_tetes_assertions_et_duree() {
    let before = entry(1, 201, 50, r#"{"id": 1, "status": "pending"}"#);
    let same_json = entry(2, 201, 50, "{\n  \"id\": 1,\n  \"status\": \"pending\"\n}");
    // Même JSON écrit autrement, Date changée : rien de notable.
    assert!(compare(&same_json, &before).is_empty());

    let mut after = entry(3, 400, 12, r#"{"error":"x"}"#);
    after.headers.push(("ETag".into(), "abc".into()));
    after.headers.push(("X-Trace".into(), "1".into()));
    after.headers[1].1 = "/orders/2".into();
    after.environment = Some("Recette".into());
    assert_eq!(
        compare(&after, &before),
        [
            "statut 400 au lieu de 201",
            "environnement Recette au lieu de Local",
            "les en-têtes ETag et X-Trace sont apparus",
            "l'en-tête Location a changé",
            "le corps a changé",
            "1 assertion réussie de moins",
            "38 ms de moins",
        ]
    );
}

#[test]
fn h4_les_differences_se_lisent_ligne_a_ligne() {
    let before = entry(1, 201, 10, r#"{"id":1,"status":"pending"}"#);
    let mut after = entry(2, 400, 10, r#"{"id":1,"status":"paid"}"#);
    after.headers.retain(|(n, _)| n != "Location");
    let diff = diff(&after, &before);
    assert_eq!(diff.previous_id, before.id);
    assert!(!diff.truncated);
    let lines: Vec<(DiffPart, bool, &str)> = diff
        .lines
        .iter()
        .map(|l| (l.part, l.added, l.text.as_str()))
        .collect();
    assert_eq!(
        lines,
        [
            (DiffPart::Status, false, "201 Created"),
            (DiffPart::Status, true, "400 Bad Request"),
            (DiffPart::Header, false, "Location: /orders/1"),
            (DiffPart::Body, false, "  \"status\": \"pending\""),
            (DiffPart::Body, true, "  \"status\": \"paid\""),
            (DiffPart::Assertion, false, "✓ status"),
            (DiffPart::Assertion, true, "✕ status"),
        ]
    );
}

#[test]
fn h4_les_differences_sont_limitees() {
    let lines = |prefix: &str| {
        (0..400)
            .map(|i| format!("{prefix}{i}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let diff = diff(
        &entry(2, 201, 1, &lines("b")),
        &entry(1, 201, 1, &lines("a")),
    );
    assert!(diff.truncated);
    assert_eq!(diff.lines.len(), MAX_DIFF_LINES);
}

#[test]
fn h4_le_plus_ancien_n_a_pas_de_differences() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    let first = entry(1, 201, 10, "a");
    let first_id = first.id;
    record(dir.path(), project, "a.yml", first).unwrap();
    let second = entry(2, 201, 10, "b");
    let second_id = second.id;
    record(dir.path(), project, "a.yml", second).unwrap();
    assert_eq!(
        diff_of(dir.path(), project, "a.yml", second_id)
            .unwrap()
            .previous_id,
        first_id
    );
    let error = diff_of(dir.path(), project, "a.yml", first_id).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
}

#[test]
fn h5_effacer_oublie_l_historique_de_la_requete() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    record(dir.path(), project, "a.yml", entry(1, 201, 1, "")).unwrap();
    record(dir.path(), project, "b.yml", entry(1, 201, 1, "")).unwrap();
    clear(dir.path(), project, "a.yml").unwrap();
    clear(dir.path(), project, "absent.yml").unwrap();
    assert!(view(dir.path(), project, "a.yml").items.is_empty());
    assert_eq!(view(dir.path(), project, "b.yml").items.len(), 1);
}

#[test]
fn h5_annuler_remet_les_envois_effaces_apres_les_nouveaux() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    for at in [1, 2] {
        record(dir.path(), project, "a.yml", entry(at, 201, 1, "")).unwrap();
    }
    let cleared = entries(dir.path(), project, "a.yml");
    clear(dir.path(), project, "a.yml").unwrap();
    record(dir.path(), project, "a.yml", entry(3, 201, 1, "")).unwrap();
    restore(dir.path(), project, "a.yml", cleared.clone()).unwrap();
    // Restaurer deux fois ne duplique rien.
    restore(dir.path(), project, "a.yml", cleared).unwrap();
    let ats: Vec<u64> = entries(dir.path(), project, "a.yml")
        .iter()
        .map(|e| e.at)
        .collect();
    assert_eq!(ats, [3, 2, 1]);
}

#[test]
fn h6_l_historique_suit_renommages_et_suppressions() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    for path in ["D/a.yml", "D/sous/b.yml", "Dossier/c.yml"] {
        record(dir.path(), project, path, entry(1, 201, 1, "")).unwrap();
    }
    rename(dir.path(), project, "D", "E");
    assert!(entries(dir.path(), project, "D/a.yml").is_empty());
    assert_eq!(entries(dir.path(), project, "E/a.yml").len(), 1);
    assert_eq!(entries(dir.path(), project, "E/sous/b.yml").len(), 1);
    // « Dossier » n'est pas sous « D ».
    assert_eq!(entries(dir.path(), project, "Dossier/c.yml").len(), 1);
    remove(dir.path(), project, "E");
    assert!(entries(dir.path(), project, "E/a.yml").is_empty());
    assert!(entries(dir.path(), project, "E/sous/b.yml").is_empty());
    assert_eq!(entries(dir.path(), project, "Dossier/c.yml").len(), 1);
}

#[test]
fn h6_un_fichier_plus_recent_n_est_jamais_reecrit() {
    let dir = tempfile::tempdir().unwrap();
    let project = Uuid::new_v4();
    let target = file(dir.path(), project, "a.yml");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, r#"{"version":99,"entries":[]}"#).unwrap();
    record(dir.path(), project, "a.yml", entry(1, 201, 1, "")).unwrap();
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        r#"{"version":99,"entries":[]}"#
    );
    // Illisible : l'historique repart de zéro.
    fs::write(&target, "{").unwrap();
    record(dir.path(), project, "a.yml", entry(1, 201, 1, "")).unwrap();
    assert_eq!(entries(dir.path(), project, "a.yml").len(), 1);
}
