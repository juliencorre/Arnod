/**
 * Wrapper typé autour d'`invoke` : seul point d'accès aux commandes Rust et aux
 * événements du cœur.
 *
 * Les composants et stores n'importent jamais `@tauri-apps/api/core` ni
 * `@tauri-apps/api/event` directement ; ils passent par `ipc` et `events`.
 */
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  CatalogLoad,
  CatalogSnapshot,
  CommandError,
  CommandErrorKind,
  CommandName,
  ContentRef,
  ContractSource,
  ContractSummary,
  ContractView,
  GenerateReport,
  LayoutPreset,
  Placement,
  ProjectChange,
  Side,
  WindowBounds,
  WindowContext,
  WindowsSnapshot,
  EnvironmentDoc,
  EnvironmentsView,
  HelloResponse,
  Assertion,
  ProposeInput,
  RequestDoc,
  RunView,
  SendOutcome,
  TreeView,
  ProjectCreated,
  ProjectCreateInput,
  ProjectLastRun,
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
  undoExpired: true,
  io: true,
  corruptState: true,
  newerVersion: true,
  unknownVariable: true,
  invalidRequest: true,
  network: true,
  timeout: true,
  cancelled: true,
  secretStore: true,
  protectedEnvironment: true,
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

  /** Inscrit un dossier projet existant (avec `arnod.yml`) dans « Tous les projets » (P12). */
  projectAddExisting: (path: string, workspaceId: string | null): Promise<ProjectCreated> =>
    call<ProjectCreated>("project_add_existing", { path, workspaceId }),

  /** Oublie un projet (« Tous les projets » et workspaces), fichiers conservés ; annulable (P11). */
  projectForget: (projectId: string): Promise<Undoable> =>
    call<Undoable>("project_forget", { projectId }),

  /** Projet à ouvrir ; échoue si son dossier est introuvable (P10). */
  projectOpen: (projectId: string): Promise<ProjectView> =>
    call<ProjectView>("project_open", { projectId }),

  /** Ouvre l'onglet d'un workspace, ou « Tous les projets » avec `null`, et le mémorise (R7). */
  workspaceOpen: (workspaceId: string | null): Promise<CatalogSnapshot> =>
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
  /** Ajoute des projets à un workspace (W6, P6, O7). */
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

  // --- Arborescence --------------------------------------------------------

  treeLoad: (project: string): Promise<TreeView> => call<TreeView>("tree_load", { project }),

  treeSetCollapsed: (project: string, collapsed: string[]): Promise<void> =>
    call<null>("tree_set_collapsed", { project, collapsed }).then(() => undefined),

  treeCreateFolder: (project: string, parent: string, name: string): Promise<string> =>
    call<string>("tree_create_folder", { project, parent, name }),

  /** Crée une requête ; sans `name`, un nom par défaut est choisi. Renvoie son chemin. */
  treeCreateRequest: (project: string, folder: string, name: string | null): Promise<string> =>
    call<string>("tree_create_request", { project, folder, name }),

  treeRename: (project: string, path: string, name: string): Promise<string> =>
    call<string>("tree_rename", { project, path, name }),

  treeMove: (project: string, path: string, targetFolder: string): Promise<string> =>
    call<string>("tree_move", { project, path, targetFolder }),

  treeDuplicate: (project: string, path: string): Promise<string> =>
    call<string>("tree_duplicate", { project, path }),

  treeDelete: (project: string, path: string, permanent: boolean): Promise<void> =>
    call<null>("tree_delete", { project, path, permanent }).then(() => undefined),

  // --- Requêtes ------------------------------------------------------------

  requestRead: (project: string, path: string): Promise<RequestDoc> =>
    call<RequestDoc>("request_read", { project, path }),

  requestSave: (project: string, path: string, request: RequestDoc): Promise<void> =>
    call<null>("request_save", { project, path, request }).then(() => undefined),

  /** Envoie la requête de l'éditeur ; `execId` permet de l'annuler. */
  requestSend: (
    project: string,
    execId: string,
    environment: string,
    request: RequestDoc,
    confirmed: boolean,
  ): Promise<SendOutcome> =>
    call<SendOutcome>("request_send", { project, execId, environment, request, confirmed }),

  requestCancel: (execId: string): Promise<void> =>
    call<null>("request_cancel", { execId }).then(() => undefined),

  requestProposeAssertions: (response: ProposeInput): Promise<Assertion[]> =>
    call<Assertion[]>("request_propose_assertions", { response }),

  // --- Environnements ------------------------------------------------------

  environmentsLoad: (project: string): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environments_load", { project }),

  environmentSelect: (project: string, name: string): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_select", { project, name }),

  /** Enregistre un environnement ; `previous` est l'ancien nom en cas de renommage. */
  environmentSave: (
    project: string,
    environment: EnvironmentDoc,
    previous: string | null,
  ): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_save", { project, environment, previous }),

  environmentDelete: (project: string, name: string): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_delete", { project, name }),

  environmentsReorder: (project: string, names: string[]): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environments_reorder", { project, names }),

  /** Valeur d'une variable locale ; un secret va dans le coffre du système (R9). */
  localSet: (
    project: string,
    environment: string,
    name: string,
    value: string,
    secret: boolean,
  ): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("local_set", { project, environment, name, value, secret }),

  // --- Runs ----------------------------------------------------------------

  contractList: (project: string): Promise<ContractSummary[]> =>
    call<ContractSummary[]>("contract_list", { project }),
  contractRead: (project: string, path: string): Promise<ContractView> =>
    call<ContractView>("contract_read", { project, path }),
  contractAdd: (project: string, source: ContractSource): Promise<ContractView> =>
    call<ContractView>("contract_add", { project, source }),
  /** Sélecteur de fichier natif ; `replace` remplace la source de ce contrat. `null` si annulé. */
  contractImportFile: (project: string, replace: string | null): Promise<ContractView | null> =>
    call<ContractView | null>("contract_import_file", { project, replace }),
  contractSave: (project: string, path: string, text: string): Promise<ContractView> =>
    call<ContractView>("contract_save", { project, path, text }),
  contractDelete: (project: string, path: string, permanent: boolean): Promise<void> =>
    call<null>("contract_delete", { project, path, permanent }).then(() => undefined),
  contractGenerate: (
    project: string,
    path: string,
    operations: string[],
    environment: string | null,
  ): Promise<GenerateReport> =>
    call<GenerateReport>("contract_generate", { project, path, operations, environment }),
  contractRun: (
    project: string,
    path: string,
    environment: string,
    confirmed: boolean,
  ): Promise<RunView> => call<RunView>("contract_run", { project, path, environment, confirmed }),
  runStart: (
    project: string,
    folder: string,
    environment: string,
    confirmed: boolean,
  ): Promise<RunView> => call<RunView>("run_start", { project, folder, environment, confirmed }),

  runStatus: (run: string): Promise<RunView> => call<RunView>("run_status", { run }),

  runCancel: (run: string): Promise<void> =>
    call<null>("run_cancel", { run }).then(() => undefined),

  runRetryFailures: (project: string, run: string, confirmed: boolean): Promise<RunView> =>
    call<RunView>("run_retry_failures", { project, run, confirmed }),

  runLast: (project: string): Promise<RunView | null> =>
    call<RunView | null>("run_last", { project }),

  /** Dernier run de chaque projet qui en a un (O12). */
  projectLastRuns: (): Promise<ProjectLastRun[]> => call<ProjectLastRun[]>("project_last_runs"),

  // --- Fenêtres et zones (F1 à F13) -----------------------------------------

  /** Étiquette de la fenêtre appelante et état de toutes les fenêtres. */
  windowsLoad: (): Promise<WindowContext> => call<WindowContext>("windows_load"),

  /** Ouvre un contenu dans cette fenêtre, dans `zone` ou la zone active (F1). */
  layoutOpen: (content: ContentRef, zone: string | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_open", { content, zone }),

  /** Ouvre la sélection de la barre latérale dans la fenêtre qui la suit (Z5). */
  layoutOpenSelection: (content: ContentRef): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_open_selection", { content }),

  layoutActivate: (zone: string, tab: string): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_activate", { zone, tab }),

  layoutClose: (tab: string): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_close", { tab }),

  /** Déplace un onglet vers une zone de cette fenêtre ou d'une autre (F3). */
  layoutMove: (
    tab: string,
    toWindow: string,
    zone: string | null,
    index: number | null,
  ): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_move", { tab, toWindow, zone, index }),

  /** Ouvre l'onglet dans une nouvelle zone à droite ou en dessous de `zone` (F2). */
  layoutSplit: (zone: string, tab: string, side: Side): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_split", { zone, tab, side }),

  layoutResize: (split: string, ratio: number): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_resize", { split, ratio }),

  layoutPreset: (preset: LayoutPreset): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_preset", { preset }),

  layoutSetPlacement: (zone: string, placement: Placement): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_set_placement", { zone, placement }),

  layoutSetFollow: (follow: boolean): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_set_follow", { follow }),

  layoutSetSidebarWidth: (width: number): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_set_sidebar_width", { width }),

  /** La zone devient une fenêtre, à `bounds` si donné (F8). */
  windowDetachZone: (zone: string, bounds: WindowBounds | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("window_detach_zone", { zone, bounds }),

  /** L'onglet seul devient une fenêtre, à `bounds` si donné (F8). */
  windowDetachTab: (tab: string, bounds: WindowBounds | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("window_detach_tab", { tab, bounds }),

  /** La barre latérale devient une fenêtre pilote (F9). */
  windowDetachSidebar: (bounds: WindowBounds | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("window_detach_sidebar", { bounds }),

  /** Rattache cette fenêtre à la fenêtre principale (F12). */
  windowReattach: (): Promise<WindowsSnapshot> => call<WindowsSnapshot>("window_reattach"),

  windowReattachAll: (): Promise<WindowsSnapshot> => call<WindowsSnapshot>("window_reattach_all"),
};

/** Événements diffusés par le cœur à toutes les fenêtres (F11). */
export const EVENTS = {
  windows: "arnod://fenetres",
  catalog: "arnod://catalogue",
  project: "arnod://projet",
  close: "arnod://fermer",
} as const;

/** Abonnements aux événements du cœur ; chacun renvoie sa fonction de désabonnement. */
export const events = {
  /** Toutes les fenêtres, après chaque changement de disposition. */
  onWindows: (handler: (snapshot: WindowsSnapshot) => void): Promise<UnlistenFn> =>
    listen<WindowsSnapshot>(EVENTS.windows, (event) => {
      handler(event.payload);
    }),

  /** Nouvel instantané du catalogue, après une modification dans n'importe quelle fenêtre. */
  onCatalog: (handler: (snapshot: CatalogSnapshot) => void): Promise<UnlistenFn> =>
    listen<CatalogSnapshot>(EVENTS.catalog, (event) => {
      handler(event.payload);
    }),

  /** Modification d'un projet (arborescence, requête, environnements). */
  onProject: (handler: (change: ProjectChange) => void): Promise<UnlistenFn> =>
    listen<ProjectChange>(EVENTS.project, (event) => {
      handler(event.payload);
    }),

  /** Fermeture demandée pour cette fenêtre détachée : enregistrer puis se rattacher. */
  onClose: (handler: () => void): Promise<UnlistenFn> =>
    listen(EVENTS.close, () => {
      handler();
    }),
};
