//! Règles du lot I, une ou plusieurs par test, sur de vrais dossiers temporaires.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uuid::Uuid;

use super::conflict::{self, ConflictSide};
use super::service::{Sink, Watchers};
use super::*;
use crate::error::CommandErrorKind;
use crate::request::file;
use crate::request::model::{
    Assertion, Body, BodyKind, Method, Operator, Origin, Pair, Request, Scalar,
};

/// Projet suivi dans un dossier temporaire, oublié à la fin du test.
struct Project {
    id: Uuid,
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("commandes")).unwrap();
        file::write(
            &dir.path().join("commandes/creer.yml"),
            &sample("Créer une commande"),
        )
        .unwrap();
        let id = Uuid::new_v4();
        add_root(id, dir.path());
        Self { id, dir }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn refresh(&self, relative: &[&str]) -> FilesChange {
        let touched: Vec<PathBuf> = relative.iter().map(|r| self.path(r)).collect();
        classify(self.id, &refresh(self.id, &touched).unwrap())
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        remove_root(self.id);
    }
}

fn sample(name: &str) -> Request {
    let mut request = Request::new(name);
    request.method = Method::Post;
    request.url = "{{baseUrl}}/orders".to_owned();
    request
}

/// Réécrit un fichier comme le ferait un éditeur ou Git, hors d'Arnod.
fn external_edit(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{text}");
    fs::write(path, text.replace(from, to)).unwrap();
}

// --- R1 : éléments ignorés -------------------------------------------------------------

#[test]
fn r1_git_caches_et_temporaires_ignores() {
    assert!(ignored(Path::new(".git/index")));
    assert!(ignored(Path::new("commandes/.creer.yml.swp")));
    assert!(ignored(Path::new("commandes/creer.yml.tmp")));
    assert!(ignored(Path::new("contracts/api.tmp")));
    assert!(ignored(Path::new("commandes/creer.yml~")));
    assert!(!ignored(Path::new("commandes/creer.yml")));
    assert!(!ignored(Path::new("environments/local.yml")));
}

#[test]
fn r1_un_changement_dans_git_ne_donne_rien() {
    let project = Project::new();
    fs::create_dir(project.path(".git")).unwrap();
    fs::write(project.path(".git/HEAD"), "ref: refs/heads/main").unwrap();
    fs::write(project.path("commandes/creer.yml.tmp"), "x").unwrap();
    let change = project.refresh(&[".git/HEAD", ".git", "commandes/creer.yml.tmp"]);
    assert!(change.is_empty(), "{change:?}");
}

// --- R2 : écho des écritures d'Arnod ---------------------------------------------------

#[test]
fn r2_une_ecriture_d_arnod_n_est_pas_un_changement() {
    let project = Project::new();
    let mut request = sample("Créer une commande");
    request.url = "{{baseUrl}}/v2/orders".to_owned();
    file::write(&project.path("commandes/creer.yml"), &request).unwrap();
    let change = project.refresh(&["commandes/creer.yml", "commandes"]);
    assert!(change.is_empty(), "{change:?}");
}

#[test]
fn r2_les_operations_de_l_arborescence_ne_sont_pas_des_changements() {
    let project = Project::new();
    let root = project.dir.path();
    let folder = crate::tree::create_folder(root, "", "clients").unwrap();
    let request = crate::tree::create_request(root, &folder, Some("Lister")).unwrap();
    let moved = crate::tree::move_to(root, &request, "commandes").unwrap();
    let renamed = crate::tree::rename(root, &moved, "Lister les clients").unwrap();
    crate::tree::delete(root, &renamed, true).unwrap();
    crate::tree::delete(root, &folder, true).unwrap();
    let change = project.refresh(&["clients", "commandes", "commandes/lister.yml", &renamed]);
    assert!(change.is_empty(), "{change:?}");
}

// --- R3 à R5 : changements faits hors d'Arnod ------------------------------------------

#[test]
fn r3_ecriture_externe_d_une_requete() {
    let project = Project::new();
    external_edit(
        &project.path("commandes/creer.yml"),
        "/orders",
        "/v3/orders",
    );
    let change = project.refresh(&["commandes/creer.yml"]);
    assert_eq!(change.modified, vec!["commandes/creer.yml".to_owned()]);
    assert!(change.tree);
    assert!(!change.environments && !change.contracts);
    // L'état connu suit : le même événement relu ne donne plus rien.
    assert!(project.refresh(&["commandes/creer.yml"]).is_empty());
}

#[test]
fn r3_requete_et_dossier_ajoutes() {
    let project = Project::new();
    fs::create_dir(project.path("clients")).unwrap();
    fs::copy(
        project.path("commandes/creer.yml"),
        project.path("clients/creer.yml"),
    )
    .unwrap();
    external_edit(&project.path("clients/creer.yml"), "/orders", "/clients");
    let change = project.refresh(&["clients"]);
    assert_eq!(change.added, vec!["clients/creer.yml".to_owned()]);
    assert!(change.tree);
}

#[test]
fn r3_portees_environnements_contrats_collection() {
    let project = Project::new();
    fs::create_dir(project.path("environments")).unwrap();
    fs::write(project.path("environments/local.yml"), "name: Local\n").unwrap();
    let env = project.refresh(&["environments/local.yml"]);
    assert!(env.environments && !env.tree && !env.contracts);

    fs::create_dir(project.path("contracts")).unwrap();
    fs::write(project.path("contracts/api.json"), "{}").unwrap();
    let contracts = project.refresh(&["contracts"]);
    assert!(contracts.contracts && contracts.tree && contracts.modified.is_empty());

    fs::write(
        project.path("opencollection.yml"),
        "opencollection: 1.0.0\n",
    )
    .unwrap();
    let root = project.refresh(&["opencollection.yml"]);
    assert!(root.tree && root.environments && root.modified.is_empty());

    fs::write(project.path("commandes/folder.yml"), "info:\n  name: x\n").unwrap();
    let folder = project.refresh(&["commandes/folder.yml"]);
    assert!(folder.tree && folder.modified.is_empty() && folder.added.is_empty());

    fs::write(project.path("notes.txt"), "rien").unwrap();
    assert!(project.refresh(&["notes.txt"]).is_empty());
}

#[test]
fn r4_requete_supprimee() {
    let project = Project::new();
    fs::remove_file(project.path("commandes/creer.yml")).unwrap();
    let change = project.refresh(&["commandes/creer.yml"]);
    assert_eq!(change.removed, vec!["commandes/creer.yml".to_owned()]);
    assert!(change.renamed.is_empty());
}

#[test]
fn r4_dossier_supprime_avec_ses_requetes() {
    let project = Project::new();
    fs::remove_dir_all(project.path("commandes")).unwrap();
    let change = project.refresh(&["commandes"]);
    assert_eq!(change.removed, vec!["commandes/creer.yml".to_owned()]);
    assert!(change.tree);
}

#[test]
fn r5_renommage_reconnu_au_contenu() {
    let project = Project::new();
    fs::rename(
        project.path("commandes/creer.yml"),
        project.path("commandes/nouvelle.yml"),
    )
    .unwrap();
    let change = project.refresh(&["commandes/creer.yml", "commandes/nouvelle.yml"]);
    assert_eq!(
        change.renamed,
        vec![Renamed {
            from: "commandes/creer.yml".to_owned(),
            to: "commandes/nouvelle.yml".to_owned(),
        }]
    );
    assert!(change.added.is_empty() && change.removed.is_empty());
}

#[test]
fn r5_dossier_renomme_ses_requetes_suivent() {
    let project = Project::new();
    fs::rename(project.path("commandes"), project.path("orders")).unwrap();
    let change = project.refresh(&["commandes", "orders"]);
    assert_eq!(
        change.renamed,
        vec![Renamed {
            from: "commandes/creer.yml".to_owned(),
            to: "orders/creer.yml".to_owned(),
        }]
    );
}

#[test]
fn r5_deux_fichiers_de_meme_contenu_ne_sont_pas_un_renommage() {
    let deltas = vec![
        Delta {
            path: "a.yml".to_owned(),
            before: Some(Entry::File(7)),
            after: None,
        },
        Delta {
            path: "b.yml".to_owned(),
            before: None,
            after: Some(Entry::File(7)),
        },
        Delta {
            path: "c.yml".to_owned(),
            before: None,
            after: Some(Entry::File(7)),
        },
    ];
    let change = classify(Uuid::nil(), &deltas);
    assert!(change.renamed.is_empty());
    assert_eq!(change.removed, vec!["a.yml".to_owned()]);
    assert_eq!(change.added, vec!["b.yml".to_owned(), "c.yml".to_owned()]);
}

#[test]
fn r6_changements_en_camel_case() {
    let change = FilesChange {
        project_id: Uuid::nil(),
        renamed: vec![Renamed {
            from: "a.yml".to_owned(),
            to: "b.yml".to_owned(),
        }],
        tree: true,
        ..FilesChange::default()
    };
    assert_eq!(
        serde_json::to_value(&change).unwrap(),
        serde_json::json!({
            "projectId": Uuid::nil(),
            "added": [],
            "modified": [],
            "removed": [],
            "renamed": [{ "from": "a.yml", "to": "b.yml" }],
            "tree": true,
            "environments": false,
            "contracts": false,
        })
    );
}

// --- R8 : pas d'enregistrement sur une version plus récente ----------------------------

#[test]
fn r8_enregistrer_sur_un_fichier_modifie_depuis_la_lecture_est_refuse() {
    let project = Project::new();
    let path = project.path("commandes/creer.yml");
    acknowledge(&path);
    assert!(check_unchanged(&path).is_ok());
    external_edit(&path, "/orders", "/v3/orders");
    let error = check_unchanged(&path).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::ChangedOnDisk);
    assert_eq!(
        serde_json::to_value(&error).unwrap()["kind"],
        serde_json::json!("changedOnDisk")
    );
    // Relue, la nouvelle version peut être remplacée.
    acknowledge(&path);
    assert!(check_unchanged(&path).is_ok());
}

#[test]
fn r8_une_ecriture_d_arnod_devient_la_version_connue() {
    let project = Project::new();
    let path = project.path("commandes/creer.yml");
    acknowledge(&path);
    file::write(&path, &sample("Autre nom")).unwrap();
    assert!(check_unchanged(&path).is_ok());
}

// --- R12 : fichier illisible jamais réécrit --------------------------------------------

#[test]
fn r12_un_fichier_illisible_n_est_jamais_reecrit() {
    let project = Project::new();
    let path = project.path("commandes/creer.yml");
    fs::write(&path, "info: [cassé").unwrap();
    let error = file::write(&path, &sample("Créer une commande")).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::InvalidRequest);
    assert_eq!(fs::read_to_string(&path).unwrap(), "info: [cassé");
}

// --- R9 à R11 : conflit élément par élément --------------------------------------------

fn assertion(target: &str, value: i64) -> Assertion {
    Assertion {
        id: format!("{target}-{value}"),
        target: target.to_owned(),
        operator: Operator::Equals,
        value: Some(Scalar::Int(value)),
        origin: Origin::Manual,
        enabled: true,
    }
}

fn base() -> Request {
    let mut request = sample("Créer une commande");
    request.body = Body::raw(BodyKind::Json, "{\"coupon\": \"BIENVENUE\"}");
    request.assertions = vec![assertion("status", 200), assertion("body.total", 59)];
    request
}

#[test]
fn r9_compare_les_elements_qui_different() {
    let base = base();
    let mut disk = base.clone();
    disk.headers
        .push(Pair::new("X-Request-Id", "{{$randomUUID}}"));
    disk.assertions.retain(|a| a.target != "body.total");
    disk.assertions[0].value = Some(Scalar::Int(201));
    let mut draft = base.clone();
    draft.body = Body::raw(BodyKind::Json, "{\"coupon\": \"NOEL26\"}");
    draft.assertions[0].value = Some(Scalar::Int(201));
    // Identifiants de session différents : la vérification reste la même.
    draft.assertions[1].id = "autre".to_owned();

    let items = conflict::compare(&base, &disk, &draft);
    let keys: Vec<&str> = items.iter().map(|item| item.key.as_str()).collect();
    assert_eq!(
        keys,
        vec![
            "body",
            "headers:x-request-id#0",
            "assertions:body.total#0",
            "assertions:status#0",
        ]
    );
    let header = &items[1];
    assert_eq!(header.label, "En-tête X-Request-Id");
    assert_eq!(header.disk.as_deref(), Some("{{$randomUUID}}"));
    assert_eq!(header.draft, None);
    assert_eq!(header.suggested, Some(ConflictSide::Disk));
    assert!(!header.identical);
    let body = &items[0];
    assert_eq!(
        body.draft.as_deref(),
        Some("json · {\"coupon\": \"NOEL26\"}")
    );
    assert_eq!(body.suggested, Some(ConflictSide::Draft));
    let total = &items[2];
    assert_eq!(total.label, "Vérification body.total");
    assert_eq!(total.disk, None);
    assert_eq!(total.draft.as_deref(), Some("égal à 59"));
    // Même changement des deux côtés : identique, en fin de liste.
    let status = &items[3];
    assert!(status.identical);
    assert_eq!(status.disk, status.draft);
    assert_eq!(status.suggested, None);
}

#[test]
fn r10_rien_de_propose_quand_les_deux_ont_change() {
    let base = base();
    let mut disk = base.clone();
    disk.url = "{{baseUrl}}/v3/orders".to_owned();
    let mut draft = base.clone();
    draft.url = "{{baseUrl}}/v4/orders".to_owned();
    let items = conflict::compare(&base, &disk, &draft);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "Adresse");
    assert_eq!(items[0].suggested, None);
}

#[test]
fn r11_fusion_selon_les_choix() {
    let base = base();
    let mut disk = base.clone();
    disk.headers
        .push(Pair::new("X-Request-Id", "{{$randomUUID}}"));
    disk.assertions.retain(|a| a.target != "body.total");
    let mut draft = base.clone();
    draft.body = Body::raw(BodyKind::Json, "{\"coupon\": \"NOEL26\"}");
    draft.url = "{{baseUrl}}/v2/orders".to_owned();
    let mut disk_url = disk.clone();
    disk_url.url = "{{baseUrl}}/v3/orders".to_owned();

    let choices = BTreeMap::from([
        ("headers:x-request-id#0".to_owned(), ConflictSide::Disk),
        ("body".to_owned(), ConflictSide::Draft),
        ("assertions:body.total#0".to_owned(), ConflictSide::Draft),
        ("url".to_owned(), ConflictSide::Disk),
    ]);
    let merged = conflict::merge(&disk_url, &draft, &choices).unwrap();
    assert_eq!(merged.url, "{{baseUrl}}/v3/orders");
    assert_eq!(merged.body, draft.body);
    assert_eq!(
        merged.headers,
        vec![Pair::new("X-Request-Id", "{{$randomUUID}}")]
    );
    let targets: Vec<&str> = merged
        .assertions
        .iter()
        .map(|a| a.target.as_str())
        .collect();
    assert_eq!(targets, vec!["status", "body.total"]);

    // Vérification retirée sur le disque, choisie « disque » : retirée.
    let choices = BTreeMap::from([
        ("headers:x-request-id#0".to_owned(), ConflictSide::Draft),
        ("body".to_owned(), ConflictSide::Disk),
        ("assertions:body.total#0".to_owned(), ConflictSide::Disk),
        ("url".to_owned(), ConflictSide::Draft),
    ]);
    let merged = conflict::merge(&disk_url, &draft, &choices).unwrap();
    assert!(merged.headers.is_empty());
    assert_eq!(merged.body, base.body);
    assert_eq!(merged.url, draft.url);
    assert_eq!(merged.assertions.len(), 1);
}

#[test]
fn r11_un_element_sans_choix_refuse_la_fusion() {
    let base = base();
    let mut disk = base.clone();
    disk.url = "{{baseUrl}}/v3/orders".to_owned();
    let error = conflict::merge(&disk, &base, &BTreeMap::new()).unwrap_err();
    assert!(error.contains("encore changé"), "{error}");
}

#[test]
fn r9_element_en_camel_case() {
    let base = base();
    let mut disk = base.clone();
    disk.method = Method::Put;
    let items = conflict::compare(&base, &disk, &base);
    assert_eq!(
        serde_json::to_value(&items).unwrap(),
        serde_json::json!([{
            "key": "method",
            "label": "Méthode",
            "disk": "PUT",
            "draft": "POST",
            "identical": false,
            "suggested": "disk",
        }])
    );
}

// --- R7 : surveillant réel ---------------------------------------------------------------

#[derive(Default)]
struct Recorder {
    changes: Mutex<Vec<FilesChange>>,
}

impl Sink for Recorder {
    fn changed(&self, change: FilesChange) {
        self.changes.lock().unwrap().push(change);
    }
}

impl Recorder {
    /// Attend un lot de changements (5 s au plus).
    fn wait(&self) -> Option<FilesChange> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(change) = self.changes.lock().unwrap().pop() {
                return Some(change);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }

    fn quiet_for(&self, duration: Duration) -> bool {
        std::thread::sleep(duration);
        self.changes.lock().unwrap().is_empty()
    }
}

#[test]
fn r7_surveillant_reel_ecriture_renommage_suppression_et_echo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    fs::create_dir(root.join("commandes")).unwrap();
    let path = root.join("commandes/creer.yml");
    file::write(&path, &sample("Créer une commande")).unwrap();
    let recorder = Arc::new(Recorder::default());
    let watchers = Watchers::new(recorder.clone());
    let project = Uuid::new_v4();
    watchers.watch(project, &root).unwrap();
    assert!(watchers.is_watched(project));

    // Écriture d'Arnod : écho ignoré.
    let mut request = sample("Créer une commande");
    request.url = "{{baseUrl}}/v2/orders".to_owned();
    file::write(&path, &request).unwrap();
    assert!(recorder.quiet_for(Duration::from_millis(900)));

    // Écriture externe (éditeur, git pull).
    external_edit(&path, "/v2/orders", "/v3/orders");
    let change = recorder.wait().expect("écriture externe");
    assert_eq!(change.project_id, project);
    assert_eq!(change.modified, vec!["commandes/creer.yml".to_owned()]);

    // Renommage externe.
    fs::rename(&path, root.join("commandes/nouvelle.yml")).unwrap();
    let change = recorder.wait().expect("renommage externe");
    assert_eq!(change.renamed.len(), 1, "{change:?}");

    // Suppression externe.
    fs::remove_file(root.join("commandes/nouvelle.yml")).unwrap();
    let change = recorder.wait().expect("suppression externe");
    assert_eq!(change.removed, vec!["commandes/nouvelle.yml".to_owned()]);

    // Arrêtée, la surveillance ne dit plus rien.
    watchers.retain(|_| false);
    assert!(!watchers.is_watched(project));
    fs::write(root.join("commandes/autre.yml"), "x").unwrap();
    assert!(recorder.quiet_for(Duration::from_millis(700)));
}
