//! Synchronisation (`docs/plan-v2.md`, lots E et F) : relier, ce qui arrive en clair (R14),
//! récupérer en fusionnant élément par élément (R12), conflit sans rien écrire (R13), envoi
//! refusé puis renvoyé (R11), transport HTTP par le réseau du poste avec le jeton en en-tête
//! (R9), historique d'un élément (CF5).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use git2::{Repository, Signature};
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

use super::describe::ItemKind;
use super::forge::Forge;
use super::merge::{Choice, WHOLE};
use super::state::Account;
use super::*;
use crate::manifest::{Manifest, create_project_folder};
use crate::network::Network;
use crate::network::proxy::{FixedProxySource, SystemProxy};
use crate::request::file;
use crate::request::model::Assertion;
use crate::secrets::MemorySecretStore;

struct Fixture {
    project: TempDir,
    data: TempDir,
    remote: TempDir,
    secrets: Arc<MemorySecretStore>,
    network: Arc<Network>,
    book: SyncBook,
    id: Uuid,
}

fn equals(target: &str, value: serde_json::Value) -> Assertion {
    serde_json::from_value(
        json!({ "id": "", "target": target, "operator": "equals", "value": value }),
    )
    .unwrap()
}

fn edit(root: &Path, path: &str, change: impl FnOnce(&mut crate::request::model::Request)) {
    let full = root.join(path);
    let mut request = file::read(&full).unwrap();
    change(&mut request);
    file::write(&full, &request).unwrap();
}

impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let id = Uuid::from_u128(0x5e);
        let _ =
            create_project_folder(project.path(), &Manifest::new(id, "boutique-tests")).unwrap();
        crate::tree::create_folder(project.path(), "", "commandes").unwrap();
        // Un sous-dossier : « commandes » est un dossier ordinaire, ses requêtes ne sont pas des
        // étapes.
        crate::tree::create_folder(project.path(), "commandes", "archives").unwrap();
        for (name, expected) in [("Lister", LISTER), ("Créer", CREER)] {
            let path =
                crate::tree::create_request(project.path(), "commandes", Some(name)).unwrap();
            assert_eq!(path, expected);
            edit(project.path(), &path, |r| {
                r.url = format!("https://api.test/{}", name.to_lowercase());
                r.assertions = vec![equals("status", json!(200))];
            });
        }
        let remote = tempfile::tempdir().unwrap();
        Repository::init_bare(remote.path()).unwrap();
        Self {
            project,
            data: tempfile::tempdir().unwrap(),
            remote,
            secrets: Arc::new(MemorySecretStore::default()),
            network: Arc::new(Network::new(
                Arc::new(MemorySecretStore::default()),
                Box::new(FixedProxySource(SystemProxy::default())),
            )),
            book: SyncBook::default(),
            id,
        }
    }

    fn ctx(&self) -> SyncCtx<'_> {
        SyncCtx {
            project: self.id,
            name: "boutique-tests",
            root: self.project.path(),
            data_dir: self.data.path(),
            network: self.network.clone(),
            secrets: self.secrets.clone(),
        }
    }

    fn url(&self) -> String {
        self.remote.path().display().to_string()
    }

    fn root(&self) -> &Path {
        self.project.path()
    }

    fn link(&self) -> SyncView {
        link(&self.ctx(), &self.book, &self.url(), "main", None).unwrap()
    }

    /// Un collègue : un clone du dépôt, où il modifie puis envoie.
    fn colleague(&self, change: impl FnOnce(&Path)) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::build::RepoBuilder::new()
            .branch("main")
            .clone(&self.url(), dir.path())
            .unwrap();
        change(dir.path());
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.update_all(["*"].iter(), None).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let marc = Signature::now("Marc", "marc@exemple.fr").unwrap();
        repo.commit(Some("HEAD"), &marc, &marc, "Pagination", &tree, &[&parent])
            .unwrap();
        repo.find_remote("origin")
            .unwrap()
            .push(&["refs/heads/main:refs/heads/main"], None)
            .unwrap();
    }

    fn remote_file(&self, path: &str) -> Option<String> {
        let repo = Repository::open_bare(self.remote.path()).unwrap();
        let commit = repo
            .find_reference("refs/heads/main")
            .ok()?
            .peel_to_commit()
            .ok()?;
        git::blob_at(&repo, commit.tree().ok().as_ref(), path)
            .map(|b| String::from_utf8(b).unwrap())
    }

    fn assertions(&self, path: &str) -> Vec<(String, String)> {
        file::read(&self.root().join(path))
            .unwrap()
            .assertions
            .into_iter()
            .map(|a| {
                (
                    a.target,
                    a.value
                        .map(|v| crate::opencollection::request::value_text(&v))
                        .unwrap_or_default(),
                )
            })
            .collect()
    }
}

const LISTER: &str = "commandes/lister.yml";
const CREER: &str = "commandes/creer.yml";

#[test]
fn sy2_relier_cree_le_depot_et_envoie_le_projet() {
    let f = Fixture::new();
    let before = view(&f.ctx(), &f.book);
    assert!(!before.repository && before.remote.is_none());
    let linked = f.link();
    assert!(linked.repository);
    assert_eq!(
        linked.remote.as_ref().map(|r| r.branch.as_str()),
        Some("main")
    );
    assert!(linked.outgoing.is_empty() && linked.incoming.is_empty());
    assert!(f.remote_file(LISTER).is_some());
    assert!(f.remote_file("opencollection.yml").is_some());
    assert_eq!(f.remote_file(".gitignore").as_deref(), Some(GITIGNORE));
    // R10 : l'état du poste reste hors du projet.
    assert!(f.data.path().join("projects").exists());
    assert!(!f.root().join("sync.json").exists());
}

#[test]
fn sy5_r14_ce_qui_arrive_en_clair_puis_recupere() {
    let f = Fixture::new();
    f.link();
    f.colleague(|dir| {
        edit(dir, LISTER, |r| {
            r.assertions = vec![equals("status", json!(201))]
        })
    });
    let checked = check(&f.ctx(), &f.book);
    assert!(checked.error.is_none(), "{:?}", checked.error);
    assert_eq!(checked.incoming.len(), 1);
    // Ce que seul le collègue a changé n'est pas « à envoyer ».
    assert!(checked.outgoing.is_empty(), "{:?}", checked.outgoing);
    let item = &checked.incoming[0];
    assert_eq!(
        (
            item.kind,
            item.label.as_str(),
            item.place.as_str(),
            item.detail.as_str()
        ),
        (
            ItemKind::Request,
            "Lister",
            "commandes",
            "assertion status modifiée"
        )
    );
    assert_eq!(item.author.as_deref(), Some("Marc"));
    assert!(checked.members.contains(&"Marc".to_owned()));
    // Vérifier n'écrit rien dans le projet.
    assert_eq!(
        f.assertions(LISTER),
        [("status".to_owned(), "200".to_owned())]
    );
    let pulled = pull(&f.ctx(), &f.book).unwrap();
    assert_eq!(pulled.written, [LISTER]);
    assert_eq!(
        f.assertions(LISTER),
        [("status".to_owned(), "201".to_owned())]
    );
    let after = view(&f.ctx(), &f.book);
    assert!(after.incoming.is_empty() && after.outgoing.is_empty());
    assert_eq!(
        received(&f.ctx()).get(LISTER).map(String::as_str),
        Some("Marc")
    );
}

#[test]
fn r12_ce_qu_un_seul_cote_a_change_est_pris_sans_question() {
    let f = Fixture::new();
    f.link();
    // Moi : l'adresse (non envoyée) ; le collègue : l'assertion du même fichier.
    edit(f.root(), LISTER, |r| {
        r.url = "https://api.test/v2/lister".to_owned()
    });
    f.colleague(|dir| {
        edit(dir, LISTER, |r| {
            r.assertions = vec![equals("status", json!(204))]
        })
    });
    let pulled = pull(&f.ctx(), &f.book).unwrap();
    assert_eq!(pulled.conflicts, 0);
    let merged = file::read(&f.root().join(LISTER)).unwrap();
    assert_eq!(merged.url, "https://api.test/v2/lister");
    assert_eq!(
        f.assertions(LISTER),
        [("status".to_owned(), "204".to_owned())]
    );
    let after = view(&f.ctx(), &f.book);
    assert_eq!(after.outgoing.len(), 1, "ma modification reste à envoyer");
    assert_eq!(after.outgoing[0].detail, "adresse modifiée");
}

#[test]
fn r13_un_conflit_n_ecrit_rien_puis_se_regle_element_par_element() {
    let f = Fixture::new();
    f.link();
    edit(f.root(), LISTER, |r| {
        r.assertions = vec![equals("status", json!(202))];
        r.docs = "Ma note".to_owned();
    });
    f.colleague(|dir| {
        edit(dir, LISTER, |r| {
            r.assertions = vec![equals("status", json!(204))];
            r.description = "Liste paginée".to_owned();
        });
    });
    let mine = std::fs::read_to_string(f.root().join(LISTER)).unwrap();
    let pulled = pull(&f.ctx(), &f.book).unwrap();
    assert_eq!(pulled.conflicts, 1);
    assert_eq!(
        std::fs::read_to_string(f.root().join(LISTER)).unwrap(),
        mine
    );
    assert_eq!(view(&f.ctx(), &f.book).conflicts, 1);
    // Envoyer est bloqué tant que le conflit n'est pas réglé.
    let error = send(&f.ctx(), &f.book, &[LISTER.to_owned()], "", false).unwrap_err();
    assert_eq!(error.kind, CommandErrorKind::Conflict);

    let listed = conflicts(&f.ctx(), &f.book);
    assert_eq!(listed.conflicts.len(), 1);
    let conflict = &listed.conflicts[0];
    assert_eq!(conflict.author, "Marc");
    assert_eq!(conflict.item.label, "Lister");
    let keys: Vec<&str> = conflict.elements.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["assertions:status#0"]);
    assert!(conflict.elements[0].both);
    let merged: Vec<(&str, bool)> = conflict
        .merged
        .iter()
        .map(|a| (a.label.as_str(), a.theirs))
        .collect();
    assert_eq!(merged, [("Description", true), ("Documentation", false)]);

    // Abandonner : le projet reste tel qu'avant (R13).
    abandon(&f.ctx(), &f.book);
    assert_eq!(view(&f.ctx(), &f.book).conflicts, 0);
    assert_eq!(
        std::fs::read_to_string(f.root().join(LISTER)).unwrap(),
        mine
    );

    assert!(
        f.book.pending.lock().unwrap().get(&f.id).is_none(),
        "abandonnée"
    );

    // Recommencer, puis garder les deux assertions.
    assert_eq!(pull(&f.ctx(), &f.book).unwrap().conflicts, 1);
    let incomplete = resolve(&f.ctx(), &f.book, &BTreeMap::new()).unwrap_err();
    assert_eq!(incomplete.kind, CommandErrorKind::InvalidRequest);
    let choices = BTreeMap::from([(
        LISTER.to_owned(),
        BTreeMap::from([("assertions:status#0".to_owned(), Choice::Both)]),
    )]);
    resolve(&f.ctx(), &f.book, &choices).unwrap();
    assert_eq!(
        f.assertions(LISTER),
        [
            ("status".to_owned(), "202".to_owned()),
            ("status".to_owned(), "204".to_owned())
        ]
    );
    let resolved = file::read(&f.root().join(LISTER)).unwrap();
    assert_eq!(
        (resolved.description.as_str(), resolved.docs.as_str()),
        ("Liste paginée", "Ma note")
    );
    let after = view(&f.ctx(), &f.book);
    assert!(after.incoming.is_empty() && after.conflicts == 0);
    // Puis envoyer : le collègue retrouvera les deux assertions.
    send(
        &f.ctx(),
        &f.book,
        &[LISTER.to_owned()],
        "Deux statuts",
        false,
    )
    .unwrap();
    assert!(f.remote_file(LISTER).unwrap().contains("202"));
}

#[test]
fn sy6_envoie_seulement_les_elements_choisis_avec_le_message() {
    let f = Fixture::new();
    f.link();
    edit(f.root(), LISTER, |r| {
        r.url = "https://api.test/v2/lister".to_owned()
    });
    edit(f.root(), CREER, |r| {
        r.url = "https://api.test/v2/creer".to_owned()
    });
    let outgoing = view(&f.ctx(), &f.book).outgoing;
    assert_eq!(outgoing.len(), 2);
    let (view, _) = send(&f.ctx(), &f.book, &[LISTER.to_owned()], "Version 2", true).unwrap();
    assert!(f.remote_file(LISTER).unwrap().contains("v2/lister"));
    assert!(!f.remote_file(CREER).unwrap().contains("v2/creer"));
    assert_eq!(
        view.outgoing
            .iter()
            .map(|i| i.key.as_str())
            .collect::<Vec<_>>(),
        [CREER]
    );
    let repo = Repository::open_bare(f.remote.path()).unwrap();
    let head = repo
        .find_reference("refs/heads/main")
        .unwrap()
        .peel_to_commit()
        .unwrap();
    assert_eq!(head.message(), Ok("Version 2"));
}

#[test]
fn r11_envoi_refuse_parce_que_l_equipe_a_avance_puis_renvoye() {
    let f = Fixture::new();
    f.link();
    edit(f.root(), LISTER, |r| {
        r.url = "https://api.test/v2/lister".to_owned()
    });
    // Le collègue envoie entre-temps (ma copie de la version de l'équipe est en retard).
    f.colleague(|dir| edit(dir, CREER, |r| r.description = "Par Marc".to_owned()));
    let (view, pulled) = send(&f.ctx(), &f.book, &[LISTER.to_owned()], "Version 2", false).unwrap();
    assert_eq!(pulled.written, [CREER], "récupéré avant de renvoyer");
    assert!(f.remote_file(LISTER).unwrap().contains("v2/lister"));
    assert!(f.remote_file(CREER).unwrap().contains("Par Marc"));
    assert!(view.outgoing.is_empty() && view.incoming.is_empty());
}

#[test]
fn cf5_historique_d_un_element_en_clair() {
    let f = Fixture::new();
    f.link();
    f.colleague(|dir| {
        edit(dir, LISTER, |r| {
            r.assertions = vec![equals("status", json!(201))]
        })
    });
    pull(&f.ctx(), &f.book).unwrap();
    let entries = journal(&f.ctx(), LISTER);
    let summary: Vec<(&str, &str)> = entries
        .iter()
        .map(|e| (e.author.as_str(), e.detail.as_str()))
        .collect();
    assert_eq!(summary[0], ("Marc", "assertion status modifiée"));
    assert_eq!(summary[1].1, "nouvelle requête");
    assert!(entries[0].before.as_deref().unwrap().contains("200"));
}

#[test]
fn q8_un_fichier_qui_n_est_pas_une_requete_se_garde_en_entier() {
    let base = b"name: Local\nvariables: []\n".as_slice();
    let ours = b"name: Local\nvariables:\n  - name: a\n    value: '1'\n".as_slice();
    let theirs = b"name: Local\nvariables:\n  - name: b\n    value: '2'\n".as_slice();
    let path = "environments/local.yml";
    let Merged::Conflict { elements, .. } =
        merge_file(path, Some(base), Some(ours), Some(theirs), &BTreeMap::new())
    else {
        panic!("conflit attendu");
    };
    assert_eq!(elements[0].key, WHOLE);
    let choices = BTreeMap::from([(WHOLE.to_owned(), Choice::Theirs)]);
    assert_eq!(
        merge_file(path, Some(base), Some(ours), Some(theirs), &choices),
        Merged::Clean(Some(theirs.to_vec()))
    );
    // Un seul côté a changé : pris sans question.
    assert_eq!(
        merge_file(path, Some(base), Some(base), Some(theirs), &BTreeMap::new()),
        Merged::Clean(Some(theirs.to_vec()))
    );
}

#[test]
fn r14_descriptions_en_clair() {
    use super::describe::describe;
    let tests = std::collections::BTreeSet::from(["commandes/parcours".to_owned()]);
    let request = |assertions: &[(&str, i64)], header: Option<&str>| {
        let mut r = crate::request::model::Request::new("Payer");
        r.url = "https://api.test/payer".to_owned();
        r.assertions = assertions
            .iter()
            .map(|(t, v)| equals(t, json!(v)))
            .collect();
        if let Some(h) = header {
            r.headers = vec![serde_json::from_value(json!({ "name": h, "value": "1" })).unwrap()];
        }
        file::render(&r).unwrap().into_bytes()
    };
    let old = request(&[("status", 200), ("body.total", 1)], None);
    let new = request(&[("status", 201), ("body.total", 2)], Some("X-Id"));
    let item = describe(
        "commandes/parcours/payer.yml",
        Some(&old),
        Some(&new),
        &tests,
    );
    assert_eq!(item.kind, ItemKind::Step);
    assert_eq!(item.place, "commandes / parcours");
    assert_eq!(item.detail, "en-tête X-Id ajouté · 2 assertions modifiées");
    let added = describe("payer.yml", None, Some(&new), &tests);
    assert_eq!(
        (added.kind, added.detail.as_str()),
        (ItemKind::Request, "nouvelle requête")
    );
    let removed = describe("payer.yml", Some(&old), None, &tests);
    assert_eq!(removed.detail, "supprimée");
    let env_old = b"name: Recette\nvariables:\n  - name: baseUrl\n    value: a\n".as_slice();
    let env_new =
        b"name: Recette\nvariables:\n  - name: baseUrl\n    value: b\n  - name: clientId\n    value: c\n"
            .as_slice();
    let env = describe(
        "environments/recette.yml",
        Some(env_old),
        Some(env_new),
        &tests,
    );
    assert_eq!(
        (env.kind, env.label.as_str(), env.detail.as_str()),
        (
            ItemKind::Environment,
            "Recette",
            "variable baseUrl · variable clientId ajoutée"
        )
    );
    assert_eq!(super::describe::initial("marc"), "M");
}

#[test]
fn r9_adresse_sans_identifiant_et_forge_reconnue() {
    assert_eq!(
        forge::clean_url("https://julien:jeton@github.com/acme/x.git"),
        "https://github.com/acme/x.git"
    );
    assert_eq!(
        forge::parse_remote("https://gitlab.exemple.fr/equipe/boutique-tests.git"),
        (
            Forge::Gitlab,
            "gitlab.exemple.fr".to_owned(),
            "equipe/boutique-tests".to_owned()
        )
    );
    assert_eq!(
        forge::api_base(Forge::Github, "github.com"),
        "https://api.github.com"
    );
    assert_eq!(
        forge::api_base(Forge::Github, "git.acme.fr"),
        "https://git.acme.fr/api/v3"
    );
    assert_eq!(Forge::Gitlab.git_user("julien"), "oauth2");
}

/// Transport HTTP contre `git http-backend` (CGI) : seulement là où Git le fournit (Unix).
#[cfg(unix)]
mod http {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use super::*;

    /// Serveur Git HTTP local : `git http-backend` (CGI), avec les en-têtes `Authorization` reçus.
    fn git_http_server(root: PathBuf) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_owned();
                let target = parts.next().unwrap_or_default().to_owned();
                let mut headers = Vec::new();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end().to_owned();
                    if header.is_empty() {
                        break;
                    }
                    headers.push(header);
                }
                let value = |name: &str| {
                    headers.iter().find_map(|h| {
                        h.split_once(':')
                            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
                            .map(|(_, v)| v.trim().to_owned())
                    })
                };
                log.lock()
                    .unwrap()
                    .push(value("authorization").unwrap_or_default());
                let length: usize = value("content-length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let mut child = std::process::Command::new("git")
                    .arg("http-backend")
                    .env("GIT_PROJECT_ROOT", &root)
                    .env("GIT_HTTP_EXPORT_ALL", "1")
                    .env("REQUEST_METHOD", &method)
                    .env("PATH_INFO", path)
                    .env("QUERY_STRING", query)
                    .env("CONTENT_TYPE", value("content-type").unwrap_or_default())
                    .env("CONTENT_LENGTH", length.to_string())
                    .env("REMOTE_USER", "julien")
                    .env("REMOTE_ADDR", "127.0.0.1")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                child.stdin.take().unwrap().write_all(&body).unwrap();
                let output = child.wait_with_output().unwrap();
                let split = output
                    .stdout
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .map(|i| (i, 4))
                    .or_else(|| {
                        output
                            .stdout
                            .windows(2)
                            .position(|w| w == b"\n\n")
                            .map(|i| (i, 2))
                    })
                    .unwrap();
                let head = String::from_utf8_lossy(&output.stdout[..split.0]).into_owned();
                let content = &output.stdout[split.0 + split.1..];
                let mut status = "200 OK".to_owned();
                let mut response = Vec::new();
                for header in head.lines() {
                    if let Some(s) = header.strip_prefix("Status:") {
                        status = s.trim().to_owned();
                    } else {
                        response.push(header.to_owned());
                    }
                }
                let mut stream = stream;
                let _ = write!(stream, "HTTP/1.1 {status}\r\n");
                for header in response {
                    let _ = write!(stream, "{header}\r\n");
                }
                let _ = write!(
                    stream,
                    "Content-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                let _ = stream.write_all(content);
            }
        });
        (format!("http://127.0.0.1:{port}"), seen)
    }

    #[test]
    fn r9_transport_http_par_le_reseau_du_poste_jeton_en_en_tete() {
        let f = Fixture::new();
        // Dépôt servi par HTTP ; envois acceptés par git http-backend.
        std::process::Command::new("git")
            .args(["config", "http.receivepack", "true"])
            .current_dir(f.remote.path())
            .status()
            .unwrap();
        let parent = f.remote.path().parent().unwrap().to_path_buf();
        let name = f
            .remote
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let (base, seen) = git_http_server(parent);
        let account = Account {
            forge: Forge::Other,
            host: "127.0.0.1".to_owned(),
            login: "julien".to_owned(),
            name: "Julien".to_owned(),
            email: "julien@exemple.fr".to_owned(),
        };
        f.secrets
            .set(&state::token_key("127.0.0.1", "julien"), "jeton-secret")
            .unwrap();
        let url = format!("{base}/{name}");
        let linked = link(&f.ctx(), &f.book, &url, "main", Some(account)).unwrap();
        assert_eq!(linked.remote.as_ref().unwrap().url, url);
        assert!(f.remote_file(LISTER).is_some(), "envoyé par HTTP");
        let expected = transport::basic("julien", "jeton-secret");
        let headers = seen.lock().unwrap().clone();
        assert!(!headers.is_empty());
        assert!(headers.iter().all(|h| h == &expected), "{headers:?}");
        // Le jeton n'est jamais écrit dans la configuration du dépôt (R9).
        let config = std::fs::read_to_string(f.root().join(".git/config")).unwrap();
        assert!(!config.contains("jeton-secret") && config.contains(&url));
        // Une modification d'un collègue arrive par le même transport.
        f.colleague(|dir| edit(dir, LISTER, |r| r.description = "Par Marc".to_owned()));
        let checked = check(&f.ctx(), &f.book);
        assert!(checked.error.is_none(), "{:?}", checked.error);
        assert_eq!(checked.incoming.len(), 1);
        assert_eq!(checked.incoming[0].detail, "description modifiée");
    }
}

mod forges {
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn network() -> Network {
        Network::new(
            Arc::new(MemorySecretStore::default()),
            Box::new(FixedProxySource(SystemProxy::default())),
        )
    }

    #[tokio::test]
    async fn sy1_compte_github_par_son_jeton_adresse_privee_sinon() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/user"))
            .and(header("authorization", "Bearer jeton"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({ "login": "julien-corre", "id": 42, "name": "Julien", "email": null }),
            ))
            .mount(&server)
            .await;
        let account = forge::account(&network(), Forge::Github, &server.uri(), "jeton", None)
            .await
            .unwrap();
        assert_eq!(account.login, "julien-corre");
        assert_eq!(account.name, "Julien");
        assert_eq!(account.email, "42+julien-corre@users.noreply.github.com");
        let refused = forge::account(&network(), Forge::Github, &server.uri(), "autre", None)
            .await
            .unwrap_err();
        assert_eq!(refused.kind, CommandErrorKind::Network);
        let other = forge::account(&network(), Forge::Other, "git.acme.fr", "t", Some("julien"))
            .await
            .unwrap();
        assert_eq!(other.email, "julien@git.acme.fr");
    }

    #[tokio::test]
    async fn sy2_depots_du_compte_filtres_et_creation() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/projects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "path_with_namespace": "acme/boutique-tests", "http_url_to_repo": "https://g/acme/boutique-tests.git", "visibility": "private", "empty_repo": true },
                { "path_with_namespace": "acme/api-partenaires-tests", "http_url_to_repo": "https://g/acme/api.git", "visibility": "public" }
            ])))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v4/projects"))
            .and(body_string_contains("\"visibility\":\"private\""))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!(
                { "path_with_namespace": "julien/nouveau", "http_url_to_repo": "https://g/julien/nouveau.git", "visibility": "private" }
            )))
            .mount(&server)
            .await;
        let account = Account {
            forge: Forge::Gitlab,
            host: server.uri(),
            login: "julien".into(),
            name: "Julien".into(),
            email: "j@x".into(),
        };
        let all = forge::repositories(&network(), &account, "t", "boutique")
            .await
            .unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].empty && all[0].private);
        let created = forge::create_repository(&network(), &account, "t", "nouveau")
            .await
            .unwrap();
        assert_eq!(created.url, "https://g/julien/nouveau.git");
        assert!(created.empty);
    }

    #[tokio::test]
    async fn sy1_connexion_par_le_navigateur_device() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/device/code"))
            .and(body_string_contains("client_id=arnod"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code": "dc", "user_code": "WDJB-MJHT",
                "verification_uri": "https://github.com/login/device", "interval": 5, "expires_in": 900
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "error": "authorization_pending" })),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "access_token": "gho_x" })),
            )
            .mount(&server)
            .await;
        let n = network();
        let code = forge::device_start_as(&n, Forge::Github, &server.uri(), "arnod")
            .await
            .unwrap();
        assert_eq!(code.user_code, "WDJB-MJHT");
        assert_eq!(
            serde_json::to_value(&code).unwrap().get("deviceCode"),
            None,
            "le code de l'appareil n'est jamais montré"
        );
        let pending =
            forge::device_poll_as(&n, Forge::Github, &server.uri(), &code.device_code, "arnod")
                .await
                .unwrap();
        assert_eq!(pending, None);
        let token =
            forge::device_poll_as(&n, Forge::Github, &server.uri(), &code.device_code, "arnod")
                .await
                .unwrap();
        assert_eq!(token.as_deref(), Some("gho_x"));
    }
}

#[test]
fn r12_une_fusion_identique_a_un_cote_garde_ses_octets() {
    let render = |status: i64, docs: &str| {
        let mut r = crate::request::model::Request::new("Lister");
        r.url = "https://api.test/lister".to_owned();
        r.docs = docs.to_owned();
        r.assertions = vec![equals("status", json!(status))];
        file::render(&r).unwrap().into_bytes()
    };
    let base = render(200, "");
    let ours = render(202, "");
    // Le collègue : même requête, autre mise en forme (commentaire YAML ajouté).
    let mut theirs = render(204, "");
    theirs.extend_from_slice(b"# relu par Marc\n");
    let choices = BTreeMap::from([("assertions:status#0".to_owned(), Choice::Theirs)]);
    assert_eq!(
        merge_file(
            "lister.yml",
            Some(&base),
            Some(&ours),
            Some(&theirs),
            &choices
        ),
        Merged::Clean(Some(theirs.clone()))
    );
    let mine = BTreeMap::from([("assertions:status#0".to_owned(), Choice::Ours)]);
    assert_eq!(
        merge_file("lister.yml", Some(&base), Some(&ours), Some(&theirs), &mine),
        Merged::Clean(Some(ours))
    );
}
