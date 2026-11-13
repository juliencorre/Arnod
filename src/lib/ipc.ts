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
  EnvironmentDoc,
  EnvironmentsView,
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
  ImportReport,
  ImportSource,
  ImportTarget,
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

  treeDelete: (project: string, path: string, permanent: boolean): Promise<void> =>
    call<null>("tree_delete", { project, path, permanent }).then(() => undefined),

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
  importRun: (path: string, target: ImportTarget): Promise<ImportDone> =>
    call<ImportDone>("import_run", { path, target }),
  /** Dernier rapport d'import d'un projet. */
  importReport: (projectId: string): Promise<ImportReport | null> =>
    call<ImportReport | null>("import_report", { projectId }),
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
  treeCreateTest: (project: string, parent: string): Promise<CreatedTest> =>
    call<CreatedTest>("tree_create_test", { project, parent }),

  /** « + Étape » : étape suivante du test ; renvoie son chemin (NT2). */
  treeAddStep: (project: string, test: string): Promise<string> =>
    call<string>("tree_add_step", { project, test }),

  /** Requêtes du projet, hors `path`, qui lisent l'une des variables `names` (EX5). */
  requestVariableUsers: (project: string, path: string, names: string[]): Promise<VariableUser[]> =>
    call<VariableUser[]>("request_variable_users", { project, path, names }),
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
};
