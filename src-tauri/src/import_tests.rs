//! Import d'une collection Bruno, d'un projet et d'un contrat, sur de vrais
//! dossiers temporaires.

use std::cell::RefCell;
use std::collections::BTreeMap;

use tempfile::TempDir;

use super::*;
use crate::bruno::ItemKind;
use crate::request::model::{AuthKind, Method};

struct Fixture {
    dir: TempDir,
    state: AppState,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::load(dir.path().join("data"), dir.path().join("home/arnod"));
    Fixture { dir, state }
}

fn source() -> PathBuf {
    crate::bruno::tests::fixture()
}

/// Contenu de tous les fichiers d'un dossier, pour vérifier qu'il n'a pas changé.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(snapshot(&path));
        } else {
            out.insert(path.clone(), fs::read(&path).unwrap());
        }
    }
    out
}

fn new_project() -> ImportTarget {
    ImportTarget::NewProject {
        location: None,
        workspace_id: None,
    }
}

fn no_store(_: &ProjectCtx, _: &SecretValue) -> Result<(), CommandError> {
    Ok(())
}

#[test]
fn reconnait_une_collection_bruno() {
    let detected = detect(&source()).unwrap();
    assert_eq!(detected.format, SourceFormat::Bruno);
    assert_eq!(detected.name, "Boutique API");
    assert_eq!(
        (
            detected.counts.requests,
            detected.counts.folders,
            detected.counts.environments,
            detected.files
        ),
        (6, 2, 2, 12)
    );
    // bruno.json ou une requête choisis : la collection.
    assert_eq!(
        detect(&source().join("bruno.json")).unwrap().path,
        detected.path
    );
    assert_eq!(
        detect(&source().join("Commandes/Lire une commande.bru"))
            .unwrap()
            .path,
        detected.path
    );
    let json = serde_json::to_value(&detected).unwrap();
    assert_eq!(json["format"], "bruno");
    assert_eq!(json["counts"]["environments"], 2);
}

#[test]
fn reconnait_un_projet_un_contrat_ou_refuse() {
    let f = fixture();
    let created = f.state.create_project("boutique", None, None).unwrap();
    let path = f.state.snapshot().unwrap().projects[0].path.clone();
    let detected = detect(Path::new(&path)).unwrap();
    assert_eq!(detected.format, SourceFormat::OpenCollection);
    assert_eq!(detected.name, "boutique");
    assert!(created.project_id != Uuid::nil());

    let contract = f.dir.path().join("petstore.yaml");
    fs::write(
        &contract,
        "openapi: 3.0.3\ninfo:\n  title: Petstore\n  version: '1'\npaths:\n  /pets:\n    get:\n      responses:\n        '200':\n          description: ok\n",
    )
    .unwrap();
    let detected = detect(&contract).unwrap();
    assert_eq!(detected.format, SourceFormat::OpenApi);
    assert_eq!(
        (detected.name.as_str(), detected.counts.requests),
        ("Petstore", 1)
    );

    let other = f.dir.path().join("notes.txt");
    fs::write(&other, "rien").unwrap();
    assert_eq!(
        detect(&other).unwrap_err().kind,
        CommandErrorKind::NotAProject
    );
    assert_eq!(
        detect(f.dir.path()).unwrap_err().kind,
        CommandErrorKind::NotAProject
    );
    assert_eq!(
        detect(Path::new("relatif")).unwrap_err().kind,
        CommandErrorKind::Io
    );
}

#[test]
fn nouveau_projet_depuis_bruno_sans_toucher_aux_fichiers_d_origine() {
    let f = fixture();
    let before = snapshot(&source());
    let stored = RefCell::new(Vec::new());
    let done = run(&f.state, &source(), &new_project(), &|_, secret| {
        stored.borrow_mut().push(secret.clone());
        Ok(())
    })
    .unwrap();
    assert_eq!(
        snapshot(&source()),
        before,
        "les fichiers .bru ne changent pas (R1)"
    );

    let snapshot = done.snapshot.unwrap();
    let project = snapshot
        .projects
        .iter()
        .find(|p| p.id == done.project_id)
        .unwrap();
    assert_eq!(project.name, "Boutique API");
    let root = PathBuf::from(&project.path);
    assert!(root.ends_with("home/arnod/Boutique API"));

    // Racine : réglages de la collection.
    let value = collection::read(&root).unwrap();
    assert_eq!(value["request"]["auth"]["type"], "bearer");
    assert_eq!(
        collection::meta(&value).environment_order,
        ["Local", "Recette"]
    );

    // Arnod lit les requêtes produites.
    let creer = crate::request::file::read(&root.join("Commandes/creer-une-commande.yml")).unwrap();
    assert_eq!(creer.method, Method::Post);
    assert_eq!(creer.assertions.len(), 6);
    assert_eq!(creer.auth.kind, AuthKind::Inherit);
    let tree = crate::tree::read(&root).unwrap();
    assert_eq!(tree.len(), 3, "deux dossiers et Santé");

    // Héritage : le dossier hérite, la collection donne le porteur.
    let inherited = crate::request::inherit::resolve(&root, "Commandes/creer-une-commande.yml");
    assert_eq!(inherited.auth.unwrap().auth.kind, AuthKind::Bearer);
    assert_eq!(inherited.headers[0].name, "x-tenant");

    // Environnements : secrets en variables locales, valeur de .env au coffre.
    let envs = environment::list(&root);
    let names: Vec<&str> = envs.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Local", "Recette"]);
    assert_eq!(envs[1].locals, ["partnerKey", "token", "password"]);
    let stored = stored.into_inner();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].environment, "Recette");
    let all = super::tests::snapshot(&root);
    assert!(
        all.values()
            .all(|bytes| !String::from_utf8_lossy(bytes).contains("ne-pas-lire")),
        "aucun secret dans le projet"
    );

    // Rapport gardé sur le poste.
    let report = done.report.unwrap();
    assert_eq!(report.counts.requests, 6);
    assert_eq!(report.source_files, 12);
    assert_eq!(report.folder, None);
    assert_eq!(
        load_report(f.state.data_dir(), done.project_id),
        Some(report.clone())
    );
    assert!(!root.join(REPORT_FILE).exists());
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["format"], "bruno");
    assert!(json.get("importedAt").is_some());
    assert!(json.get("sourceFiles").is_some());
}

#[test]
fn secret_non_range_reste_a_renseigner() {
    let f = fixture();
    let done = run(&f.state, &source(), &new_project(), &|_, _| {
        Err(CommandError::new(
            CommandErrorKind::SecretStore,
            "Le coffre du système est indisponible.",
        ))
    })
    .unwrap();
    let report = done.report.unwrap();
    let item = report
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Env && i.name == "partnerKey")
        .unwrap();
    assert_eq!(item.status, ItemStatus::ToFill);
    assert!(item.detail.contains("coffre"), "{}", item.detail);
}

#[test]
fn nom_deja_pris_rien_n_est_ecrit() {
    let f = fixture();
    f.state.create_project("boutique api", None, None).unwrap();
    let error = run(&f.state, &source(), &new_project(), &no_store).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NameTaken);
    assert_eq!(f.state.snapshot().unwrap().projects.len(), 1);
}

#[test]
fn dans_un_projet_existant_un_dossier_et_des_environnements_completes() {
    let f = fixture();
    let created = f.state.create_project("boutique", None, None).unwrap();
    let target = ImportTarget::Project {
        project_id: created.project_id,
    };
    let done = run(&f.state, &source(), &target, &no_store).unwrap();
    assert!(done.snapshot.is_none());
    let report = done.report.unwrap();
    assert_eq!(report.folder.as_deref(), Some("Boutique API"));
    let tests = report
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Tests)
        .unwrap();
    assert_eq!(
        tests.request.as_deref(),
        Some("Boutique API/Commandes/creer-une-commande.yml")
    );
    let root = f.state.project_ctx(created.project_id).unwrap().root;
    let folder = yaml::read(&root.join("Boutique API/folder.yml")).unwrap();
    assert_eq!(folder["info"]["type"], "folder");
    assert_eq!(folder["request"]["auth"]["type"], "bearer");
    let inherited =
        crate::request::inherit::resolve(&root, "Boutique API/Commandes/lire-une-commande.yml");
    assert_eq!(inherited.auth.unwrap().auth.kind, AuthKind::Bearer);

    // « Local » existait : baseUrl garde sa valeur ; « Recette » est créé.
    let envs = environment::list(&root);
    assert_eq!(envs.len(), 2);
    assert_eq!(envs[0].variables[0].value, "");
    assert_eq!(envs[1].name, "Recette");

    // Un deuxième import ne remplace rien : un autre dossier.
    let again = run(&f.state, &source(), &target, &no_store).unwrap();
    assert_eq!(
        again.report.unwrap().folder.as_deref(),
        Some("Boutique API 2")
    );
}

#[test]
fn projet_existant_ajoute_tel_quel() {
    let f = fixture();
    let created = f.state.create_project("existant", None, None).unwrap();
    let path = f.state.project_ctx(created.project_id).unwrap().root;
    f.state
        .mutate(|c| c.forget_project(created.project_id).map(|_| ()))
        .unwrap();
    let done = run(&f.state, &path, &new_project(), &no_store).unwrap();
    assert_eq!(done.project_id, created.project_id);
    assert!(done.report.is_none());
}

#[test]
fn contrat_openapi_dans_un_nouveau_projet() {
    let f = fixture();
    let contract = f.dir.path().join("petstore.yaml");
    fs::write(
        &contract,
        "openapi: 3.0.3\ninfo:\n  title: Petstore\n  version: '1'\npaths: {}\n",
    )
    .unwrap();
    let done = run(&f.state, &contract, &new_project(), &no_store).unwrap();
    assert_eq!(done.contract.as_deref(), Some("contracts/petstore.yaml"));
    let root = f.state.project_ctx(done.project_id).unwrap().root;
    assert!(root.join("contracts/petstore.yaml").is_file());
    assert_eq!(environment::list(&root)[0].name, "Local");
}

#[test]
fn rapport_absent_ou_illisible() {
    let f = fixture();
    let id = Uuid::new_v4();
    assert_eq!(load_report(f.state.data_dir(), id), None);
    let dir = crate::local::project_dir(f.state.data_dir(), id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(REPORT_FILE), "{").unwrap();
    assert_eq!(load_report(f.state.data_dir(), id), None);
}

#[test]
fn cible_serialisee() {
    let target: ImportTarget = serde_json::from_value(serde_json::json!({
        "kind": "newProject", "location": null, "workspaceId": null
    }))
    .unwrap();
    assert_eq!(target, new_project());
    let id = Uuid::new_v4();
    let target: ImportTarget =
        serde_json::from_value(serde_json::json!({ "kind": "project", "projectId": id })).unwrap();
    assert_eq!(target, ImportTarget::Project { project_id: id });
}
