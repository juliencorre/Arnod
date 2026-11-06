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
  | "project_open"
  | "workspace_open"
  | "workspace_create"
  | "workspace_rename"
  | "workspace_set_color"
  | "workspace_set_default"
  | "workspace_add_projects"
  | "workspace_remove_project"
  | "workspace_delete"
  | "window_set_title";

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
  | "alreadyRegistered" // Le projet est déjà inscrit au référentiel.
  | "notAProject" // Le dossier ne contient pas de manifeste arnod.yml.
  | "notFound" // Projet ou workspace introuvable.
  | "defaultWorkspace" // Action interdite sur le workspace par défaut.
  | "undoExpired" // Le jeton d'annulation n'est plus valable.
  | "io" // Erreur d'entrée-sortie : disque, droits, chemin.
  | "corruptState" // Le fichier d'état de l'application est illisible.
  | "newerVersion"; // Fichier d'état écrit par une version plus récente d'Arnod.

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
 * Catalogue : projets du référentiel et workspaces.
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

/** Projet du référentiel. Équivalent Rust : `ProjectView`. */
export interface ProjectView {
  id: string;
  name: string;
  /** Dossier du projet, chemin absolu. */
  path: string;
  /** `missing` si le dossier a été déplacé ou supprimé hors d'Arnod. */
  status: ProjectStatus;
  /** Workspaces qui contiennent le projet : défaut d'abord, puis par nom. */
  workspaceIds: string[];
}

/** Workspace. Équivalent Rust : `WorkspaceView`. */
export interface WorkspaceView {
  id: string;
  name: string;
  color: WorkspaceColor;
  isDefault: boolean;
  /** Projets du workspace, dans l'ordre où ils ont été ajoutés. */
  projectIds: string[];
}

/** État complet du catalogue, renvoyé par chaque modification. Équivalent Rust : `CatalogSnapshot`. */
export interface CatalogSnapshot {
  /** Référentiel, dans l'ordre de création. */
  projects: ProjectView[];
  /** Workspace par défaut d'abord, puis les autres par nom. */
  workspaces: WorkspaceView[];
  defaultWorkspaceId: string;
  /** Dernier workspace ouvert, à rouvrir au démarrage. */
  lastWorkspaceId: string;
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
  /** Workspace choisi ; le projet va aussi dans le workspace par défaut. */
  workspaceId: string;
}
