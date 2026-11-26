//! Miroir des types IPC : déclarations TypeScript générées depuis le Rust.
//!
//! Les types échangés restent écrits à la main dans `src/types/ipc.ts` ; ts-rs (dépendance de
//! test seulement, Q2 de `docs/plan-qualite-mvp.md`) déduit de chaque type sérialisé sa forme
//! TypeScript, attributs serde compris. Le test compare `tests/ipc/coeur.ts` à ce qu'il génère ;
//! `tests/ipc/miroir.ts` vérifie ensuite à la compilation (vue-tsc, `npm run check`) que chaque
//! type de `src/types/ipc.ts` est identique à son homologue généré du même nom (`ts(rename)`
//! donne au type Rust le nom de son miroir).
//!
//! Régénérer après une modification d'un type sérialisé :
//! `ARNOD_IPC_ECRIRE=1 cargo test --lib ts_mirror`.

use std::any::TypeId;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use ts_rs::{Config, TS, TypeVisitor};

/// Fichier généré, relatif à la racine du dépôt.
const GENERATED: &str = "tests/ipc/coeur.ts";

/// Collecte la déclaration de chaque type rencontré, dépendances comprises.
struct Collect {
    cfg: Config,
    seen: HashSet<TypeId>,
    decls: BTreeMap<String, String>,
}

impl TypeVisitor for Collect {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        // Types primitifs, conteneurs et enveloppes : rien à déclarer.
        if T::output_path().is_none() || !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        let name = T::ident(&self.cfg);
        let decl = without_docs(&T::decl(&self.cfg));
        if let Some(previous) = self.decls.insert(name.clone(), decl.clone()) {
            assert_eq!(
                previous, decl,
                "deux types Rust se nomment `{name}` en TypeScript : ajouter `ts(rename)`"
            );
        }
        T::visit_dependencies(self);
    }
}

/// Déclaration sans ses commentaires `/** … */` (ils restent dans le Rust).
fn without_docs(decl: &str) -> String {
    let mut out = String::with_capacity(decl.len());
    let mut rest = decl;
    while let Some(start) = rest.find("/**") {
        out.push_str(rest[..start].trim_end_matches([' ', '\n']));
        let after = &rest[start..];
        let end = after.find("*/").map_or(after.len(), |i| i + 2);
        rest = after[end..].trim_start();
        if !out.is_empty() {
            out.push(' ');
        }
    }
    out.push_str(rest);
    out
}

/// Noms exportés par `src/types/ipc.ts` (`export type X` ou `export interface X`).
fn ipc_names() -> Vec<String> {
    crate::ts_contract::IPC_TS
        .lines()
        .filter_map(|line| {
            let rest = line
                .strip_prefix("export type ")
                .or_else(|| line.strip_prefix("export interface "))?;
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            Some(rest[..end].to_owned())
        })
        .collect()
}

/// Visite chaque type sérialisé du cœur.
fn visit_all(v: &mut Collect) {
    v.visit::<crate::bruno::ItemKind>();
    v.visit::<crate::bruno::ItemStatus>();
    v.visit::<crate::bruno::ReportItem>();
    v.visit::<crate::bruno::ConversionExample>();
    v.visit::<crate::bruno::ImportCounts>();
    v.visit::<crate::catalog::model::Project>();
    v.visit::<crate::catalog::model::Workspace>();
    v.visit::<crate::catalog::model::WorkspaceColor>();
    v.visit::<crate::catalog::model::CatalogFile>();
    v.visit::<crate::catalog::model::CatalogFileV1>();
    v.visit::<crate::catalog::snapshot::CatalogLoad>();
    v.visit::<crate::catalog::snapshot::Undoable>();
    v.visit::<crate::catalog::snapshot::ProjectCreated>();
    v.visit::<crate::catalog::snapshot::WorkspaceCreated>();
    v.visit::<crate::catalog::snapshot::CatalogSnapshot>();
    v.visit::<crate::catalog::snapshot::ProjectView>();
    v.visit::<crate::catalog::snapshot::ProjectStatus>();
    v.visit::<crate::catalog::snapshot::WorkspaceView>();
    v.visit::<crate::commands::batch::BatchResult>();
    v.visit::<crate::commands::hello::HelloResponse>();
    v.visit::<crate::commands::input::CurlCopy>();
    v.visit::<crate::commands::network::ProjectAuthView>();
    v.visit::<crate::commands::requests::ProposeInput>();
    v.visit::<crate::commands::requests::ExampleInput>();
    v.visit::<crate::commands::transfer::TransferDone>();
    v.visit::<crate::commands::tree::TreeView>();
    v.visit::<crate::commands::trial::TrialResult>();
    v.visit::<crate::commands::trial::FiledTrials>();
    v.visit::<crate::commands::window::WindowContext>();
    v.visit::<crate::contract::file::ContractSummary>();
    v.visit::<crate::contract::file::ResourceView>();
    v.visit::<crate::contract::file::OperationNode>();
    v.visit::<crate::contract::file::LinkedView>();
    v.visit::<crate::contract::file::ExampleRef>();
    v.visit::<crate::contract::file::OperationView>();
    v.visit::<crate::contract::file::OrphanView>();
    v.visit::<crate::contract::file::ContractView>();
    v.visit::<crate::contract::file::ContractSource>();
    v.visit::<crate::contract::file::SkippedOperation>();
    v.visit::<crate::contract::file::BaseUrlSet>();
    v.visit::<crate::contract::file::GenerateReport>();
    v.visit::<crate::contract::parse::SpecKind>();
    v.visit::<crate::curl::parse::Ignored>();
    v.visit::<crate::curl::parse::Secret>();
    v.visit::<crate::environment::Variable>();
    v.visit::<crate::environment::Environment>();
    v.visit::<crate::error::CommandErrorKind>();
    v.visit::<crate::error::CommandError>();
    v.visit::<crate::http::HttpResponse>();
    v.visit::<crate::http::Timeline>();
    v.visit::<crate::import::SourceFormat>();
    v.visit::<crate::import::ImportSource>();
    v.visit::<crate::import::ImportTarget>();
    v.visit::<crate::import::ImportReport>();
    v.visit::<crate::import::ImportDone>();
    v.visit::<crate::input::added::AddedOrigin>();
    v.visit::<crate::input::added::AddedHeader>();
    v.visit::<crate::input::RequestInput>();
    v.visit::<crate::input::secret::ClearSecret>();
    v.visit::<crate::input::text::PairsFormat>();
    v.visit::<crate::input::text::TextError>();
    v.visit::<crate::input::text::PairsText>();
    v.visit::<crate::input::variables::VariableSource>();
    v.visit::<crate::input::variables::UnknownVariable>();
    v.visit::<crate::layout::Content>();
    v.visit::<crate::layout::Tab>();
    v.visit::<crate::layout::Placement>();
    v.visit::<crate::layout::Zone>();
    v.visit::<crate::layout::Axis>();
    v.visit::<crate::layout::Side>();
    v.visit::<crate::layout::Node>();
    v.visit::<crate::layout::Preset>();
    v.visit::<crate::layout::Layout>();
    v.visit::<crate::layout::windows::WindowKind>();
    v.visit::<crate::layout::windows::Bounds>();
    v.visit::<crate::layout::windows::WindowState>();
    v.visit::<crate::layout::windows::ActiveRequest>();
    v.visit::<crate::layout::windows::WindowsFile>();
    v.visit::<crate::local::LocalVar>();
    v.visit::<crate::local::LocalData>();
    v.visit::<crate::menu::Shortcut>();
    v.visit::<crate::network::cert::CertificateInfo>();
    v.visit::<crate::network::cookies::StoredCookie>();
    v.visit::<crate::network::cookies::Jar>();
    v.visit::<crate::network::cookies::CookieView>();
    v.visit::<crate::network::cookies::CookieDomain>();
    v.visit::<crate::network::cookies::CookieJarView>();
    v.visit::<crate::network::failure::FailureCause>();
    v.visit::<crate::network::failure::CertificateFailure>();
    v.visit::<crate::network::failure::NetworkFailure>();
    v.visit::<crate::network::AuthorityView>();
    v.visit::<crate::network::NetworkView>();
    v.visit::<crate::network::NetworkPrefsInput>();
    v.visit::<crate::network::oauth::TokenStatus>();
    v.visit::<crate::network::prefs::ManualProxy>();
    v.visit::<crate::network::prefs::ProxyPrefs>();
    v.visit::<crate::network::prefs::StoredAuthority>();
    v.visit::<crate::network::prefs::NetworkPrefs>();
    v.visit::<crate::network::project_auth::AuthOverride>();
    v.visit::<crate::network::project_auth::AuthUsage>();
    v.visit::<crate::network::proxy::ProxyMode>();
    v.visit::<crate::network::proxy::ProxySource>();
    v.visit::<crate::network::proxy::SystemProxy>();
    v.visit::<crate::network::tls::CertificateProblem>();
    v.visit::<crate::request::assertions::AssertionResult>();
    v.visit::<crate::request::dynamic::DynamicValue>();
    v.visit::<crate::request::extract::ExtractedValue>();
    v.visit::<crate::request::model::Method>();
    v.visit::<crate::request::model::ParamKind>();
    v.visit::<crate::request::model::Pair>();
    v.visit::<crate::request::model::AuthKind>();
    v.visit::<crate::request::model::CredentialsPlacement>();
    v.visit::<crate::request::model::TokenPlacement>();
    v.visit::<crate::request::model::OAuth2>();
    v.visit::<crate::request::model::ApiKeyLocation>();
    v.visit::<crate::request::model::Auth>();
    v.visit::<crate::request::model::BodyKind>();
    v.visit::<crate::request::model::BodyVariant>();
    v.visit::<crate::request::model::Body>();
    v.visit::<crate::request::model::Scalar>();
    v.visit::<crate::request::model::Operator>();
    v.visit::<crate::request::model::Origin>();
    v.visit::<crate::request::model::Assertion>();
    v.visit::<crate::request::model::Extraction>();
    v.visit::<crate::request::model::ContractLink>();
    v.visit::<crate::request::model::Settings>();
    v.visit::<crate::request::model::ResolvedSettings>();
    v.visit::<crate::request::model::ExampleBodyKind>();
    v.visit::<crate::request::model::ResponseHeader>();
    v.visit::<crate::request::model::Example>();
    v.visit::<crate::request::model::InheritedAuth>();
    v.visit::<crate::request::model::InheritedHeader>();
    v.visit::<crate::request::model::Inherited>();
    v.visit::<crate::request::model::Request>();
    v.visit::<crate::request::resolve::DrawnValue>();
    v.visit::<crate::response::filter::FilterMatch>();
    v.visit::<crate::response::filter::FilterResult>();
    v.visit::<crate::response::format::ResponseFormat>();
    v.visit::<crate::response::preview::HtmlNode>();
    v.visit::<crate::response::render::TokenKind>();
    v.visit::<crate::response::render::Token>();
    v.visit::<crate::response::render::RenderedBody>();
    v.visit::<crate::results::RequestResult>();
    v.visit::<crate::results::ResultChange>();
    v.visit::<crate::startup::ProblemNature>();
    v.visit::<crate::startup::StartupAction>();
    v.visit::<crate::startup::StartupTarget>();
    v.visit::<crate::startup::StartupOption>();
    v.visit::<crate::startup::RetryPlan>();
    v.visit::<crate::startup::DetailRow>();
    v.visit::<crate::startup::StartupProblem>();
    v.visit::<crate::statuses::RequestStatus>();
    v.visit::<crate::steps::definition::Family>();
    v.visit::<crate::steps::definition::Engine>();
    v.visit::<crate::steps::definition::Subject>();
    v.visit::<crate::steps::definition::Rule>();
    v.visit::<crate::steps::definition::FieldKind>();
    v.visit::<crate::steps::definition::VisibleWhen>();
    v.visit::<crate::steps::definition::Field>();
    v.visit::<crate::steps::definition::Definition>();
    v.visit::<crate::steps::detect::CurlImport>();
    v.visit::<crate::steps::detect::Detection>();
    v.visit::<crate::steps::Origin>();
    v.visit::<crate::steps::StepNature>();
    v.visit::<crate::steps::CatalogSource>();
    v.visit::<crate::steps::StepCatalog>();
    v.visit::<crate::tree::batch::BatchAction>();
    v.visit::<crate::tree::batch::BatchDone>();
    v.visit::<crate::tree::TreeNode>();
    v.visit::<crate::tree::CreatedTest>();
    v.visit::<crate::tree::StepView>();
    v.visit::<crate::tree::TestView>();
    v.visit::<crate::tree::VariableUser>();
    v.visit::<crate::tree::transfer::Mode>();
    v.visit::<crate::tree::transfer::Target>();
    v.visit::<crate::tree::transfer::OperationTarget>();
    v.visit::<crate::tree::transfer::FolderTarget>();
    v.visit::<crate::tree::transfer::TestTarget>();
    v.visit::<crate::tree::transfer::Targets>();
    v.visit::<crate::tree::transfer::Change>();
    v.visit::<crate::tree::transfer::Transferred>();
    v.visit::<crate::trial::Transferred>();
    v.visit::<crate::trial::filing::Filed>();
    v.visit::<crate::trial::generalize::PathParam>();
    v.visit::<crate::trial::generalize::Template>();
    v.visit::<crate::trial::generalize::Suggestion>();
    v.visit::<crate::updates::Channel>();
    v.visit::<crate::updates::UpdatePrefs>();
    v.visit::<crate::updates::GhRelease>();
    v.visit::<crate::updates::GhAsset>();
    v.visit::<crate::updates::Release>();
    v.visit::<crate::updates::InstallState>();
    v.visit::<crate::updates::UpdatesView>();
    v.visit::<crate::watch::conflict::ConflictSide>();
    v.visit::<crate::watch::conflict::ConflictItem>();
    v.visit::<crate::watch::conflict::RequestConflict>();
    v.visit::<crate::watch::Renamed>();
    v.visit::<crate::watch::FilesChange>();
    v.visit::<crate::windows::StatusesChange>();
    v.visit::<crate::windows::RunChange>();
    v.visit::<crate::windows::ProjectScope>();
    v.visit::<crate::windows::ProjectChange>();
    v.visit::<crate::workbench::LocalVarView>();
    v.visit::<crate::workbench::EnvironmentView>();
    v.visit::<crate::workbench::EnvironmentsView>();
    v.visit::<crate::commands::environments::EnvironmentDeleted>();
    v.visit::<crate::workbench::SendOutcome>();
    v.visit::<crate::workbench::UsedVariable>();
    v.visit::<crate::workbench::UsedSource>();
    v.visit::<crate::workbench::Sent>();
    v.visit::<crate::workbench::ItemStatus>();
    v.visit::<crate::workbench::RunItem>();
    v.visit::<crate::workbench::RunTest>();
    v.visit::<crate::workbench::RunGroup>();
    v.visit::<crate::workbench::RunView>();
    v.visit::<crate::workbench::ProjectLastRun>();
}

/// Contenu attendu de `tests/ipc/coeur.ts`.
fn generate() -> String {
    let mut collect = Collect {
        cfg: Config::new().with_large_int("number"),
        seen: HashSet::new(),
        decls: BTreeMap::new(),
    };
    visit_all(&mut collect);
    let mut out = String::from(
        "// Généré par `ARNOD_IPC_ECRIRE=1 cargo test --lib ts_mirror` (src-tauri/src/ts_mirror.rs) :\n\
         // forme TypeScript des types sérialisés du cœur. Ne pas modifier à la main.\n\n\
         import type * as Ipc from \"@/types/ipc\";\n",
    );
    for decl in collect.decls.values() {
        out.push_str("\nexport ");
        out.push_str(decl.trim());
        out.push('\n');
    }
    let (shared, alone): (Vec<String>, Vec<String>) = ipc_names()
        .into_iter()
        .partition(|name| collect.decls.contains_key(name));
    out.push_str(
        "\n// --- Concordance avec src/types/ipc.ts (vérifiée par vue-tsc) ---\n\n\
         /** Vrai si chaque type est assignable à l'autre et qu'ils ont les mêmes clés. */\n\
         type Same<A, B> = [A] extends [B]\n  \
         ? [B] extends [A]\n    \
         ? [keyof A] extends [keyof B]\n      \
         ? [keyof B] extends [keyof A]\n        \
         ? true\n        \
         : false\n      \
         : false\n    \
         : false\n  \
         : false;\n\
         type Check<T extends true> = T;\n\n",
    );
    for name in &shared {
        out.push_str(&format!(
            "export type Check{name} = Check<Same<Ipc.{name}, {name}>>;\n"
        ));
    }
    out.push_str("\n// Sans homologue généré (écrits seulement côté interface) :\n");
    for name in &alone {
        out.push_str(&format!("// - {name}\n"));
    }
    out
}

/// Compare `relative` (depuis la racine du dépôt) à `expected`, ou l'écrit avec
/// `ARNOD_IPC_ECRIRE=1`.
fn check_or_write(relative: &str, expected: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    if std::env::var_os("ARNOD_IPC_ECRIRE").is_some_and(|v| !v.is_empty() && v != "0") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, expected).unwrap();
        return;
    }
    let actual = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        actual.replace("\r\n", "\n") == expected,
        "{relative} n'est pas à jour : lancer `ARNOD_IPC_ECRIRE=1 cargo test --lib ts_mirror` \
         dans src-tauri, puis corriger src/types/ipc.ts ou le faux cœur (tests/) si \
         `npm run check` signale un écart"
    );
}

#[test]
fn ts_mirror_a_jour() {
    check_or_write(GENERATED, &generate());
}

/// Échantillons produits par le vrai cœur, que le faux cœur (`tests/fakeWorkbench.ts`) doit
/// reproduire pour le même projet (`tests/ipc/echantillons.spec.ts`).
const SAMPLES: &str = "tests/ipc/echantillons.json";

/// Projet de `openProject` (`tests/workbench/helpers.ts`) : le dossier « Clients » et ses deux
/// requêtes, « Santé » à la racine, et un fichier illisible.
fn samples() -> String {
    use crate::request::{file, inherit, model::Method, model::Request};
    use crate::tree;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("opencollection.yml"),
        "opencollection: 1.0.0\ninfo:\n  name: boutique-tests\n",
    )
    .unwrap();
    std::fs::create_dir(root.join("Clients")).unwrap();
    let write = |path: &str, name: &str, method: Method, url: &str| {
        let mut request = Request::new(name);
        request.method = method;
        request.url = url.to_owned();
        file::write(&root.join(path), &request).unwrap();
    };
    write(
        "Clients/lister.yml",
        "Lister",
        Method::Get,
        "{{baseUrl}}/clients",
    );
    write("Clients/creer.yml", "Créer", Method::Post, "{{baseUrl}}/");
    write("sante.yml", "Santé", Method::Get, "{{baseUrl}}/");
    std::fs::write(root.join("casse.yml"), "info: [\n").unwrap();

    let nodes = tree::read(root).unwrap();
    let tests = tree::tests_view(&nodes, &HashSet::new());
    let tree_view = crate::commands::tree::TreeView {
        nodes,
        tests,
        collapsed: Vec::new(),
    };
    let mut request = file::read(&root.join("Clients/lister.yml")).unwrap();
    request.inherited = inherit::resolve(root, "Clients/lister.yml");
    let mut out = serde_json::to_string_pretty(&serde_json::json!({
        "treeLoad": tree_view,
        "requestRead": request,
    }))
    .unwrap();
    out.push('\n');
    out
}

#[test]
fn echantillons_a_jour() {
    check_or_write(SAMPLES, &samples());
}
