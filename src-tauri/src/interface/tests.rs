//! Règles des préférences d'interface (`docs/plan-v1.md`, R1 à R8), une par test.

use super::*;

fn conflict_of(result: Result<Assigned, CommandError>) -> ShortcutConflict {
    match result.unwrap() {
        Assigned::Conflict(conflict) => conflict,
        Assigned::Done => panic!("conflit attendu"),
    }
}

fn keys_of(overrides: &Overrides, id: &str) -> Vec<String> {
    effective(overrides)
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
        .keys
}

#[test]
fn r1_par_defaut_le_theme_suit_le_systeme() {
    let prefs = InterfacePrefs::default();
    assert_eq!(prefs.appearance.theme, Theme::System);
    assert_eq!(prefs.appearance.zoom, 100);
    assert_eq!(prefs.appearance.editor_font, None);
    assert_eq!(prefs.appearance.editor_font_size, 13);
    let json = serde_json::to_value(&prefs.appearance).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "theme": "system", "zoom": 100, "editorFont": null, "editorFontSize": 13 })
    );
}

#[test]
fn r2_r3_seules_les_valeurs_proposees_sont_acceptees() {
    let ok = Appearance {
        theme: Theme::Dark,
        zoom: 125,
        editor_font: Some("Consolas".into()),
        editor_font_size: 15,
    };
    assert!(validate(&ok).is_ok());
    for bad in [
        Appearance {
            zoom: 105,
            ..ok.clone()
        },
        Appearance {
            editor_font: Some("Comic Sans".into()),
            ..ok.clone()
        },
        Appearance {
            editor_font_size: 40,
            ..ok.clone()
        },
    ] {
        let error = validate(&bad).unwrap_err();
        assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
        assert!(error.message.contains("non propos"), "{}", error.message);
    }
}

#[test]
fn r4_une_combinaison_se_note_sous_forme_canonique() {
    assert_eq!(normalize("Ctrl+Shift+k").unwrap(), "Ctrl+Shift+K");
    assert_eq!(normalize("Ctrl+Enter").unwrap(), "Ctrl+Enter");
    assert_eq!(normalize("Ctrl+Backslash").unwrap(), "Ctrl+Backslash");
    assert_eq!(normalize("Alt+,").unwrap(), "Alt+,");
    assert_eq!(normalize("F5").unwrap(), "F5");
    assert_eq!(normalize("Shift+F2").unwrap(), "Shift+F2");
    // Ordre des modificateurs imposé, sans doublon.
    assert!(normalize("Shift+Ctrl+K").is_ok());
    assert_eq!(normalize("Shift+Ctrl+K").unwrap(), "Ctrl+Shift+K");
    for bad in [
        "",
        "Ctrl+",
        "Ctrl+Ctrl+K",
        "Hyper+K",
        "Ctrl+Escape",
        "Ctrl+Truc",
    ] {
        assert!(normalize(bad).is_err(), "{bad}");
    }
}

#[test]
fn r5_une_lettre_seule_ou_une_combinaison_reservee_est_refusee() {
    let error = normalize("Shift+K").unwrap_err();
    assert!(
        error.message.contains("ajoutez Ctrl ou Alt"),
        "{}",
        error.message
    );
    let mut overrides = Overrides::new();
    for (combo, usage) in [
        ("Ctrl+C", "l'édition du texte"),
        ("Ctrl+3", "les workspaces"),
        ("Ctrl+Tab", "passer d'un onglet"),
        ("Ctrl+=", "le zoom"),
    ] {
        let error = assign(&mut overrides, "send", Some(combo), true).unwrap_err();
        assert!(error.message.contains(usage), "{combo} : {}", error.message);
    }
    assert!(overrides.is_empty());
}

#[test]
fn r6_un_conflit_est_signale_avant_d_etre_remplace() {
    let mut overrides = Overrides::new();
    let conflict = conflict_of(assign(&mut overrides, "copyCurl", Some("Ctrl+J"), false));
    assert_eq!(
        conflict,
        ShortcutConflict {
            keys: "Ctrl+J".into(),
            action: "togglePlacement".into(),
            label: "Résultat dessous ou à côté".into(),
        }
    );
    // Rien n'a changé tant que « Remplacer » n'est pas choisi.
    assert!(overrides.is_empty());
    assert_eq!(
        assign(&mut overrides, "copyCurl", Some("Ctrl+J"), true).unwrap(),
        Assigned::Done
    );
    assert_eq!(keys_of(&overrides, "copyCurl"), ["Ctrl+J"]);
    assert!(keys_of(&overrides, "togglePlacement").is_empty());
}

#[test]
fn r6_deux_portees_locales_differentes_partagent_une_combinaison() {
    // Ctrl+F filtre la réponse ou le référentiel selon le focus : pas de conflit.
    let views = effective(&Overrides::new());
    let ctrl_f: Vec<&str> = views
        .iter()
        .filter(|s| s.keys == ["Ctrl+F"])
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(ctrl_f, ["filterResponse", "filterTree"]);
    let mut overrides = Overrides::new();
    assert_eq!(
        assign(&mut overrides, "saveContract", Some("Ctrl+F"), false).unwrap(),
        Assigned::Done
    );
    // Une action globale gêne toutes les portées.
    let conflict = conflict_of(assign(&mut overrides, "closeTab", Some("Ctrl+F"), false));
    assert_eq!(conflict.action, "filterResponse");
}

#[test]
fn r6_aucune_combinaison_par_defaut_n_est_en_conflit() {
    let views = effective(&Overrides::new());
    for (i, a) in views.iter().enumerate() {
        for b in &views[i + 1..] {
            let shared = a.keys.iter().any(|k| b.keys.contains(k));
            assert!(
                !(shared && overlap(a.scope, b.scope)),
                "{} et {} se gênent",
                a.id,
                b.id
            );
        }
        if a.editable {
            for keys in &a.keys {
                assert_eq!(&normalize(keys).unwrap(), keys);
                assert!(reserved(keys).is_none(), "{keys}");
            }
        }
    }
}

#[test]
fn r7_revenir_au_defaut_efface_la_modification_et_retirer_est_garde() {
    let mut overrides = Overrides::new();
    assign(&mut overrides, "send", Some("Ctrl+Shift+Enter"), false).unwrap();
    assert_eq!(keys_of(&overrides, "send"), ["Ctrl+Shift+Enter"]);
    assign(&mut overrides, "send", Some("Ctrl+Enter"), false).unwrap();
    assert!(overrides.is_empty());
    assign(&mut overrides, "send", None, false).unwrap();
    assert!(keys_of(&overrides, "send").is_empty());
    assert_eq!(overrides.get("send"), Some(&None));
}

#[test]
fn r4_une_action_fixe_ou_inconnue_ne_se_change_pas() {
    let mut overrides = Overrides::new();
    let error = assign(&mut overrides, "workspaces", Some("Alt+1"), false).unwrap_err();
    assert!(error.message.contains("ne se change pas"));
    let error = assign(&mut overrides, "inconnue", Some("Alt+1"), false).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
    // Les combinaisons d'une action fixe sont réservées : rien ne les lui prend.
    let error = assign(&mut overrides, "send", Some("Ctrl+0"), true).unwrap_err();
    assert!(error.message.contains("workspaces"), "{}", error.message);
    assert!(overrides.is_empty());
}

#[test]
fn r8_le_fichier_se_relit_et_ecarte_ce_qui_n_est_plus_valable() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(load(dir.path()), InterfacePrefs::default());
    let mut prefs = InterfacePrefs::default();
    prefs.appearance.theme = Theme::Dark;
    prefs.appearance.zoom = 110;
    assign(&mut prefs.shortcuts, "send", Some("Alt+Enter"), false).unwrap();
    save(dir.path(), &prefs).unwrap();
    assert_eq!(load(dir.path()), prefs);
    assert!(!dir.path().join("interface.json.tmp").exists());

    std::fs::write(
        dir.path().join("interface.json"),
        r#"{"appearance":{"theme":"dark","zoom":333},"shortcuts":{"send":"Alt+Enter","inconnue":"Alt+K","workspaces":"Alt+1","closeTab":"n'importe quoi"}}"#,
    )
    .unwrap();
    let read = load(dir.path());
    assert_eq!(read.appearance, Appearance::default());
    assert_eq!(
        read.shortcuts,
        Overrides::from([("send".to_owned(), Some("Alt+Enter".to_owned()))])
    );
    std::fs::write(dir.path().join("interface.json"), "{").unwrap();
    assert_eq!(load(dir.path()), InterfacePrefs::default());
}

/// `DEFAULT_SHORTCUTS` de `src/lib/shortcuts.ts` reprend les combinaisons par défaut du cœur :
/// l'interface s'en sert avant d'avoir lu les préférences.
#[test]
fn raccourcis_par_defaut_identiques_cote_typescript() {
    let ts = include_str!("../../../src/lib/shortcuts.ts");
    let start = ts
        .find("export const DEFAULT_SHORTCUTS")
        .expect("DEFAULT_SHORTCUTS absent de src/lib/shortcuts.ts");
    let block = &ts[start..];
    let block = &block[block.find('{').unwrap() + 1..block.find("};").unwrap()];
    let mut found = Vec::new();
    for line in block.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (id, rest) = line.split_once(':').unwrap();
        let keys: Vec<String> = rest
            .trim()
            .trim_end_matches(',')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(", ")
            .filter(|k| !k.is_empty())
            .map(|k| k.trim_matches('"').replace("\\\\", "\\"))
            .collect();
        found.push((id.trim().to_owned(), keys));
    }
    let expected: Vec<(String, Vec<String>)> = ACTIONS
        .iter()
        .map(|a| {
            (
                a.id.to_owned(),
                a.defaults.iter().map(|&k| k.to_owned()).collect(),
            )
        })
        .collect();
    assert_eq!(found, expected);
}
