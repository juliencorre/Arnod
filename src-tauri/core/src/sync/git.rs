//! Dépôt Git d'un projet (V2, lot E, Q1) : libgit2 par `git2`, sans transport HTTPS ni SSH
//! propres (voir [`super::transport`]). Rien ici ne parle à l'utilisateur en termes Git : les
//! fonctions rendent des fichiers et des versions, [`super::describe`] les traduit (R14).

use std::collections::BTreeMap;
use std::path::Path;

use git2::{
    BranchType, Commit, Diff, FetchOptions, IndexEntry, IndexTime, ObjectType, Oid, PushOptions,
    RemoteCallbacks, Repository, RepositoryInitOptions, ResetType, Signature, Sort, StatusOptions,
    Tree,
};

use crate::error::{CommandError, CommandErrorKind};

/// Nom du dépôt distant relié.
pub const ORIGIN: &str = "origin";

pub fn failure(context: &str, error: &git2::Error) -> CommandError {
    let kind = match error.class() {
        git2::ErrorClass::Net | git2::ErrorClass::Http | git2::ErrorClass::Ssl => {
            CommandErrorKind::Network
        }
        _ => CommandErrorKind::Io,
    };
    CommandError::new(kind, format!("{context} : {}", error.message()))
}

/// Dépôt dont le dossier de travail est exactement `root` (un projet rangé plus bas dans un
/// autre dépôt n'est pas relié : il se partage avec ce dépôt-là).
pub fn open(root: &Path) -> Option<Repository> {
    let repo = Repository::open(root).ok()?;
    let workdir = repo.workdir()?.to_path_buf();
    let same = dunce::canonicalize(&workdir).ok()? == dunce::canonicalize(root).ok()?;
    same.then_some(repo)
}

/// « Relier » (Q10) : crée le dépôt s'il n'existe pas, sur la branche `branch`.
pub fn init(root: &Path, branch: &str) -> Result<Repository, CommandError> {
    if let Some(repo) = open(root) {
        return Ok(repo);
    }
    let mut options = RepositoryInitOptions::new();
    options.initial_head(branch);
    Repository::init_opts(root, &options).map_err(|e| failure("Création du dépôt impossible", &e))
}

/// Adresse du dépôt distant, sans identifiant (R9).
pub fn remote_url(repo: &Repository) -> Option<String> {
    repo.find_remote(ORIGIN)
        .ok()
        .and_then(|remote| remote.url().ok().map(str::to_owned))
}

pub fn set_remote(repo: &Repository, url: &str) -> Result<(), CommandError> {
    let result = if repo.find_remote(ORIGIN).is_ok() {
        repo.remote_set_url(ORIGIN, url)
    } else {
        repo.remote(ORIGIN, url).map(|_| ())
    };
    result.map_err(|e| failure("Le dépôt distant n'a pas pu être enregistré", &e))
}

/// Branche courante (`main`), sinon `None` (HEAD détaché).
pub fn current_branch(repo: &Repository) -> Option<String> {
    let head = repo.find_reference("HEAD").ok()?;
    let target = head.symbolic_target().ok()??;
    target.strip_prefix("refs/heads/").map(str::to_owned)
}

pub fn head_commit(repo: &Repository) -> Option<Commit<'_>> {
    repo.head().ok()?.peel_to_commit().ok()
}

/// Version de l'équipe connue à la dernière récupération (`origin/<branche>`).
pub fn upstream_commit<'r>(repo: &'r Repository, branch: &str) -> Option<Commit<'r>> {
    repo.find_branch(&format!("{ORIGIN}/{branch}"), BranchType::Remote)
        .ok()?
        .get()
        .peel_to_commit()
        .ok()
}

/// Contenu d'un fichier dans un arbre.
pub fn blob_at(repo: &Repository, tree: Option<&Tree<'_>>, path: &str) -> Option<Vec<u8>> {
    let entry = tree?.get_path(Path::new(path)).ok()?;
    if entry.kind() != Some(ObjectType::Blob) {
        return None;
    }
    let object = entry.to_object(repo).ok()?;
    object.as_blob().map(|blob| blob.content().to_vec())
}

/// Fichiers qui diffèrent d'un arbre à l'autre.
pub fn changed_paths(
    repo: &Repository,
    old: Option<&Tree<'_>>,
    new: Option<&Tree<'_>>,
) -> Result<Vec<String>, CommandError> {
    let diff = repo
        .diff_tree_to_tree(old, new, None)
        .map_err(|e| failure("Comparaison des versions impossible", &e))?;
    Ok(paths_of(&diff))
}

fn paths_of(diff: &Diff<'_>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for delta in diff.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            if let Some(path) = file.path().and_then(Path::to_str) {
                let path = path.replace('\\', "/");
                if !out.contains(&path) {
                    out.push(path);
                }
            }
        }
    }
    out
}

/// Fichiers modifiés dans le dossier de travail par rapport à `HEAD` (non envoyés), sans les
/// fichiers ignorés.
pub fn workdir_changes(repo: &Repository) -> Result<Vec<String>, CommandError> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(false);
    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(|e| failure("État du dépôt illisible", &e))?;
    let mut out = Vec::new();
    for entry in statuses.iter() {
        if entry.status().is_ignored() {
            continue;
        }
        if let Ok(path) = entry.path() {
            out.push(path.replace('\\', "/"));
        }
    }
    Ok(out)
}

/// Commit qui a touché `path` en dernier parmi ceux de `from` non contenus dans `hide`, avec
/// son auteur et sa date (millisecondes).
pub fn authors(
    repo: &Repository,
    from: Oid,
    hide: Option<Oid>,
) -> Result<BTreeMap<String, (String, u64)>, CommandError> {
    let mut walk = repo
        .revwalk()
        .map_err(|e| failure("Historique illisible", &e))?;
    walk.set_sorting(Sort::TIME)
        .map_err(|e| failure("Historique illisible", &e))?;
    walk.push(from)
        .map_err(|e| failure("Historique illisible", &e))?;
    if let Some(hide) = hide {
        let _ = walk.hide(hide);
    }
    let mut out = BTreeMap::new();
    for oid in walk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        let tree = commit.tree().ok();
        let parent = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let author = commit.author();
        let who = author
            .name()
            .ok()
            .or_else(|| author.email().ok())
            .unwrap_or("?")
            .to_owned();
        let when = u64::try_from(author.when().seconds()).unwrap_or(0) * 1000;
        for path in changed_paths(repo, parent.as_ref(), tree.as_ref())? {
            out.entry(path).or_insert_with(|| (who.clone(), when));
        }
    }
    Ok(out)
}

/// Commits qui ont touché `path`, du plus récent au plus ancien : (commit, auteur, date).
pub fn history(
    repo: &Repository,
    path: &str,
    limit: usize,
) -> Result<Vec<(Oid, String, u64, String)>, CommandError> {
    let Some(head) = head_commit(repo) else {
        return Ok(Vec::new());
    };
    let mut walk = repo
        .revwalk()
        .map_err(|e| failure("Historique illisible", &e))?;
    walk.set_sorting(Sort::TIME)
        .map_err(|e| failure("Historique illisible", &e))?;
    walk.push(head.id())
        .map_err(|e| failure("Historique illisible", &e))?;
    let mut out = Vec::new();
    for oid in walk.flatten() {
        if out.len() >= limit {
            break;
        }
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        let tree = commit.tree().ok();
        let parent = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let now = blob_at(repo, tree.as_ref(), path);
        let before = blob_at(repo, parent.as_ref(), path);
        if now != before {
            let author = commit.author();
            let who = author
                .name()
                .ok()
                .or_else(|| author.email().ok())
                .unwrap_or("?")
                .to_owned();
            let when = u64::try_from(author.when().seconds()).unwrap_or(0) * 1000;
            let message = commit
                .summary()
                .ok()
                .flatten()
                .unwrap_or_default()
                .to_owned();
            out.push((oid, who, when, message));
        }
    }
    Ok(out)
}

/// Récupère la branche `branch` du dépôt distant dans `origin/<branche>`.
pub fn fetch(repo: &Repository, branch: &str) -> Result<(), CommandError> {
    let mut remote = repo
        .find_remote(ORIGIN)
        .map_err(|e| failure("Le projet n'est relié à aucun dépôt", &e))?;
    let mut options = FetchOptions::new();
    options.download_tags(git2::AutotagOption::None);
    let refspec = format!("+refs/heads/{branch}:refs/remotes/{ORIGIN}/{branch}");
    remote
        .fetch(&[refspec.as_str()], Some(&mut options), None)
        .map_err(|e| failure("La récupération a échoué", &e))
}

/// Envoie la branche ; `Ok(false)` si le dépôt distant a avancé entre-temps (R11).
pub fn push(repo: &Repository, branch: &str) -> Result<bool, CommandError> {
    let mut remote = repo
        .find_remote(ORIGIN)
        .map_err(|e| failure("Le projet n'est relié à aucun dépôt", &e))?;
    let mut rejected: Option<String> = None;
    let result = {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.push_update_reference(|_, status| {
            if let Some(status) = status {
                rejected = Some(status.to_owned());
            }
            Ok(())
        });
        let mut options = PushOptions::new();
        options.remote_callbacks(callbacks);
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        remote.push(&[refspec.as_str()], Some(&mut options))
    };
    match result {
        Ok(()) if rejected.is_none() => {}
        Ok(()) => return Ok(false),
        Err(error) if error.code() == git2::ErrorCode::NotFastForward => return Ok(false),
        Err(error)
            if error.message().contains("non-fast-forward")
                || error.message().contains("fetch first") =>
        {
            return Ok(false);
        }
        Err(error) => return Err(failure("L'envoi a échoué", &error)),
    }
    // La version de l'équipe est maintenant la nôtre.
    if let Some(head) = head_commit(repo) {
        let _ = repo.reference(
            &format!("refs/remotes/{ORIGIN}/{branch}"),
            head.id(),
            true,
            "arnod : envoi",
        );
    }
    Ok(true)
}

/// Signature des commits : le compte relié (Q11), sinon celle du dépôt, sinon « Arnod ».
pub fn signature(repo: &Repository, name: Option<&str>, email: Option<&str>) -> Signature<'static> {
    if let (Some(name), Some(email)) = (name, email)
        && let Ok(signature) = Signature::now(name, email)
    {
        return signature;
    }
    repo.signature()
        .map(|s| s.to_owned())
        .or_else(|_| Signature::now("Arnod", "arnod@poste.local"))
        .unwrap_or_else(|_| {
            Signature::new("Arnod", "arnod@poste.local", &git2::Time::new(0, 0)).expect("signature")
        })
}

/// Enregistre dans un commit les fichiers `paths` du dossier de travail (ajoutés, modifiés ou
/// supprimés), sur `HEAD`.
pub fn commit_paths(
    repo: &Repository,
    paths: &[String],
    message: &str,
    signature: &Signature<'_>,
) -> Result<Oid, CommandError> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| CommandError::new(CommandErrorKind::Io, "Dépôt sans dossier de travail."))?
        .to_path_buf();
    let mut index = repo
        .index()
        .map_err(|e| failure("Index du dépôt illisible", &e))?;
    for path in paths {
        let result = if workdir.join(path).exists() {
            index.add_path(Path::new(path))
        } else {
            index.remove_path(Path::new(path))
        };
        result.map_err(|e| failure("Préparation de l'envoi impossible", &e))?;
    }
    index
        .write()
        .map_err(|e| failure("Index du dépôt illisible", &e))?;
    let tree_id = index
        .write_tree()
        .map_err(|e| failure("Préparation de l'envoi impossible", &e))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| failure("Préparation de l'envoi impossible", &e))?;
    let parent = head_commit(repo);
    let parents: Vec<&Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), signature, signature, message, &tree, &parents)
        .map_err(|e| failure("L'envoi n'a pas pu être préparé", &e))
}

/// Arbre de `base` où chaque fichier de `files` prend le contenu donné (`None` : supprimé).
pub fn tree_with(
    repo: &Repository,
    base: Option<&Tree<'_>>,
    files: &BTreeMap<String, Option<Vec<u8>>>,
) -> Result<Oid, CommandError> {
    let mut index = git2::Index::new().map_err(|e| failure("Fusion impossible", &e))?;
    if let Some(base) = base {
        index
            .read_tree(base)
            .map_err(|e| failure("Fusion impossible", &e))?;
    }
    for (path, content) in files {
        match content {
            Some(bytes) => {
                let id = repo
                    .blob(bytes)
                    .map_err(|e| failure("Fusion impossible", &e))?;
                let entry = IndexEntry {
                    ctime: IndexTime::new(0, 0),
                    mtime: IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100_644,
                    uid: 0,
                    gid: 0,
                    file_size: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
                    id,
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                };
                index
                    .add(&entry)
                    .map_err(|e| failure("Fusion impossible", &e))?;
            }
            None => {
                let _ = index.remove_path(Path::new(path));
            }
        }
    }
    index
        .write_tree_to(repo)
        .map_err(|e| failure("Fusion impossible", &e))
}

/// Déplace la branche sur `commit` et aligne l'index, sans toucher au dossier de travail (déjà
/// écrit par Arnod) : ce qui y diffère reste « à envoyer ».
pub fn move_branch(repo: &Repository, branch: &str, commit: Oid) -> Result<(), CommandError> {
    repo.reference(
        &format!("refs/heads/{branch}"),
        commit,
        true,
        "arnod : récupération",
    )
    .map_err(|e| failure("Récupération impossible", &e))?;
    repo.set_head(&format!("refs/heads/{branch}"))
        .map_err(|e| failure("Récupération impossible", &e))?;
    let object = repo
        .find_object(commit, None)
        .map_err(|e| failure("Récupération impossible", &e))?;
    repo.reset(&object, ResetType::Mixed, None)
        .map_err(|e| failure("Récupération impossible", &e))
}
