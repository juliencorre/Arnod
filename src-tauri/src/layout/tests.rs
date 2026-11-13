//! Tests des zones et fenêtres, regroupés par règle (Z1 à Z8) et cas d'usage.

use uuid::Uuid;

use super::windows::{
    ActiveRequest, Bounds, MAIN, MAX_SIDEBAR_WIDTH, WindowKind, WindowState, Windows, WindowsFile,
};
use super::*;
use crate::error::CommandErrorKind;

fn request(name: &str) -> Content {
    Content::Request {
        project_id: Uuid::from_u128(1),
        path: format!("{name}.yml"),
    }
}

/// Disposition d'une zone avec ces requêtes ouvertes ; renvoie aussi les onglets.
fn layout_with(names: &[&str]) -> (Layout, Vec<Uuid>) {
    let mut layout = Layout::single(Vec::new());
    let tabs = names
        .iter()
        .map(|name| layout.open(request(name), None).unwrap())
        .collect();
    (layout, tabs)
}

fn names(layout: &Layout) -> Vec<Vec<String>> {
    layout
        .zones()
        .iter()
        .map(|zone| {
            zone.tabs
                .iter()
                .map(|tab| match &tab.content {
                    Content::Request { path, .. } => path.trim_end_matches(".yml").to_owned(),
                    Content::Run { .. } => "run".to_owned(),
                    Content::Contract { path, .. } => format!("contrat:{path}"),
                    Content::Result => "résultat".to_owned(),
                    Content::ImportReport { .. } => "rapport".to_owned(),
                    Content::Test { path, .. } => format!("test:{path}"),
                })
                .collect()
        })
        .collect()
}

fn first_zone(layout: &Layout) -> Uuid {
    layout.zones()[0].id
}

fn kind<T: std::fmt::Debug>(result: Result<T, crate::error::CommandError>) -> CommandErrorKind {
    result.unwrap_err().kind
}

// F1, Z3 - onglets

#[test]
fn f1_ouvrir_ajoute_un_onglet_actif_dans_la_zone_active() {
    let (layout, tabs) = layout_with(&["lister", "creer"]);

    assert_eq!(names(&layout), [["lister", "creer"]]);
    assert_eq!(layout.zones()[0].active_tab, Some(tabs[1]));
}

#[test]
fn z3_un_contenu_deja_ouvert_est_seulement_active() {
    let (mut layout, tabs) = layout_with(&["lister", "creer"]);

    let again = layout.open(request("lister"), None).unwrap();

    assert_eq!(again, tabs[0]);
    assert_eq!(names(&layout), [["lister", "creer"]]);
    assert_eq!(layout.zones()[0].active_tab, Some(tabs[0]));
}

#[test]
fn f1_fermer_active_l_onglet_voisin() {
    let (mut layout, tabs) = layout_with(&["a", "b", "c"]);
    layout.activate(first_zone(&layout), tabs[1]).unwrap();

    layout.close(tabs[1]).unwrap();

    assert_eq!(names(&layout), [["a", "c"]]);
    assert_eq!(layout.zones()[0].active_tab, Some(tabs[2]));
    assert_eq!(kind(layout.close(tabs[1])), CommandErrorKind::NotFound);
}

// F2, F3, Z1, Z2 - diviser

#[test]
fn f2_diviser_a_droite_puis_en_dessous() {
    let (mut layout, tabs) = layout_with(&["a", "b", "c"]);
    let left = first_zone(&layout);

    let right = layout.split(left, tabs[2], Side::Right).unwrap();
    layout.split(right, tabs[1], Side::Bottom).unwrap();

    assert_eq!(names(&layout), vec![vec!["a"], vec!["c"], vec!["b"]]);
    let Node::Split { axis, second, .. } = &layout.root else {
        panic!("séparation attendue");
    };
    assert_eq!(*axis, Axis::Row);
    assert!(matches!(
        second.as_ref(),
        Node::Split {
            axis: Axis::Column,
            ..
        }
    ));
    assert_eq!(layout.active_zone, layout.zones()[2].id);
}

#[test]
fn z1_au_plus_quatre_zones() {
    let (mut layout, tabs) = layout_with(&["a", "b", "c", "d", "e"]);
    let zone = first_zone(&layout);
    for tab in &tabs[1..4] {
        layout.split(zone, *tab, Side::Right).unwrap();
    }

    let error = layout.split(zone, tabs[4], Side::Bottom).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert!(error.message.starts_with("Au plus 4 zones par fenêtre"));
    assert_eq!(layout.zones().len(), 4);
}

#[test]
fn f2_le_seul_onglet_d_une_zone_ne_divise_pas_sa_propre_zone() {
    let (mut layout, tabs) = layout_with(&["a"]);

    let error = layout
        .split(first_zone(&layout), tabs[0], Side::Right)
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert_eq!(layout.zones().len(), 1);
}

#[test]
fn z2_une_zone_videe_disparait_et_sa_voisine_prend_la_place() {
    let (mut layout, tabs) = layout_with(&["a", "b"]);
    let left = first_zone(&layout);
    let right = layout.split(left, tabs[1], Side::Right).unwrap();

    layout.close(tabs[1]).unwrap();

    assert!(matches!(layout.root, Node::Zone(_)));
    assert_eq!(names(&layout), [["a"]]);
    assert_eq!(layout.active_zone, left);
    assert_eq!(kind(layout.zone(right)), CommandErrorKind::NotFound);
}

#[test]
fn z2_la_derniere_zone_reste_vide() {
    let (mut layout, tabs) = layout_with(&["a"]);

    layout.close(tabs[0]).unwrap();

    assert_eq!(layout.zones().len(), 1);
    assert!(layout.zones()[0].tabs.is_empty());
    assert_eq!(layout.zones()[0].active_tab, None);
}

#[test]
fn f3_deplacer_un_onglet_vers_une_autre_zone_ou_dans_la_sienne() {
    let (mut layout, tabs) = layout_with(&["a", "b", "c"]);
    let left = first_zone(&layout);
    let right = layout.split(left, tabs[2], Side::Right).unwrap();

    layout.move_tab(tabs[0], right, Some(0)).unwrap();
    assert_eq!(names(&layout), vec![vec!["b"], vec!["a", "c"]]);

    layout.move_tab(tabs[0], right, None).unwrap();
    assert_eq!(names(&layout), vec![vec!["b"], vec!["c", "a"]]);

    // Le dernier onglet de gauche part : la zone de gauche disparaît (Z2).
    layout.move_tab(tabs[1], right, None).unwrap();
    assert_eq!(names(&layout), vec![vec!["c", "a", "b"]]);
}

// F4, Z4 - séparations

#[test]
fn z4_la_proportion_reste_entre_10_et_90_pour_cent() {
    let (mut layout, tabs) = layout_with(&["a", "b"]);
    layout
        .split(first_zone(&layout), tabs[1], Side::Right)
        .unwrap();
    let Node::Split { id, .. } = layout.root else {
        panic!("séparation attendue");
    };
    let ratio = |layout: &Layout| match layout.root {
        Node::Split { ratio, .. } => ratio,
        Node::Zone(_) => f64::NAN,
    };

    layout.resize(id, 0.7).unwrap();
    assert!((ratio(&layout) - 0.7).abs() < f64::EPSILON);
    layout.resize(id, 0.01).unwrap();
    assert!((ratio(&layout) - MIN_RATIO).abs() < f64::EPSILON);
    layout.resize(id, 2.0).unwrap();
    assert!((ratio(&layout) - MAX_RATIO).abs() < f64::EPSILON);
    layout.resize(id, f64::NAN).unwrap();
    assert!((ratio(&layout) - 0.5).abs() < f64::EPSILON);
    assert_eq!(
        kind(layout.resize(Uuid::new_v4(), 0.5)),
        CommandErrorKind::NotFound
    );
}

// F6 - place du résultat

#[test]
fn f6_place_du_resultat_par_zone() {
    let (mut layout, _) = layout_with(&["a"]);
    let zone = first_zone(&layout);
    assert_eq!(layout.zones()[0].placement, Placement::Auto);

    layout.set_placement(zone, Placement::Below).unwrap();

    assert_eq!(layout.zones()[0].placement, Placement::Below);
}

// F7 - dispositions

#[test]
fn f7_dispositions_gardent_les_onglets() {
    let (mut layout, tabs) = layout_with(&["a", "b", "c"]);
    let zone = first_zone(&layout);
    layout.split(zone, tabs[1], Side::Right).unwrap();
    layout.split(zone, tabs[2], Side::Bottom).unwrap();
    assert_eq!(names(&layout), vec![vec!["a"], vec!["c"], vec!["b"]]);

    layout.apply_preset(Preset::Four);
    assert_eq!(layout.zones().len(), 4);
    let mut all: Vec<String> = names(&layout).concat();
    all.sort();
    assert_eq!(all, ["a", "b", "c"]);

    layout.apply_preset(Preset::One);
    assert_eq!(layout.zones().len(), 1);
    assert_eq!(names(&layout)[0].len(), 3);

    layout.apply_preset(Preset::TwoRows);
    assert!(matches!(
        layout.root,
        Node::Split {
            axis: Axis::Column,
            ..
        }
    ));
    assert_eq!(names(&layout), vec![vec!["a", "c", "b"], vec![]]);
}

#[test]
fn format_json_d_une_disposition() {
    let zone = Zone {
        id: Uuid::from_u128(2),
        tabs: vec![Tab {
            id: Uuid::from_u128(3),
            content: Content::Run {
                project_id: Uuid::from_u128(1),
            },
        }],
        active_tab: Some(Uuid::from_u128(3)),
        placement: Placement::Right,
    };
    let layout = Layout {
        active_zone: zone.id,
        root: Node::Zone(zone),
    };
    let id = |n: u128| Uuid::from_u128(n).to_string();

    assert_eq!(
        serde_json::to_value(&layout).unwrap(),
        serde_json::json!({
            "root": {
                "kind": "zone",
                "id": id(2),
                "tabs": [{ "id": id(3), "content": { "kind": "run", "projectId": id(1) } }],
                "activeTab": id(3),
                "placement": "right"
            },
            "activeZone": id(2)
        })
    );
}

// Résultat détachable : docs/plan-resultat-detachable.md (D1 à D6, R1 à R4)

fn active(name: &str) -> Option<ActiveRequest> {
    Some(ActiveRequest {
        project_id: Uuid::from_u128(1),
        path: format!("{name}.yml"),
    })
}

#[test]
fn d1_le_resultat_s_ouvre_dans_sa_fenetre_qui_ne_prend_pas_la_selection() {
    let mut windows = Windows::default();
    windows
        .layout_mut(MAIN)
        .unwrap()
        .open(request("a"), None)
        .unwrap();

    let label = windows.detach_result(None);

    assert_eq!(tab_names(&windows, &label), [["résultat"]]);
    let window = windows.get(&label).unwrap();
    assert_eq!(window.kind, WindowKind::Content);
    assert!(!window.follow);
    // R2 : la sélection de la barre latérale va dans la principale.
    windows.focus(&label);
    assert_eq!(windows.open_selection(request("b")).unwrap(), MAIN);
    assert_eq!(tab_names(&windows, MAIN), [["a", "b"]]);
}

#[test]
fn r1_un_seul_resultat_le_redemander_renvoie_sa_fenetre() {
    let mut windows = Windows::default();
    let first = windows.detach_result(None);

    let again = windows.detach_result(None);

    assert_eq!(first, again);
    assert_eq!(windows.labels().len(), 2);
    assert_eq!(
        windows.result_window().map(|w| w.label.clone()),
        Some(first)
    );
}

#[test]
fn d2_la_requete_active_suit_la_derniere_fenetre_utilisee() {
    let mut windows = Windows::default();
    assert_eq!(windows.active_request(), None);
    let layout = windows.layout_mut(MAIN).unwrap();
    layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    assert_eq!(windows.active_request(), active("b"));

    let content = windows.detach_tab(MAIN, b, None).unwrap();
    assert_eq!(windows.active_request(), active("b"));
    windows.focus(MAIN);
    assert_eq!(windows.active_request(), active("a"));

    // Le focus de la fenêtre de résultat ne change pas la requête active.
    let result = windows.detach_result(None);
    windows.focus(&result);
    assert_eq!(windows.active_request(), active("a"));
    windows.focus(&content);
    assert_eq!(windows.active_request(), active("b"));
}

#[test]
fn d2_une_fenetre_sans_requete_active_est_passee() {
    let mut windows = Windows::default();
    let layout = windows.layout_mut(MAIN).unwrap();
    layout.open(request("a"), None).unwrap();
    layout
        .open(
            Content::Run {
                project_id: Uuid::from_u128(1),
            },
            None,
        )
        .unwrap();

    assert_eq!(windows.active_request(), None);

    let run_tab = windows.get(MAIN).unwrap().layout.as_ref().unwrap().tabs()[1].id;
    let label = windows.detach_tab(MAIN, run_tab, None).unwrap();
    windows.focus(&label);
    assert_eq!(windows.active_request(), active("a"));
}

#[test]
fn d2_le_resultat_dans_une_zone_laisse_active_la_requete_de_l_autre_zone() {
    let mut windows = Windows::default();
    let layout = windows.layout_mut(MAIN).unwrap();
    layout.open(request("a"), None).unwrap();
    let result = layout.open(Content::Result, None).unwrap();
    let zone = first_zone(layout);
    layout.split(zone, result, Side::Right).unwrap();

    assert_eq!(windows.active_request(), active("a"));
}

#[test]
fn r4_rattacher_le_resultat_le_ferme_sans_rien_deplacer() {
    let mut windows = Windows::default();
    windows
        .layout_mut(MAIN)
        .unwrap()
        .open(request("a"), None)
        .unwrap();
    let label = windows.detach_result(None);

    windows.reattach(&label).unwrap();

    assert_eq!(windows.labels(), [MAIN]);
    assert_eq!(tab_names(&windows, MAIN), [["a"]]);
    assert!(windows.result_window().is_none());
}

#[test]
fn d6_la_fenetre_de_resultat_est_restauree_et_ignoree_par_les_projets_oublies() {
    let mut windows = Windows::default();
    windows
        .layout_mut(MAIN)
        .unwrap()
        .open(request("a"), None)
        .unwrap();
    let label = windows.detach_result(None);
    let json = serde_json::to_string(&windows.to_file()).unwrap();
    assert!(json.contains(r#"{"kind":"result"}"#));

    let mut restored = Windows::from_file(serde_json::from_str::<WindowsFile>(&json).unwrap());
    assert_eq!(tab_names(&restored, &label), [["résultat"]]);

    assert!(restored.retain_projects(&[]));
    assert_eq!(restored.labels(), [MAIN.to_owned(), label.clone()]);
    assert_eq!(tab_names(&restored, &label), [["résultat"]]);
}

// Fenêtres : F8 à F13, Z5 à Z8

fn main_layout(windows: &mut Windows) -> &mut Layout {
    windows.layout_mut(MAIN).unwrap()
}

fn tab_names(windows: &Windows, label: &str) -> Vec<Vec<String>> {
    names(windows.get(label).unwrap().layout.as_ref().unwrap())
}

#[test]
fn f8_une_zone_devient_une_fenetre_de_contenu() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    let a = layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    let left = first_zone(layout);
    let right = layout.split(left, b, Side::Right).unwrap();
    let bounds = Bounds {
        x: 1920,
        y: 40,
        width: 900,
        height: 700,
    };

    let label = windows.detach_zone(MAIN, right, Some(bounds)).unwrap();

    assert!(label.starts_with("fenetre-"));
    assert_eq!(tab_names(&windows, MAIN), [["a"]]);
    assert_eq!(tab_names(&windows, &label), [["b"]]);
    let detached = windows.get(&label).unwrap();
    assert_eq!(detached.kind, WindowKind::Content);
    assert!(detached.follow);
    assert_eq!(detached.bounds, Some(bounds));
    let _ = a;
}

#[test]
fn f8_detacher_la_seule_zone_laisse_une_zone_vide() {
    let mut windows = Windows::default();
    main_layout(&mut windows).open(request("a"), None).unwrap();
    let zone = first_zone(main_layout(&mut windows));

    let label = windows.detach_zone(MAIN, zone, None).unwrap();

    assert_eq!(tab_names(&windows, MAIN), [Vec::<String>::new()]);
    assert_eq!(tab_names(&windows, &label), [["a"]]);
    // Une zone vide ne se détache pas.
    assert_eq!(
        kind(windows.detach_zone(MAIN, zone, None)),
        CommandErrorKind::InvalidRequest
    );
}

#[test]
fn f8_un_onglet_seul_en_fenetre_puis_fenetre_videe_fermee() {
    let mut windows = Windows::default();
    let tab = main_layout(&mut windows).open(request("a"), None).unwrap();

    let label = windows.detach_tab(MAIN, tab, None).unwrap();
    assert_eq!(tab_names(&windows, &label), [["a"]]);

    let moved = windows.get(&label).unwrap().layout.as_ref().unwrap().tabs()[0].id;
    windows.close_tab(&label, moved).unwrap();

    assert_eq!(windows.labels(), [MAIN]);
}

#[test]
fn f3_deplacer_un_onglet_vers_une_autre_fenetre() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    let a = layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    let label = windows.detach_tab(MAIN, b, None).unwrap();

    windows.move_tab(MAIN, a, &label, None, None).unwrap();
    assert_eq!(tab_names(&windows, &label), [["b", "a"]]);

    // Retour : la fenêtre de contenu vidée se ferme.
    for tab in [a, b] {
        windows.move_tab(&label, tab, MAIN, None, None).unwrap();
    }
    assert_eq!(windows.labels(), [MAIN]);
    assert_eq!(tab_names(&windows, MAIN), [["a", "b"]]);
}

#[test]
fn z5_la_selection_va_a_la_derniere_fenetre_qui_la_suit() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    let a = layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    let pinned = windows.detach_tab(MAIN, a, None).unwrap();
    let following = windows.detach_tab(MAIN, b, None).unwrap();
    windows.set_follow(&pinned, false).unwrap();
    let sidebar = windows.detach_sidebar(None);

    windows.focus(&following);
    windows.focus(&pinned);
    windows.focus(&sidebar);
    assert_eq!(windows.selection_target(), following);
    assert_eq!(windows.followers(), 2);

    let target = windows.open_selection(request("c")).unwrap();
    assert_eq!(target, following);
    assert_eq!(tab_names(&windows, &following), [["b", "c"]]);
    assert_eq!(tab_names(&windows, &pinned), [["a"]]);

    windows.focus(MAIN);
    assert_eq!(windows.open_selection(request("d")).unwrap(), MAIN);
    assert_eq!(
        kind(windows.set_follow(MAIN, false)),
        CommandErrorKind::InvalidRequest
    );
}

#[test]
fn f9_la_barre_laterale_detachee_puis_rattachee() {
    let mut windows = Windows::default();

    let label = windows.detach_sidebar(None);
    assert!(windows.get(MAIN).unwrap().sidebar_detached);
    assert_eq!(windows.get(&label).unwrap().kind, WindowKind::Sidebar);
    assert_eq!(
        windows.detach_sidebar(None),
        label,
        "une seule barre détachée"
    );
    assert_eq!(
        kind(windows.layout_mut(&label)),
        CommandErrorKind::InvalidRequest
    );

    windows.reattach(&label).unwrap();

    assert!(!windows.get(MAIN).unwrap().sidebar_detached);
    assert_eq!(windows.labels(), [MAIN]);
}

#[test]
fn z6_rattacher_ramene_les_onglets_dans_la_zone_active_sans_doublon() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    let label = windows.detach_tab(MAIN, b, None).unwrap();
    windows
        .layout_mut(&label)
        .unwrap()
        .open(request("c"), None)
        .unwrap();
    // « a » est aussi ouvert dans la fenêtre détachée.
    windows
        .layout_mut(&label)
        .unwrap()
        .open(request("a"), None)
        .unwrap();

    windows.reattach(&label).unwrap();

    assert_eq!(windows.labels(), [MAIN]);
    assert_eq!(tab_names(&windows, MAIN), [["a", "b", "c"]]);
    assert_eq!(
        kind(windows.reattach(MAIN)),
        CommandErrorKind::InvalidRequest
    );
    assert_eq!(
        kind(windows.reattach("fenetre-inconnue")),
        CommandErrorKind::NotFound
    );
}

#[test]
fn f12_rattacher_toutes_les_fenetres() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    let a = layout.open(request("a"), None).unwrap();
    let b = layout.open(request("b"), None).unwrap();
    windows.detach_tab(MAIN, a, None).unwrap();
    windows.detach_tab(MAIN, b, None).unwrap();
    windows.detach_sidebar(None);

    windows.reattach_all().unwrap();

    assert_eq!(windows.labels(), [MAIN]);
    assert!(!windows.get(MAIN).unwrap().sidebar_detached);
    assert_eq!(tab_names(&windows, MAIN), [["a", "b"]]);
}

#[test]
fn f5_largeur_de_la_barre_laterale_bornee() {
    let mut windows = Windows::default();

    windows.set_sidebar_width(10_000);

    assert_eq!(windows.get(MAIN).unwrap().sidebar_width, MAX_SIDEBAR_WIDTH);
}

#[test]
fn oublier_un_projet_ferme_ses_onglets_partout() {
    let mut windows = Windows::default();
    let layout = main_layout(&mut windows);
    let a = layout.open(request("a"), None).unwrap();
    layout
        .open(
            Content::Run {
                project_id: Uuid::from_u128(9),
            },
            None,
        )
        .unwrap();
    windows.detach_tab(MAIN, a, None).unwrap();

    windows.forget_project(Uuid::from_u128(1));

    assert_eq!(windows.labels(), [MAIN]);
    assert_eq!(tab_names(&windows, MAIN), [["run"]]);
}

#[test]
fn f11_garder_les_onglets_des_seuls_projets_connus() {
    let mut windows = Windows::default();
    main_layout(&mut windows).open(request("a"), None).unwrap();
    assert!(!windows.retain_projects(&[Uuid::from_u128(1)]));

    assert!(windows.retain_projects(&[]));

    assert!(main_layout(&mut windows).tabs().is_empty());
}

#[test]
fn f11_renommer_ou_supprimer_un_chemin_met_les_onglets_a_jour_partout() {
    let mut windows = Windows::default();
    let project = Uuid::from_u128(1);
    let open = |windows: &mut Windows, path: &str| {
        main_layout(windows)
            .open(
                Content::Request {
                    project_id: project,
                    path: path.to_owned(),
                },
                None,
            )
            .unwrap()
    };
    open(&mut windows, "clients/lister.yml");
    let detached = open(&mut windows, "clients/v2/creer.yml");
    open(&mut windows, "clients-old/a.yml");
    let label = windows.detach_tab(MAIN, detached, None).unwrap();

    windows.rename_path(project, "clients", "comptes");

    assert_eq!(
        tab_names(&windows, MAIN),
        [["comptes/lister", "clients-old/a"]]
    );
    assert_eq!(tab_names(&windows, &label), [["comptes/v2/creer"]]);

    windows.close_path(project, "comptes/v2");
    assert_eq!(windows.labels(), [MAIN], "fenêtre vidée fermée");
    windows.close_path(project, "comptes/lister.yml");
    assert_eq!(tab_names(&windows, MAIN), [["clients-old/a"]]);
}

#[test]
fn c11_supprimer_un_contrat_ferme_ses_onglets() {
    let mut windows = Windows::default();
    let project = Uuid::from_u128(1);
    let contract = |path: &str| Content::Contract {
        project_id: project,
        path: path.to_owned(),
    };
    main_layout(&mut windows)
        .open(contract("contracts/petstore.yaml"), None)
        .unwrap();
    main_layout(&mut windows)
        .open(contract("contracts/commandes.json"), None)
        .unwrap();
    // Même contenu : une seule fois par fenêtre (Z3).
    main_layout(&mut windows)
        .open(contract("contracts/petstore.yaml"), None)
        .unwrap();
    assert_eq!(
        tab_names(&windows, MAIN),
        [[
            "contrat:contracts/petstore.yaml",
            "contrat:contracts/commandes.json"
        ]]
    );

    windows.close_path(project, "contracts/petstore.yaml");
    assert_eq!(
        tab_names(&windows, MAIN),
        [["contrat:contracts/commandes.json"]]
    );
}

#[test]
fn le_menu_fenetre_agit_sur_la_derniere_fenetre_a_zones() {
    let mut windows = Windows::default();
    let a = main_layout(&mut windows).open(request("a"), None).unwrap();
    let detached = windows.detach_tab(MAIN, a, None).unwrap();
    let sidebar = windows.detach_sidebar(None);

    windows.focus(&sidebar);
    assert_eq!(windows.last_with_layout(), detached);
    windows.focus(MAIN);
    assert_eq!(windows.last_with_layout(), MAIN);
}

#[test]
fn z8_aller_retour_par_le_fichier() {
    let mut windows = Windows::default();
    let b = main_layout(&mut windows).open(request("b"), None).unwrap();
    main_layout(&mut windows).open(request("a"), None).unwrap();
    windows.detach_tab(MAIN, b, None).unwrap();
    windows.detach_sidebar(None);
    windows.set_sidebar_width(320);

    let restored = Windows::from_file(windows.to_file());

    assert_eq!(restored.to_file(), windows.to_file());
}

#[test]
fn z8_fichier_incoherent_repare() {
    let content = WindowState {
        label: "fenetre-vide".to_owned(),
        kind: WindowKind::Content,
        layout: Some(Layout::single(Vec::new())),
        follow: true,
        sidebar_width: 260,
        sidebar_detached: false,
        bounds: None,
    };
    let foreign = WindowState {
        label: "autre".to_owned(),
        kind: WindowKind::Sidebar,
        ..content.clone()
    };
    let mut main = Windows::default().to_file().windows.remove(0);
    main.sidebar_detached = true;

    let windows = Windows::from_file(WindowsFile {
        version: 1,
        windows: vec![content, foreign, main.clone(), main],
    });

    // Fenêtre de contenu vide et étiquette étrangère écartées, principale unique,
    // barre latérale réputée rattachée faute de sa fenêtre.
    assert_eq!(windows.labels(), [MAIN]);
    assert!(!windows.get(MAIN).unwrap().sidebar_detached);
    assert_eq!(
        Windows::from_file(WindowsFile {
            version: 1,
            windows: vec![]
        })
        .labels(),
        [MAIN]
    );
}

#[test]
fn unions_typescript_concordantes() {
    use crate::ts_contract::{serialized_name, union_members};
    use crate::windows::ProjectScope;
    use std::collections::BTreeSet;

    fn names<T: serde::Serialize>(values: impl IntoIterator<Item = T>) -> BTreeSet<String> {
        values.into_iter().map(serialized_name).collect()
    }

    assert_eq!(
        union_members("Placement"),
        names([Placement::Auto, Placement::Right, Placement::Below])
    );
    assert_eq!(union_members("Axis"), names([Axis::Row, Axis::Column]));
    assert_eq!(union_members("Side"), names([Side::Right, Side::Bottom]));
    assert_eq!(
        union_members("LayoutPreset"),
        names([
            Preset::One,
            Preset::TwoColumns,
            Preset::TwoRows,
            Preset::Three,
            Preset::Four
        ])
    );
    assert_eq!(
        union_members("WindowKind"),
        names([WindowKind::Main, WindowKind::Content, WindowKind::Sidebar])
    );
    assert_eq!(
        union_members("ProjectScope"),
        names([
            ProjectScope::Tree,
            ProjectScope::Request,
            ProjectScope::Environments,
            ProjectScope::Contracts
        ])
    );
}

// Lot C - Import Bruno.
#[test]
fn rapport_d_import_forme_json_et_projet() {
    let project_id = Uuid::from_u128(7);
    let content = Content::ImportReport { project_id };
    assert_eq!(
        serde_json::to_value(&content).unwrap(),
        serde_json::json!({ "kind": "importReport", "projectId": project_id })
    );
    assert_eq!(content.project_id(), Some(project_id));
}

#[test]
fn lot_e_un_onglet_de_test_suit_renommage_et_suppression() {
    let project = Uuid::from_u128(9);
    let mut layout = Layout::single(Vec::new());
    layout
        .open(
            Content::Test {
                project_id: project,
                path: "Clients/Créer".into(),
            },
            None,
        )
        .unwrap();
    layout.rename_path(project, "Clients", "Comptes");
    assert_eq!(names(&layout), [["test:Comptes/Créer"]]);
    let json = serde_json::to_value(&layout.zones()[0].tabs[0].content).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "kind": "test", "projectId": project.to_string(), "path": "Comptes/Créer" })
    );
    layout.close_path(project, "Comptes");
    assert!(layout.zones()[0].tabs.is_empty());
}
