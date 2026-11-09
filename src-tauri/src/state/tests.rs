//! Tests de l'état partagé, sur de vrais dossiers temporaires.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::manifest::{MANIFEST_FILE_NAME, read_manifest};
use crate::storage::{STATE_FILE_NAME, state_file};

/// Dossier temporaire contenant `data/` (état) et `home/arnod/` (projets).
struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn data_dir(&self) -> PathBuf {
        self.root.path().join("data")
    }

    fn default_location(&self) -> PathBuf {
        self.root.path().join("home").join(DEFAULT_LOCATION_DIR)
    }

    fn load(&self) -> AppState {
        AppState::load(self.data_dir(), self.default_location())
    }
}

/// Workspace « Mes projets », créé au premier appel.
fn default_workspace(state: &AppState) -> Uuid {
    let snapshot = state.snapshot().unwrap();
    match snapshot.workspaces.iter().find(|w| w.name == "Mes projets") {
        Some(workspace) => workspace.id,
        None => {
            state
                .mutate(|catalog| catalog.create_workspace("Mes projets"))
                .unwrap()
                .0
        }
    }
}

fn kind<T: std::fmt::Debug>(result: Result<T, CommandError>) -> CommandErrorKind {
    result.unwrap_err().kind
}

#[test]
fn premier_lancement_enregistre_l_etat() {
    let fixture = Fixture::new();

    let state = fixture.load();
    let loaded = state.load_catalog().unwrap();

    assert!(state_file(&fixture.data_dir()).is_file());
    assert!(loaded.warning.is_none());
    assert!(loaded.snapshot.workspaces.is_empty());
    assert_eq!(loaded.snapshot.last_workspace_id, None);
    // Les identifiants survivent au redémarrage.
    assert_eq!(
        fixture.load().load_catalog().unwrap().snapshot,
        loaded.snapshot
    );
}

#[test]
fn creer_un_projet_cree_son_dossier_et_l_inscrit() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);

    let created = state
        .create_project(" boutique-tests ", None, Some(default))
        .unwrap();

    let folder = fixture.default_location().join("boutique-tests");
    let manifest = read_manifest(&folder).unwrap();
    assert_eq!(manifest.id, created.project_id);
    assert_eq!(manifest.name, "boutique-tests");
    let project = &created.snapshot.projects[0];
    assert_eq!(project.name, "boutique-tests");
    assert_eq!(project.status, ProjectStatus::Ok);
    assert_eq!(project.path, canonical(&folder).display().to_string());
    assert_eq!(project.workspace_ids, vec![default]);
}

#[test]
fn creer_un_projet_dans_un_autre_emplacement_et_workspace() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let (boutique, _) = state
        .mutate(|catalog| catalog.create_workspace("Boutique"))
        .unwrap();
    let location = fixture.root.path().join("dev");

    let created = state
        .create_project(
            "paiements-v3",
            Some(&location.display().to_string()),
            Some(boutique),
        )
        .unwrap();

    assert!(
        location
            .join("paiements-v3")
            .join(MANIFEST_FILE_NAME)
            .is_file()
    );
    let project = &created.snapshot.projects[0];
    assert_eq!(project.workspace_ids, vec![boutique]);
}

#[test]
fn le_projet_cree_survit_au_redemarrage() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let created = state
        .create_project("boutique-tests", None, Some(default_workspace(&state)))
        .unwrap();

    let reloaded = fixture.load().load_catalog().unwrap().snapshot;

    assert_eq!(reloaded, created.snapshot);
}

#[test]
fn nom_deja_pris_refuse_avant_de_toucher_au_disque() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);
    state
        .create_project("boutique", None, Some(default))
        .unwrap();
    let other_location = fixture.root.path().join("ailleurs");

    let error = state
        .create_project(
            "BOUTIQUE",
            Some(&other_location.display().to_string()),
            Some(default),
        )
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert!(!other_location.exists());
}

#[test]
fn nom_invalide_ou_emplacement_relatif_refuse() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);

    assert_eq!(
        kind(state.create_project("api/v2", None, Some(default))),
        CommandErrorKind::InvalidName
    );
    let error = state
        .create_project("boutique", Some("relatif/dev"), Some(default))
        .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::Io);
    assert!(
        error
            .message
            .starts_with("L'emplacement du projet doit être un chemin absolu")
    );
    assert!(!fixture.default_location().exists());
}

#[test]
fn dossier_non_vide_refuse_sans_inscription() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let folder = fixture.default_location().join("boutique");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("notes.txt"), "").unwrap();

    let error = state
        .create_project("boutique", None, Some(default_workspace(&state)))
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::FolderNotEmpty);
    assert!(state.load_catalog().unwrap().snapshot.projects.is_empty());
}

#[test]
fn echec_d_enregistrement_supprime_le_dossier_cree() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);
    // Un dossier à la place du fichier temporaire fait échouer l'écriture de l'état.
    fs::create_dir(fixture.data_dir().join(format!("{STATE_FILE_NAME}.tmp"))).unwrap();

    let error = state
        .create_project("boutique", None, Some(default))
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::Io);
    assert!(!fixture.default_location().join("boutique").exists());
    assert!(state.load_catalog().unwrap().snapshot.projects.is_empty());
}

#[test]
fn modification_refusee_ne_change_rien() {
    let fixture = Fixture::new();
    let state = fixture.load();
    default_workspace(&state);
    let before = state.load_catalog().unwrap().snapshot;

    let error = state
        .mutate(|catalog| catalog.create_workspace("mes projets"))
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(state.load_catalog().unwrap().snapshot, before);
}

#[test]
fn annuler_une_suppression_puis_jeton_expire() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let (boutique, before) = state
        .mutate(|catalog| catalog.create_workspace("Boutique"))
        .unwrap();

    let deleted = state
        .mutate_undoable(|catalog| catalog.delete_workspace(boutique))
        .unwrap();
    assert!(deleted.snapshot.workspaces.is_empty());

    let restored = state.undo(deleted.undo_token).unwrap();
    assert_eq!(restored, before);
    assert_eq!(fixture.load().load_catalog().unwrap().snapshot, before);
    assert_eq!(
        kind(state.undo(deleted.undo_token)),
        CommandErrorKind::UndoExpired
    );
}

#[test]
fn fichier_illisible_signale_une_seule_fois() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.data_dir()).unwrap();
    fs::write(state_file(&fixture.data_dir()), "pas du JSON").unwrap();

    let state = fixture.load();

    let warning = state.load_catalog().unwrap().warning.unwrap();
    assert_eq!(warning.kind, CommandErrorKind::CorruptState);
    assert!(
        warning
            .message
            .starts_with("L'état d'Arnod était illisible (JSON invalide")
    );
    assert!(state.load_catalog().unwrap().warning.is_none());
    // Un état neuf et valide a remplacé le fichier mis de côté.
    assert!(fixture.load().load_catalog().unwrap().warning.is_none());
}

#[test]
fn version_plus_recente_bloque_tout_sans_ecrire() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.data_dir()).unwrap();
    let content = r#"{ "version": 99 }"#;
    fs::write(state_file(&fixture.data_dir()), content).unwrap();

    let state = fixture.load();

    assert_eq!(kind(state.load_catalog()), CommandErrorKind::NewerVersion);
    assert_eq!(
        kind(state.mutate(|catalog| catalog.create_workspace("Boutique"))),
        CommandErrorKind::NewerVersion
    );
    assert_eq!(
        kind(state.create_project("boutique", None, Some(Uuid::new_v4()))),
        CommandErrorKind::NewerVersion
    );
    assert_eq!(
        fs::read_to_string(state_file(&fixture.data_dir())).unwrap(),
        content
    );
}

#[test]
fn ouvrir_un_projet_dont_le_dossier_a_disparu() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let created = state
        .create_project("boutique", None, Some(default_workspace(&state)))
        .unwrap();
    assert_eq!(
        state.open_project(created.project_id).unwrap().name,
        "boutique"
    );

    fs::remove_dir_all(fixture.default_location().join("boutique")).unwrap();

    let error = state.open_project(created.project_id).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert!(
        error
            .message
            .starts_with("Le dossier du projet « boutique » est introuvable")
    );
    assert_eq!(
        state.load_catalog().unwrap().snapshot.projects[0].status,
        ProjectStatus::Missing
    );
    assert_eq!(
        kind(state.open_project(Uuid::new_v4())),
        CommandErrorKind::NotFound
    );
}

#[test]
fn dossier_propose_pour_un_nom() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = fixture.default_location();

    assert_eq!(
        state.suggest_folder(" boutique-tests ", None),
        default.join("boutique-tests").display().to_string()
    );
    assert_eq!(
        state.suggest_folder("  ", None),
        default.display().to_string()
    );
    let elsewhere = fixture.root.path().join("dev");
    assert_eq!(
        state.suggest_folder("api", Some(&elsewhere.display().to_string())),
        elsewhere.join("api").display().to_string()
    );
}

/// Crée `name` dans un premier état, puis renvoie le dossier et l'identifiant.
fn existing_project(fixture: &Fixture, name: &str) -> (PathBuf, Uuid) {
    let state = fixture.load();
    let created = state
        .create_project(name, None, Some(default_workspace(&state)))
        .unwrap();
    (fixture.default_location().join(name), created.project_id)
}

#[test]
fn ajoute_un_dossier_projet_existant_avec_son_identifiant() {
    let origin = Fixture::new();
    let (folder, id) = existing_project(&origin, "boutique-tests");
    let fixture = Fixture::new();
    let state = fixture.load();
    let (boutique, _) = state
        .mutate(|catalog| catalog.create_workspace("Boutique"))
        .unwrap();

    let added = state
        .add_existing_project(&folder.display().to_string(), Some(boutique))
        .unwrap();

    assert_eq!(added.project_id, id);
    let project = &added.snapshot.projects[0];
    assert_eq!(project.name, "boutique-tests");
    assert_eq!(project.path, canonical(&folder).display().to_string());
    assert_eq!(project.workspace_ids, vec![boutique]);
    assert_eq!(read_manifest(&folder).unwrap().id, id, "manifeste intact");
}

#[test]
fn dossier_sans_manifeste_refuse() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let empty = fixture.root.path().join("vide");
    fs::create_dir_all(&empty).unwrap();

    let error = state
        .add_existing_project(
            &empty.display().to_string(),
            Some(default_workspace(&state)),
        )
        .unwrap_err();

    assert_eq!(error.kind, CommandErrorKind::NotAProject);
    assert!(state.load_catalog().unwrap().snapshot.projects.is_empty());
}

#[test]
fn projet_deja_inscrit_ou_nom_pris_refuse() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);
    state
        .create_project("boutique", None, Some(default))
        .unwrap();
    let folder = fixture.default_location().join("boutique");

    assert_eq!(
        kind(state.add_existing_project(&folder.display().to_string(), Some(default))),
        CommandErrorKind::AlreadyRegistered
    );

    let other = Fixture::new();
    let (homonym, _) = existing_project(&other, "BOUTIQUE");
    assert_eq!(
        kind(state.add_existing_project(&homonym.display().to_string(), Some(default))),
        CommandErrorKind::NameTaken
    );
    assert_eq!(state.load_catalog().unwrap().snapshot.projects.len(), 1);
}

#[test]
fn chemin_relatif_ou_manifeste_plus_recent_refuse() {
    let fixture = Fixture::new();
    let state = fixture.load();
    let default = default_workspace(&state);
    assert_eq!(
        kind(state.add_existing_project("relatif/boutique", Some(default))),
        CommandErrorKind::Io
    );

    let newer = fixture.root.path().join("futur");
    fs::create_dir_all(&newer).unwrap();
    fs::write(
        newer.join(MANIFEST_FILE_NAME),
        format!("version: 2\nid: {}\nname: futur\n", Uuid::new_v4()),
    )
    .unwrap();
    assert_eq!(
        kind(state.add_existing_project(&newer.display().to_string(), Some(default))),
        CommandErrorKind::NewerVersion
    );
}
