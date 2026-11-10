//! Tests du catalogue, regroupés par règle (R1 à R12) et cas d'usage. R3, R4,
//! R7, R8, R11 et R12 suivent `docs/plan-workspaces-onglets.md`.

use super::*;

fn path(name: &str) -> PathBuf {
    PathBuf::from("/dev").join(name)
}

fn every_folder_exists(_: &Path) -> bool {
    true
}

/// Inscrit un projet de dossier `/dev/<name>` dans `workspace_id`.
fn add_project(catalog: &mut Catalog, name: &str, workspace_id: Uuid) -> Uuid {
    add_project_to(catalog, name, Some(workspace_id))
}

/// Inscrit un projet de dossier `/dev/<name>`, dans un workspace ou aucun.
fn add_project_to(catalog: &mut Catalog, name: &str, workspace_id: Option<Uuid>) -> Uuid {
    let id = Uuid::new_v4();
    catalog
        .register_project(id, name, path(name), workspace_id)
        .unwrap();
    id
}

/// Catalogue avec un workspace « Mes projets », ouvert.
fn with_workspace() -> (Catalog, Uuid) {
    let mut catalog = Catalog::first_launch();
    let id = catalog.create_workspace("Mes projets").unwrap();
    catalog.open_workspace(Some(id)).unwrap();
    (catalog, id)
}

fn kind<T: std::fmt::Debug>(result: Result<T, CommandError>) -> CommandErrorKind {
    result.unwrap_err().kind
}

fn workspace_names(catalog: &Catalog) -> Vec<String> {
    catalog
        .snapshot(every_folder_exists)
        .workspaces
        .into_iter()
        .map(|workspace| workspace.name)
        .collect()
}

fn project_ids_of(catalog: &Catalog, workspace_id: Uuid) -> Vec<Uuid> {
    catalog.workspace(workspace_id).unwrap().project_ids.clone()
}

// R3 - pas de workspace par défaut

#[test]
fn r3_premier_lancement_sans_workspace_sur_tous_les_projets() {
    let catalog = Catalog::first_launch();
    let snapshot = catalog.snapshot(every_folder_exists);

    assert!(snapshot.projects.is_empty());
    assert!(snapshot.workspaces.is_empty());
    assert_eq!(snapshot.last_workspace_id, None);
}

// R1 - un projet n'existe qu'une fois dans le référentiel

#[test]
fn r1_un_dossier_ne_peut_etre_inscrit_qu_une_fois() {
    let (mut catalog, default) = with_workspace();
    add_project(&mut catalog, "boutique", default);

    let error = catalog
        .register_project(Uuid::new_v4(), "autre", path("boutique"), Some(default))
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::AlreadyRegistered);
    assert_eq!(
        error.message,
        "Ce dossier est déjà celui du projet « boutique »."
    );
    assert_eq!(catalog.projects.len(), 1);
}

#[test]
fn r1_chemins_compares_par_composants() {
    let (mut catalog, default) = with_workspace();
    add_project(&mut catalog, "boutique", default);

    assert_eq!(
        kind(catalog.check_new_project("autre", Path::new("/dev//boutique/"), Some(default))),
        CommandErrorKind::AlreadyRegistered
    );
}

#[test]
fn r1_comparaison_sans_casse_par_composants() {
    // Règle appliquée sous Windows, testée sur toutes les plateformes.
    assert!(same_path_ignoring_case(
        Path::new("/dev//Boutique/"),
        Path::new("/dev/boutique")
    ));
    assert!(!same_path_ignoring_case(
        Path::new("/dev/boutique"),
        Path::new("/dev/boutiques")
    ));
}

#[test]
fn r1_un_identifiant_ne_peut_etre_inscrit_qu_une_fois() {
    let (mut catalog, default) = with_workspace();
    let id = add_project(&mut catalog, "boutique", default);

    assert_eq!(
        kind(catalog.register_project(id, "autre", path("autre"), Some(default))),
        CommandErrorKind::AlreadyRegistered
    );
}

// R2 - un projet appartient à zéro, un ou plusieurs workspaces

#[test]
fn r2_un_projet_dans_plusieurs_workspaces_puis_dans_aucun() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);
    catalog.add_projects(default, &[project]).unwrap();

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects[0].workspace_ids, vec![default, boutique]);

    catalog.remove_project(default, project).unwrap();
    catalog.remove_project(boutique, project).unwrap();

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects.len(), 1);
    assert!(snapshot.projects[0].workspace_ids.is_empty());
}

#[test]
fn r3_tout_workspace_peut_etre_supprime() {
    let (mut catalog, only) = with_workspace();

    catalog.delete_workspace(only).unwrap();

    assert!(catalog.snapshot(every_folder_exists).workspaces.is_empty());
}

// R4 - un nouveau projet va dans « Tous les projets », et dans le workspace choisi

#[test]
fn r4_nouveau_projet_dans_le_seul_workspace_choisi() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    let project = add_project(&mut catalog, "paiements-v3", boutique);

    assert!(project_ids_of(&catalog, default).is_empty());
    assert_eq!(project_ids_of(&catalog, boutique), vec![project]);
}

#[test]
fn r4_nouveau_projet_sans_workspace() {
    let (mut catalog, default) = with_workspace();

    let project = add_project_to(&mut catalog, "boutique-tests", None);

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects[0].id, project);
    assert!(snapshot.projects[0].workspace_ids.is_empty());
    assert!(project_ids_of(&catalog, default).is_empty());
}

#[test]
fn r4_workspace_inconnu_refuse_sans_rien_inscrire() {
    let mut catalog = Catalog::first_launch();

    assert_eq!(
        kind(catalog.register_project(
            Uuid::new_v4(),
            "boutique",
            path("boutique"),
            Some(Uuid::new_v4())
        )),
        CommandErrorKind::NotFound
    );
    assert!(catalog.projects.is_empty());
}

// R5 - retirer ou supprimer ne supprime jamais un projet

#[test]
fn r5_retirer_un_projet_le_garde_au_referentiel() {
    let (mut catalog, default) = with_workspace();
    let project = add_project(&mut catalog, "boutique-tests", default);

    catalog.remove_project(default, project).unwrap();

    assert!(project_ids_of(&catalog, default).is_empty());
    assert!(catalog.project(project).is_ok());
}

#[test]
fn r5_supprimer_un_workspace_garde_ses_projets_ailleurs() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);
    catalog.add_projects(default, &[project]).unwrap();

    catalog.delete_workspace(boutique).unwrap();

    assert!(catalog.workspace(boutique).is_err());
    assert!(catalog.project(project).is_ok());
    assert_eq!(project_ids_of(&catalog, default), vec![project]);
}

#[test]
fn r5_retirer_un_projet_absent_du_workspace() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", default);

    let error = catalog.remove_project(boutique, project).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert_eq!(error.message, "Ce projet n'est pas dans ce workspace.");
}

// R6 - « Annuler » au lieu d'une confirmation

#[test]
fn r6_annuler_le_retrait_remet_le_projet_a_sa_place() {
    let (mut catalog, default) = with_workspace();
    let first = add_project(&mut catalog, "a", default);
    let second = add_project(&mut catalog, "b", default);
    let third = add_project(&mut catalog, "c", default);

    let action = catalog.remove_project(default, second).unwrap();
    catalog.undo(action.clone()).unwrap();

    assert_eq!(
        project_ids_of(&catalog, default),
        vec![first, second, third]
    );

    // Annuler une seconde fois est sans effet.
    catalog.undo(action).unwrap();
    assert_eq!(
        project_ids_of(&catalog, default),
        vec![first, second, third]
    );
}

#[test]
fn r6_annuler_la_suppression_retablit_le_workspace_ouvert_a_sa_place() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    catalog.create_workspace("Recette").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);
    catalog.open_workspace(Some(boutique)).unwrap();
    let before = catalog.clone();

    let action = catalog.delete_workspace(boutique).unwrap();
    assert_eq!(catalog.last_workspace_id(), None);
    catalog.undo(action.clone()).unwrap();

    assert_eq!(catalog, before);
    assert_eq!(project_ids_of(&catalog, boutique), vec![project]);
    assert_eq!(catalog.last_workspace_id(), Some(boutique));

    catalog.undo(action).unwrap();
    assert_eq!(catalog, before);
}

#[test]
fn r6_annulation_impossible_si_le_nom_a_ete_repris() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let action = catalog.delete_workspace(boutique).unwrap();
    catalog.create_workspace("boutique").unwrap();

    let error = catalog.undo(action).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(
        error.message,
        "Impossible d'annuler : un autre workspace s'appelle déjà « Boutique »."
    );
}

#[test]
fn r6_annuler_un_retrait_dans_un_workspace_supprime() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);
    let action = catalog.remove_project(boutique, project).unwrap();
    catalog.delete_workspace(boutique).unwrap();

    assert_eq!(kind(catalog.undo(action)), CommandErrorKind::NotFound);
}

// P11 - oublier un projet

#[test]
fn p11_oublier_retire_de_tous_les_projets_et_des_workspaces() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let first = add_project(&mut catalog, "a", boutique);
    let forgotten = add_project(&mut catalog, "b", boutique);
    let last = add_project(&mut catalog, "c", default);
    catalog.add_projects(default, &[first, forgotten]).unwrap();
    let before = catalog.clone();

    let action = catalog.forget_project(forgotten).unwrap();

    assert_eq!(kind(catalog.project(forgotten)), CommandErrorKind::NotFound);
    assert_eq!(project_ids_of(&catalog, default), vec![last, first]);
    assert_eq!(project_ids_of(&catalog, boutique), vec![first]);

    catalog.undo(action.clone()).unwrap();
    assert_eq!(catalog, before);
    catalog.undo(action).unwrap();
    assert_eq!(catalog, before);
    assert_eq!(
        kind(catalog.forget_project(Uuid::new_v4())),
        CommandErrorKind::NotFound
    );
}

#[test]
fn p11_annulation_impossible_si_le_nom_a_ete_repris() {
    let (mut catalog, default) = with_workspace();
    let forgotten = add_project(&mut catalog, "boutique", default);
    let action = catalog.forget_project(forgotten).unwrap();
    catalog
        .register_project(Uuid::new_v4(), "BOUTIQUE", path("ailleurs"), Some(default))
        .unwrap();

    let error = catalog.undo(action).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(
        error.message,
        "Impossible d'annuler : un autre projet s'appelle déjà « BOUTIQUE »."
    );
}

// R7 - rouvrir le dernier onglet utilisé

#[test]
fn r7_l_onglet_ouvert_est_memorise() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    catalog.open_workspace(Some(boutique)).unwrap();
    assert_eq!(catalog.last_workspace_id(), Some(boutique));
    assert_eq!(catalog.to_file().last_workspace_id, Some(boutique));

    catalog.open_workspace(None).unwrap();
    assert_eq!(catalog.last_workspace_id(), None);
    assert_eq!(
        kind(catalog.open_workspace(Some(Uuid::new_v4()))),
        CommandErrorKind::NotFound
    );
    assert_eq!(catalog.last_workspace_id(), None);
}

#[test]
fn r7_dernier_workspace_disparu_remplace_par_tous_les_projets() {
    let mut file = with_workspace().0.to_file();
    file.last_workspace_id = Some(Uuid::new_v4());

    let catalog = Catalog::from_file(file).unwrap();

    assert_eq!(catalog.last_workspace_id(), None);
}

// R8 - supprimer le workspace ouvert ramène sur « Tous les projets »

#[test]
fn r8_supprimer_le_workspace_ouvert_ramene_sur_tous_les_projets() {
    let (mut catalog, _) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    catalog.open_workspace(Some(boutique)).unwrap();

    catalog.delete_workspace(boutique).unwrap();

    assert_eq!(catalog.last_workspace_id(), None);
}

#[test]
fn r8_supprimer_un_autre_workspace_ne_change_pas_l_onglet_ouvert() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let recette = catalog.create_workspace("Recette").unwrap();
    catalog.open_workspace(Some(boutique)).unwrap();

    catalog.delete_workspace(recette).unwrap();

    assert_eq!(catalog.last_workspace_id(), Some(boutique));
}

// R9 - noms valides et uniques sans tenir compte de la casse

#[test]
fn r9_noms_de_workspace_uniques_sans_tenir_compte_de_la_casse() {
    let (mut catalog, _) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    let error = catalog.create_workspace("  mes PROJETS ").unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(error.message, "Un workspace « Mes projets » existe déjà.");
    assert_eq!(
        kind(catalog.rename_workspace(boutique, "MES PROJETS")),
        CommandErrorKind::NameTaken
    );
}

#[test]
fn r9_renommer_en_changeant_la_casse_de_son_propre_nom() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("boutique").unwrap();

    catalog.rename_workspace(boutique, "  Boutique ").unwrap();

    assert_eq!(catalog.workspace(boutique).unwrap().name, "Boutique");
}

#[test]
fn r9_nom_de_workspace_invalide() {
    let (mut catalog, default) = with_workspace();

    assert_eq!(
        kind(catalog.create_workspace(" ")),
        CommandErrorKind::InvalidName
    );
    assert_eq!(
        kind(catalog.rename_workspace(default, "")),
        CommandErrorKind::InvalidName
    );
    assert_eq!(catalog.workspace(default).unwrap().name, "Mes projets");
}

#[test]
fn r9_noms_de_projet_uniques_sans_tenir_compte_de_la_casse() {
    let (mut catalog, default) = with_workspace();
    add_project(&mut catalog, "boutique-tests", default);

    let error = catalog
        .check_new_project("Boutique-Tests", &path("ailleurs"), Some(default))
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(
        error.message,
        "Un projet « boutique-tests » existe déjà dans « Tous les projets »."
    );
}

// R10 - le nom d'un projet est un nom de dossier valide

#[test]
fn r10_nom_de_projet_invalide_refuse() {
    let (mut catalog, default) = with_workspace();

    assert_eq!(
        kind(catalog.check_new_project("api/v2", &path("api"), Some(default))),
        CommandErrorKind::InvalidName
    );
    assert_eq!(
        kind(catalog.register_project(Uuid::new_v4(), "nul", path("nul"), Some(default))),
        CommandErrorKind::InvalidName
    );
    assert!(catalog.projects.is_empty());
}

#[test]
fn r10_nom_de_projet_normalise() {
    let (catalog, _) = with_workspace();

    assert_eq!(
        catalog
            .check_new_project("  boutique-tests ", &path("boutique-tests"), None)
            .unwrap(),
        "boutique-tests"
    );
}

// R11 - ordre des onglets et des projets

#[test]
fn r11_onglets_dans_l_ordre_de_creation() {
    let mut catalog = Catalog::first_launch();
    catalog.create_workspace("zeta").unwrap();
    catalog.create_workspace("Alpha").unwrap();
    let beta = catalog.create_workspace("beta").unwrap();

    assert_eq!(workspace_names(&catalog), ["zeta", "Alpha", "beta"]);

    catalog.rename_workspace(beta, "Aa").unwrap();
    assert_eq!(workspace_names(&catalog), ["zeta", "Alpha", "Aa"]);
}

#[test]
fn r11_workspaces_d_un_projet_dans_l_ordre_des_onglets() {
    let mut catalog = Catalog::first_launch();
    let zeta = catalog.create_workspace("zeta").unwrap();
    let alpha = catalog.create_workspace("Alpha").unwrap();
    let project = add_project(&mut catalog, "boutique", alpha);
    catalog.add_projects(zeta, &[project]).unwrap();

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects[0].workspace_ids, vec![zeta, alpha]);
}

#[test]
fn r11_projets_par_ordre_de_creation() {
    let (mut catalog, default) = with_workspace();
    for name in ["notifications", "api-facturation", "boutique-tests"] {
        add_project(&mut catalog, name, default);
    }

    let names: Vec<String> = catalog
        .snapshot(every_folder_exists)
        .projects
        .into_iter()
        .map(|project| project.name)
        .collect();

    assert_eq!(
        names,
        ["notifications", "api-facturation", "boutique-tests"]
    );
}

// W5, W8 - couleurs des workspaces

#[test]
fn w5_couleurs_attribuees_sans_doublon_puis_en_rotation() {
    let mut catalog = Catalog::first_launch();
    let colors: Vec<WorkspaceColor> = (1..=9)
        .map(|index| {
            let id = catalog.create_workspace(&format!("ws {index}")).unwrap();
            catalog.workspace(id).unwrap().color
        })
        .collect();

    use WorkspaceColor::*;
    assert_eq!(
        colors,
        [Orange, Blue, Green, Purple, Red, Yellow, Teal, Gray, Orange]
    );
}

#[test]
fn w8_changer_la_couleur() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    catalog
        .set_workspace_color(boutique, WorkspaceColor::Teal)
        .unwrap();

    assert_eq!(
        catalog.workspace(boutique).unwrap().color,
        WorkspaceColor::Teal
    );
    assert_eq!(
        kind(catalog.set_workspace_color(Uuid::new_v4(), WorkspaceColor::Red)),
        CommandErrorKind::NotFound
    );
}

// W6, P6 - ajouter des projets à un workspace

#[test]
fn w6_ajouter_des_projets_ignore_ceux_deja_presents() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let first = add_project(&mut catalog, "a", boutique);
    let second = add_project(&mut catalog, "b", default);

    catalog
        .add_projects(boutique, &[second, first, second])
        .unwrap();

    assert_eq!(project_ids_of(&catalog, boutique), vec![first, second]);
}

#[test]
fn w6_projet_inconnu_n_ajoute_rien() {
    let (mut catalog, default) = with_workspace();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let known = add_project(&mut catalog, "a", default);

    let error = catalog
        .add_projects(boutique, &[known, Uuid::new_v4()])
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert_eq!(
        error.message,
        "Ce projet n'est plus dans « Tous les projets »."
    );
    assert!(project_ids_of(&catalog, boutique).is_empty());
}

// P10 - projet introuvable

#[test]
fn p10_dossier_absent_marque_introuvable() {
    let (mut catalog, default) = with_workspace();
    add_project(&mut catalog, "present", default);
    add_project(&mut catalog, "absent", default);

    let snapshot = catalog.snapshot(|folder| folder.ends_with("present"));

    let statuses: Vec<ProjectStatus> = snapshot
        .projects
        .iter()
        .map(|project| project.status)
        .collect();
    assert_eq!(statuses, [ProjectStatus::Ok, ProjectStatus::Missing]);
}

#[test]
fn p10_etats_identiques_cote_typescript() {
    let rust: std::collections::BTreeSet<String> = [ProjectStatus::Ok, ProjectStatus::Missing]
        .into_iter()
        .map(crate::ts_contract::serialized_name)
        .collect();
    assert_eq!(rust, crate::ts_contract::union_members("ProjectStatus"));
}

// Format disque

#[test]
fn aller_retour_par_le_format_disque() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    add_project(&mut catalog, "boutique-tests", boutique);
    add_project_to(&mut catalog, "sans-workspace", None);
    catalog.open_workspace(Some(boutique)).unwrap();

    let file = catalog.to_file();

    assert_eq!(file.version, FILE_VERSION);
    assert_eq!(Catalog::from_file(file).unwrap(), catalog);
}

#[test]
fn fichier_repare_les_references_orphelines_et_doublons() {
    let (mut catalog, default) = with_workspace();
    let project = add_project(&mut catalog, "boutique-tests", default);
    let mut file = catalog.to_file();
    file.workspaces[0].project_ids = vec![Uuid::new_v4(), project, project];

    let repaired = Catalog::from_file(file).unwrap();

    assert_eq!(project_ids_of(&repaired, default), vec![project]);
}

#[test]
fn fichier_avec_identifiants_en_double_refuse() {
    let (mut catalog, default) = with_workspace();
    add_project(&mut catalog, "boutique-tests", default);
    let mut file = catalog.to_file();
    file.projects.push(file.projects[0].clone());
    assert!(Catalog::from_file(file).unwrap_err().contains("en double"));

    let mut file = with_workspace().0.to_file();
    file.workspaces.push(file.workspaces[0].clone());
    assert!(Catalog::from_file(file).unwrap_err().contains("en double"));
}

// R12 - migration du format version 1

fn file_v1(default_projects: Vec<Uuid>, last: Uuid) -> (CatalogFileV1, Uuid, Uuid) {
    let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
    let (mes, boutique) = (Uuid::from_u128(10), Uuid::from_u128(11));
    let project = |id: Uuid, name: &str| Project {
        id,
        name: name.to_owned(),
        path: path(name),
    };
    let file = CatalogFileV1 {
        projects: vec![project(a, "a"), project(b, "b")],
        workspaces: vec![
            Workspace {
                id: mes,
                name: "Mes projets".to_owned(),
                color: WorkspaceColor::Gray,
                project_ids: default_projects,
            },
            Workspace {
                id: boutique,
                name: "Boutique".to_owned(),
                color: WorkspaceColor::Orange,
                project_ids: vec![a],
            },
        ],
        default_workspace_id: mes,
        last_workspace_id: if last == Uuid::nil() { mes } else { last },
    };
    (file, mes, boutique)
}

#[test]
fn r12_le_workspace_par_defaut_complet_disparait() {
    let (file, _, boutique) = file_v1(vec![Uuid::from_u128(1), Uuid::from_u128(2)], Uuid::nil());

    let migrated = Catalog::migrate_v1(file);

    assert_eq!(migrated.version, FILE_VERSION);
    assert_eq!(migrated.projects.len(), 2);
    assert_eq!(
        migrated.workspaces.iter().map(|w| w.id).collect::<Vec<_>>(),
        [boutique]
    );
    assert_eq!(migrated.last_workspace_id, None);
}

#[test]
fn r12_le_workspace_par_defaut_partiel_reste() {
    let (file, mes, boutique) = file_v1(vec![Uuid::from_u128(2)], Uuid::from_u128(11));

    let migrated = Catalog::migrate_v1(file);

    assert_eq!(
        migrated.workspaces.iter().map(|w| w.id).collect::<Vec<_>>(),
        [mes, boutique]
    );
    assert_eq!(migrated.last_workspace_id, Some(boutique));
}

// Forme JSON de l'instantané (miroir TypeScript au lot C)

#[test]
fn instantane_serialise_en_camel_case() {
    let boutique = Uuid::from_u128(2);
    let project = Uuid::from_u128(3);
    let catalog = Catalog::from_file(CatalogFile {
        version: FILE_VERSION,
        projects: vec![Project {
            id: project,
            name: "boutique-tests".to_owned(),
            path: PathBuf::from("/dev/boutique-tests"),
        }],
        workspaces: vec![Workspace {
            id: boutique,
            name: "Boutique".to_owned(),
            color: WorkspaceColor::Orange,
            project_ids: vec![project],
        }],
        last_workspace_id: None,
        inbox_project_id: None,
    })
    .unwrap();

    let json = serde_json::to_value(catalog.snapshot(|_| false)).unwrap();

    let id = |n: u128| Uuid::from_u128(n).to_string();
    assert_eq!(
        json,
        serde_json::json!({
            "projects": [{
                "id": id(3),
                "name": "boutique-tests",
                "path": PathBuf::from("/dev/boutique-tests").display().to_string(),
                "status": "missing",
                "workspaceIds": [id(2)]
            }],
            "workspaces": [
                { "id": id(2), "name": "Boutique", "color": "orange", "projectIds": [id(3)] }
            ],
            "lastWorkspaceId": null,
            "inboxProjectId": null
        })
    );
}

// « Non classés » (docs/plan-accueil-non-classes.md, R4, R5)

fn with_inbox() -> (Catalog, Uuid, Uuid, Uuid) {
    let mut catalog = Catalog::first_launch();
    let boutique = Uuid::from_u128(10);
    let inbox = Uuid::from_u128(11);
    catalog
        .register_project(
            boutique,
            "boutique-tests",
            PathBuf::from("/p/boutique"),
            None,
        )
        .unwrap();
    let workspace = catalog.create_workspace("E-commerce").unwrap();
    catalog
        .register_project(
            inbox,
            "Non classés",
            PathBuf::from("/p/inbox"),
            Some(workspace),
        )
        .unwrap();
    catalog.set_inbox(inbox).unwrap();
    (catalog, boutique, inbox, workspace)
}

#[test]
fn r4_non_classes_retenu_et_relu() {
    let (catalog, _, inbox, _) = with_inbox();
    assert_eq!(catalog.inbox_project_id(), Some(inbox));
    let file = catalog.to_file();
    assert_eq!(file.inbox_project_id, Some(inbox));
    let json = serde_json::to_value(&file).unwrap();
    assert_eq!(json["inboxProjectId"], inbox.to_string());
    let reread = Catalog::from_file(file).unwrap();
    assert_eq!(reread.inbox_project_id(), Some(inbox));
}

#[test]
fn r4_champ_absent_ou_projet_inconnu_ignore() {
    let (catalog, _, _, _) = with_inbox();
    let mut file = catalog.to_file();
    file.inbox_project_id = Some(Uuid::from_u128(99));
    assert_eq!(Catalog::from_file(file).unwrap().inbox_project_id(), None);
    let json = serde_json::to_value(Catalog::first_launch().to_file()).unwrap();
    assert!(json.get("inboxProjectId").is_none(), "{json}");
}

#[test]
fn r5_non_classes_hors_des_workspaces() {
    let (mut catalog, _, inbox, workspace) = with_inbox();
    assert!(catalog.workspace(workspace).unwrap().project_ids.is_empty());
    let error = catalog.add_projects(workspace, &[inbox]).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert!(catalog.workspace(workspace).unwrap().project_ids.is_empty());
}

#[test]
fn r5_non_classes_en_tete_de_tous_les_projets() {
    let (catalog, boutique, inbox, _) = with_inbox();
    let snapshot = catalog.snapshot(|_| true);
    let ids: Vec<Uuid> = snapshot.projects.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![inbox, boutique]);
    assert_eq!(snapshot.inbox_project_id, Some(inbox));
}

#[test]
fn r5_non_classes_oublie_puis_restaure() {
    let (mut catalog, _, inbox, _) = with_inbox();
    let undo = catalog.forget_project(inbox).unwrap();
    assert_eq!(catalog.inbox_project_id(), None);
    assert_eq!(catalog.snapshot(|_| true).inbox_project_id, None);
    catalog.undo(undo).unwrap();
    assert_eq!(catalog.inbox_project_id(), Some(inbox));
}
