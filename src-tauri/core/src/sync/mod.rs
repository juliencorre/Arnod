//! Synchronisation d'un projet avec son équipe par Git (`docs/plan-v2.md`, lots E et F).
//!
//! Deux gestes : **récupérer** et **envoyer** ; Arnod récupère toujours avant d'envoyer (R11).
//! Une récupération fusionne élément par élément (R12, [`merge`]) ; un conflit non réglé ne
//! touche à rien (R13) : la fusion se calcule en mémoire et ne s'écrit qu'une fois complète. Ce
//! qui arrive et ce qui part se décrit en éléments partagés, en clair ([`describe`], R14). Seuls
//! les fichiers du projet voyagent (R10) ; l'état propre au poste est dans `sync.json`
//! ([`state`]). Les échanges passent par le réseau du poste ([`transport`], R9).
//!
//! Les opérations sont synchrones (libgit2) : l'appelant les lance hors d'un runtime tokio.

pub mod describe;
pub mod forge;
pub mod git;
pub mod merge;
pub mod state;
pub mod transport;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use git2::{Oid, Repository};
use serde::Serialize;
use uuid::Uuid;

use self::describe::{SharedItem, describe};
use self::forge::Forge;
use self::merge::{Auto, Choice, Element, Merged, merge_file};
use self::state::{Account, Received, SyncState};
use crate::error::{CommandError, CommandErrorKind};
use crate::network::{Network, now_ms};
use crate::secrets::SecretStore;
use crate::tree::{self, TreeNode};

/// Branche proposée en reliant un projet.
pub const DEFAULT_BRANCH: &str = "main";

/// Fichier ajouté au dépôt créé par « Relier » : les rapports d'`arnod run` ne voyagent pas.
const GITIGNORE: &str = "# Rapports d'arnod run (propres à chaque poste).\nrapports/\n";

/// Ce dont une opération a besoin.
pub struct SyncCtx<'a> {
    pub project: Uuid,
    pub name: &'a str,
    pub root: &'a Path,
    pub data_dir: &'a Path,
    pub network: Arc<Network>,
    pub secrets: Arc<dyn SecretStore>,
}

/// Dépôt distant relié.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncRemote {
    /// Adresse du dépôt, sans identifiant.
    pub url: String,
    pub forge: Forge,
    pub host: String,
    /// `acme/boutique-tests`.
    pub name: String,
    pub branch: String,
}

/// État de la synchronisation d'un projet, pour la barre d'état et le panneau (7.1-b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncView {
    pub project_id: Uuid,
    /// Le dossier du projet est un dépôt Git.
    pub repository: bool,
    /// Dépôt distant relié, sinon « Pas encore partagé ».
    pub remote: Option<SyncRemote>,
    pub account: Option<Account>,
    /// Ce qui arrive des collègues depuis la dernière récupération (vérifié sans rien écrire).
    pub incoming: Vec<SharedItem>,
    /// Mes modifications, pas encore envoyées.
    pub outgoing: Vec<SharedItem>,
    /// Éléments en conflit d'une récupération en attente (lot F).
    pub conflicts: usize,
    pub checked_at: Option<u64>,
    /// Dernière vérification impossible, en clair.
    pub error: Option<String>,
    /// Auteurs de l'historique du dépôt (pastilles du panneau).
    pub members: Vec<String>,
    /// Éléments changés par un collègue à la dernière récupération, avec son nom (pastilles de
    /// la barre latérale, CF4) : chemin de la requête ou du dossier → auteur.
    pub received: BTreeMap<String, String>,
}

/// Récupération en attente d'un choix (R13) : rien n'est écrit tant qu'elle n'est pas réglée.
#[derive(Debug, Clone)]
struct Pending {
    head: Option<Oid>,
    theirs: Oid,
    base: Option<Oid>,
    /// Contenus (version commune, la mienne, celle de l'équipe) des fichiers à fusionner.
    files: BTreeMap<String, [Option<Vec<u8>>; 3]>,
    authors: BTreeMap<String, (String, u64)>,
}

/// Récupérations en attente, par projet, et verrou des opérations (une à la fois).
#[derive(Default)]
pub struct SyncBook {
    pending: Mutex<HashMap<Uuid, Pending>>,
    busy: Mutex<()>,
}

/// Fichier en conflit (7.1-c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FileConflict {
    pub item: SharedItem,
    /// Auteur de la version de l'équipe.
    pub author: String,
    pub at: u64,
    /// Éléments à choisir.
    pub elements: Vec<Element>,
    /// Éléments pris sans question (CF2).
    pub merged: Vec<Auto>,
    /// Versions complètes, pour « Voir le YAML ».
    pub ours: Option<String>,
    pub theirs: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncConflicts {
    pub project_id: Uuid,
    pub conflicts: Vec<FileConflict>,
}

/// Issue d'une récupération : fichiers écrits ou supprimés, conflits en attente.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pulled {
    pub written: Vec<String>,
    pub removed: Vec<String>,
    pub conflicts: usize,
}

fn conflict_error(count: usize) -> CommandError {
    CommandError::new(
        CommandErrorKind::Conflict,
        if count > 1 {
            format!("{count} conflits à résoudre avant d'envoyer.")
        } else {
            "1 conflit à résoudre avant d'envoyer.".to_owned()
        },
    )
}

fn not_linked() -> CommandError {
    CommandError::new(
        CommandErrorKind::InvalidRequest,
        "Ce projet n'est relié à aucun dépôt.",
    )
}

impl SyncCtx<'_> {
    fn state(&self) -> SyncState {
        state::load(self.data_dir, self.project)
    }

    fn save(&self, state: &SyncState) -> Result<(), CommandError> {
        state::save(self.data_dir, self.project, state)
    }

    /// Contexte des échanges : jeton du compte relié, au coffre (R9).
    fn transport(&self, state: &SyncState) -> transport::Context {
        let authorization = state.account.as_ref().and_then(|account| {
            let token = self
                .secrets
                .get(&state::token_key(&account.host, &account.login))
                .ok()
                .flatten()?;
            Some(transport::basic(
                account.forge.git_user(&account.login),
                &token,
            ))
        });
        transport::Context {
            network: self.network.clone(),
            authorization,
        }
    }

    fn workdir(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.root.join(path)).ok()
    }
}

fn branch_of(repo: &Repository, state: &SyncState) -> String {
    state
        .branch
        .clone()
        .or_else(|| git::current_branch(repo))
        .unwrap_or_else(|| DEFAULT_BRANCH.to_owned())
}

/// Dossiers qui sont des tests (pour nommer une étape).
fn test_folders(root: &Path) -> BTreeSet<String> {
    fn walk(nodes: &[TreeNode], out: &mut BTreeSet<String>) {
        for node in nodes {
            if let TreeNode::Folder {
                path,
                children,
                test,
                ..
            } = node
            {
                if *test {
                    out.insert(path.clone());
                }
                walk(children, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&tree::read(root).unwrap_or_default(), &mut out);
    out
}

fn remote_of(repo: &Repository, state: &SyncState) -> Option<SyncRemote> {
    let url = git::remote_url(repo)?;
    let (forge, host, name) = forge::parse_remote(&url);
    let forge = state.account.as_ref().map_or(forge, |a| a.forge);
    Some(SyncRemote {
        url: forge::clean_url(&url),
        forge,
        host,
        name,
        branch: branch_of(repo, state),
    })
}

/// Version commune avec l'équipe : l'ancêtre commun de `HEAD` et de la version de l'équipe,
/// sinon celle de l'équipe (rien d'envoyé encore), sinon rien (jamais partagé).
fn common_tree<'r>(repo: &'r Repository, branch: &str) -> Option<git2::Tree<'r>> {
    let head = git::head_commit(repo);
    let upstream = git::upstream_commit(repo, branch)?;
    let base = match &head {
        Some(head) => repo.merge_base(head.id(), upstream.id()).ok()?,
        None => upstream.id(),
    };
    repo.find_commit(base).ok()?.tree().ok()
}

/// Mes modifications : fichiers changés depuis la version commune avec l'équipe (envoyés ou
/// non), jamais ceux que seule l'équipe a changés.
fn outgoing_paths(repo: &Repository, branch: &str) -> Result<Vec<String>, CommandError> {
    let mut paths = git::workdir_changes(repo)?;
    let head_tree = git::head_commit(repo).and_then(|c| c.tree().ok());
    if head_tree.is_some() {
        let base = common_tree(repo, branch);
        for path in git::changed_paths(repo, base.as_ref(), head_tree.as_ref())? {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

/// État du projet, sans réseau.
pub fn view(ctx: &SyncCtx<'_>, book: &SyncBook) -> SyncView {
    let state = ctx.state();
    let conflicts = book
        .pending
        .lock()
        .ok()
        .and_then(|pending| pending.get(&ctx.project).map(count_conflicts))
        .unwrap_or(0);
    let mut view = SyncView {
        project_id: ctx.project,
        repository: false,
        remote: None,
        account: state.account.clone(),
        incoming: Vec::new(),
        outgoing: Vec::new(),
        conflicts,
        checked_at: state.checked_at,
        error: state.error.clone(),
        members: Vec::new(),
        received: state
            .received
            .iter()
            .map(|(path, r)| (path.clone(), r.author.clone()))
            .collect(),
    };
    let Some(repo) = git::open(ctx.root) else {
        return view;
    };
    view.repository = true;
    view.remote = remote_of(&repo, &state);
    if view.remote.is_none() {
        return view;
    }
    let branch = branch_of(&repo, &state);
    let tests = test_folders(ctx.root);
    let upstream = git::upstream_commit(&repo, &branch);
    let upstream_tree = upstream.as_ref().and_then(|c| c.tree().ok());
    let common = common_tree(&repo, &branch);
    if let Ok(paths) = outgoing_paths(&repo, &branch) {
        for path in paths {
            let old = git::blob_at(&repo, common.as_ref(), &path);
            let new = ctx.workdir(&path);
            if old != new {
                view.outgoing
                    .push(describe(&path, old.as_deref(), new.as_deref(), &tests));
            }
        }
    }
    let head = git::head_commit(&repo);
    if let (Some(head), Some(upstream)) = (&head, &upstream)
        && let Ok(base) = repo.merge_base(head.id(), upstream.id())
        && base != upstream.id()
    {
        let base_tree = repo.find_commit(base).ok().and_then(|c| c.tree().ok());
        let authors = git::authors(&repo, upstream.id(), Some(head.id())).unwrap_or_default();
        if let Ok(paths) = git::changed_paths(&repo, base_tree.as_ref(), upstream_tree.as_ref()) {
            for path in paths {
                let old = git::blob_at(&repo, base_tree.as_ref(), &path);
                let new = git::blob_at(&repo, upstream_tree.as_ref(), &path);
                let mut item = describe(&path, old.as_deref(), new.as_deref(), &tests);
                if let Some((author, at)) = authors.get(&path) {
                    item.author = Some(author.clone());
                    item.at = Some(*at);
                }
                view.incoming.push(item);
            }
        }
    }
    if let Some(start) = upstream.as_ref().or(head.as_ref()) {
        let mut members: Vec<String> = git::authors(&repo, start.id(), None)
            .unwrap_or_default()
            .into_values()
            .map(|(author, _)| author)
            .collect();
        members.sort();
        members.dedup();
        view.members = members;
    }
    view
}

/// Vérifie ce qui arrive (Q6) : récupère l'historique de l'équipe sans rien écrire dans le
/// projet. Un échec est gardé et montré, sans erreur.
pub fn check(ctx: &SyncCtx<'_>, book: &SyncBook) -> SyncView {
    let _busy = book.busy.lock();
    let mut state = ctx.state();
    if let Some(repo) = git::open(ctx.root)
        && git::remote_url(&repo).is_some()
    {
        let branch = branch_of(&repo, &state);
        let fetched = transport::with_context(ctx.transport(&state), || git::fetch(&repo, &branch));
        state.checked_at = Some(now_ms());
        state.error = fetched.err().map(|e| e.message);
        let _ = ctx.save(&state);
    }
    view(ctx, book)
}

/// Écrit un fichier du projet (fusion d'une récupération), comme toute écriture d'Arnod.
fn write_file(root: &Path, path: &str, content: Option<&[u8]>) -> Result<(), CommandError> {
    let full = tree::resolve(root, path)?;
    match content {
        Some(bytes) => {
            if let Some(dir) = full.parent() {
                fs::create_dir_all(dir)
                    .map_err(|e| CommandError::io("Écriture impossible", dir, &e))?;
            }
            match std::str::from_utf8(bytes) {
                Ok(text) => crate::contract::file::write_text(&full, text),
                Err(_) => {
                    fs::write(&full, bytes)
                        .map_err(|e| CommandError::io("Écriture impossible", &full, &e))?;
                    crate::watch::record_write(&full, bytes);
                    Ok(())
                }
            }
        }
        None => {
            if full.exists() {
                fs::remove_file(&full)
                    .map_err(|e| CommandError::io("Suppression impossible", &full, &e))?;
                crate::watch::settle(&full);
            }
            Ok(())
        }
    }
}

/// « Relier et envoyer » (7.1-a, SY2, Q10) : crée le dépôt au besoin, le relie à `url`, puis
/// envoie le projet. Un dépôt distant qui contient déjà un autre projet est refusé.
pub fn link(
    ctx: &SyncCtx<'_>,
    book: &SyncBook,
    url: &str,
    branch: &str,
    account: Option<Account>,
) -> Result<SyncView, CommandError> {
    let url = forge::clean_url(url);
    if url.is_empty() {
        return Err(CommandError::new(
            CommandErrorKind::InvalidRequest,
            "Indiquez l'adresse du dépôt.",
        ));
    }
    let branch = branch.trim();
    let branch = if branch.is_empty() {
        DEFAULT_BRANCH
    } else {
        branch
    };
    {
        let _busy = book.busy.lock();
        let created = git::open(ctx.root).is_none();
        let repo = git::init(ctx.root, branch)?;
        if created && !ctx.root.join(".gitignore").exists() {
            write_file(ctx.root, ".gitignore", Some(GITIGNORE.as_bytes()))?;
        }
        git::set_remote(&repo, &url)?;
        let mut state = ctx.state();
        if account.is_some() {
            state.account = account;
        }
        state.branch = Some(branch.to_owned());
        state.error = None;
        ctx.save(&state)?;
        let context = ctx.transport(&state);
        transport::with_context(context, || -> Result<(), CommandError> {
            git::fetch(&repo, branch)?;
            let head = git::head_commit(&repo);
            let upstream = git::upstream_commit(&repo, branch);
            match (&head, &upstream) {
                (None, Some(_)) => {
                    return Err(CommandError::new(
                        CommandErrorKind::InvalidRequest,
                        "Ce dépôt contient déjà un projet : récupérez-le avec Git (clone), puis \
                         ouvrez son dossier dans Arnod.",
                    ));
                }
                (Some(head), Some(upstream))
                    if repo.merge_base(head.id(), upstream.id()).is_err() =>
                {
                    return Err(CommandError::new(
                        CommandErrorKind::InvalidRequest,
                        "Ce dépôt contient déjà un autre projet : choisissez un dépôt vide.",
                    ));
                }
                _ => {}
            }
            if head.is_none() {
                let paths = git::workdir_changes(&repo)?;
                let signature = signature(&repo, &state);
                git::commit_paths(
                    &repo,
                    &paths,
                    &format!("Partage du projet « {} »", ctx.name),
                    &signature,
                )?;
            }
            if !git::push(&repo, branch)? {
                return Err(CommandError::new(
                    CommandErrorKind::Network,
                    "Le dépôt distant a refusé l'envoi.",
                ));
            }
            Ok(())
        })?;
        let mut state = ctx.state();
        state.checked_at = Some(now_ms());
        ctx.save(&state)?;
    }
    Ok(view(ctx, book))
}

fn signature(repo: &Repository, state: &SyncState) -> git2::Signature<'static> {
    let account = state.account.as_ref();
    git::signature(
        repo,
        account.map(|a| a.name.as_str()),
        account.map(|a| a.email.as_str()),
    )
}

fn count_conflicts(pending: &Pending) -> usize {
    pending
        .files
        .iter()
        .filter(|(path, [base, ours, theirs])| {
            matches!(
                merge_file(
                    path,
                    base.as_deref(),
                    ours.as_deref(),
                    theirs.as_deref(),
                    &BTreeMap::new()
                ),
                Merged::Conflict { .. }
            )
        })
        .count()
}

/// Fusion de tous les fichiers ; `Err` donne le nombre de fichiers en conflit.
fn merge_all(
    pending: &Pending,
    choices: &BTreeMap<String, BTreeMap<String, Choice>>,
) -> Result<BTreeMap<String, Option<Vec<u8>>>, usize> {
    let empty = BTreeMap::new();
    let mut out = BTreeMap::new();
    let mut conflicts = 0;
    for (path, [base, ours, theirs]) in &pending.files {
        match merge_file(
            path,
            base.as_deref(),
            ours.as_deref(),
            theirs.as_deref(),
            choices.get(path).unwrap_or(&empty),
        ) {
            Merged::Clean(content) => {
                out.insert(path.clone(), content);
            }
            Merged::Conflict { .. } => conflicts += 1,
        }
    }
    if conflicts > 0 {
        Err(conflicts)
    } else {
        Ok(out)
    }
}

/// Applique une récupération réglée : fichiers du projet écrits, branche déplacée sur la
/// version de l'équipe (ou un commit de fusion s'il y avait des envois en attente).
fn apply(
    ctx: &SyncCtx<'_>,
    repo: &Repository,
    pending: &Pending,
    merged: &BTreeMap<String, Option<Vec<u8>>>,
) -> Result<Pulled, CommandError> {
    let mut state = ctx.state();
    let branch = branch_of(repo, &state);
    let mut pulled = Pulled::default();
    for (path, content) in merged {
        if ctx.workdir(path).as_deref() == content.as_deref() {
            continue;
        }
        write_file(ctx.root, path, content.as_deref())?;
        if content.is_some() {
            pulled.written.push(path.clone());
        } else {
            pulled.removed.push(path.clone());
        }
    }
    let target = match pending.head {
        Some(head) if Some(head) != pending.base => {
            // Envois en attente : commit de fusion, arbre de HEAD avec les versions fusionnées.
            let head_commit = repo
                .find_commit(head)
                .map_err(|e| git::failure("Récupération impossible", &e))?;
            let theirs_commit = repo
                .find_commit(pending.theirs)
                .map_err(|e| git::failure("Récupération impossible", &e))?;
            let head_tree = head_commit.tree().ok();
            let base_tree = pending
                .base
                .and_then(|b| repo.find_commit(b).ok())
                .and_then(|c| c.tree().ok());
            let theirs_tree = theirs_commit.tree().ok();
            let mut files: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
            for path in pending.files.keys() {
                let base = git::blob_at(repo, base_tree.as_ref(), path);
                let mine = git::blob_at(repo, head_tree.as_ref(), path);
                let theirs = git::blob_at(repo, theirs_tree.as_ref(), path);
                let content = if mine == base {
                    theirs
                } else if theirs == base {
                    mine
                } else {
                    merged.get(path).cloned().flatten()
                };
                files.insert(path.clone(), content);
            }
            let tree_id = git::tree_with(repo, head_tree.as_ref(), &files)?;
            let tree = repo
                .find_tree(tree_id)
                .map_err(|e| git::failure("Récupération impossible", &e))?;
            let signature = signature(repo, &state);
            repo.commit(
                None,
                &signature,
                &signature,
                "Récupération des modifications de l'équipe",
                &tree,
                &[&head_commit, &theirs_commit],
            )
            .map_err(|e| git::failure("Récupération impossible", &e))?
        }
        _ => pending.theirs,
    };
    git::move_branch(repo, &branch, target)?;
    // Pastilles (CF4) : ce que les collègues ont changé à cette récupération.
    let me = state.account.as_ref().map(|a| a.name.clone());
    state.received = pending
        .files
        .keys()
        .filter_map(|path| {
            let (author, at) = pending.authors.get(path)?;
            (Some(author) != me.as_ref()).then(|| {
                (
                    path.clone(),
                    Received {
                        author: author.clone(),
                        at: *at,
                    },
                )
            })
        })
        .collect();
    ctx.save(&state)?;
    Ok(pulled)
}

/// Récupère (SY6, SY7) : fusion élément par élément (R12) ; en conflit, rien n'est écrit et la
/// récupération attend un choix (R13).
pub fn pull(ctx: &SyncCtx<'_>, book: &SyncBook) -> Result<Pulled, CommandError> {
    let _busy = book.busy.lock();
    pull_locked(ctx, book)
}

fn pull_locked(ctx: &SyncCtx<'_>, book: &SyncBook) -> Result<Pulled, CommandError> {
    let repo = git::open(ctx.root).ok_or_else(not_linked)?;
    git::remote_url(&repo).ok_or_else(not_linked)?;
    let mut state = ctx.state();
    let branch = branch_of(&repo, &state);
    transport::with_context(ctx.transport(&state), || git::fetch(&repo, &branch))?;
    state.checked_at = Some(now_ms());
    state.error = None;
    ctx.save(&state)?;
    let Some(theirs) = git::upstream_commit(&repo, &branch) else {
        return Ok(Pulled::default());
    };
    let head = git::head_commit(&repo);
    let base = match &head {
        Some(head) => Some(
            repo.merge_base(head.id(), theirs.id())
                .map_err(|e| git::failure("Historiques sans ancêtre commun", &e))?,
        ),
        None => None,
    };
    if base == Some(theirs.id()) {
        return Ok(Pulled::default());
    }
    let base_tree = base
        .and_then(|b| repo.find_commit(b).ok())
        .and_then(|c| c.tree().ok());
    let theirs_tree = theirs.tree().ok();
    let mut files = BTreeMap::new();
    for path in git::changed_paths(&repo, base_tree.as_ref(), theirs_tree.as_ref())? {
        let base_content = git::blob_at(&repo, base_tree.as_ref(), &path);
        let theirs_content = git::blob_at(&repo, theirs_tree.as_ref(), &path);
        files.insert(
            path.clone(),
            [base_content, ctx.workdir(&path), theirs_content],
        );
    }
    let pending = Pending {
        head: head.as_ref().map(git2::Commit::id),
        theirs: theirs.id(),
        base,
        authors: git::authors(&repo, theirs.id(), head.as_ref().map(git2::Commit::id))?,
        files,
    };
    match merge_all(&pending, &BTreeMap::new()) {
        Ok(merged) => {
            let pulled = apply(ctx, &repo, &pending, &merged)?;
            if let Ok(mut all) = book.pending.lock() {
                all.remove(&ctx.project);
            }
            Ok(pulled)
        }
        Err(conflicts) => {
            if let Ok(mut all) = book.pending.lock() {
                all.insert(ctx.project, pending);
            }
            Ok(Pulled {
                conflicts,
                ..Pulled::default()
            })
        }
    }
}

/// Envoie les éléments choisis (`keys`) avec `message` (SY6) ; `pull_first` récupère d'abord
/// (« Récupérer puis envoyer »). Un envoi refusé parce que l'équipe a avancé récupère de nouveau
/// puis renvoie, jamais de force (R11).
pub fn send(
    ctx: &SyncCtx<'_>,
    book: &SyncBook,
    keys: &[String],
    message: &str,
    pull_first: bool,
) -> Result<(SyncView, Pulled), CommandError> {
    let mut pulled = Pulled::default();
    {
        let _busy = book.busy.lock();
        if let Some(count) = book
            .pending
            .lock()
            .ok()
            .and_then(|p| p.get(&ctx.project).map(count_conflicts))
            .filter(|c| *c > 0)
        {
            return Err(conflict_error(count));
        }
        if pull_first {
            pulled = pull_locked(ctx, book)?;
            if pulled.conflicts > 0 {
                return Err(conflict_error(pulled.conflicts));
            }
        }
        let repo = git::open(ctx.root).ok_or_else(not_linked)?;
        let state = ctx.state();
        let branch = branch_of(&repo, &state);
        let available = outgoing_paths(&repo, &branch)?;
        let chosen: Vec<String> = available
            .into_iter()
            .filter(|path| keys.contains(path))
            .collect();
        let workdir = git::workdir_changes(&repo)?;
        let to_commit: Vec<String> = chosen
            .iter()
            .filter(|p| workdir.contains(p))
            .cloned()
            .collect();
        if !to_commit.is_empty() {
            let message = message.trim();
            let message = if message.is_empty() {
                format!(
                    "{} élément{} de « {} »",
                    to_commit.len(),
                    if to_commit.len() > 1 { "s" } else { "" },
                    ctx.name
                )
            } else {
                message.to_owned()
            };
            git::commit_paths(&repo, &to_commit, &message, &signature(&repo, &state))?;
        }
        let context = ctx.transport(&state);
        let mut sent = transport::with_context(context.clone(), || git::push(&repo, &branch))?;
        if !sent {
            // L'équipe a avancé entre-temps : récupérer, puis renvoyer (R11).
            let again = pull_locked(ctx, book)?;
            if again.conflicts > 0 {
                return Err(conflict_error(again.conflicts));
            }
            pulled.written.extend(again.written);
            pulled.removed.extend(again.removed);
            sent = transport::with_context(context, || git::push(&repo, &branch))?;
        }
        if !sent {
            return Err(CommandError::new(
                CommandErrorKind::Network,
                "L'équipe a encore envoyé des modifications : réessayez.",
            ));
        }
    }
    Ok((view(ctx, book), pulled))
}

/// Conflits de la récupération en attente (7.1-c, CF1, CF2).
pub fn conflicts(ctx: &SyncCtx<'_>, book: &SyncBook) -> SyncConflicts {
    let pending = book
        .pending
        .lock()
        .ok()
        .and_then(|all| all.get(&ctx.project).cloned());
    let tests = test_folders(ctx.root);
    let text = |bytes: &Option<Vec<u8>>| {
        bytes
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
    };
    let conflicts = pending
        .map(|pending| {
            pending
                .files
                .iter()
                .filter_map(|(path, [base, ours, theirs])| {
                    let Merged::Conflict { elements, auto } = merge_file(
                        path,
                        base.as_deref(),
                        ours.as_deref(),
                        theirs.as_deref(),
                        &BTreeMap::new(),
                    ) else {
                        return None;
                    };
                    let (author, at) = pending.authors.get(path).cloned().unwrap_or_default();
                    Some(FileConflict {
                        item: describe(path, base.as_deref(), theirs.as_deref(), &tests),
                        author,
                        at,
                        elements,
                        merged: auto,
                        ours: text(ours),
                        theirs: text(theirs),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    SyncConflicts {
        project_id: ctx.project,
        conflicts,
    }
}

/// « Valider la résolution » (CF3) : avec un choix pour chaque élément en conflit, la
/// récupération s'écrit ; sinon rien ne change.
pub fn resolve(
    ctx: &SyncCtx<'_>,
    book: &SyncBook,
    choices: &BTreeMap<String, BTreeMap<String, Choice>>,
) -> Result<Pulled, CommandError> {
    let _busy = book.busy.lock();
    let pending = book
        .pending
        .lock()
        .ok()
        .and_then(|all| all.get(&ctx.project).cloned())
        .ok_or_else(|| {
            CommandError::new(CommandErrorKind::NotFound, "Aucun conflit en attente.")
        })?;
    let merged = merge_all(&pending, choices).map_err(|count| {
        CommandError::new(
            CommandErrorKind::InvalidRequest,
            format!(
                "Choisissez une version pour chaque élément ({count} fichier{} en attente).",
                if count > 1 { "s" } else { "" }
            ),
        )
    })?;
    let repo = git::open(ctx.root).ok_or_else(not_linked)?;
    let pulled = apply(ctx, &repo, &pending, &merged)?;
    if let Ok(mut all) = book.pending.lock() {
        all.remove(&ctx.project);
    }
    Ok(pulled)
}

/// Abandonne la récupération en attente : le projet reste tel qu'avant (R13).
pub fn abandon(ctx: &SyncCtx<'_>, book: &SyncBook) {
    if let Ok(mut all) = book.pending.lock() {
        all.remove(&ctx.project);
    }
}

/// Une entrée de l'historique d'un élément (7.1-d, CF5, R14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub author: String,
    pub at: u64,
    /// Ce qui a changé, en clair.
    pub detail: String,
    /// Message de l'envoi.
    pub message: String,
    /// Versions avant et après, pour « Voir ce qui a changé ».
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Historique d'un élément du projet, du plus récent au plus ancien (CF5).
pub fn journal(ctx: &SyncCtx<'_>, path: &str) -> Vec<JournalEntry> {
    let Some(repo) = git::open(ctx.root) else {
        return Vec::new();
    };
    let tests = test_folders(ctx.root);
    git::history(&repo, path, 50)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(oid, author, at, message)| {
            let commit = repo.find_commit(oid).ok()?;
            let tree = commit.tree().ok();
            let parent = commit.parent(0).ok().and_then(|p| p.tree().ok());
            let after = git::blob_at(&repo, tree.as_ref(), path);
            let before = git::blob_at(&repo, parent.as_ref(), path);
            let item = describe(path, before.as_deref(), after.as_deref(), &tests);
            let text =
                |bytes: Option<Vec<u8>>| bytes.map(|b| String::from_utf8_lossy(&b).into_owned());
            Some(JournalEntry {
                author,
                at,
                detail: item.detail,
                message,
                before: text(before),
                after: text(after),
            })
        })
        .collect()
}

/// Changements reçus des collègues à la dernière récupération (pastilles, CF4) : chemin →
/// auteur.
pub fn received(ctx: &SyncCtx<'_>) -> BTreeMap<String, String> {
    ctx.state()
        .received
        .into_iter()
        .map(|(path, received)| (path, received.author))
        .collect()
}
