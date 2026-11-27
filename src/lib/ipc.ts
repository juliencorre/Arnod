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
  Appearance,
  CasePreview,
  CasesOutcome,
  DataSet,
  DataTable,
  Flow,
  FolderHooks,
  FolderProperty,
  HookPhase,
  HooksView,
  InstanceView,
  Literal,
  StepAdded,
  StepKind,
  StepPlacement,
  TemplateCandidate,
  TemplateParam,
  TemplateSummary,
  TemplateView,
  HistoryDiff,
  HistoryEntry,
  HistoryView,
  SearchResults,
  CliShell,
  JournalEntry,
  MissingSecrets,
  SyncAccount,
  SyncChoice,
  SyncConflicts,
  SyncDeviceCode,
  SyncForge,
  SyncLoginOptions,
  SyncRepository,
  SyncView,
  InterfaceView,
  ShortcutChange,
  ConflictSide,
  FilesChange,
  RequestConflict,
  ActiveRequest,
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
  EnvironmentDeleted,
  EnvironmentDoc,
  EnvironmentsView,
  EnvironmentUsage,
  NewEnvironment,
  HelloResponse,
  Assertion,
  Example,
  ExampleInput,
  ProposeInput,
  RequestDoc,
  RequestResult,
  ResultChange,
  RequestStatuses,
  RunChange,
  RunView,
  StatusesChange,
  SendOutcome,
  StepCatalog,
  Detection,
  TrialResult,
  CurlCopy,
  Pair,
  PairsFormat,
  PairsText,
  RequestInput,
  ImportDone,
  ImportOptions,
  ImportReport,
  ImportSource,
  ImportTarget,
  ScriptChoice,
  ScriptResolved,
  TreeView,
  ProjectCreated,
  ProjectCreateInput,
  ProjectLastRun,
  ProjectView,
  Undoable,
  WorkspaceColor,
  WorkspaceCreated,
  FilterResult,
  HtmlNode,
  RenderedBody,
  ResponseFormat,
  CreatedTest,
  TestView,
  VariableUser,
  PathGeneralization,
  TransferredTrial,
  FiledTrials,
  DynamicValue,
  Auth,
  CertificateInfo,
  CookieJarView,
  NetworkPrefsInput,
  NetworkView,
  ProjectAuthView,
  NetworkFailure,
  MenuShortcut,
  StartupProblem,
  StartupTarget,
  TransferChange,
  TransferDone,
  TransferMode,
  TransferTarget,
  TransferTargets,
  UpdateChannel,
  UpdatesView,
  BatchAction,
  BatchResult,
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
  changedOnDisk: true,
  conflict: true,
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
  /** Lot H : échec réseau classé, s'il y en a un (5.1-i). */
  readonly failure: NetworkFailure | null;

  constructor(command: CommandName, cause: unknown) {
    const commandError = isCommandError(cause) ? cause : null;
    super(commandError?.message ?? `La commande « ${command} » a échoué : ${describe(cause)}`, {
      cause,
    });
    this.name = "IpcError";
    this.command = command;
    this.kind = commandError?.kind ?? null;
    this.failure = commandError?.failure ?? null;
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

  /** Réordonne le contenu d'un dossier : rang de 1 à n dans l'ordre `order` (AM38). */
  treeReorder: (project: string, folder: string, order: string[]): Promise<void> =>
    call<null>("tree_reorder", { project, folder, order }).then(() => undefined),

  // --- Requêtes ------------------------------------------------------------

  requestRead: (project: string, path: string): Promise<RequestDoc> =>
    call<RequestDoc>("request_read", { project, path }),

  requestSave: (project: string, path: string, request: RequestDoc): Promise<void> =>
    call<null>("request_save", { project, path, request }).then(() => undefined),

  /**
   * Envoie la requête de l'éditeur ; `execId` permet de l'annuler, `path` la situe pour
   * l'héritage de ses dossiers (R3, R4).
   */
  requestSend: (
    project: string,
    execId: string,
    environment: string,
    path: string | null,
    request: RequestDoc,
    confirmed: boolean,
  ): Promise<SendOutcome> =>
    call<SendOutcome>("request_send", {
      project,
      execId,
      environment,
      path,
      request,
      confirmed,
    }),

  requestCancel: (execId: string): Promise<void> =>
    call<null>("request_cancel", { execId }).then(() => undefined),

  /** Dernier envoi de la requête gardé par le cœur, ou `null` (R5). */
  requestResult: (project: string, path: string): Promise<RequestResult | null> =>
    call<RequestResult | null>("request_result", { project, path }),

  requestProposeAssertions: (response: ProposeInput): Promise<Assertion[]> =>
    call<Assertion[]>("request_propose_assertions", { response }),

  /** Réponse attendue reprise d'une réponse reçue (R7). */
  requestExampleFromResponse: (response: ExampleInput, name: string): Promise<Example> =>
    call<Example>("request_example_from_response", { response, name }),

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

  /** Supprime un environnement ; rend les environnements et le jeton de « Annuler » (ER1). */
  environmentDelete: (project: string, name: string): Promise<EnvironmentDeleted> =>
    call<EnvironmentDeleted>("environment_delete", { project, name }),
  /** « Annuler » la suppression : fichiers, place, valeurs locales et secrets reviennent. */
  environmentDeleteUndo: (undoToken: string): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_delete_undo", { undoToken }),

  /** Crée un environnement (3.3-b, AM34) : variables reprises, place dans la chaîne. */
  environmentCreate: (project: string, environment: NewEnvironment): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_create", { project, environment }),

  /** « Copier depuis un autre projet… » (3.3-a, AM43) : sans valeurs locales ni secrets. */
  environmentCopy: (
    project: string,
    fromProject: string,
    names: string[],
  ): Promise<EnvironmentsView> =>
    call<EnvironmentsView>("environment_copy", { project, fromProject, names }),

  /** Dernier run de chaque environnement du projet (3.3-a, AM43, « Utilisation »). */
  environmentUsage: (project: string): Promise<EnvironmentUsage[]> =>
    call<EnvironmentUsage[]>("environment_usage", { project }),

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
  /** Met un contrat à la corbeille ; rend le jeton de « Annuler » (ER1). */
  contractDelete: (project: string, path: string, permanent: boolean): Promise<string> =>
    call<string>("contract_delete", { project, path, permanent }),
  /** « Annuler » la suppression d'un contrat : son fichier revient tel qu'il était. */
  contractDeleteUndo: (undoToken: string): Promise<void> =>
    call<null>("contract_delete_undo", { undoToken }).then(() => undefined),
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

  // --- Catalogue des étapes et essais (docs/plan-accueil-non-classes.md)

  stepsCatalog: (): Promise<StepCatalog> => call<StepCatalog>("steps_catalog"),

  stepsDetect: (input: string): Promise<Detection | null> =>
    call<Detection | null>("steps_detect", { input }),

  /** `secrets` : secrets d'une commande cURL collée, gardés au coffre à l'envoi. */
  trialSend: (
    execId: string,
    nature: string,
    subject: string,
    values: Record<string, string>,
    secrets: Record<string, string> = {},
  ): Promise<TrialResult> =>
    call<TrialResult>("trial_send", { execId, nature, subject, values, secrets }),

  // Lot C - Import Bruno (docs/plan-mvp-c-import-bruno.md).
  /** Sélecteur natif : un dossier, ou un fichier avec `file` ; `null` si annulé. */
  importPick: (file: boolean): Promise<string | null> =>
    call<string | null>("import_pick", { file }),
  /** Reconnaît le format d'un dossier ou d'un fichier, sans rien écrire. */
  importDetect: (path: string): Promise<ImportSource> =>
    call<ImportSource>("import_detect", { path }),
  /** Importe dans un nouveau projet ou dans un projet existant. */
  importRun: (path: string, target: ImportTarget, options: ImportOptions): Promise<ImportDone> =>
    call<ImportDone>("import_run", { path, target, options }),
  /** Dernier rapport d'import d'un projet. */
  importReport: (projectId: string): Promise<ImportReport | null> =>
    call<ImportReport | null>("import_report", { projectId }),
  /** Reprend un script Bruno du rapport : retiré du fichier, sans lui ou par une assertion (AM42). */
  importScript: (projectId: string, index: number, choice: ScriptChoice): Promise<ScriptResolved> =>
    call<ScriptResolved>("import_script", { projectId, index, choice }),
  /** « Annuler » un script repris : il revient dans son fichier. */
  importScriptUndo: (undoToken: string): Promise<ImportReport> =>
    call<ImportReport>("import_script_undo", { undoToken }),
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

  /** Ignore l'échec d'une vérification dans une ligne d'un run (R7 de la fin du premier usage). */
  runIgnoreCheck: (project: string, run: string, path: string, check: string): Promise<RunView> =>
    call<RunView>("run_ignore_check", { project, run, path, check }),

  /** Statut de la dernière exécution de chaque requête du projet (R5). */
  requestStatuses: (project: string): Promise<RequestStatuses> =>
    call<RequestStatuses>("request_statuses", { project }),

  /** Ignore l'échec d'une vérification dans le dernier résultat d'une requête (R7). */
  resultIgnoreCheck: (project: string, path: string, check: string): Promise<RequestResult> =>
    call<RequestResult>("result_ignore_check", { project, path, check }),

  /** Dernier run de chaque projet qui en a un (O12). */
  projectLastRuns: (): Promise<ProjectLastRun[]> => call<ProjectLastRun[]>("project_last_runs"),

  // --- Fenêtres et zones (F1 à F13) -----------------------------------------

  /** Étiquette de la fenêtre appelante et état de toutes les fenêtres. */
  windowsLoad: (): Promise<WindowContext> => call<WindowContext>("windows_load"),

  /** Ouvre un contenu dans cette fenêtre, dans `zone` ou la zone active (F1). */
  layoutOpen: (content: ContentRef, zone: string | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_open", { content, zone }),

  /**
   * Ouvre la sélection de la barre latérale dans la fenêtre qui la suit (Z5) ; en aperçu si
   * `preview` (simple clic, 1.3-a).
   */
  layoutOpenSelection: (content: ContentRef, preview = false): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>(
      "layout_open_selection",
      // Sans aperçu, l'argument est omis : le cœur le prend à `false`.
      preview ? { content, preview } : { content },
    ),

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

  /** Ouvre le résultat dans sa fenêtre, ou met la sienne au premier plan (D1, R1). */
  windowDetachResult: (bounds: WindowBounds | null): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("window_detach_result", { bounds }),

  /** Rattache cette fenêtre à la fenêtre principale (F12). */
  windowReattach: (): Promise<WindowsSnapshot> => call<WindowsSnapshot>("window_reattach"),

  windowReattachAll: (): Promise<WindowsSnapshot> => call<WindowsSnapshot>("window_reattach_all"),

  // --- Lot A - Réponse (docs/plan-mvp-a-reponse.md) ---------------------------------------

  /** Corps mis en forme dans `format`, ou dans le format détecté (`null`). */
  responseRender: (
    body: string,
    headers: [string, string][],
    format: ResponseFormat | null,
  ): Promise<RenderedBody> => call<RenderedBody>("response_render", { body, headers, format }),

  /** Aperçu d'un corps HTML, sans script ni ressource distante. */
  responsePreview: (body: string): Promise<HtmlNode[]> =>
    call<HtmlNode[]>("response_preview", { body }),

  /** Filtre JSONPath évalué par le cœur. */
  responseFilter: (body: string, expression: string): Promise<FilterResult> =>
    call<FilterResult>("response_filter", { body, expression }),

  /** Enregistre le corps (sélecteur natif) ; renvoie le chemin écrit, `null` si annulé. */
  responseSave: (
    body: string,
    name: string,
    format: ResponseFormat,
    truncated: boolean,
  ): Promise<string | null> =>
    call<string | null>("response_save", { body, name, format, truncated }),

  /** Efface la réponse affichée d'une requête, dans toutes les fenêtres. */
  responseClear: (project: string, path: string): Promise<RequestResult> =>
    call<RequestResult>("response_clear", { project, path }),
  // --- Lot D - Barre latérale (docs/plan-mvp-d-barre-laterale.md) -------------

  /** Nombre de contrats du projet, sans les lire (W1, R5). */
  contractCount: (project: string): Promise<number> => call<number>("contract_count", { project }),

  // --- Lot B : saisie, texte et cURL (docs/plan-mvp-b-saisie-curl.md) -------------

  /** Variables connues et inconnues, en-têtes ajoutés à l'envoi (R1 à R3). */
  requestInput: (
    project: string,
    environment: string,
    path: string | null,
    request: RequestDoc,
  ): Promise<RequestInput> =>
    call<RequestInput>("request_input", { project, environment, path, request }),

  /** La requête en commande cURL, secrets remplacés par une variable du shell (R8 à R10). */
  requestCurl: (
    project: string,
    environment: string,
    path: string | null,
    request: RequestDoc,
  ): Promise<CurlCopy> => call<CurlCopy>("request_curl", { project, environment, path, request }),

  /** Une liste de paires en texte, une ligne par paire (R11). */
  pairsToText: (pairs: Pair[], format: PairsFormat): Promise<string> =>
    call<string>("pairs_to_text", { pairs, format }),

  /** Les paires d'un texte et ses lignes illisibles (R12, R13). */
  pairsFromText: (text: string, format: PairsFormat, previous: Pair[]): Promise<PairsText> =>
    call<PairsText>("pairs_from_text", { text, format, previous }),

  // --- Lot E : tests et étapes (docs/plan-mvp-e-tests-etapes.md)

  /** Test ouvert dans un onglet : ses étapes dans l'ordre (TE2). */
  testRead: (project: string, path: string): Promise<TestView> =>
    call<TestView>("test_read", { project, path }),

  /** « Nouveau test » : dossier du test et son étape 1 (NT1). */
  /** « Nouveau test » sous `parent`, nommé `name` (vue du dossier, AM37) ou par défaut. */
  treeCreateTest: (
    project: string,
    parent: string,
    name: string | null = null,
  ): Promise<CreatedTest> => call<CreatedTest>("tree_create_test", { project, parent, name }),

  /** « + Étape » : étape suivante du test ; renvoie son chemin (NT2). */
  treeAddStep: (project: string, test: string): Promise<string> =>
    call<string>("tree_add_step", { project, test }),

  // --- Version 4 (docs/plan-v4.md) : tests en schéma, données, templates ---------------
  /** Enregistre le schéma d'un test (R1, R3) ; rend l'onglet du test. */
  flowSave: (project: string, path: string, flow: Flow): Promise<TestView> =>
    call<TestView>("flow_save", { project, path, flow }),
  /** Ajoute une étape sur un lien ou en fin de liste (4.3-e, 4.3-b bis). */
  flowAddStep: (
    project: string,
    path: string,
    kind: StepKind,
    placement: StepPlacement,
    name: string | null = null,
  ): Promise<StepAdded> =>
    call<StepAdded>("flow_add_step", { project, path, kind, placement, name }),
  /** Retire une étape (« Supprimer l'étape », « Annuler l'insertion ») ; rend le test. */
  flowRemoveStep: (project: string, path: string, step: string): Promise<TestView> =>
    call<TestView>("flow_remove_step", { project, path, step }),
  /** Étape requête insérée : la requête d'une opération d'un contrat, sans lien (4.3-b quater). */
  flowStepOperation: (
    project: string,
    path: string,
    step: string,
    contract: string,
    operation: string,
    environment: string | null,
  ): Promise<TestView> =>
    call<TestView>("flow_step_operation", {
      project,
      path,
      step,
      contract,
      operation,
      environment,
    }),
  hooksRemoveStep: (
    project: string,
    path: string,
    phase: HookPhase,
    step: string,
  ): Promise<HooksView> => call<HooksView>("hooks_remove_step", { project, path, phase, step }),
  /** Avant / Après d'un dossier (4.7-c). */
  hooksRead: (project: string, path: string): Promise<HooksView> =>
    call<HooksView>("hooks_read", { project, path }),
  hooksSave: (project: string, path: string, hooks: FolderHooks): Promise<HooksView> =>
    call<HooksView>("hooks_save", { project, path, hooks }),
  hooksAddStep: (
    project: string,
    path: string,
    phase: HookPhase,
    kind: StepKind,
    name: string | null = null,
  ): Promise<HooksView> => call<HooksView>("hooks_add_step", { project, path, phase, kind, name }),
  /** Propriétés d'un dossier et leur origine (4.8-a). */
  folderProperties: (
    project: string,
    path: string,
    environment: string,
  ): Promise<FolderProperty[]> =>
    call<FolderProperty[]>("folder_properties", { project, path, environment }),
  folderSetProperties: (
    project: string,
    path: string,
    properties: { name: string; value: string }[],
    environment: string,
  ): Promise<FolderProperty[]> =>
    call<FolderProperty[]>("folder_set_properties", { project, path, properties, environment }),
  /** Jeu de données d'une requête (4.7-a). */
  dataRead: (project: string, path: string): Promise<DataSet | null> =>
    call<DataSet | null>("data_read", { project, path }),
  dataSave: (project: string, path: string, table: DataTable): Promise<DataSet> =>
    call<DataSet>("data_save", { project, path, table }),
  /** « Fichier CSV… » : sélecteur natif (4.7-a bis) ; `null` si l'utilisateur annule. */
  dataImport: (project: string, path: string): Promise<DataSet | null> =>
    call<DataSet | null>("data_import", { project, path }),
  dataRemove: (project: string, path: string): Promise<void> =>
    call<undefined>("data_remove", { project, path }),
  /** Lignes d'un texte collé depuis un tableur (JD2). */
  dataParse: (text: string): Promise<string[][]> => call<string[][]>("data_parse", { text }),
  dataOpen: (project: string, path: string): Promise<void> =>
    call<undefined>("data_open", { project, path }),
  /** Requête d'un cas, résolue et masquée (4.7-a). */
  dataCase: (
    project: string,
    path: string,
    request: RequestDoc,
    environment: string,
    caseNumber: number,
  ): Promise<CasePreview> =>
    call<CasePreview>("data_case", { project, path, request, environment, case: caseNumber }),
  /** Envoie les cas d'un jeu de données (4.7-b, R12). */
  requestSendCases: (
    project: string,
    execId: string,
    environment: string,
    path: string,
    request: RequestDoc,
    confirmed: boolean,
    cases: number[] | null = null,
  ): Promise<CasesOutcome> =>
    call<CasesOutcome>("request_send_cases", {
      project,
      execId,
      environment,
      path,
      request,
      confirmed,
      cases,
    }),
  /** Templates du projet (R16). */
  templateList: (project: string): Promise<TemplateSummary[]> =>
    call<TemplateSummary[]>("template_list", { project }),
  templateCategories: (project: string): Promise<string[]> =>
    call<string[]>("template_categories", { project }),
  templateRead: (project: string, path: string): Promise<TemplateView> =>
    call<TemplateView>("template_read", { project, path }),
  templateSaveParams: (
    project: string,
    path: string,
    params: TemplateParam[],
  ): Promise<TemplateView> => call<TemplateView>("template_save_params", { project, path, params }),
  /** Paramètres proposés à la création d'un template (4.8-b, R18). */
  templateCandidates: (
    project: string,
    folder: string,
    environment: string,
  ): Promise<TemplateCandidate[]> =>
    call<TemplateCandidate[]>("template_candidates", { project, folder, environment }),
  templateCreate: (
    project: string,
    folder: string,
    name: string,
    category: string,
    params: TemplateParam[],
    replace: boolean,
  ): Promise<TemplateView> =>
    call<TemplateView>("template_create", { project, folder, name, category, params, replace }),
  /** « Utiliser dans un dossier… » (4.8-d) : chemin de l'utilisation. */
  templateUse: (project: string, template: string, folder: string, name: string): Promise<string> =>
    call<string>("template_use", { project, template, folder, name }),
  templateInstance: (project: string, path: string): Promise<InstanceView> =>
    call<InstanceView>("template_instance", { project, path }),
  templateSetValues: (
    project: string,
    path: string,
    values: Record<string, string>,
  ): Promise<InstanceView> => call<InstanceView>("template_set_values", { project, path, values }),
  /** Valeurs écrites d'un endpoint (3.2-d, R19). */
  templateLiterals: (project: string, path: string): Promise<Literal[]> =>
    call<Literal[]>("template_literals", { project, path }),
  templateFromRequest: (
    project: string,
    path: string,
    name: string,
    category: string,
    params: string[],
  ): Promise<TemplateView> =>
    call<TemplateView>("template_from_request", { project, path, name, category, params }),
  /** « Afficher » une section depuis « Plus » (1.2-d bis, R21). */
  sectionsShow: (project: string, section: string, shown: boolean): Promise<string[]> =>
    call<string[]>("sections_show", { project, section, shown }),

  /** Requêtes du projet, hors `path`, qui lisent l'une des variables `names` (EX5). */
  requestVariableUsers: (project: string, path: string, names: string[]): Promise<VariableUser[]> =>
    call<VariableUser[]>("request_variable_users", { project, path, names }),
  /** Chemin généralisé proposé pour un essai, s'il y en a un (2.2-c). */
  trialGeneralization: (project: string, path: string): Promise<PathGeneralization | null> =>
    call<PathGeneralization | null>("trial_generalization", { project, path }),
  /** « Généraliser » le chemin d'un essai ; renvoie les requêtes modifiées (2.2-c). */
  trialGeneralize: (project: string, path: string): Promise<string[]> =>
    call<string[]>("trial_generalize", { project, path }),
  /** « Garder tel quel » le chemin d'un essai (2.2-c). */
  trialKeepPath: (project: string, path: string): Promise<void> =>
    call<null>("trial_keep_path", { project, path }).then(() => undefined),
  /** « Transférer au référentiel de tests » : l'essai devient l'étape 1 d'un test (2.2-b). */
  trialTransfer: (
    project: string,
    path: string,
    name: string,
    parent: string,
  ): Promise<TransferredTrial> =>
    call<TransferredTrial>("trial_transfer", { project, path, name, parent }),
  /** « Ranger dans un projet » les essais d'un contrat déduit de « Non classés » (2.2-d). */
  trialFile: (
    project: string,
    contract: string,
    target: string,
    into: string | null,
  ): Promise<FiledTrials> => call<FiledTrials>("trial_file", { project, contract, target, into }),
  /** « Annuler » un rangement d'essais (2.2-d). */
  trialFileUndo: (undoToken: string): Promise<void> =>
    call<null>("trial_file_undo", { undoToken }).then(() => undefined),
  /** Valeurs dynamiques proposées à la frappe de `{{$`, avec un exemple (5.1-h). */
  dynamicValues: (): Promise<DynamicValue[]> => call<DynamicValue[]>("dynamic_values"),

  // --- Lot H - Réseau et authentification (docs/plan-mvp-h-reseau-auth.md) ----
  /** Préférences réseau du poste (1.6-e). */
  networkLoad: (): Promise<NetworkView> => call<NetworkView>("network_load"),
  networkSave: (prefs: NetworkPrefsInput): Promise<NetworkView> =>
    call<NetworkView>("network_save", { prefs }),
  /** Choisit un certificat et le lit, sans l'ajouter ; `null` : choix annulé (R4). */
  networkPickAuthority: (): Promise<CertificateInfo | null> =>
    call<CertificateInfo | null>("network_pick_authority"),
  networkAddAuthority: (pem: string): Promise<NetworkView> =>
    call<NetworkView>("network_add_authority", { pem }),
  networkRemoveAuthority: (fingerprint: string): Promise<NetworkView> =>
    call<NetworkView>("network_remove_authority", { fingerprint }),
  /** Authentification du projet, jeton et usage (3.4-a). */
  projectAuthLoad: (project: string, environment: string): Promise<ProjectAuthView> =>
    call<ProjectAuthView>("project_auth_load", { project, environment }),
  projectAuthSave: (
    project: string,
    environment: string,
    auth: Auth,
    clientSecret: string | null,
  ): Promise<ProjectAuthView> =>
    call<ProjectAuthView>("project_auth_save", { project, environment, auth, clientSecret }),
  oauth2TokenRenew: (project: string, environment: string): Promise<ProjectAuthView> =>
    call<ProjectAuthView>("oauth2_token_renew", { project, environment }),
  oauth2TokenForget: (project: string, environment: string): Promise<ProjectAuthView> =>
    call<ProjectAuthView>("oauth2_token_forget", { project, environment }),
  /** Cookies gardés de l'environnement ; avec `url`, ceux de son hôte (3.4-g). */
  cookiesList: (project: string, environment: string, url: string | null): Promise<CookieJarView> =>
    call<CookieJarView>("cookies_list", { project, environment, url }),
  cookiesClear: (
    project: string,
    environment: string,
    domain: string | null,
    url: string | null,
  ): Promise<CookieJarView> =>
    call<CookieJarView>("cookies_clear", { project, environment, domain, url }),
  cookiesSetSending: (
    project: string,
    environment: string,
    domain: string,
    enabled: boolean,
    url: string | null,
  ): Promise<CookieJarView> =>
    call<CookieJarView>("cookies_set_sending", { project, environment, domain, enabled, url }),
  // --- Lot F : onglets et clavier ---------------------------------------------

  /** L'onglet d'aperçu devient permanent : double-clic, modification, glisser (1.3-a). */
  layoutPin: (tab: string): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_pin", { tab }),

  /** Ferme plusieurs onglets de cette fenêtre : « Fermer les autres », « Fermer tout » (1.3-b). */
  layoutCloseTabs: (tabs: string[]): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_close_tabs", { tabs }),

  /** Masque ou affiche la barre latérale de la fenêtre principale (Ctrl+\, 1.4-j). */
  layoutSetSidebarHidden: (hidden: boolean): Promise<WindowsSnapshot> =>
    call<WindowsSnapshot>("layout_set_sidebar_hidden", { hidden }),
  // --- Lot I - Fichiers modifiés hors d'Arnod ---

  /** Éléments où le fichier et le brouillon diffèrent ; `base` : version de départ (R9). */
  requestConflict: (
    project: string,
    path: string,
    base: RequestDoc,
    draft: RequestDoc,
  ): Promise<RequestConflict> =>
    call<RequestConflict>("request_conflict", { project, path, base, draft }),

  /** Règle le conflit selon les choix et écrit le fichier une seule fois (R11). */
  requestConflictResolve: (
    project: string,
    path: string,
    draft: RequestDoc,
    choices: Record<string, ConflictSide>,
  ): Promise<RequestDoc> =>
    call<RequestDoc>("request_conflict_resolve", { project, path, draft, choices }),
  // Lot G - Démarrage et erreurs.

  /** Erreur bloquante du démarrage, décrite ; `null` si Arnod peut démarrer. */
  startupStatus: (): Promise<StartupProblem | null> =>
    call<StartupProblem | null>("startup_status"),

  /** Relit le fichier en échec ; `automatic` pour un essai automatique (R4). */
  startupRetry: (automatic: boolean): Promise<StartupProblem | null> =>
    call<StartupProblem | null>("startup_retry", { automatic }),

  /** Met le fichier en échec de côté (renommé, intact) et repart d'un état vide (R3). */
  startupStartEmpty: (): Promise<StartupProblem | null> =>
    call<StartupProblem | null>("startup_start_empty"),

  /** Ouvre le dossier de données, la page des versions ou l'issue préremplie (R6). */
  startupOpen: (target: StartupTarget): Promise<null> => call<null>("startup_open", { target }),

  /** Quitte Arnod sans rien écrire. */
  startupQuit: (): Promise<null> => call<null>("startup_quit"),

  /** L'interface est prête : la fenêtre, créée cachée, apparaît (0.3-a). */
  windowReady: (): Promise<null> => call<null>("window_ready"),

  // Lot J - Copier et déplacer (4.1-f, 4.1-g).
  /** Destinations de la requête `path` de `project` dans le projet `target`. */
  transferTargets: (project: string, path: string, target: string): Promise<TransferTargets> =>
    call<TransferTargets>("transfer_targets", { project, path, target }),
  /** « Ce qui change » si la requête va à `destination` du projet `target`. */
  transferPreview: (
    project: string,
    path: string,
    target: string,
    destination: TransferTarget,
    mode: TransferMode,
  ): Promise<TransferChange[]> =>
    call<TransferChange[]>("transfer_preview", { project, path, target, destination, mode }),
  /** Copie ou déplace la requête ; le jeton sert à « Annuler ». */
  transferApply: (
    project: string,
    path: string,
    target: string,
    destination: TransferTarget,
    mode: TransferMode,
  ): Promise<TransferDone> =>
    call<TransferDone>("transfer_apply", { project, path, target, destination, mode }),
  /** « Annuler » une copie ou un déplacement. */
  transferUndo: (undoToken: string): Promise<void> =>
    call<null>("transfer_undo", { undoToken }).then(() => undefined),

  // Lot L allégé - Mises à jour (1.6-g).
  /** Préférences et dernière version trouvée. */
  updatesLoad: (): Promise<UpdatesView> => call<UpdatesView>("updates_load"),
  /** Enregistre la vérification automatique et le canal. */
  updatesSave: (automatic: boolean, channel: UpdateChannel): Promise<UpdatesView> =>
    call<UpdatesView>("updates_save", { automatic, channel }),
  /** Cherche une version plus récente ; `automatic` : seulement si c'est dû. */
  updatesCheck: (automatic: boolean): Promise<UpdatesView> =>
    call<UpdatesView>("updates_check", { automatic }),
  /** Ouvre la page d'une release dans le navigateur du système. */
  updatesOpen: (url: string): Promise<void> =>
    call<null>("updates_open", { url }).then(() => undefined),

  // Sélection multiple (4.1-a quater).
  /** Déplace, duplique ou met à la corbeille les éléments `paths` ; le jeton sert à « Annuler ». */
  treeBatch: (project: string, paths: string[], action: BatchAction): Promise<BatchResult> =>
    call<BatchResult>("tree_batch", { project, paths, action }),
  /** « Annuler » une action sur la sélection. */
  treeBatchUndo: (undoToken: string): Promise<void> =>
    call<null>("tree_batch_undo", { undoToken }).then(() => undefined),
  /** Exécute les requêtes de la sélection, contenu des dossiers compris. */
  runSelection: (
    project: string,
    paths: string[],
    environment: string,
    confirmed: boolean,
  ): Promise<RunView> => call<RunView>("run_selection", { project, paths, environment, confirmed }),

  // Lot L - Mises à jour installées par Arnod (1.6-g).
  /** Télécharge et vérifie la version trouvée ; la progression arrive par `onUpdates`. */
  updatesDownload: (): Promise<UpdatesView> => call<UpdatesView>("updates_download"),
  /** Arrête le téléchargement et oublie le paquet téléchargé. */
  updatesCancel: (): Promise<UpdatesView> => call<UpdatesView>("updates_cancel"),
  /**
   * Installe le paquet vérifié : `now`, Arnod se ferme et redémarre (la promesse ne se résout
   * pas) ; sinon, à la fermeture d'Arnod.
   */
  updatesInstall: (now: boolean): Promise<UpdatesView> =>
    call<UpdatesView>("updates_install", { now }),

  // --- Plan qualité, lot C : sortie sans perdre un brouillon (R6) ---
  /** Réponse à `arnod://quitter` : `saved` faux abandonne la sortie d'Arnod. */
  windowQuitReady: (saved: boolean): Promise<void> =>
    call<null>("window_quit_ready", { saved }).then(() => undefined),

  // --- Version 1 (docs/plan-v1.md) : apparence, raccourcis (1.6-f, 1.6-f bis) ---
  /** Préférences d'interface ; la fenêtre prend son thème et son zoom. */
  interfaceLoad: (): Promise<InterfaceView> => call<InterfaceView>("interface_load"),
  /** Thème, zoom et police des éditeurs ; appliqués et diffusés à toutes les fenêtres. */
  interfaceSaveAppearance: (appearance: Appearance): Promise<InterfaceView> =>
    call<InterfaceView>("interface_save_appearance", { appearance }),
  /**
   * Donne `keys` à l'action (`null` : retire son raccourci). Un conflit revient sans rien
   * changer, sauf `replace`.
   */
  interfaceSetShortcut: (
    action: string,
    keys: string | null,
    replace: boolean,
  ): Promise<ShortcutChange> =>
    call<ShortcutChange>("interface_set_shortcut", { action, keys, replace }),
  /** « Rétablir les raccourcis par défaut ». */
  interfaceResetShortcuts: (): Promise<InterfaceView> =>
    call<InterfaceView>("interface_reset_shortcuts"),

  // --- Version 1 : historique des envois (5.1-c bis) ---
  /** Envois gardés de la requête, chacun comparé au précédent. */
  historyLoad: (project: string, path: string): Promise<HistoryView> =>
    call<HistoryView>("history_load", { project, path }),
  /** Différences d'un envoi avec le précédent. */
  historyDiff: (project: string, path: string, id: string): Promise<HistoryDiff> =>
    call<HistoryDiff>("history_diff", { project, path, id }),
  /** « Effacer » l'historique de la requête. */
  historyClear: (project: string, path: string): Promise<HistoryView> =>
    call<HistoryView>("history_clear", { project, path }),
  /** « Annuler » après « Effacer » : les envois effacés reviennent. */
  historyRestore: (project: string, path: string, entries: HistoryEntry[]): Promise<HistoryView> =>
    call<HistoryView>("history_restore", { project, path, entries }),

  // --- Version 2, lot A : recherche Ctrl+K (1.5-a, 1.5-b) ---
  /**
   * Résultats de `text` dans les projets du workspace (`null` : tous les projets). `refresh` relit
   * les projets (à l'ouverture de la recherche) ; sinon le cœur reprend ceux qu'il a lus.
   */
  searchQuery: (workspace: string | null, text: string, refresh: boolean): Promise<SearchResults> =>
    call<SearchResults>("search_query", { workspace, text, refresh }),

  // --- Version 2, lot B : étiquettes partagées (4.9-b) ---
  /** Étiquettes d'un dossier : elles valent pour tout ce qu'il contient. */
  treeSetFolderTags: (project: string, path: string, tags: string[]): Promise<void> =>
    call<null>("tree_set_folder_tags", { project, path, tags }).then(() => undefined),

  // --- Version 2, lot D : ligne de commande (5.3-c) ---
  /**
   * Commande `arnod run` d'un test ou d'un dossier (`path`), avec l'environnement donné, citée
   * pour le terminal du poste (`shell` absent) ; refusée (`notFound`) si le dossier n'existe plus.
   */
  cliCommand: (
    project: string,
    path: string,
    environment: string | null,
    shell: CliShell | null = null,
  ): Promise<string> => call<string>("cli_command", { project, path, environment, shell }),

  // --- Version 2, lots E et F : synchronisation Git (7.1-a à 7.1-e) ---
  /** État de la synchronisation du projet, sans réseau. */
  syncStatus: (project: string): Promise<SyncView> => call<SyncView>("sync_status", { project }),
  /** Vérifie ce qui arrive des collègues (Q6), sans rien écrire dans le projet. */
  syncCheck: (project: string): Promise<SyncView> => call<SyncView>("sync_check", { project }),
  syncLoginOptions: (): Promise<SyncLoginOptions> => call<SyncLoginOptions>("sync_login_options"),
  /** Connexion par le navigateur : ouvre la page de la forge, rend le code à saisir. */
  syncConnectStart: (forge: SyncForge, host: string): Promise<SyncDeviceCode> =>
    call<SyncDeviceCode>("sync_connect_start", { forge, host }),
  /** Attend la validation dans le navigateur ; le jeton va au coffre. */
  syncConnectWait: (forge: SyncForge, host: string): Promise<SyncAccount> =>
    call<SyncAccount>("sync_connect_wait", { forge, host }),
  syncConnectCancel: (forge: SyncForge, host: string): Promise<void> =>
    call<null>("sync_connect_cancel", { forge, host }).then(() => undefined),
  /** Jeton personnel saisi, vérifié par la forge puis rangé au coffre. */
  syncToken: (
    forge: SyncForge,
    host: string,
    token: string,
    login: string | null,
  ): Promise<SyncAccount> => call<SyncAccount>("sync_token", { forge, host, token, login }),
  syncRepositories: (account: SyncAccount, query: string): Promise<SyncRepository[]> =>
    call<SyncRepository[]>("sync_repositories", { account, query }),
  syncCreateRepository: (account: SyncAccount, name: string): Promise<SyncRepository> =>
    call<SyncRepository>("sync_create_repository", { account, name }),
  /** « Relier et envoyer » (7.1-a). */
  syncLink: (
    project: string,
    url: string,
    branch: string,
    account: SyncAccount | null,
  ): Promise<SyncView> => call<SyncView>("sync_link", { project, url, branch, account }),
  /** Récupérer (`push` faux), récupérer puis envoyer, ou envoyer seulement (`pull` faux). */
  syncPullPush: (
    project: string,
    items: string[],
    message: string,
    pull: boolean,
    push: boolean,
  ): Promise<SyncView> => call<SyncView>("sync_pull_push", { project, items, message, pull, push }),
  syncConflicts: (project: string): Promise<SyncConflicts> =>
    call<SyncConflicts>("sync_conflicts", { project }),
  /** « Valider la résolution » : chemin → élément → version gardée. */
  syncResolve: (
    project: string,
    choices: Record<string, Record<string, SyncChoice>>,
  ): Promise<SyncView> => call<SyncView>("sync_resolve", { project, choices }),
  syncAbandon: (project: string): Promise<SyncView> => call<SyncView>("sync_abandon", { project }),
  syncJournal: (project: string, path: string): Promise<JournalEntry[]> =>
    call<JournalEntry[]>("sync_journal", { project, path }),
  syncMissingSecrets: (project: string): Promise<MissingSecrets[]> =>
    call<MissingSecrets[]>("sync_missing_secrets", { project }),
  syncSecretsLater: (project: string, environment: string, names: string[]): Promise<void> =>
    call<null>("sync_secrets_later", { project, environment, names }).then(() => undefined),
};

/** Événements diffusés par le cœur à toutes les fenêtres (F11). */
export const EVENTS = {
  windows: "arnod://fenetres",
  catalog: "arnod://catalogue",
  project: "arnod://projet",
  close: "arnod://fermer",
  activeRequest: "arnod://requete-active",
  result: "arnod://resultat",
  run: "arnod://run",
  statuses: "arnod://statuts",
  // Lot H : « Préférences… » du menu natif.
  preferences: "arnod://preferences",
  // Lot F : raccourci choisi dans le menu natif Fenêtre.
  shortcut: "arnod://raccourci",
  // Lot I : fichiers modifiés hors d'Arnod.
  files: "arnod://fichiers",
  // Lot L : version trouvée et installation.
  updates: "arnod://mise-a-jour",
  // Plan qualité, lot C : sortie demandée, chaque fenêtre enregistre puis répond (R6).
  quit: "arnod://quitter",
  // Version 1 : préférences d'interface du poste (thème, police, raccourcis).
  interface: "arnod://interface",
  // Version 2, lot E : état de la synchronisation d'un projet.
  sync: "arnod://synchro",
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

  /** Requête active changée (onglet, fenêtre) : la fenêtre de résultat la suit (D2). */
  onActiveRequest: (handler: (active: ActiveRequest | null) => void): Promise<UnlistenFn> =>
    listen<ActiveRequest | null>(EVENTS.activeRequest, (event) => {
      handler(event.payload);
    }),

  /** Début ou fin d'un envoi, dans n'importe quelle fenêtre (R5). */
  onResult: (handler: (change: ResultChange) => void): Promise<UnlistenFn> =>
    listen<ResultChange>(EVENTS.result, (event) => {
      handler(event.payload);
    }),

  /** Run démarré dans n'importe quelle fenêtre (R7), ou avancé par le cœur (R3). */
  onRun: (handler: (change: RunChange) => void): Promise<UnlistenFn> =>
    listen<RunChange>(EVENTS.run, (event) => {
      handler(event.payload);
    }),

  /** Statuts des requêtes d'un projet, après un envoi, un run ou un renommage (R4). */
  onStatuses: (handler: (change: StatusesChange) => void): Promise<UnlistenFn> =>
    listen<StatusesChange>(EVENTS.statuses, (event) => {
      handler(event.payload);
    }),

  /** Lot H : « Préférences… » du menu, pour la fenêtre nommée. */
  onPreferences: (handler: (window: string) => void): Promise<UnlistenFn> =>
    listen<{ window: string }>(EVENTS.preferences, (event) => {
      handler(event.payload.window);
    }),

  /** Fermeture demandée pour cette fenêtre détachée : enregistrer puis se rattacher. */
  onClose: (handler: () => void): Promise<UnlistenFn> =>
    listen(EVENTS.close, () => {
      handler();
    }),

  /** Sortie d'Arnod demandée : enregistrer, puis répondre par `windowQuitReady` (R6). */
  onQuit: (handler: () => void): Promise<UnlistenFn> =>
    listen(EVENTS.quit, () => {
      handler();
    }),

  /** Raccourci choisi dans le menu natif Fenêtre, pour cette fenêtre (lot F, 1.4-j). */
  onShortcut: (handler: (shortcut: MenuShortcut) => void): Promise<UnlistenFn> =>
    listen<MenuShortcut>(EVENTS.shortcut, (event) => {
      handler(event.payload);
    }),
  /** Fichiers d'un projet modifiés hors d'Arnod : renommés, supprimés… (lot I, R6). */
  onFiles: (handler: (change: FilesChange) => void): Promise<UnlistenFn> =>
    listen<FilesChange>(EVENTS.files, (event) => {
      handler(event.payload);
    }),

  /** Lot L : version trouvée et installation, à chaque étape du téléchargement (R8). */
  onUpdates: (handler: (view: UpdatesView) => void): Promise<UnlistenFn> =>
    listen<UpdatesView>(EVENTS.updates, (event) => {
      handler(event.payload);
    }),

  /** Version 1 : préférences d'interface changées dans n'importe quelle fenêtre. */
  onInterface: (handler: (view: InterfaceView) => void): Promise<UnlistenFn> =>
    listen<InterfaceView>(EVENTS.interface, (event) => {
      handler(event.payload);
    }),
  /** Version 2, lot E : synchronisation vérifiée, récupérée ou envoyée (toutes les fenêtres). */
  onSync: (handler: (view: SyncView) => void): Promise<UnlistenFn> =>
    listen<SyncView>(EVENTS.sync, (event) => {
      handler(event.payload);
    }),
};
