//! Tests du catalogue, regroupés par règle du plan (R1 à R11) et cas d'usage.

use super::*;

fn path(name: &str) -> PathBuf {
    PathBuf::from("/dev").join(name)
}

fn every_folder_exists(_: &Path) -> bool {
    true
}

/// Inscrit un projet de dossier `/dev/<name>` dans `workspace_id`.
fn add_project(catalog: &mut Catalog, name: &str, workspace_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    catalog
        .register_project(id, name, path(name), workspace_id)
        .unwrap();
    id
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

// W1 - premier lancement

#[test]
fn w1_premier_lancement_cree_mes_projets_par_defaut_et_ouvert() {
    let catalog = Catalog::first_launch();
    let snapshot = catalog.snapshot(every_folder_exists);

    assert!(snapshot.projects.is_empty());
    assert_eq!(snapshot.workspaces.len(), 1);
    let workspace = &snapshot.workspaces[0];
    assert_eq!(workspace.name, "Mes projets");
    assert_eq!(workspace.color, WorkspaceColor::Gray);
    assert!(workspace.is_default);
    assert!(workspace.project_ids.is_empty());
    assert_eq!(snapshot.default_workspace_id, workspace.id);
    assert_eq!(snapshot.last_workspace_id, workspace.id);
}

// R1 - un projet n'existe qu'une fois dans le référentiel

#[test]
fn r1_un_dossier_ne_peut_etre_inscrit_qu_une_fois() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    add_project(&mut catalog, "boutique", default);

    let error = catalog
        .register_project(Uuid::new_v4(), "autre", path("boutique"), default)
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
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    add_project(&mut catalog, "boutique", default);

    assert_eq!(
        kind(catalog.check_new_project("autre", Path::new("/dev//boutique/"), default)),
        CommandErrorKind::AlreadyRegistered
    );
}

#[test]
fn r1_un_identifiant_ne_peut_etre_inscrit_qu_une_fois() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let id = add_project(&mut catalog, "boutique", default);

    assert_eq!(
        kind(catalog.register_project(id, "autre", path("autre"), default)),
        CommandErrorKind::AlreadyRegistered
    );
}

// R2 - un projet appartient à zéro, un ou plusieurs workspaces

#[test]
fn r2_un_projet_dans_plusieurs_workspaces_puis_dans_aucun() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects[0].workspace_ids, vec![default, boutique]);

    catalog.remove_project(default, project).unwrap();
    catalog.remove_project(boutique, project).unwrap();

    let snapshot = catalog.snapshot(every_folder_exists);
    assert_eq!(snapshot.projects.len(), 1);
    assert!(snapshot.projects[0].workspace_ids.is_empty());
}

// R3 - exactement un workspace par défaut, non supprimable

#[test]
fn r3_le_workspace_par_defaut_ne_peut_pas_etre_supprime() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();

    let error = catalog.delete_workspace(default).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::DefaultWorkspace);
    assert!(catalog.workspace(default).is_ok());
}

#[test]
fn r3_le_role_par_defaut_se_transfere() {
    let mut catalog = Catalog::first_launch();
    let former = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    catalog.set_default_workspace(boutique).unwrap();

    let snapshot = catalog.snapshot(every_folder_exists);
    let defaults: Vec<&str> = snapshot
        .workspaces
        .iter()
        .filter(|workspace| workspace.is_default)
        .map(|workspace| workspace.name.as_str())
        .collect();
    assert_eq!(defaults, ["Boutique"]);
    assert!(catalog.delete_workspace(former).is_ok());
}

#[test]
fn r3_fichier_sans_workspace_par_defaut_refuse() {
    let mut file = Catalog::first_launch().to_file();
    file.default_workspace_id = Uuid::new_v4();

    assert_eq!(
        Catalog::from_file(file).unwrap_err(),
        "workspace par défaut absent"
    );
}

// R4 - tout nouveau projet va dans le workspace par défaut

#[test]
fn r4_nouveau_projet_dans_le_defaut_et_le_workspace_choisi() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    let project = add_project(&mut catalog, "paiements-v3", boutique);

    assert_eq!(project_ids_of(&catalog, default), vec![project]);
    assert_eq!(project_ids_of(&catalog, boutique), vec![project]);
}

#[test]
fn r4_nouveau_projet_dans_le_defaut_une_seule_fois() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();

    let project = add_project(&mut catalog, "boutique-tests", default);

    assert_eq!(project_ids_of(&catalog, default), vec![project]);
}

#[test]
fn r4_workspace_inconnu_refuse_sans_rien_inscrire() {
    let mut catalog = Catalog::first_launch();

    assert_eq!(
        kind(catalog.register_project(
            Uuid::new_v4(),
            "boutique",
            path("boutique"),
            Uuid::new_v4()
        )),
        CommandErrorKind::NotFound
    );
    assert!(catalog.projects.is_empty());
}

// R5 - retirer ou supprimer ne supprime jamais un projet

#[test]
fn r5_retirer_un_projet_le_garde_au_referentiel() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let project = add_project(&mut catalog, "boutique-tests", default);

    catalog.remove_project(default, project).unwrap();

    assert!(project_ids_of(&catalog, default).is_empty());
    assert!(catalog.project(project).is_ok());
}

#[test]
fn r5_supprimer_un_workspace_garde_ses_projets_ailleurs() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", boutique);

    catalog.delete_workspace(boutique).unwrap();

    assert!(catalog.workspace(boutique).is_err());
    assert!(catalog.project(project).is_ok());
    assert_eq!(project_ids_of(&catalog, default), vec![project]);
}

#[test]
fn r5_retirer_un_projet_absent_du_workspace() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let project = add_project(&mut catalog, "boutique-tests", default);

    let error = catalog.remove_project(boutique, project).unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert_eq!(error.message, "Ce projet n'est pas dans ce workspace.");
}

// R6 - « Annuler » au lieu d'une confirmation

#[test]
fn r6_annuler_le_retrait_remet_le_projet_a_sa_place() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
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
    catalog.open_workspace(boutique).unwrap();
    let before = catalog.clone();

    let action = catalog.delete_workspace(boutique).unwrap();
    catalog.undo(action.clone()).unwrap();

    assert_eq!(catalog, before);
    assert_eq!(project_ids_of(&catalog, boutique), vec![project]);
    assert_eq!(catalog.last_workspace_id(), boutique);

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

// R7 - rouvrir le dernier workspace utilisé

#[test]
fn r7_le_workspace_ouvert_est_memorise() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();

    catalog.open_workspace(boutique).unwrap();

    assert_eq!(catalog.last_workspace_id(), boutique);
    assert_eq!(catalog.to_file().last_workspace_id, boutique);
    assert_eq!(
        kind(catalog.open_workspace(Uuid::new_v4())),
        CommandErrorKind::NotFound
    );
}

#[test]
fn r7_dernier_workspace_disparu_remplace_par_le_defaut() {
    let mut file = Catalog::first_launch().to_file();
    file.last_workspace_id = Uuid::new_v4();

    let catalog = Catalog::from_file(file).unwrap();

    assert_eq!(catalog.last_workspace_id(), catalog.default_workspace_id());
}

// R8 - supprimer le workspace ouvert rouvre le workspace par défaut

#[test]
fn r8_supprimer_le_workspace_ouvert_rouvre_le_defaut() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    catalog.open_workspace(boutique).unwrap();

    catalog.delete_workspace(boutique).unwrap();

    assert_eq!(catalog.last_workspace_id(), catalog.default_workspace_id());
}

#[test]
fn r8_supprimer_un_autre_workspace_ne_change_pas_le_workspace_ouvert() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let recette = catalog.create_workspace("Recette").unwrap();
    catalog.open_workspace(boutique).unwrap();

    catalog.delete_workspace(recette).unwrap();

    assert_eq!(catalog.last_workspace_id(), boutique);
}

// R9 - noms valides et uniques sans tenir compte de la casse

#[test]
fn r9_noms_de_workspace_uniques_sans_tenir_compte_de_la_casse() {
    let mut catalog = Catalog::first_launch();
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
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();

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
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    add_project(&mut catalog, "boutique-tests", default);

    let error = catalog
        .check_new_project("Boutique-Tests", &path("ailleurs"), default)
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(
        error.message,
        "Un projet « boutique-tests » existe déjà dans le référentiel."
    );
}

// R10 - le nom d'un projet est un nom de dossier valide

#[test]
fn r10_nom_de_projet_invalide_refuse() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();

    assert_eq!(
        kind(catalog.check_new_project("api/v2", &path("api"), default)),
        CommandErrorKind::InvalidName
    );
    assert_eq!(
        kind(catalog.register_project(Uuid::new_v4(), "nul", path("nul"), default)),
        CommandErrorKind::InvalidName
    );
    assert!(catalog.projects.is_empty());
}

#[test]
fn r10_nom_de_projet_normalise() {
    let catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();

    assert_eq!(
        catalog
            .check_new_project("  boutique-tests ", &path("boutique-tests"), default)
            .unwrap(),
        "boutique-tests"
    );
}

// R11 - ordre d'affichage

#[test]
fn r11_workspaces_par_defaut_puis_par_nom() {
    let mut catalog = Catalog::first_launch();
    let zeta = catalog.create_workspace("zeta").unwrap();
    catalog.create_workspace("Alpha").unwrap();
    catalog.create_workspace("beta").unwrap();

    assert_eq!(
        workspace_names(&catalog),
        ["Mes projets", "Alpha", "beta", "zeta"]
    );

    catalog.set_default_workspace(zeta).unwrap();
    assert_eq!(
        workspace_names(&catalog),
        ["zeta", "Alpha", "beta", "Mes projets"]
    );
}

#[test]
fn r11_projets_par_ordre_de_creation() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
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
    let colors: Vec<WorkspaceColor> = (1..=8)
        .map(|index| {
            let id = catalog.create_workspace(&format!("ws {index}")).unwrap();
            catalog.workspace(id).unwrap().color
        })
        .collect();

    use WorkspaceColor::*;
    assert_eq!(
        colors,
        [Orange, Blue, Green, Purple, Red, Yellow, Teal, Orange]
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
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
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
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    let known = add_project(&mut catalog, "a", default);

    let error = catalog
        .add_projects(boutique, &[known, Uuid::new_v4()])
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert_eq!(
        error.message,
        "Ce projet n'existe plus dans le référentiel."
    );
    assert!(project_ids_of(&catalog, boutique).is_empty());
}

// P10 - projet introuvable

#[test]
fn p10_dossier_absent_marque_introuvable() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
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

// Format disque

#[test]
fn aller_retour_par_le_format_disque() {
    let mut catalog = Catalog::first_launch();
    let boutique = catalog.create_workspace("Boutique").unwrap();
    add_project(&mut catalog, "boutique-tests", boutique);
    catalog.open_workspace(boutique).unwrap();

    let file = catalog.to_file();

    assert_eq!(file.version, FILE_VERSION);
    assert_eq!(Catalog::from_file(file).unwrap(), catalog);
}

#[test]
fn fichier_repare_les_references_orphelines_et_doublons() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    let project = add_project(&mut catalog, "boutique-tests", default);
    let mut file = catalog.to_file();
    file.workspaces[0].project_ids = vec![Uuid::new_v4(), project, project];

    let repaired = Catalog::from_file(file).unwrap();

    assert_eq!(project_ids_of(&repaired, default), vec![project]);
}

#[test]
fn fichier_avec_identifiants_en_double_refuse() {
    let mut catalog = Catalog::first_launch();
    let default = catalog.default_workspace_id();
    add_project(&mut catalog, "boutique-tests", default);
    let mut file = catalog.to_file();
    file.projects.push(file.projects[0].clone());
    assert!(Catalog::from_file(file).unwrap_err().contains("en double"));

    let mut file = Catalog::first_launch().to_file();
    file.workspaces.push(file.workspaces[0].clone());
    assert!(Catalog::from_file(file).unwrap_err().contains("en double"));
}

// Forme JSON de l'instantané (miroir TypeScript au lot C)

#[test]
fn instantane_serialise_en_camel_case() {
    let default = Uuid::from_u128(1);
    let boutique = Uuid::from_u128(2);
    let project = Uuid::from_u128(3);
    let catalog = Catalog::from_file(CatalogFile {
        version: FILE_VERSION,
        projects: vec![Project {
            id: project,
            name: "boutique-tests".to_owned(),
            path: PathBuf::from("/dev/boutique-tests"),
        }],
        workspaces: vec![
            Workspace {
                id: default,
                name: "Mes projets".to_owned(),
                color: WorkspaceColor::Gray,
                project_ids: vec![project],
            },
            Workspace {
                id: boutique,
                name: "Boutique".to_owned(),
                color: WorkspaceColor::Orange,
                project_ids: vec![project],
            },
        ],
        default_workspace_id: default,
        last_workspace_id: boutique,
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
                "workspaceIds": [id(1), id(2)]
            }],
            "workspaces": [
                { "id": id(1), "name": "Mes projets", "color": "gray", "isDefault": true, "projectIds": [id(3)] },
                { "id": id(2), "name": "Boutique", "color": "orange", "isDefault": false, "projectIds": [id(3)] }
            ],
            "defaultWorkspaceId": id(1),
            "lastWorkspaceId": id(2)
        })
    );
}
