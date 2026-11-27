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
    let done = run(
        &f.state,
        &source(),
        &new_project(),
        &ImportOptions::default(),
        &|_, secret| {
            stored.borrow_mut().push(secret.clone());
            Ok(())
        },
    )
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
    let done = run(
        &f.state,
        &source(),
        &new_project(),
        &ImportOptions::default(),
        &|_, _| {
            Err(CommandError::new(
                CommandErrorKind::SecretStore,
                "Le coffre du système est indisponible.",
            ))
        },
    )
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
    let error = run(
        &f.state,
        &source(),
        &new_project(),
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap_err();
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
    let done = run(
        &f.state,
        &source(),
        &target,
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
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
    let again = run(
        &f.state,
        &source(),
        &target,
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
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
    let done = run(
        &f.state,
        &path,
        &new_project(),
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
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
    let done = run(
        &f.state,
        &contract,
        &new_project(),
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
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

#[test]
fn coffre_en_echec_sans_message_du_systeme() {
    let f = fixture();
    let done = run(
        &f.state,
        &source(),
        &new_project(),
        &ImportOptions::default(),
        &|_, _| {
            Err(CommandError::new(
                CommandErrorKind::SecretStore,
                "Platform secure storage failure",
            ))
        },
    )
    .unwrap();
    let report = done.report.unwrap();
    let item = report
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Env && i.name == "partnerKey")
        .unwrap();
    assert!(!item.detail.contains("Platform"), "{}", item.detail);
    assert!(!item.detail.contains("à renseigner"), "{}", item.detail);
}

#[test]
fn nom_qui_ressemble_a_un_secret() {
    for name in [
        "token",
        "accessToken",
        "password",
        "db_password",
        "apiKey",
        "API_KEY",
        "clientSecret",
        "pwd",
        "key",
    ] {
        assert!(looks_secret(name), "{name}");
    }
    for name in [
        "baseUrl",
        "tokenUrl",
        "token_endpoint",
        "passwordPolicy",
        "partnerKeyName",
        "monkey",
        "userId",
    ] {
        assert!(!looks_secret(name), "{name}");
    }
}

/// Collection d'exemple convertie, avec deux variables partagées de plus dans « Recette » :
/// un jeton en clair et l'adresse du jeton.
fn converted_with_clear_token() -> Converted {
    let mut converted = convert::convert(&bruno::read(&source()).unwrap());
    let recette = converted
        .environments
        .iter_mut()
        .find(|env| env["name"] == "Recette")
        .unwrap();
    let variables = recette["variables"].as_array_mut().unwrap();
    variables.push(json!({ "name": "apiToken", "value": "s3cr3t" }));
    variables.push(json!({ "name": "tokenUrl", "value": "https://auth.boutique.dev/token" }));
    variables.push(json!({ "name": "refreshToken", "value": "{{token}}" }));
    converted
}

#[test]
fn source_decrit_environnements_et_secrets() {
    let detected = detect(&source()).unwrap();
    assert_eq!(
        detected.environments,
        vec![
            SourceEnvironment {
                name: "Local".into(),
                variables: vec!["baseUrl".into()],
            },
            SourceEnvironment {
                name: "Recette".into(),
                variables: vec![
                    "baseUrl".into(),
                    "partnerKey".into(),
                    "token".into(),
                    "password".into(),
                ],
            },
        ]
    );
    // Les secrets de Bruno sont déjà des variables locales : rien ne ressemble à un secret en clair.
    assert!(detected.secrets.is_empty());
    assert_eq!(
        secret_names(&converted_with_clear_token()),
        vec!["apiToken".to_owned()]
    );
}

#[test]
fn valeurs_qui_ressemblent_a_des_secrets_vont_au_coffre() {
    let mut converted = converted_with_clear_token();
    let before = converted.counts.secrets;
    apply_options(&mut converted, &ImportOptions::default());
    let recette = converted
        .environments
        .iter()
        .find(|env| env["name"] == "Recette")
        .unwrap();
    let token = recette["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "apiToken")
        .unwrap();
    assert_eq!(token, &json!({ "name": "apiToken", "secret": true }));
    assert!(
        recette["variables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "tokenUrl" && v["value"] == "https://auth.boutique.dev/token")
    );
    assert!(converted.secrets.contains(&SecretValue {
        environment: "Recette".into(),
        name: "apiToken".into(),
        value: "s3cr3t".into(),
    }));
    assert_eq!(converted.counts.secrets, before + 1);
    let item = converted
        .items
        .iter()
        .find(|i| i.name == "apiToken")
        .unwrap();
    assert_eq!(item.status, ItemStatus::Stored);
    assert_eq!(item.code, None);
    assert!(schema::check(Part::Environment, recette).is_ok());

    // Case décochée : la valeur reste partagée.
    let mut kept = converted_with_clear_token();
    apply_options(
        &mut kept,
        &ImportOptions {
            environments: Vec::new(),
            secrets_local: false,
        },
    );
    assert!(!kept.secrets.iter().any(|s| s.name == "apiToken"));
}

#[test]
fn environnements_vers_leur_cible() {
    let mut converted = converted_with_clear_token();
    apply_options(
        &mut converted,
        &ImportOptions {
            environments: vec![EnvironmentChoice {
                from: "Recette".into(),
                to: "Recette fonctionnelle".into(),
            }],
            secrets_local: true,
        },
    );
    let names: Vec<String> = converted.environments.iter().map(env_name).collect();
    assert_eq!(names, ["Local", "Recette fonctionnelle"]);
    assert!(
        converted
            .secrets
            .iter()
            .all(|s| s.environment == "Recette fonctionnelle")
    );
    assert!(
        converted
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Env)
            .all(|i| i.place == "Recette fonctionnelle")
    );

    // Deux environnements vers la même cible n'en font qu'un ; le premier garde ses valeurs.
    let mut merged = convert::convert(&bruno::read(&source()).unwrap());
    apply_options(
        &mut merged,
        &ImportOptions {
            environments: vec![EnvironmentChoice {
                from: "Local".into(),
                to: "Recette".into(),
            }],
            secrets_local: true,
        },
    );
    assert_eq!(merged.environments.len(), 1);
    let variables = merged.environments[0]["variables"].as_array().unwrap();
    assert_eq!(variables[0]["value"], "http://localhost:8080");
    assert!(variables.iter().any(|v| v["name"] == "token"));
}

#[test]
fn dans_un_projet_existant_l_environnement_choisi_est_complete() {
    let f = fixture();
    let created = f.state.create_project("boutique", None, None).unwrap();
    let target = ImportTarget::Project {
        project_id: created.project_id,
    };
    let stored = RefCell::new(Vec::new());
    run(
        &f.state,
        &source(),
        &target,
        &ImportOptions {
            environments: vec![EnvironmentChoice {
                from: "Recette".into(),
                to: "Local".into(),
            }],
            secrets_local: true,
        },
        &|_, secret| {
            stored.borrow_mut().push(secret.clone());
            Ok(())
        },
    )
    .unwrap();
    let root = f.state.project_ctx(created.project_id).unwrap().root;
    let envs = environment::list(&root);
    assert_eq!(envs.len(), 1, "aucun environnement « Recette » créé");
    assert_eq!(envs[0].name, "Local");
    assert_eq!(envs[0].locals, ["partnerKey", "token", "password"]);
    assert!(
        stored
            .borrow()
            .iter()
            .all(|secret| secret.environment == "Local")
    );
}

/// Nouveau projet importé de la collection d'exemple : racine et rapport.
fn imported() -> (Fixture, Uuid, PathBuf) {
    let f = fixture();
    let done = run(
        &f.state,
        &source(),
        &new_project(),
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
    let root = f.state.project_ctx(done.project_id).unwrap().root;
    (f, done.project_id, root)
}

fn item_index(f: &Fixture, project: Uuid, kind: ItemKind) -> usize {
    load_report(f.state.data_dir(), project)
        .unwrap()
        .items
        .iter()
        .position(|i| i.kind == kind)
        .unwrap()
}

fn scripts_of(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|h| h.get("scripts"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| s["type"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn accepter_sans_ce_script_le_retire_puis_annuler_le_remet() {
    let (f, project, root) = imported();
    let index = item_index(&f, project, ItemKind::Tests);
    let file = root.join("Commandes/creer-une-commande.yml");
    let before = fs::read_to_string(&file).unwrap();
    assert!(scripts_of(&yaml::read(&file).unwrap(), "runtime").contains(&"tests".to_owned()));

    let (report, undo) = resolve_script(
        f.state.data_dir(),
        &root,
        project,
        index,
        ScriptChoice::Accept,
    )
    .unwrap();
    assert_eq!(report.items[index].status, ItemStatus::Accepted);
    assert!(!scripts_of(&yaml::read(&file).unwrap(), "runtime").contains(&"tests".to_owned()));
    assert!(schema::check(Part::HttpRequest, &yaml::read(&file).unwrap()).is_ok());
    // Gardé sur le poste : le rapport relu dit « repris ».
    assert_eq!(
        load_report(f.state.data_dir(), project).unwrap().items[index].status,
        ItemStatus::Accepted
    );
    // Une deuxième fois : ce n'est plus un script à reprendre.
    assert_eq!(
        resolve_script(
            f.state.data_dir(),
            &root,
            project,
            index,
            ScriptChoice::Accept
        )
        .unwrap_err()
        .kind,
        CommandErrorKind::InvalidRequest
    );

    let back = undo_script(f.state.data_dir(), &undo).unwrap();
    assert_eq!(back.items[index].status, ItemStatus::NotRun);
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
}

#[test]
fn remplacer_un_script_de_dossier() {
    let (f, project, root) = imported();
    let index = item_index(&f, project, ItemKind::PreRequest);
    let folder = root.join("Commandes/folder.yml");
    assert_eq!(
        scripts_of(&yaml::read(&folder).unwrap(), "request"),
        ["before-request"]
    );
    let (report, _) = resolve_script(
        f.state.data_dir(),
        &root,
        project,
        index,
        ScriptChoice::Replace,
    )
    .unwrap();
    assert_eq!(report.items[index].status, ItemStatus::Replaced);
    let value = yaml::read(&folder).unwrap();
    assert!(scripts_of(&value, "request").is_empty());
    assert!(schema::check(Part::Folder, &value).is_ok());
}

#[test]
fn script_modifie_a_la_main_refuse_sans_rien_ecrire() {
    let (f, project, root) = imported();
    let index = item_index(&f, project, ItemKind::PostResponse);
    let report = load_report(f.state.data_dir(), project).unwrap();
    let file = root.join(report.items[index].request.as_deref().unwrap());
    let text = fs::read_to_string(&file)
        .unwrap()
        .replace("res.body.access_token", "res.body.token");
    fs::write(&file, &text).unwrap();
    let error = resolve_script(
        f.state.data_dir(),
        &root,
        project,
        index,
        ScriptChoice::Accept,
    )
    .unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::NotFound);
    assert_eq!(fs::read_to_string(&file).unwrap(), text);
    assert_eq!(
        load_report(f.state.data_dir(), project).unwrap().items[index].status,
        ItemStatus::NotRun
    );
}

#[test]
fn script_d_une_collection_importee_dans_un_projet() {
    let f = fixture();
    let created = f.state.create_project("boutique", None, None).unwrap();
    let target = ImportTarget::Project {
        project_id: created.project_id,
    };
    run(
        &f.state,
        &source(),
        &target,
        &ImportOptions::default(),
        &no_store,
    )
    .unwrap();
    let root = f.state.project_ctx(created.project_id).unwrap().root;
    for kind in [
        ItemKind::Tests,
        ItemKind::PreRequest,
        ItemKind::PostResponse,
    ] {
        let index = item_index(&f, created.project_id, kind);
        resolve_script(
            f.state.data_dir(),
            &root,
            created.project_id,
            index,
            ScriptChoice::Accept,
        )
        .unwrap();
    }
    assert!(
        scripts_of(
            &yaml::read(&root.join("Boutique API/Commandes/folder.yml")).unwrap(),
            "request"
        )
        .is_empty()
    );
}
