/**
 * Types échangés entre l'interface et le cœur Rust via IPC.
 *
 * Maintenus à la main : chaque type doit refléter exactement la
 * sérialisation serde de son équivalent Rust (champs en camelCase).
 * Toute modification côté Rust doit être reportée ici, et inversement.
 */

/**
 * Noms des commandes Rust enregistrées dans `generate_handler!` (`src-tauri/src/lib.rs`).
 *
 * Un test Rust vérifie que cette liste concorde avec les commandes enregistrées.
 */
export type CommandName =
  | "hello"
  | "catalog_load"
  | "undo"
  | "project_suggest_folder"
  | "project_pick_folder"
  | "project_create"
  | "project_add_existing"
  | "project_forget"
  | "project_open"
  | "workspace_open"
  | "workspace_create"
  | "workspace_rename"
  | "workspace_set_color"
  | "workspace_add_projects"
  | "workspace_remove_project"
  | "workspace_delete"
  | "window_set_title"
  | "windows_load"
  | "layout_open"
  | "layout_open_selection"
  | "layout_activate"
  | "layout_close"
  | "layout_move"
  | "layout_split"
  | "layout_resize"
  | "layout_preset"
  | "layout_set_placement"
  | "layout_set_follow"
  | "layout_set_sidebar_width"
  | "window_detach_zone"
  | "window_detach_tab"
  | "window_detach_sidebar"
  | "window_detach_result"
  | "window_reattach"
  | "window_reattach_all"
  | "tree_load"
  | "tree_set_collapsed"
  | "tree_create_folder"
  | "tree_create_request"
  | "tree_rename"
  | "tree_move"
  | "tree_duplicate"
  | "tree_delete"
  | "request_read"
  | "request_save"
  | "request_send"
  | "request_cancel"
  | "request_result"
  | "request_statuses"
  | "result_ignore_check"
  | "request_propose_assertions"
  | "request_example_from_response"
  | "environments_load"
  | "environment_select"
  | "environment_save"
  | "environment_delete"
  | "environments_reorder"
  | "local_set"
  | "run_start"
  | "run_status"
  | "run_cancel"
  | "run_retry_failures"
  | "run_last"
  | "run_ignore_check"
  | "project_last_runs"
  | "contract_list"
  | "contract_read"
  | "contract_add"
  | "contract_import_file"
  | "contract_save"
  | "contract_delete"
  | "contract_generate"
  | "contract_run"
  | "steps_catalog"
  | "steps_detect"
  | "trial_send"
  | "response_render"
  | "response_preview"
  | "response_filter"
  | "response_save"
  | "response_clear"
  // Lot D - Barre latérale
  | "contract_count"
  // Lot B : saisie et cURL
  | "request_input"
  | "request_curl"
  | "pairs_to_text"
  | "pairs_from_text"
  // Lot C - Import Bruno.
  | "import_pick"
  | "import_detect"
  | "import_run"
  | "import_report"
  // Lot E : tests et étapes (docs/plan-mvp-e-tests-etapes.md).
  | "test_read"
  | "tree_create_test"
  | "tree_add_step"
  | "request_variable_users"
  // Lot F : onglets et clavier (docs/plan-mvp-f-onglets-clavier.md).
  | "layout_pin"
  | "layout_close_tabs"
  | "layout_set_sidebar_hidden"
  // Lot I - Fichiers modifiés hors d'Arnod (docs/plan-mvp-i-fichiers-externes.md).
  | "request_conflict"
  | "request_conflict_resolve"
  // Lot G - Démarrage et erreurs.
  | "startup_status"
  | "startup_retry"
  | "startup_start_empty"
  | "startup_open"
  | "startup_quit"
  | "window_ready";

/**
 * Nature d'une erreur de commande.
 *
 * Équivalent Rust : `CommandErrorKind` dans `src-tauri/src/error.rs`. Un test Rust
 * vérifie que les deux listes concordent (pas de guillemet droit dans les commentaires
 * de cette déclaration : le test en extrait les chaînes).
 */
export type CommandErrorKind =
  | "invalidName" // Nom vide, trop long ou inutilisable comme nom de dossier.
  | "nameTaken" // Nom déjà utilisé, sans tenir compte de la casse.
  | "folderNotEmpty" // Le dossier cible existe déjà et n'est pas vide.
  | "alreadyRegistered" // Le projet est déjà dans Tous les projets.
  | "notAProject" // Le dossier ne contient pas de manifeste arnod.yml.
  | "notFound" // Projet ou workspace introuvable.
  | "undoExpired" // Le jeton d'annulation n'est plus valable.
  | "io" // Erreur d'entrée-sortie : disque, droits, chemin.
  | "corruptState" // Le fichier d'état de l'application est illisible.
  | "newerVersion" // Fichier d'état écrit par une version plus récente d'Arnod.
  | "unknownVariable" // Une variable {{nom}} n'existe pas dans l'environnement actif.
  | "invalidRequest" // Fichier de requête invalide ou opération impossible dessus.
  | "network" // Échec réseau : hôte introuvable, connexion refusée, certificat.
  | "timeout" // Délai dépassé.
  | "cancelled" // Exécution annulée par l'utilisateur.
  | "secretStore" // Le coffre du système est indisponible.
  | "protectedEnvironment" // Envoi sur un environnement protégé sans confirmation.
  | "changedOnDisk"; // Fichier modifié hors d'Arnod depuis sa lecture, rien n'est écrasé (lot I).

/**
 * Erreur renvoyée par une commande Rust qui échoue : la promesse d'`invoke` est
 * rejetée avec cet objet.
 *
 * Équivalent Rust : `CommandError` dans `src-tauri/src/error.rs`.
 */
export interface CommandError {
  /** Nature de l'erreur, pour réagir selon le cas. */
  kind: CommandErrorKind;
  /** Message en français, prêt à afficher. */
  message: string;
}

/**
 * Réponse de la commande `hello`.
 *
 * Équivalent Rust : `HelloResponse` dans `src-tauri/src/commands/hello.rs`.
 */
export interface HelloResponse {
  /** Texte de salutation produit par le cœur natif (« Hello World »). */
  message: string;
  /** Heure du cœur natif au moment de l'appel, en millisecondes depuis l'epoch Unix (UTC). */
  timestampMs: number;
}

/*
 * Catalogue : « Tous les projets » et workspaces.
 * Équivalents Rust dans `src-tauri/src/catalog/model.rs` et `src-tauri/src/catalog/snapshot.rs`.
 * Les identifiants sont des UUID sérialisés en chaînes.
 */

/**
 * Couleur d'un workspace, rendue par le jeton `--color-ws-<couleur>` de `src/styles.css`.
 *
 * Équivalent Rust : `WorkspaceColor`. Un test Rust vérifie la concordance.
 */
export type WorkspaceColor =
  "gray" | "orange" | "blue" | "green" | "purple" | "red" | "yellow" | "teal";

/** Présence du dossier d'un projet sur disque (P10). */
export type ProjectStatus = "ok" | "missing";

/** Projet de « Tous les projets ». Équivalent Rust : `ProjectView`. */
export interface ProjectView {
  id: string;
  name: string;
  /** Dossier du projet, chemin absolu. */
  path: string;
  /** `missing` si le dossier a été déplacé ou supprimé hors d'Arnod. */
  status: ProjectStatus;
  /** Workspaces qui contiennent le projet, dans l'ordre des onglets ; vide si aucun. */
  workspaceIds: string[];
}

/** Workspace. Équivalent Rust : `WorkspaceView`. */
export interface WorkspaceView {
  id: string;
  name: string;
  color: WorkspaceColor;
  /** Projets du workspace, dans l'ordre où ils ont été ajoutés. */
  projectIds: string[];
}

/** État complet du catalogue, renvoyé par chaque modification. Équivalent Rust : `CatalogSnapshot`. */
export interface CatalogSnapshot {
  /** « Tous les projets », dans l'ordre de création. */
  projects: ProjectView[];
  /** Workspaces dans l'ordre des onglets : l'ordre de création (R11). */
  workspaces: WorkspaceView[];
  /** Dernier onglet ouvert, à rouvrir au démarrage ; `null` pour « Tous les projets » (R7). */
  lastWorkspaceId: string | null;
  /** Projet « Non classés », en tête de `projects` ; `null` s'il n'existe pas. */
  inboxProjectId: string | null;
}

/** Réponse de `catalog_load`. Équivalent Rust : `CatalogLoad`. */
export interface CatalogLoad {
  snapshot: CatalogSnapshot;
  /** Fichier d'état illisible mis de côté (nature `corruptState`), signalé une seule fois. */
  warning: CommandError | null;
}

/** Réponse d'une action annulable. Équivalent Rust : `Undoable`. */
export interface Undoable {
  snapshot: CatalogSnapshot;
  /** Jeton à passer à `undo`. */
  undoToken: string;
}

/** Réponse de `project_create`. Équivalent Rust : `ProjectCreated`. */
export interface ProjectCreated {
  snapshot: CatalogSnapshot;
  projectId: string;
}

/** Réponse de `workspace_create`. Équivalent Rust : `WorkspaceCreated`. */
export interface WorkspaceCreated {
  snapshot: CatalogSnapshot;
  workspaceId: string;
}

/** Données du formulaire de création d'un projet (P1, P2). */
export interface ProjectCreateInput {
  name: string;
  /** Emplacement parent ; `null` pour `~/arnod`. */
  location: string | null;
  /** Workspace choisi, facultatif : le projet est toujours dans « Tous les projets » (R4). */
  workspaceId: string | null;
}

/** Dernier run d'un projet, pour « Tous les projets » (O12). Équivalent Rust : `ProjectLastRun`. */
export interface ProjectLastRun {
  projectId: string;
  /** Début du run, en millisecondes depuis l'epoch Unix. */
  startedAt: number;
  /** Requêtes réussies. */
  passed: number;
  /** Requêtes exécutées. */
  total: number;
}

// ---------------------------------------------------------------------------
// Premier usage : arborescence, requêtes, environnements, runs
// ---------------------------------------------------------------------------

/** Méthode HTTP (`Method` en Rust). */
export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

/** Noeud de l'arborescence d'un projet (`TreeNode` en Rust). */
export type TreeNode =
  | {
      kind: "folder";
      name: string;
      path: string;
      children: TreeNode[];
      requestCount: number;
      /** Test : au moins une requête et aucun sous-dossier (docs/plan-mvp-e-tests-etapes.md, R1). */
      test: boolean;
    }
  | {
      kind: "request";
      name: string;
      path: string;
      method: HttpMethod;
      assertionCount: number;
      invalid: boolean;
      /** Contrat lié (`contracts/petstore.yaml`), d'après l'étiquette `contract:`. */
      contract: string | null;
      /** Adresse, pour le filtre (docs/plan-mvp-d-barre-laterale.md, R1) ; vide si illisible. */
      url: string;
      /** Étiquettes, sans celle du lien au contrat (R1). */
      tags: string[];
    };

/** Arborescence et dossiers repliés (`TreeView`). */
export interface TreeView {
  nodes: TreeNode[];
  /**
   * Référentiel de tests : l'arborescence sans les requêtes rangées sous un contrat
   * (docs/plan-requetes-dans-contrats.md, R1, R5).
   */
  tests: TreeNode[];
  collapsed: string[];
}

/** Emplacement d'un paramètre : adresse (`?nom=valeur`) ou chemin (`:nom`). */
export type ParamKind = "query" | "path";

/** Paire nom-valeur activable (paramètre, en-tête, champ de formulaire). */
export interface Pair {
  name: string;
  value: string;
  enabled: boolean;
  /** Sans objet pour un en-tête ou un champ de formulaire (`query`). */
  kind: ParamKind;
  /** Description de la ligne (EP2, EP4). */
  description: string;
}

/**
 * Type d'authentification. `inherit` (héritée du dossier) et `other` (digest, OAuth…)
 * sont conservés dans le fichier mais rien n'est envoyé.
 */
export type AuthKind = "none" | "basic" | "bearer" | "apiKey" | "inherit" | "other";
export type ApiKeyLocation = "header" | "query";

/** Authentification (`Auth` en Rust, structure plate). */
export interface Auth {
  type: AuthKind;
  username: string;
  password: string;
  token: string;
  name: string;
  value: string;
  location: ApiKeyLocation;
  /** Type OpenCollection d'une authentification `other` (`oauth2`…). */
  other: string;
}

/** Type de corps, commun aux variantes ; `other` (multipart, fichier…) est conservé mais pas envoyé. */
export type BodyKind = "none" | "json" | "text" | "xml" | "form" | "other";

/** Variante d'un corps (EP3) ; un titre vide pour un corps simple. */
export interface BodyVariant {
  title: string;
  content: string;
  form: Pair[];
}

/** Corps : une ou plusieurs variantes, la sélectionnée part à l'envoi (R2). */
export interface Body {
  kind: BodyKind;
  variants: BodyVariant[];
  selected: number;
  /** Type OpenCollection d'un corps `other` (`multipart-form`…). */
  other: string;
}

/** Réglages propres à la requête (EP7) ; `null` : hérité (R3). */
export interface RequestSettings {
  /** Délai en millisecondes ; 0 : aucun délai. */
  timeoutMs: number | null;
  followRedirects: boolean | null;
  maxRedirects: number | null;
  encodeUrl: boolean | null;
}

/** Réglages effectifs, après héritage (`ResolvedSettings`). */
export interface ResolvedSettings {
  timeoutMs: number;
  followRedirects: boolean;
  maxRedirects: number;
  encodeUrl: boolean;
}

/** Type du corps d'une réponse attendue. */
export type ExampleBodyKind = "none" | "json" | "text" | "xml" | "html" | "binary";

/** En-tête d'une réponse attendue. */
export interface ResponseHeader {
  name: string;
  value: string;
}

/** Réponse attendue (EP8) : un exemple OpenCollection. */
export interface Example {
  name: string;
  description: string;
  status: number | null;
  statusText: string;
  headers: ResponseHeader[];
  bodyKind: ExampleBodyKind;
  body: string;
}

/** Ce que la requête hérite de ses dossiers et de la collection (R3, R4). */
export interface Inherited {
  auth: { auth: Auth; source: string } | null;
  headers: { name: string; value: string; source: string }[];
  settings: ResolvedSettings;
}

/** Opérateurs d'OpenCollection et de Bruno (`Operator` en Rust). */
export type AssertionOperator =
  | "equals"
  | "notEquals"
  | "exists"
  | "contains"
  | "notContains"
  | "lessThan"
  | "lessOrEqual"
  | "greaterThan"
  | "greaterOrEqual"
  | "in"
  | "notIn"
  | "between"
  | "startsWith"
  | "endsWith"
  | "length"
  | "matches"
  | "notMatches"
  | "isNull"
  | "isUndefined"
  | "isEmpty"
  | "isNotEmpty"
  | "isTruthy"
  | "isFalsy"
  | "isJson"
  | "isNumber"
  | "isText"
  | "isBoolean"
  | "isArray";
/** Origine d'une vérification, connue pendant la session seulement. */
export type AssertionOrigin = "proposed" | "manual" | "default";
export type Scalar = boolean | number | string;

/** Vérification d'une requête (R10, R11). */
export interface Assertion {
  /** Identifiant de session, recalculé à chaque lecture du fichier. */
  id: string;
  /** `status`, `responseTime`, `body.chemin`, `headers.nom` ou une autre expression. */
  target: string;
  operator: AssertionOperator;
  value?: Scalar | null;
  origin: AssertionOrigin;
  /** Une vérification désactivée n'est pas évaluée. */
  enabled: boolean;
}

/** Requête éditée, lue d'un fichier OpenCollection : un endpoint. */
export interface RequestDoc {
  name: string;
  /** Description courte (EP1). */
  description: string;
  /** Étiquettes, sans celle du lien au contrat. */
  tags: string[];
  method: HttpMethod;
  url: string;
  params: Pair[];
  headers: Pair[];
  auth: Auth;
  body: Body;
  settings: RequestSettings;
  /** Documentation en Markdown (EP6). */
  docs: string;
  assertions: Assertion[];
  /** Extractions, appliquées après la réponse (lot E, R2 du plan d'ensemble). */
  extractions: Extraction[];
  /** Rang de l'étape dans son dossier (`info.seq`), s'il est écrit. */
  seq?: number;
  /** Réponses attendues (EP8). */
  examples: Example[];
  /** Opération du contrat liée (étiquette `contract:` du fichier, C7). */
  contract?: ContractLink;
  /** Parties du fichier conservées sans être gérées par Arnod (scripts, variables…). */
  preserved: string[];
  /** Héritage, calculé à la lecture ; ignoré à l'enregistrement. */
  inherited: Inherited;
}

/** Lien d'une requête à une opération de contrat (`ContractLink`). */
export interface ContractLink {
  /** Fichier du contrat : `contracts/petstore.yaml`. */
  file: string;
  /** Clé de l'opération : `GET /pets/{petId}`. */
  operation: string;
}

/** Résultat d'une vérification (V3). */
export interface AssertionResult {
  id: string;
  /** Cible (`body.amount`), ou `contrat` pour la conformité au contrat lié. */
  target: string;
  passed: boolean;
  message: string | null;
  actual: string | null;
  /** Échec ignoré pour le résultat affiché (`docs/plan-fin-premier-usage.md`, R7). */
  ignored: boolean;
}

/** Réponse HTTP reçue. */
export interface HttpResponse {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
  truncated: boolean;
  sizeBytes: number;
  waitMs: number;
  receiveMs: number;
  totalMs: number;
}

/** Résultat d'un envoi. */
export interface SendOutcome {
  response: HttpResponse;
  assertions: AssertionResult[];
  passed: boolean;
  slow: boolean;
  /** Ce qui est parti, pour l'onglet « Envoyé » (lot E, VD2). */
  sent: Sent | null;
}

/** Variable partagée d'un environnement. */
export interface EnvVariable {
  name: string;
  value: string;
}

/** Couleurs proposées pour un environnement. */
export type EnvironmentColor = "gray" | "blue" | "green" | "orange" | "red" | "purple";

/** Environnement, tel que l'interface l'édite. */
export interface EnvironmentDoc {
  name: string;
  color: string;
  protected?: boolean;
  variables: EnvVariable[];
  locals: string[];
}

/** Variable locale ; un secret n'expose jamais sa valeur (R9). */
export interface LocalVarView {
  name: string;
  secret: boolean;
  value: string | null;
  hasValue: boolean;
}

export interface EnvironmentView extends EnvironmentDoc {
  localVars: LocalVarView[];
}

export interface EnvironmentsView {
  environments: EnvironmentView[];
  active: string;
}

export type RunItemStatus =
  | "pending"
  | "running"
  | "passed"
  | "failed"
  | "error"
  | "cancelled"
  /** Non jouée : une étape précédente du test a échoué (lot E, R3). */
  | "skipped";

export interface RunItem {
  path: string;
  name: string;
  method: HttpMethod;
  status: RunItemStatus;
  httpStatus: number | null;
  durationMs: number | null;
  slow: boolean;
  assertions: AssertionResult[];
  error: string | null;
  /** Test dont la requête est une étape (lot E), sinon `null`. */
  test: string | null;
  /** Numéro de l'étape dans son test, à partir de 1. */
  step: number | null;
  /** Valeurs extraites par la ligne, masquées. */
  extracted: ExtractedValue[];
}

/** Résultat d'un groupe de requêtes d'un run (`RunGroup`, R2). */
export interface RunGroup {
  /** Dossier, ou « Projet » pour la racine. */
  name: string;
  /** Chemin du dossier (vide pour la racine). */
  path: string;
  passed: number;
  /** En échec ou en erreur. */
  failed: number;
  total: number;
}

export interface RunView {
  id: string;
  /** Numéro du run dans le projet (R1) ; 0 pour un run gardé par une version précédente. */
  number: number;
  folder: string;
  environment: string;
  /** Requêtes envoyées en même temps. */
  concurrency: number;
  startedAt: number;
  finishedAt: number | null;
  finished: boolean;
  cancelled: boolean;
  items: RunItem[];
  /** Nom du contrat exécuté (C10), sinon `null`. */
  contract: string | null;
  /** Révision, croissante à chaque changement (R3). */
  revision: number;
  /** Résultat par dossier (R2). */
  groups: RunGroup[];
  /** Verdict de chaque test (lot E, R4). */
  tests: RunTest[];
}

/** Dernière exécution d'une requête (`RequestStatus`, R4). */
export interface RequestStatus {
  passed: boolean;
  /** Fin de l'exécution, en millisecondes depuis l'époque Unix. */
  at: number;
}

/** Statuts d'un projet, par chemin de requête (`Statuses`, R5). */
export type RequestStatuses = Record<string, RequestStatus>;

/** Statuts diffusés par `arnod://statuts` (`StatusesChange`). */
export interface StatusesChange {
  projectId: string;
  statuses: RequestStatuses;
}

/** Réponse sur laquelle proposer des vérifications. */
export interface ProposeInput {
  status: number;
  durationMs: number;
  headers: [string, string][];
  body: string;
}

/** Réponse reçue à reprendre comme réponse attendue (R7). */
export interface ExampleInput {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
}

// ---------------------------------------------------------------------------
// Fenêtres et zones (docs/plan-fenetres-zones.md)
// ---------------------------------------------------------------------------

/** Ce qu'affiche un onglet de contenu (`Content` en Rust). */
export type ContentRef =
  | { kind: "request"; projectId: string; path: string }
  | { kind: "run"; projectId: string }
  | { kind: "contract"; projectId: string; path: string }
  /** Résultat de la requête active, dans sa fenêtre (docs/plan-resultat-detachable.md). */
  | { kind: "result" }
  /** Dernier rapport d'import d'un projet (docs/plan-mvp-c-import-bruno.md, 2.3-f). */
  | { kind: "importReport"; projectId: string }
  /** Test du projet, dossier d'étapes (docs/plan-mvp-e-tests-etapes.md, TE2). */
  | { kind: "test"; projectId: string; path: string };

/** Onglet de contenu d'une zone (`Tab`). */
export interface ContentTab {
  id: string;
  content: ContentRef;
  /** Onglet d'aperçu (1.3-a) : ouvert d'un simple clic, remplacé par le prochain aperçu. */
  preview: boolean;
}

/** Place du résultat d'une requête dans sa zone (F6). */
export type Placement = "auto" | "right" | "below";

/** Orientation d'une séparation : `row` côte à côte, `column` l'une sous l'autre. */
export type Axis = "row" | "column";

/** Côté d'une nouvelle zone (F2, F3). */
export type Side = "right" | "bottom";

/** Découpages du menu Fenêtre › Dispositions (F7). */
export type LayoutPreset = "one" | "twoColumns" | "twoRows" | "three" | "four";

/** Zone : onglets, onglet actif, place du résultat. */
export interface ZoneView {
  id: string;
  tabs: ContentTab[];
  activeTab: string | null;
  placement: Placement;
  /** Onglets dans l'ordre d'utilisation, le plus récent d'abord (Ctrl+Tab, 1.3-c). */
  recent: string[];
}

/** Nœud de la disposition : une zone ou une séparation (`Node`). */
export type LayoutNode =
  | ({ kind: "zone" } & ZoneView)
  | {
      kind: "split";
      id: string;
      axis: Axis;
      /** Part de `first`, entre 0,1 et 0,9. */
      ratio: number;
      first: LayoutNode;
      second: LayoutNode;
    };

/** Disposition d'une fenêtre (`Layout`). */
export interface WindowLayout {
  root: LayoutNode;
  activeZone: string;
}

/** Rôle d'une fenêtre (`WindowKind`). */
export type WindowKind = "main" | "content" | "sidebar";

/** Position et taille logiques d'une fenêtre (`Bounds`). */
export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** État d'une fenêtre (`WindowState`). */
export interface WindowView {
  label: string;
  kind: WindowKind;
  /** Zones ; `null` pour la barre latérale détachée. */
  layout: WindowLayout | null;
  /** Fenêtre de contenu : suit la sélection (sinon épinglée). */
  follow: boolean;
  /** Fenêtre principale : largeur de la barre latérale, en pixels. */
  sidebarWidth: number;
  /** Fenêtre principale : barre latérale détachée. */
  sidebarDetached: boolean;
  bounds: WindowBounds | null;
  /** Fenêtre principale : barre latérale masquée (Ctrl+\, 1.4-j). */
  sidebarHidden: boolean;
}

/** Toutes les fenêtres (`WindowsFile`), aussi diffusé par `arnod://fenetres`. */
export interface WindowsSnapshot {
  version: number;
  windows: WindowView[];
}

/** Fenêtre appelante et état de toutes les fenêtres (`WindowContext`). */
export interface WindowContext {
  label: string;
  windows: WindowsSnapshot;
  /** Requête active, que suit la fenêtre de résultat (D2). */
  activeRequest: ActiveRequest | null;
}

/** Requête active (`ActiveRequest`), diffusée par `arnod://requete-active`. */
export interface ActiveRequest {
  projectId: string;
  path: string;
}

/** Dernier envoi d'une requête (`RequestResult`), gardé par le cœur (R5). */
export interface RequestResult {
  projectId: string;
  path: string;
  sending: boolean;
  outcome: SendOutcome | null;
  error: string | null;
  /** Réception de la réponse, en millisecondes depuis l'époque Unix. */
  receivedAt: number | null;
  /** Requête telle qu'envoyée (lot E, R12), `null` avant le premier envoi réussi. */
  request: RequestDoc | null;
}

/** Envoi diffusé par `arnod://resultat` (`ResultChange`). */
export interface ResultChange {
  result: RequestResult;
  /** Fenêtre à l'origine de l'envoi, qui l'a déjà appliqué. */
  origin: string;
}

/**
 * Run démarré ou avancé, diffusé par `arnod://run` (`RunChange`) : toutes les fenêtres le suivent
 * (R7). `origin` vide : avancement diffusé par le cœur (R3 de la fin du premier usage).
 */
export interface RunChange {
  projectId: string;
  run: RunView;
  origin: string;
}

/** Partie d'un projet modifiée (`ProjectScope`). */
export type ProjectScope = "tree" | "request" | "environments" | "contracts";

/** Modification d'un projet diffusée par `arnod://projet` (`ProjectChange`). */
export interface ProjectChange {
  projectId: string;
  scope: ProjectScope;
  path: string | null;
  /** Fenêtre à l'origine du changement, qui l'a déjà appliqué. */
  origin: string;
}

// --- Contrats de services (docs/plan-contrats-services.md) ---

/** Famille de spécification d'un contrat (`SpecKind`). */
export type SpecKind = "openApi3" | "swagger2";

/** Contrat de la barre latérale (`ContractSummary`). */
export interface ContractSummary {
  /** Contrat déduit des échanges réels (`x-arnod-deduced`) : version non affichée. */
  deduced: boolean;
  /** Chemin relatif au projet : `contracts/petstore.yaml`. */
  path: string;
  name: string;
  version: string;
  operationCount: number;
  /** Fichier illisible (R10) : la raison. */
  error: string | null;
  /** Ressources, opérations et requêtes liées, pour la barre latérale (R2, R3). */
  resources: ResourceView[];
  /** Requêtes liées sans opération dans le contrat, ou contrat illisible (R4). */
  unplaced: LinkedView[];
}

/** Ressource d'un contrat : premier segment du chemin de ses opérations (`ResourceView`). */
export interface ResourceView {
  /** `/pets`, ou `/` pour la racine. */
  path: string;
  operations: OperationNode[];
}

/** Opération d'une ressource et ses requêtes liées ; la première est l'endpoint (`OperationNode`). */
export interface OperationNode {
  key: string;
  method: string;
  path: string;
  summary: string | null;
  /** Méthode envoyable : une requête peut être créée. */
  supported: boolean;
  requests: LinkedView[];
}

/** Requête liée, telle que la barre latérale la montre (`LinkedView`). */
export interface LinkedView {
  path: string;
  name: string;
  method: HttpMethod;
  /** Réponses attendues, dans l'ordre du fichier (docs/plan-mvp-d-barre-laterale.md, R2). */
  examples: ExampleRef[];
}

/** Réponse attendue d'une requête liée, pour la barre latérale (`ExampleRef`, 3.2-a3). */
export interface ExampleRef {
  /** Position dans `examples` de la requête. */
  index: number;
  status: number | null;
  name: string;
}

/** Opération affichée dans l'onglet du contrat (`OperationView`). */
export interface OperationView {
  key: string;
  method: string;
  path: string;
  summary: string | null;
  operationId: string | null;
  tag: string | null;
  deprecated: boolean;
  /** Méthode envoyable (TRACE ne l'est pas). */
  supported: boolean;
  externalRefs: boolean;
  /** Requête liée (R4), s'il y en a une. */
  request: string | null;
}

/** Requête liée dont l'opération n'est plus dans le contrat (R11). */
export interface OrphanView {
  path: string;
  name: string;
  operation: string;
}

/** Contrat affiché dans son onglet (`ContractView`). */
export interface ContractView {
  /** Contrat déduit des échanges réels (`x-arnod-deduced`). */
  deduced: boolean;
  path: string;
  name: string;
  version: string;
  kind: SpecKind | null;
  specVersion: string;
  description: string | null;
  servers: string[];
  source: string;
  error: string | null;
  operations: OperationView[];
  orphans: OrphanView[];
}

/** Origine d'un contrat ajouté (`ContractSource`). */
export type ContractSource =
  | { kind: "text"; text: string }
  | { kind: "url"; url: string }
  | { kind: "template"; name: string };

/** Opération sans requête créée, et pourquoi. */
export interface SkippedOperation {
  operation: string;
  reason: string;
}

/** `baseUrl` renseignée à la création des requêtes (R8). */
export interface BaseUrlSet {
  environment: string;
  value: string;
}

/** Bilan de « Créer les requêtes » (`GenerateReport`). */
export interface GenerateReport {
  folder: string;
  created: string[];
  skipped: SkippedOperation[];
  baseUrl: BaseUrlSet | null;
}

/*
 * Catalogue des étapes et essais (docs/plan-accueil-non-classes.md).
 * Équivalents Rust dans `src-tauri/src/steps/` et `src-tauri/src/commands/trial.rs`.
 */

/**
 * Famille d'une nature d'étape, qui donne sa couleur.
 *
 * Équivalent Rust : `Family` (`steps/definition.rs`) ; un test vérifie la concordance.
 */
export type StepFamily = "calls" | "data" | "messages" | "jobs" | "interface" | "code" | "files";

/** Nature d'un champ de formulaire. Équivalent Rust : `FieldKind`. */
export type StepFieldKind = "choice" | "text" | "multiline";

/** Moteur d'une nature (`Engine`). */
export type StepEngine =
  | { kind: "builtin"; id: string; abi: number }
  | { kind: "wasm"; module: string; sha256: string; abi: number };

/** Champ principal, saisi dans l'entrée de l'accueil (`Subject`). */
export interface StepSubject {
  key: string;
  label: string;
  placeholder: string;
}

/** Condition d'affichage d'un champ (`VisibleWhen`). */
export interface StepVisibleWhen {
  key: string;
  in: string[];
}

/** Champ déclaratif du formulaire d'une nature (`Field`). */
export interface StepField {
  key: string;
  label: string;
  kind: StepFieldKind;
  default: string | null;
  placeholder: string | null;
  hint: string | null;
  options: string[];
  visibleWhen: StepVisibleWhen | null;
}

/** Règle de reconnaissance (`Rule`), affichée dans la fiche d'une nature. */
export type StepRule =
  | { kind: "url"; schemes: string[]; methodField: string | null }
  | { kind: "keywords"; words: string[] }
  | { kind: "prefix"; text: string }
  | { kind: "suffix"; text: string }
  | { kind: "curl" };

/** Nature d'étape disponible (`StepNature`). */
export interface StepNature {
  format: number;
  id: string;
  version: string;
  name: string;
  title: string;
  description: string;
  family: StepFamily;
  publisher: string;
  engine: StepEngine;
  /** Verbe du bouton : « Envoyer », « Exécuter »… */
  action: string;
  subject: StepSubject;
  detect: StepRule[];
  form: StepField[];
  /** Livrée avec Arnod. */
  origin: "builtin";
}

/** Source des définitions (`CatalogSource`). */
export interface StepCatalogSource {
  name: string;
  /** Date des définitions, `AAAA-MM-JJ`. */
  updated: string;
  /** Synchronisation en place ; `false` : définitions intégrées seulement. */
  synced: boolean;
}

/** Réponse de `steps_catalog`. */
export interface StepCatalog {
  source: StepCatalogSource;
  natures: StepNature[];
}

/** Réponse de `steps_detect` : nature reconnue, champ principal et valeurs lues. */
export interface Detection {
  nature: string;
  subject: string;
  values: Record<string, string>;
  /** Commande cURL reconnue : secrets retirés des en-têtes, options signalées. */
  curl?: CurlImport;
}

/** Réponse de `trial_send` : l'essai enregistré dans « Non classés ». */
export interface TrialResult {
  snapshot: CatalogSnapshot;
  projectId: string;
  /** Requête « Essai N », relative au projet. */
  path: string;
  /** Contrat déduit (`contracts/…`). */
  contract: string;
  outcome: SendOutcome;
}

// --- Lot A - Réponse (docs/plan-mvp-a-reponse.md, écran 5.1-b bis) -------------------------

/**
 * Format d'affichage d'un corps de réponse, détecté d'après `content-type` (R1).
 *
 * Équivalent Rust : `ResponseFormat` (`src-tauri/src/response/format.rs`).
 */
export type ResponseFormat = "json" | "xml" | "html" | "text";

/** Nature d'un jeton de la mise en forme, pour sa couleur (`TokenKind`). */
export type TokenKind =
  | "key"
  | "string"
  | "number"
  | "literal"
  | "punct"
  | "tag"
  | "attr"
  | "attrValue"
  | "comment"
  | "text";

/** Morceau d'une ligne mise en forme (`Token`). */
export interface BodyToken {
  text: string;
  kind: TokenKind;
  /** Chemin canonique de la valeur JSON (`$.items[0].sku`), comme ceux du filtre. */
  path?: string;
}

/** Réponse de `response_render` (`RenderedBody`). */
export interface RenderedBody {
  /** Format appliqué : celui choisi, sinon celui détecté. */
  format: ResponseFormat;
  detected: ResponseFormat;
  lines: BodyToken[][];
  /** Pourquoi le corps est montré sans mise en forme. */
  notice: string | null;
}

/** Nœud de l'aperçu HTML (`HtmlNode`) : balises sûres, sans attribut. */
export type HtmlNode =
  { kind: "text"; text: string } | { kind: "element"; tag: string; children: HtmlNode[] };

/** Correspondance du filtre JSONPath (`FilterMatch`). */
export interface FilterMatch {
  path: string;
  /** Valeur en JSON compact, raccourcie au-delà de 200 caractères. */
  value: string;
  /** Vérification tirée de la valeur ; absente pour un nom qui ne s'écrit pas en notation pointée. */
  assertion: Assertion | null;
}

/** Réponse de `response_filter` (`FilterResult`). */
export interface FilterResult {
  matches: FilterMatch[];
  /** Nombre total de correspondances (500 renvoyées au plus). */
  total: number;
  /** Expression invalide ou corps non JSON, en français. */
  error: string | null;
}

// --- Lot B : saisie, texte et cURL (docs/plan-mvp-b-saisie-curl.md) ------------

/** Environnement qui définit une variable inconnue de l'environnement actif (`VariableSource`). */
export interface VariableSource {
  environment: string;
  /** Valeur d'une variable partagée ; `null` pour une variable locale. */
  value: string | null;
}

/** Variable utilisée par la requête et absente de l'environnement actif (R2). */
export interface UnknownVariable {
  name: string;
  definedIn: VariableSource[];
}

/** D'où vient un en-tête ajouté à l'envoi (`AddedOrigin`). */
export type AddedOrigin = "auth" | "body" | "client";

/** En-tête ajouté par Arnod à l'envoi (R3), montré grisé. */
export interface AddedHeader {
  name: string;
  /** Valeur, secret masqué, ou ce qui la donne (« calculé à l'envoi »). */
  value: string;
  origin: AddedOrigin;
  /** Dossier ou collection d'une authentification héritée. */
  source: string | null;
}

/** Réponse de `request_input` : variables connues et inconnues, en-têtes ajoutés. */
export interface RequestInput {
  environment: string;
  /** Variables connues : environnement actif, dossiers et collection (R1). */
  known: string[];
  /** Variables utilisées et inconnues, valeurs dynamiques `$…` exclues (R2). */
  unknown: UnknownVariable[];
  added: AddedHeader[];
}

/** Réponse de `request_curl` : la requête en commande cURL (R8 à R10). */
export interface CurlCopy {
  command: string;
  /** Environnement dont les variables sont résolues. */
  environment: string;
}

/** Écriture d'une liste en texte (`PairsFormat`) : `Nom: valeur` ou `nom=valeur`. */
export type PairsFormat = "header" | "query";

/** Ligne illisible d'une liste en texte. */
export interface TextError {
  /** Numéro de la ligne, à partir de 1. */
  line: number;
  message: string;
}

/** Réponse de `pairs_from_text` (R12, R13). */
export interface PairsText {
  pairs: Pair[];
  errors: TextError[];
}

/** Secret retiré d'une commande cURL collée (`Secret`), gardé au coffre à l'envoi. */
export interface CurlSecret {
  /** Variable locale qui le remplace (`token`). */
  name: string;
  value: string;
  /** En-tête d'où il vient. */
  header: string;
}

/** Option cURL sans équivalent (`Ignored`), signalée (R6). */
export interface CurlIgnored {
  option: string;
  reason: string;
}

/** Ce qu'une commande cURL collée a donné en plus des valeurs (`CurlImport`). */
export interface CurlImport {
  secrets: CurlSecret[];
  ignored: CurlIgnored[];
}

// ---------------------------------------------------------------------------
// Lot C - Import Bruno (docs/plan-mvp-c-import-bruno.md)
// ---------------------------------------------------------------------------

/** Format reconnu par l'import (`SourceFormat`). */
export type ImportFormat = "bruno" | "openCollection" | "openApi";

/** Réponse de `import_detect` : ce que l'import a reconnu, avant toute écriture. */
export interface ImportSource {
  /** Dossier (ou fichier, pour un contrat) à importer. */
  path: string;
  format: ImportFormat;
  /** Nom de la collection, du projet, ou titre du contrat. */
  name: string;
  /** Ce que la conversion reprendra ; `requests` compte les opérations d'un contrat. */
  counts: ImportCounts;
  /** Fichiers `.bru` lus. */
  files: number;
}

/** Destination d'un import (`ImportTarget`). */
export type ImportTarget =
  | { kind: "newProject"; location: string | null; workspaceId: string | null }
  | { kind: "project"; projectId: string };

/** Nature d'un élément du rapport (`ItemKind`). */
export type ImportItemKind =
  | "tests"
  | "preRequest"
  | "postResponse"
  | "env"
  | "auth"
  | "body"
  | "request"
  | "assertion"
  | "extraction"
  | "option";

/** Suite à donner à un élément du rapport (`ItemStatus`). */
export type ImportItemStatus = "notRun" | "toFill" | "toCheck" | "skipped" | "stored";

/** Élément signalé par l'import, avec son fichier d'origine. */
export interface ImportReportItem {
  kind: ImportItemKind;
  status: ImportItemStatus;
  name: string;
  /** Dossier ou environnement ; vide à la racine. */
  place: string;
  /** Fichier d'origine, relatif à la collection. */
  file: string;
  /** Bloc d'origine (`tests`, `auth:oauth2`…). */
  block: string | null;
  /** Requête créée dans le projet, relative à sa racine. */
  request: string | null;
  detail: string;
  /** Contenu d'origine (script, valeur). */
  code: string | null;
}

/** Exemple de conversion sans code. */
export interface ConversionExample {
  before: string;
  after: string;
}

/** Comptes du rapport (`ImportCounts`). */
export interface ImportCounts {
  folders: number;
  requests: number;
  environments: number;
  assertions: number;
  extractions: number;
  secrets: number;
  dynamicValues: number;
  scripts: number;
}

/** Rapport d'import (2.3-f), gardé sur le poste. */
export interface ImportReport {
  collection: string;
  format: ImportFormat;
  source: string;
  sourceFiles: number;
  projectId: string;
  projectName: string;
  /** Dossier créé dans un projet existant ; `null` pour un nouveau projet. */
  folder: string | null;
  /** Millisecondes depuis 1970. */
  importedAt: number;
  counts: ImportCounts;
  items: ImportReportItem[];
  examples: ConversionExample[];
}

/** Réponse de `import_run`. */
export interface ImportDone {
  projectId: string;
  /** Nouvel instantané si le catalogue a changé. */
  snapshot: CatalogSnapshot | null;
  report: ImportReport | null;
  /** Contrat ajouté (OpenAPI), relatif au projet. */
  contract: string | null;
}

// ---------------------------------------------------------------------------
// Lot E : tests et étapes (docs/plan-mvp-e-tests-etapes.md)
// ---------------------------------------------------------------------------

/** Extraction (`Extraction`) : une valeur de la réponse rangée dans une variable (R7). */
export interface Extraction {
  /** Nom de la variable (`client.id`). */
  name: string;
  /** `body.id`, `headers.Location`, `status`, ou `request.body.ref` pour une valeur envoyée. */
  source: string;
  enabled: boolean;
}

/** Valeur extraite à un envoi (`ExtractedValue`). */
export interface ExtractedValue {
  name: string;
  source: string;
  /** `null` : source absente, la variable n'est pas remplie. */
  value: string | null;
  message: string | null;
}

/** Valeur dynamique tirée (`DrawnValue`, R9, R10). */
export interface DrawnValue {
  /** Nom avec son `$` : `$randomUUID`. */
  name: string;
  value: string;
  /** Source de la valeur envoyée (`request.body.ref`), pour « Garder ». */
  source: string | null;
}

/** Variable lue par un envoi (`UsedVariable`) ; un secret est masqué. */
export interface UsedVariable {
  name: string;
  value: string;
  /** Variable d'exécution (valeur extraite), et non de l'environnement. */
  runtime: boolean;
}

/** Ce qui est parti (`Sent`, VD2). */
export interface Sent {
  method: string;
  url: string;
  headers: [string, string][];
  body: string | null;
  drawn: DrawnValue[];
  used: UsedVariable[];
  extracted: ExtractedValue[];
}

/** Verdict d'un test dans un run (`RunTest`, R4). */
export interface RunTest {
  path: string;
  name: string;
  status: RunItemStatus;
  steps: number;
  passed: number;
  /** Étape qui a arrêté le test (R3). */
  failedStep: number | null;
}

/** Étape d'un test (`StepView`). */
export interface StepView {
  path: string;
  name: string;
  method: HttpMethod | null;
  url: string;
  assertionCount: number;
  /** Variables extraites par l'étape. */
  extractions: string[];
  invalid: boolean;
}

/** Test ouvert dans un onglet (`TestView`, TE2). */
export interface TestView {
  path: string;
  name: string;
  /** Le dossier est un test ; sinon il range des dossiers. */
  test: boolean;
  steps: StepView[];
}

/** Réponse de `tree_create_test` (`CreatedTest`, NT1). */
export interface CreatedTest {
  folder: string;
  step: string;
}

/** Requête qui lit une variable (`VariableUser`, EX5). */
export interface VariableUser {
  path: string;
  name: string;
  method: HttpMethod;
  url: string;
  variables: string[];
}

// Lot F : onglets et clavier (docs/plan-mvp-f-onglets-clavier.md).

/** Raccourci choisi dans le menu natif Fenêtre (`Shortcut`), reçu par `arnod://raccourci`. */
export type MenuShortcut = "placement" | "sidebar";
// --- Lot I - Fichiers modifiés hors d'Arnod (docs/plan-mvp-i-fichiers-externes.md) ---

/** Requête renommée ou déplacée hors d'Arnod (`Renamed`, R5). */
export interface RenamedFile {
  from: string;
  to: string;
}

/** Changements d'un projet faits hors d'Arnod, diffusés par `arnod://fichiers` (`FilesChange`). */
export interface FilesChange {
  projectId: string;
  /** Requêtes ajoutées, modifiées, supprimées (chemins relatifs au projet). */
  added: string[];
  modified: string[];
  removed: string[];
  renamed: RenamedFile[];
  tree: boolean;
  environments: boolean;
  contracts: boolean;
}

/** Version gardée pour un élément en conflit (`ConflictSide`). */
export type ConflictSide = "disk" | "draft";

/** Élément d'une requête en conflit avec le disque (`ConflictItem`, R9, R10). */
export interface ConflictItem {
  /** Clé stable (`url`, `headers:x-request-id#0`). */
  key: string;
  /** Libellé en français (« En-tête X-Request-Id »). */
  label: string;
  /** Valeur sur le disque, prête à afficher ; `null` : absent sur le disque. */
  disk: string | null;
  /** Valeur du brouillon ; `null` : absent du brouillon. */
  draft: string | null;
  /** Même valeur des deux côtés, changée de la même façon. */
  identical: boolean;
  /** Version proposée : celle qui seule a changé. */
  suggested: ConflictSide | null;
}

/** Conflit d'une requête ouverte (`RequestConflict`). */
export interface RequestConflict {
  items: ConflictItem[];
  /** Date de modification du fichier (ms depuis l'époque Unix). */
  modifiedAt: number | null;
}

// Lot G - Démarrage et erreurs (docs/plan-mvp-g-demarrage-erreurs.md).

/** Nature d'une erreur bloquante au démarrage (`startup::ProblemNature`). */
export type ProblemNature = "newerVersion" | "corruptState" | "locked" | "inaccessible";

/** Issue proposée par l'écran d'erreur (`startup::StartupAction`). */
export type StartupAction = "openReleases" | "startEmpty" | "retry" | "openFolder";

/** Ce que `startup_open` ouvre avec le programme du système (`startup::StartupTarget`). */
export type StartupTarget = "folder" | "releases" | "issue";

/** Une issue et ses conséquences (`startup::StartupOption`). */
export interface StartupOption {
  action: StartupAction;
  title: string;
  description: string;
  button: string;
  /** La recommandée, toujours en premier. */
  recommended: boolean;
  /** Ligne de plus : essais automatiques épuisés. */
  note: string | null;
}

/** Prochain essai automatique d'un fichier verrouillé (`startup::RetryPlan`). */
export interface RetryPlan {
  attempt: number;
  maxAttempts: number;
  delaySeconds: number;
}

/** Ligne des détails pour le support (`startup::DetailRow`). */
export interface DetailRow {
  label: string;
  value: string;
}

/** Erreur bloquante décrite par le cœur (`startup::StartupProblem`). */
export interface StartupProblem {
  nature: ProblemNature;
  /** `arnod.json` ou `windows.json`. */
  file: string;
  title: string;
  /** La cause en clair. */
  message: string;
  /** Ce qui est intact : début en gras, puis la suite. */
  intactLead: string;
  intactText: string;
  /** Deux issues, la recommandée en premier. */
  options: StartupOption[];
  pathLabel: string;
  path: string;
  retry: RetryPlan | null;
  details: DetailRow[];
  /** Détails en texte, pour « Copier le détail ». */
  supportText: string;
  /** Issue GitHub préremplie : rien n'est envoyé sans l'utilisateur. */
  issueUrl: string;
  footer: string;
}
