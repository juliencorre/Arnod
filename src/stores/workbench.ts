import { defineStore } from "pinia";
import { reactive, ref, watch } from "vue";

import { events, ipc, IpcError } from "@/lib/ipc";
import { contentKey, useWindowsStore } from "@/stores/windows";
import type {
  EnvironmentDoc,
  EnvironmentsView,
  ProjectChange,
  RequestDoc,
  RequestResult,
  RunView,
  SendOutcome,
  TreeNode,
  TreeView,
} from "@/types/ipc";

/** Intervalle de lecture de l'avancement d'un run. */
export const RUN_POLL_MS = 300;

/** Attente après la dernière modification avant l'enregistrement automatique (EP1). */
export const AUTOSAVE_MS = 600;

/** État d'enregistrement d'une requête ouverte (EP1). */
export type SaveState = "saved" | "pending" | "saving" | "error";

/** Requête ouverte : brouillon, version enregistrée, dernière réponse. */
export interface Editor {
  projectId: string;
  path: string;
  draft: RequestDoc | null;
  /** Brouillon tel qu'enregistré, en JSON, pour savoir s'il a changé. */
  saved: string;
  outcome: SendOutcome | null;
  /** Heure de réception de la dernière réponse (EP9). */
  receivedAt: Date | null;
  sending: boolean;
  sendError: string | null;
  /** Lecture impossible (fichier supprimé, illisible). */
  loadError: string | null;
  /** Modifiée dans une autre fenêtre alors qu'elle l'était aussi ici (F11). */
  changedElsewhere: boolean;
  execId: string | null;
  saveState: SaveState;
  /** Raison d'un enregistrement impossible (B7). */
  saveError: string | null;
}

/** Action en attente de la confirmation d'un environnement protégé (R8). */
export type PendingConfirmation =
  | { kind: "send"; key: string }
  | { kind: "run"; projectId: string; folder: string }
  | { kind: "contractRun"; projectId: string; path: string }
  | { kind: "retry"; projectId: string };

/** Clé d'une requête ouverte. */
export function requestKey(projectId: string, path: string): string {
  return contentKey({ kind: "request", projectId, path });
}

/** Nœud de requête d'une arborescence, par son chemin. */
export function findRequest(nodes: TreeNode[], path: string): TreeNode | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.kind === "folder") {
      const found = findRequest(node.children, path);
      if (found) return found;
    }
  }
  return null;
}

/**
 * Établi : données des projets ouverts (arborescence, environnements, run) et
 * requêtes ouvertes dans les onglets de cette fenêtre. Aucune règle métier :
 * le store affiche l'état renvoyé par le cœur. Les modifications faites dans
 * une autre fenêtre arrivent par `arnod://projet` (F11).
 */
export const useWorkbenchStore = defineStore("workbench", () => {
  const windows = useWindowsStore();
  const trees = ref<Record<string, TreeView>>({});
  const environments = ref<Record<string, EnvironmentsView>>({});
  const runs = ref<Record<string, RunView | null>>({});
  const editors = reactive<Record<string, Editor>>({});
  /** Projets dépliés dans la barre latérale. */
  const expanded = ref<string[]>([]);
  const pending = ref<PendingConfirmation | null>(null);
  const loading = new Map<string, Promise<void>>();
  const pollTimers = new Map<string, ReturnType<typeof setTimeout>>();
  let subscribed = false;

  // --- Projets ---------------------------------------------------------------

  /** Charge un projet une fois : arborescence, environnements, dernier run. */
  function loadProject(projectId: string): Promise<void> {
    if (trees.value[projectId]) return Promise.resolve();
    const existing = loading.get(projectId);
    if (existing) return existing;
    const load = (async () => {
      const [tree, envs, last] = await Promise.all([
        ipc.treeLoad(projectId),
        ipc.environmentsLoad(projectId),
        ipc.runLast(projectId),
      ]);
      trees.value[projectId] = tree;
      environments.value[projectId] = envs;
      if (!(projectId in runs.value)) runs.value[projectId] = last;
    })().finally(() => loading.delete(projectId));
    loading.set(projectId, load);
    return load;
  }

  async function refreshTree(projectId: string): Promise<void> {
    trees.value[projectId] = await ipc.treeLoad(projectId);
  }

  /**
   * Environnements d'un projet replié, pour sa pastille dans la barre latérale : lus une fois,
   * sans déplier le projet ; une erreur (projet illisible) laisse la pastille absente.
   */
  async function peekEnvironments(projectId: string): Promise<void> {
    if (projectId in environments.value) return;
    try {
      const view = await ipc.environmentsLoad(projectId);
      if (!(projectId in environments.value)) environments.value[projectId] = view;
    } catch {
      // Le projet s'ouvrira avec son erreur quand on le dépliera.
    }
  }

  async function refreshEnvironments(projectId: string): Promise<void> {
    environments.value[projectId] = await ipc.environmentsLoad(projectId);
  }

  function isExpanded(projectId: string): boolean {
    return expanded.value.includes(projectId);
  }

  async function expand(projectId: string): Promise<void> {
    if (!isExpanded(projectId)) expanded.value = [...expanded.value, projectId];
    await loadProject(projectId);
  }

  function collapse(projectId: string): void {
    expanded.value = expanded.value.filter((id) => id !== projectId);
  }

  async function toggleFolder(projectId: string, path: string): Promise<void> {
    const tree = trees.value[projectId];
    if (!tree) return;
    tree.collapsed = tree.collapsed.includes(path)
      ? tree.collapsed.filter((p) => p !== path)
      : [...tree.collapsed, path];
    await ipc.treeSetCollapsed(projectId, tree.collapsed);
  }

  function activeEnvironment(projectId: string): string | null {
    return environments.value[projectId]?.active ?? null;
  }

  // --- Requêtes ouvertes -----------------------------------------------------

  /** Brouillon d'une requête ouverte, lu une fois. */
  async function openEditor(projectId: string, path: string): Promise<Editor> {
    const key = requestKey(projectId, path);
    const existing = editors[key];
    if (existing) return existing;
    const editor: Editor = {
      projectId,
      path,
      draft: null,
      saved: "",
      outcome: null,
      receivedAt: null,
      sending: false,
      sendError: null,
      loadError: null,
      changedElsewhere: false,
      execId: null,
      saveState: "saved",
      saveError: null,
    };
    editors[key] = editor;
    await Promise.all([
      loadProject(projectId).catch(() => undefined),
      reload(key),
      loadResult(key),
    ]);
    return editors[key] ?? editor;
  }

  /**
   * Applique le dernier envoi d'une requête, fait dans une autre fenêtre (R5) :
   * « Envoi en cours… », puis la réponse ou l'erreur.
   */
  function applyResult(result: RequestResult): void {
    const editor = editors[requestKey(result.projectId, result.path)];
    if (!editor) return;
    editor.sending = result.sending;
    editor.outcome = result.outcome;
    editor.sendError = result.error;
    editor.receivedAt = result.receivedAt === null ? null : new Date(result.receivedAt);
  }

  /** Reprend le dernier envoi gardé par le cœur, pour une requête ouverte ici (D2). */
  async function loadResult(key: string): Promise<void> {
    const opened = editors[key];
    if (!opened) return;
    const result = await ipc.requestResult(opened.projectId, opened.path).catch(() => null);
    const editor = editors[key];
    if (result && editor && !editor.sending && editor.outcome === null) applyResult(result);
  }

  async function reload(key: string): Promise<void> {
    const editor = editors[key];
    if (!editor) return;
    try {
      const request = await ipc.requestRead(editor.projectId, editor.path);
      editor.draft = request;
      editor.saved = JSON.stringify(request);
      editor.loadError = null;
      editor.changedElsewhere = false;
    } catch (caught) {
      editor.draft = null;
      editor.loadError = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function isDirty(key: string): boolean {
    const editor = editors[key];
    return !!editor?.draft && JSON.stringify(editor.draft) !== editor.saved;
  }

  const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const saving = new Map<string, Promise<void>>();

  function clearSaveTimer(key: string): void {
    const timer = saveTimers.get(key);
    if (timer !== undefined) clearTimeout(timer);
    saveTimers.delete(key);
  }

  /** Enregistrement automatique, 600 ms après la dernière modification (EP1). */
  function scheduleSave(key: string): void {
    const editor = editors[key];
    if (!editor) return;
    clearSaveTimer(key);
    editor.saveState = "pending";
    saveTimers.set(
      key,
      setTimeout(() => {
        saveTimers.delete(key);
        void save(key).catch(() => undefined);
      }, AUTOSAVE_MS),
    );
  }

  // Toute modification d'un brouillon déclenche son enregistrement automatique.
  watch(
    () => Object.keys(editors).map((key) => `${key}\u0000${JSON.stringify(editors[key]?.draft)}`),
    () => {
      for (const key of Object.keys(editors)) {
        if (isDirty(key)) scheduleSave(key);
      }
    },
  );

  /**
   * Enregistre une requête modifiée. Le contenu envoyé est figé avant l'écriture : une frappe
   * pendant l'enregistrement en relance un autre. Un échec est signalé (B7) puis relancé.
   */
  async function save(key: string): Promise<void> {
    const running = saving.get(key);
    if (running) await running.catch(() => undefined);
    const editor = editors[key];
    if (!editor?.draft || !isDirty(key)) {
      if (editor && editor.saveState !== "error") editor.saveState = "saved";
      return;
    }
    clearSaveTimer(key);
    const snapshot = JSON.stringify(editor.draft);
    editor.saveState = "saving";
    const write = (async () => {
      try {
        await ipc.requestSave(editor.projectId, editor.path, JSON.parse(snapshot) as RequestDoc);
        editor.saved = snapshot;
        editor.changedElsewhere = false;
        editor.saveError = null;
        editor.saveState = isDirty(key) ? "pending" : "saved";
      } catch (caught) {
        editor.saveState = "error";
        editor.saveError = caught instanceof Error ? caught.message : String(caught);
        throw caught;
      }
      if (trees.value[editor.projectId]) await refreshTree(editor.projectId);
    })();
    saving.set(key, write);
    try {
      await write;
    } finally {
      if (saving.get(key) === write) saving.delete(key);
    }
  }

  /** Enregistre toutes les requêtes modifiées de la fenêtre (avant fermeture, Z6). */
  async function saveAll(): Promise<void> {
    for (const key of Object.keys(editors)) {
      if (isDirty(key)) await save(key);
    }
  }

  /** Oublie une requête qui n'est plus ouverte dans aucun onglet de la fenêtre. */
  function release(key: string): void {
    if (isDirty(key)) return;
    clearSaveTimer(key);
    Reflect.deleteProperty(editors, key);
  }

  /** Envoie la requête (E1) ; un environnement protégé demande confirmation (R8). */
  async function send(key: string, confirmed = false): Promise<void> {
    const editor = editors[key];
    const environment = editor ? activeEnvironment(editor.projectId) : null;
    if (!editor?.draft || environment === null || editor.sending) return;
    await save(key);
    editor.sending = true;
    editor.sendError = null;
    editor.execId = crypto.randomUUID();
    try {
      editor.outcome = await ipc.requestSend(
        editor.projectId,
        editor.execId,
        environment,
        editor.path,
        editor.draft,
        confirmed,
      );
      editor.receivedAt = new Date();
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "send", key };
      } else if (!(caught instanceof IpcError && caught.kind === "cancelled")) {
        editor.outcome = null;
        editor.sendError = caught instanceof Error ? caught.message : String(caught);
      }
    } finally {
      editor.sending = false;
      editor.execId = null;
    }
  }

  async function cancelSend(key: string): Promise<void> {
    const execId = editors[key]?.execId;
    if (execId) await ipc.requestCancel(execId);
  }

  /** Ajoute au brouillon les vérifications proposées d'après la réponse (R10). */
  async function proposeAssertions(key: string): Promise<void> {
    const editor = editors[key];
    const current = editor?.outcome;
    if (!editor?.draft || !current) return;
    const proposed = await ipc.requestProposeAssertions({
      status: current.response.status,
      durationMs: current.response.totalMs,
      headers: current.response.headers,
      body: current.response.body,
    });
    // Les identifiants sont de session : une vérification déjà présente se reconnaît
    // à sa cible et à son opérateur.
    const signature = (a: { target: string; operator: string }): string =>
      `${a.target}\u0000${a.operator}`;
    const known = new Set(editor.draft.assertions.map(signature));
    const ids = new Set(editor.draft.assertions.map((a) => a.id));
    editor.draft.assertions = [
      ...editor.draft.assertions,
      ...proposed
        .filter((a) => !known.has(signature(a)))
        .map((a) =>
          ids.has(a.id) ? { ...a, id: `${a.id}-${crypto.randomUUID().slice(0, 8)}` } : a,
        ),
    ];
  }

  /**
   * Ajoute la dernière réponse reçue aux réponses attendues (EP9, R7) ; renvoie son index.
   * Le nom donne la date de réception.
   */
  async function addExampleFromResponse(key: string): Promise<number | null> {
    const editor = editors[key];
    const response = editor?.outcome?.response;
    if (!editor?.draft || !response) return null;
    const at = editor.receivedAt ?? new Date();
    const name = `Reçue le ${at.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" })} à ${at.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`;
    const example = await ipc.requestExampleFromResponse(
      {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers,
        body: response.body,
      },
      name,
    );
    editor.draft.examples.push(example);
    return editor.draft.examples.length - 1;
  }

  // --- Arborescence ----------------------------------------------------------

  /** Ouvre une requête depuis la barre latérale, dans la fenêtre qui suit la sélection (Z5). */
  function openRequest(projectId: string, path: string): Promise<boolean> {
    return windows.openSelection({ kind: "request", projectId, path });
  }

  async function createRequest(
    projectId: string,
    folder: string,
    name: string | null = null,
  ): Promise<string> {
    const path = await ipc.treeCreateRequest(projectId, folder, name);
    const tree = trees.value[projectId];
    if (tree?.collapsed.includes(folder)) await toggleFolder(projectId, folder);
    await refreshTree(projectId);
    await openRequest(projectId, path);
    return path;
  }

  async function createFolder(projectId: string, parent: string, name: string): Promise<string> {
    const path = await ipc.treeCreateFolder(projectId, parent, name);
    await refreshTree(projectId);
    return path;
  }

  /** Les brouillons ouverts suivent un renommage ou un déplacement. */
  function moveEditors(projectId: string, from: string, to: string): void {
    for (const [key, editor] of Object.entries(editors)) {
      if (editor.projectId !== projectId) continue;
      if (editor.path !== from && !editor.path.startsWith(`${from}/`)) continue;
      editor.path = `${to}${editor.path.slice(from.length)}`;
      Reflect.deleteProperty(editors, key);
      editors[requestKey(projectId, editor.path)] = editor;
    }
  }

  async function rename(projectId: string, path: string, name: string): Promise<void> {
    await saveUnder(projectId, path);
    const next = await ipc.treeRename(projectId, path, name);
    moveEditors(projectId, path, next);
    await refreshTree(projectId);
    for (const editor of Object.values(editors)) {
      if (editor.projectId === projectId && editor.path === next) {
        await reload(requestKey(projectId, next));
      }
    }
  }

  async function duplicate(projectId: string, path: string): Promise<void> {
    await saveUnder(projectId, path);
    const copy = await ipc.treeDuplicate(projectId, path);
    await refreshTree(projectId);
    await openRequest(projectId, copy);
  }

  async function move(projectId: string, path: string, targetFolder: string): Promise<void> {
    await saveUnder(projectId, path);
    const next = await ipc.treeMove(projectId, path, targetFolder);
    moveEditors(projectId, path, next);
    await refreshTree(projectId);
  }

  async function remove(projectId: string, path: string, permanent: boolean): Promise<void> {
    await ipc.treeDelete(projectId, path, permanent);
    for (const [key, editor] of Object.entries(editors)) {
      if (
        editor.projectId === projectId &&
        (editor.path === path || editor.path.startsWith(`${path}/`))
      ) {
        Reflect.deleteProperty(editors, key);
      }
    }
    await refreshTree(projectId);
  }

  /** Enregistre les requêtes modifiées sous `path` avant de le déplacer. */
  async function saveUnder(projectId: string, path: string): Promise<void> {
    for (const [key, editor] of Object.entries(editors)) {
      if (
        editor.projectId === projectId &&
        (editor.path === path || editor.path.startsWith(`${path}/`))
      ) {
        await save(key);
      }
    }
  }

  // --- Environnements --------------------------------------------------------

  async function selectEnvironment(projectId: string, name: string): Promise<void> {
    environments.value[projectId] = await ipc.environmentSelect(projectId, name);
  }

  async function saveEnvironment(
    projectId: string,
    env: EnvironmentDoc,
    previous: string | null,
  ): Promise<void> {
    environments.value[projectId] = await ipc.environmentSave(projectId, env, previous);
  }

  async function deleteEnvironment(projectId: string, name: string): Promise<void> {
    environments.value[projectId] = await ipc.environmentDelete(projectId, name);
  }

  async function reorderEnvironments(projectId: string, names: string[]): Promise<void> {
    environments.value[projectId] = await ipc.environmentsReorder(projectId, names);
  }

  async function setLocal(
    projectId: string,
    environment: string,
    name: string,
    value: string,
    secret: boolean,
  ): Promise<void> {
    environments.value[projectId] = await ipc.localSet(projectId, environment, name, value, secret);
  }

  // --- Runs ------------------------------------------------------------------

  function stopPolling(projectId: string): void {
    const timer = pollTimers.get(projectId);
    if (timer !== undefined) clearTimeout(timer);
    pollTimers.delete(projectId);
  }

  function poll(projectId: string, runId: string): void {
    stopPolling(projectId);
    pollTimers.set(
      projectId,
      setTimeout(() => {
        ipc
          .runStatus(runId)
          .then((view) => {
            runs.value[projectId] = view;
            if (!view.finished) poll(projectId, runId);
          })
          .catch(() => {
            stopPolling(projectId);
          });
      }, RUN_POLL_MS),
    );
  }

  function follow(projectId: string, view: RunView): void {
    runs.value[projectId] = view;
    if (!view.finished) poll(projectId, view.id);
  }

  /** Exécute un dossier et ouvre l'onglet du run, là où suit la sélection. */
  async function runFolder(projectId: string, folder: string, confirmed = false): Promise<void> {
    const environment = activeEnvironment(projectId);
    if (environment === null) return;
    await saveAll();
    try {
      follow(projectId, await ipc.runStart(projectId, folder, environment, confirmed));
      await windows.openSelection({ kind: "run", projectId });
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "run", projectId, folder };
      } else throw caught;
    }
  }

  /** Exécute les requêtes liées au contrat `path` (C10) et ouvre l'onglet du run. */
  async function runContract(projectId: string, path: string, confirmed = false): Promise<void> {
    const environment = activeEnvironment(projectId);
    if (environment === null) return;
    await saveAll();
    try {
      follow(projectId, await ipc.contractRun(projectId, path, environment, confirmed));
      await windows.openSelection({ kind: "run", projectId });
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "contractRun", projectId, path };
      } else throw caught;
    }
  }

  async function cancelRun(projectId: string): Promise<void> {
    const current = runs.value[projectId];
    if (current && !current.finished) await ipc.runCancel(current.id);
  }

  async function retryFailures(projectId: string, confirmed = false): Promise<void> {
    const current = runs.value[projectId];
    if (!current) return;
    try {
      follow(projectId, await ipc.runRetryFailures(projectId, current.id, confirmed));
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "retry", projectId };
      } else throw caught;
    }
  }

  /** Poursuit l'action mise en attente, après confirmation (R8). */
  async function confirmPending(): Promise<void> {
    const action = pending.value;
    pending.value = null;
    if (action === null) return;
    if (action.kind === "send") await send(action.key, true);
    else if (action.kind === "run") await runFolder(action.projectId, action.folder, true);
    else if (action.kind === "contractRun") await runContract(action.projectId, action.path, true);
    else await retryFailures(action.projectId, true);
  }

  function dismissPending(): void {
    pending.value = null;
  }

  /** Environnement concerné par la confirmation en attente. */
  function pendingEnvironment(): string {
    const action = pending.value;
    if (action === null) return "";
    const projectId = action.kind === "send" ? editors[action.key]?.projectId : action.projectId;
    return projectId ? (activeEnvironment(projectId) ?? "") : "";
  }

  // --- Interconnexion (F11) --------------------------------------------------

  /** Modification faite dans une autre fenêtre. */
  async function onProjectChange(change: ProjectChange): Promise<void> {
    // Les contrats ont leur store ; leurs requêtes créées arrivent en portée `tree`.
    if (change.origin === windows.label || change.scope === "contracts") return;
    const { projectId } = change;
    if (change.scope === "environments") {
      if (environments.value[projectId]) await refreshEnvironments(projectId);
      return;
    }
    if (trees.value[projectId]) await refreshTree(projectId);
    if (change.scope === "request" && change.path !== null) {
      const key = requestKey(projectId, change.path);
      if (!editors[key]) return;
      if (isDirty(key)) {
        const editor = editors[key];
        editor.changedElsewhere = true;
      } else await reload(key);
    }
  }

  async function subscribe(): Promise<void> {
    if (subscribed) return;
    subscribed = true;
    await events.onProject((change) => {
      void onProjectChange(change);
    });
    // Envoi fait dans une autre fenêtre : la réponse s'affiche ici aussi (R5).
    await events.onResult((change) => {
      if (change.origin !== windows.label) applyResult(change.result);
    });
    // Run démarré dans une autre fenêtre : son onglet le suit ici aussi (R7).
    await events.onRun((change) => {
      if (change.origin !== windows.label) follow(change.projectId, change.run);
    });
  }

  return {
    trees,
    environments,
    runs,
    editors,
    expanded,
    pending,
    loadProject,
    refreshTree,
    refreshEnvironments,
    peekEnvironments,
    isExpanded,
    expand,
    collapse,
    toggleFolder,
    activeEnvironment,
    openEditor,
    applyResult,
    loadResult,
    reload,
    isDirty,
    save,
    saveAll,
    release,
    send,
    cancelSend,
    proposeAssertions,
    addExampleFromResponse,
    openRequest,
    createRequest,
    createFolder,
    rename,
    duplicate,
    move,
    remove,
    selectEnvironment,
    saveEnvironment,
    deleteEnvironment,
    reorderEnvironments,
    setLocal,
    runFolder,
    runContract,
    cancelRun,
    retryFailures,
    confirmPending,
    dismissPending,
    pendingEnvironment,
    onProjectChange,
    subscribe,
  };
});
