/**
 * Wrapper typé autour d'`invoke` : seul point d'accès aux commandes Rust.
 *
 * Les composants et stores n'importent jamais `@tauri-apps/api/core`
 * directement ; ils passent par `ipc`.
 */
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";

import type {
  CatalogLoad,
  CatalogSnapshot,
  CommandError,
  CommandErrorKind,
  CommandName,
  HelloResponse,
  ProjectCreated,
  ProjectCreateInput,
  ProjectView,
  Undoable,
  WorkspaceColor,
  WorkspaceCreated,
} from "@/types/ipc";

/** Natures connues : le type impose d'y lister chaque membre de `CommandErrorKind`. */
const COMMAND_ERROR_KINDS: Record<CommandErrorKind, true> = {
  invalidName: true,
  nameTaken: true,
  folderNotEmpty: true,
  alreadyRegistered: true,
  notAProject: true,
  notFound: true,
  defaultWorkspace: true,
  undoExpired: true,
  io: true,
  corruptState: true,
  newerVersion: true,
};

/** Indique si la cause d'un rejet est une `CommandError` renvoyée par le cœur natif. */
function isCommandError(cause: unknown): cause is CommandError {
  if (typeof cause !== "object" || cause === null) return false;
  if (!("kind" in cause) || !("message" in cause)) return false;
  const { kind, message } = cause;
  return (
    typeof kind === "string" &&
    Object.hasOwn(COMMAND_ERROR_KINDS, kind) &&
    typeof message === "string"
  );
}

/**
 * Erreur levée quand une commande Rust échoue ou que l'IPC est indisponible.
 *
 * Si le cœur natif a renvoyé une `CommandError`, `kind` en donne la nature et
 * `message` reprend son texte, prêt à afficher. Sinon `kind` vaut `null` et
 * `message` décrit la cause brute.
 */
export class IpcError extends Error {
  readonly command: CommandName;
  readonly kind: CommandErrorKind | null;

  constructor(command: CommandName, cause: unknown) {
    const commandError = isCommandError(cause) ? cause : null;
    super(commandError?.message ?? `La commande « ${command} » a échoué : ${describe(cause)}`, {
      cause,
    });
    this.name = "IpcError";
    this.command = command;
    this.kind = commandError?.kind ?? null;
  }
}

function describe(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  try {
    return JSON.stringify(cause);
  } catch {
    return String(cause);
  }
}

async function call<T>(command: CommandName, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw new IpcError(command, cause);
  }
}

/** Commandes du cœur natif, une fonction par commande. */
export const ipc = {
  /** Renvoie « Hello World » et l'heure du cœur natif. */
  hello: (): Promise<HelloResponse> => call<HelloResponse>("hello"),

  /** Catalogue au démarrage, avec l'avertissement éventuel d'un état illisible mis de côté. */
  catalogLoad: (): Promise<CatalogLoad> => call<CatalogLoad>("catalog_load"),

  /** Annule le retrait d'un projet ou la suppression d'un workspace (R6). */
  undo: (undoToken: string): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("undo", { undoToken }),

  /** Dossier proposé `<emplacement>/<nom>` ; `location` vaut `null` pour `~/arnod`. */
  projectSuggestFolder: (name: string, location: string | null): Promise<string> =>
    call<string>("project_suggest_folder", { name, location }),

  /** Sélecteur de dossier natif, ouvert sur `location` ; `null` si l'utilisateur annule. */
  projectPickFolder: (location: string | null): Promise<string | null> =>
    call<string | null>("project_pick_folder", { location }),

  /** Crée le dossier et le manifeste du projet, puis l'inscrit (P1, P2). */
  projectCreate: (input: ProjectCreateInput): Promise<ProjectCreated> =>
    call<ProjectCreated>("project_create", { ...input }),

  /** Inscrit un dossier projet existant (avec `arnod.yml`) au référentiel (P12). */
  projectAddExisting: (path: string, workspaceId: string): Promise<ProjectCreated> =>
    call<ProjectCreated>("project_add_existing", { path, workspaceId }),

  /** Projet à ouvrir ; échoue si son dossier est introuvable (P10). */
  projectOpen: (projectId: string): Promise<ProjectView> =>
    call<ProjectView>("project_open", { projectId }),

  /** Ouvre un workspace et le mémorise pour le prochain démarrage (W2). */
  workspaceOpen: (workspaceId: string): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("workspace_open", { workspaceId }),

  /** Crée un workspace vide (W5). */
  workspaceCreate: (name: string): Promise<WorkspaceCreated> =>
    call<WorkspaceCreated>("workspace_create", { name }),

  /** Renomme un workspace (W7). */
  workspaceRename: (workspaceId: string, name: string): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("workspace_rename", { workspaceId, name }),

  /** Change la couleur d'un workspace (W8). */
  workspaceSetColor: (workspaceId: string, color: WorkspaceColor): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("workspace_set_color", { workspaceId, color }),

  /** Transfère le rôle de workspace par défaut (W9). */
  workspaceSetDefault: (workspaceId: string): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("workspace_set_default", { workspaceId }),

  /** Ajoute des projets du référentiel à un workspace (W6, P6). */
  workspaceAddProjects: (workspaceId: string, projectIds: string[]): Promise<CatalogSnapshot> =>
    call<CatalogSnapshot>("workspace_add_projects", { workspaceId, projectIds }),

  /** Retire un projet d'un workspace, sans le supprimer ; annulable (P7). */
  workspaceRemoveProject: (workspaceId: string, projectId: string): Promise<Undoable> =>
    call<Undoable>("workspace_remove_project", { workspaceId, projectId }),

  /** Supprime un workspace, sans supprimer ses projets ; annulable (W11). */
  workspaceDelete: (workspaceId: string): Promise<Undoable> =>
    call<Undoable>("workspace_delete", { workspaceId }),

  /** Titre de la fenêtre : « Arnod — <sujet> », ou « Arnod » si `subject` vaut `null`. */
  windowSetTitle: (subject: string | null): Promise<void> =>
    call<null>("window_set_title", { subject }).then(() => undefined),
};
