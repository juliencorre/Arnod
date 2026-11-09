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
  | "request_propose_assertions"
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
  | "project_last_runs";

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
  | "protectedEnvironment"; // Envoi sur un environnement protégé sans confirmation.

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
  | { kind: "folder"; name: string; path: string; children: TreeNode[]; requestCount: number }
  | {
      kind: "request";
      name: string;
      path: string;
      method: HttpMethod;
      assertionCount: number;
      invalid: boolean;
    };

/** Arborescence et dossiers repliés (`TreeView`). */
export interface TreeView {
  nodes: TreeNode[];
  collapsed: string[];
}

/** Paire nom-valeur activable (paramètre, en-tête, champ de formulaire). */
export interface Pair {
  name: string;
  value: string;
  enabled: boolean;
}

export type AuthKind = "none" | "basic" | "bearer" | "apiKey";
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
}

export type BodyKind = "none" | "json" | "text" | "form";

export interface Body {
  kind: BodyKind;
  content: string;
  form: Pair[];
}

export type AssertionOperator =
  | "equals"
  | "notEquals"
  | "exists"
  | "contains"
  | "lessThan"
  | "greaterThan"
  | "isNumber"
  | "isText";
export type AssertionOrigin = "proposed" | "manual" | "default";
export type Scalar = boolean | number | string;

/** Vérification d'une requête (R10, R11). */
export interface Assertion {
  id: string;
  target: string;
  operator: AssertionOperator;
  value?: Scalar | null;
  origin: AssertionOrigin;
}

/** Requête, telle que stockée dans son fichier YAML. */
export interface RequestDoc {
  version: number;
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  params: Pair[];
  headers: Pair[];
  auth: Auth;
  body: Body;
  timeoutMs: number;
  assertions: Assertion[];
}

/** Résultat d'une vérification (V3). */
export interface AssertionResult {
  id: string;
  passed: boolean;
  message: string | null;
  actual: string | null;
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
}

/** Variable partagée d'un environnement. */
export interface EnvVariable {
  name: string;
  value: string;
}

/** Couleurs proposées pour un environnement. */
export type EnvironmentColor = "gray" | "blue" | "green" | "orange" | "red" | "purple";

/** Contenu d'un fichier d'environnement. */
export interface EnvironmentDoc {
  version: number;
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

export type RunItemStatus = "pending" | "running" | "passed" | "failed" | "error" | "cancelled";

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
}

export interface RunView {
  id: string;
  folder: string;
  environment: string;
  startedAt: number;
  finished: boolean;
  cancelled: boolean;
  items: RunItem[];
}

/** Réponse sur laquelle proposer des vérifications. */
export interface ProposeInput {
  status: number;
  durationMs: number;
  headers: [string, string][];
  body: string;
}
