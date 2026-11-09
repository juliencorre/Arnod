import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { ipc, IpcError } from "@/lib/ipc";
import type {
  EnvironmentDoc,
  EnvironmentsView,
  RequestDoc,
  RunView,
  SendOutcome,
  TreeNode,
} from "@/types/ipc";

/** Intervalle de lecture de l'avancement d'un run. */
export const RUN_POLL_MS = 300;

/** Action en attente de la confirmation d'un environnement protégé (R8). */
export type PendingConfirmation =
  { kind: "send" } | { kind: "run"; folder: string } | { kind: "retry"; runId: string };

/**
 * Projet ouvert : arborescence, requête en cours d'édition, réponse, environnements
 * et run. Le store n'a aucune règle métier : il affiche l'état renvoyé par le cœur.
 */
export const useWorkbenchStore = defineStore("workbench", () => {
  const projectId = ref<string | null>(null);
  const nodes = ref<TreeNode[]>([]);
  const collapsed = ref<string[]>([]);
  const selectedPath = ref<string | null>(null);
  const draft = ref<RequestDoc | null>(null);
  const savedSnapshot = ref("");
  const outcome = ref<SendOutcome | null>(null);
  const sending = ref(false);
  const sendError = ref<string | null>(null);
  const environments = ref<EnvironmentsView | null>(null);
  const run = ref<RunView | null>(null);
  const pending = ref<PendingConfirmation | null>(null);
  let execId: string | null = null;
  let pollTimer: ReturnType<typeof setTimeout> | undefined;

  const dirty = computed(
    () => draft.value !== null && JSON.stringify(draft.value) !== savedSnapshot.value,
  );
  const activeEnvironment = computed(
    () =>
      environments.value?.environments.find((e) => e.name === environments.value?.active) ?? null,
  );

  function requireProject(): string {
    if (projectId.value === null) throw new Error("Aucun projet ouvert.");
    return projectId.value;
  }

  function reset(): void {
    stopPolling();
    nodes.value = [];
    collapsed.value = [];
    selectedPath.value = null;
    draft.value = null;
    savedSnapshot.value = "";
    outcome.value = null;
    sendError.value = null;
    environments.value = null;
    run.value = null;
    pending.value = null;
  }

  /** Ouvre un projet : arborescence, environnements et dernier run. */
  async function open(id: string): Promise<void> {
    if (projectId.value !== id) reset();
    projectId.value = id;
    const [tree, envs, last] = await Promise.all([
      ipc.treeLoad(id),
      ipc.environmentsLoad(id),
      ipc.runLast(id),
    ]);
    nodes.value = tree.nodes;
    collapsed.value = tree.collapsed;
    environments.value = envs;
    run.value = last;
  }

  async function refreshTree(): Promise<void> {
    const tree = await ipc.treeLoad(requireProject());
    nodes.value = tree.nodes;
    collapsed.value = tree.collapsed;
  }

  async function toggleFolder(path: string): Promise<void> {
    collapsed.value = collapsed.value.includes(path)
      ? collapsed.value.filter((p) => p !== path)
      : [...collapsed.value, path];
    await ipc.treeSetCollapsed(requireProject(), collapsed.value);
  }

  /** Enregistre la requête en cours si elle a changé. */
  async function save(): Promise<void> {
    if (draft.value === null || selectedPath.value === null || !dirty.value) return;
    await ipc.requestSave(requireProject(), selectedPath.value, draft.value);
    savedSnapshot.value = JSON.stringify(draft.value);
    await refreshTree();
  }

  async function select(path: string): Promise<void> {
    if (path === selectedPath.value) return;
    await save();
    const request = await ipc.requestRead(requireProject(), path);
    selectedPath.value = path;
    draft.value = request;
    savedSnapshot.value = JSON.stringify(request);
    outcome.value = null;
    sendError.value = null;
  }

  function clearSelection(): void {
    selectedPath.value = null;
    draft.value = null;
    savedSnapshot.value = "";
    outcome.value = null;
  }

  async function createRequest(folder: string, name: string | null = null): Promise<string> {
    await save();
    const path = await ipc.treeCreateRequest(requireProject(), folder, name);
    if (collapsed.value.includes(folder)) await toggleFolder(folder);
    await refreshTree();
    await select(path);
    return path;
  }

  async function createFolder(parent: string, name: string): Promise<string> {
    const path = await ipc.treeCreateFolder(requireProject(), parent, name);
    await refreshTree();
    return path;
  }

  async function rename(path: string, name: string): Promise<void> {
    await save();
    const next = await ipc.treeRename(requireProject(), path, name);
    const wasSelected = selectedPath.value === path;
    await refreshTree();
    if (wasSelected) {
      selectedPath.value = null;
      await select(next);
    }
  }

  async function duplicate(path: string): Promise<void> {
    await save();
    const copy = await ipc.treeDuplicate(requireProject(), path);
    await refreshTree();
    await select(copy);
  }

  async function move(path: string, targetFolder: string): Promise<void> {
    await save();
    const next = await ipc.treeMove(requireProject(), path, targetFolder);
    const wasSelected = selectedPath.value === path;
    await refreshTree();
    if (wasSelected) {
      selectedPath.value = null;
      await select(next);
    }
  }

  async function remove(path: string, permanent: boolean): Promise<void> {
    await ipc.treeDelete(requireProject(), path, permanent);
    const selected = selectedPath.value;
    if (selected !== null && (selected === path || selected.startsWith(`${path}/`)))
      clearSelection();
    await refreshTree();
  }

  /** Envoie la requête de l'éditeur (E1) ; un environnement protégé demande confirmation (R8). */
  async function send(confirmed = false): Promise<void> {
    const request = draft.value;
    const envs = environments.value;
    if (request === null || envs === null || sending.value) return;
    await save();
    sending.value = true;
    sendError.value = null;
    execId = crypto.randomUUID();
    try {
      outcome.value = await ipc.requestSend(
        requireProject(),
        execId,
        envs.active,
        request,
        confirmed,
      );
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "send" };
      } else if (caught instanceof IpcError && caught.kind === "cancelled") {
        sendError.value = null;
      } else {
        outcome.value = null;
        sendError.value = caught instanceof Error ? caught.message : String(caught);
      }
    } finally {
      sending.value = false;
      execId = null;
    }
  }

  async function cancelSend(): Promise<void> {
    if (execId !== null) await ipc.requestCancel(execId);
  }

  /** Ajoute au brouillon les vérifications proposées d'après la réponse (R10). */
  async function proposeAssertions(): Promise<void> {
    const current = outcome.value;
    const request = draft.value;
    if (current === null || request === null) return;
    const proposed = await ipc.requestProposeAssertions({
      status: current.response.status,
      durationMs: current.response.totalMs,
      headers: current.response.headers,
      body: current.response.body,
    });
    const known = new Set(request.assertions.map((a) => a.id));
    request.assertions = [...request.assertions, ...proposed.filter((a) => !known.has(a.id))];
  }

  async function selectEnvironment(name: string): Promise<void> {
    environments.value = await ipc.environmentSelect(requireProject(), name);
  }

  async function saveEnvironment(env: EnvironmentDoc, previous: string | null): Promise<void> {
    environments.value = await ipc.environmentSave(requireProject(), env, previous);
  }

  async function deleteEnvironment(name: string): Promise<void> {
    environments.value = await ipc.environmentDelete(requireProject(), name);
  }

  async function reorderEnvironments(names: string[]): Promise<void> {
    environments.value = await ipc.environmentsReorder(requireProject(), names);
  }

  async function setLocal(
    environment: string,
    name: string,
    value: string,
    secret: boolean,
  ): Promise<void> {
    environments.value = await ipc.localSet(requireProject(), environment, name, value, secret);
  }

  // --- Runs ----------------------------------------------------------------

  function stopPolling(): void {
    if (pollTimer !== undefined) clearTimeout(pollTimer);
    pollTimer = undefined;
  }

  function poll(runId: string): void {
    stopPolling();
    pollTimer = setTimeout(() => {
      ipc
        .runStatus(runId)
        .then((view) => {
          run.value = view;
          if (!view.finished) poll(runId);
        })
        .catch(() => {
          stopPolling();
        });
    }, RUN_POLL_MS);
  }

  function follow(view: RunView): void {
    run.value = view;
    if (!view.finished) poll(view.id);
  }

  async function runFolder(folder: string, confirmed = false): Promise<void> {
    const envs = environments.value;
    if (envs === null) return;
    await save();
    try {
      follow(await ipc.runStart(requireProject(), folder, envs.active, confirmed));
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "run", folder };
      } else throw caught;
    }
  }

  async function cancelRun(): Promise<void> {
    if (run.value !== null && !run.value.finished) await ipc.runCancel(run.value.id);
  }

  async function retryFailures(confirmed = false): Promise<void> {
    const current = run.value;
    if (current === null) return;
    try {
      follow(await ipc.runRetryFailures(requireProject(), current.id, confirmed));
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "protectedEnvironment") {
        pending.value = { kind: "retry", runId: current.id };
      } else throw caught;
    }
  }

  /** Poursuit l'action mise en attente, après confirmation (R8). */
  async function confirmPending(): Promise<void> {
    const action = pending.value;
    pending.value = null;
    if (action === null) return;
    if (action.kind === "send") await send(true);
    else if (action.kind === "run") await runFolder(action.folder, true);
    else await retryFailures(true);
  }

  function dismissPending(): void {
    pending.value = null;
  }

  function closeRun(): void {
    stopPolling();
    run.value = null;
  }

  return {
    projectId,
    nodes,
    collapsed,
    selectedPath,
    draft,
    dirty,
    outcome,
    sending,
    sendError,
    environments,
    activeEnvironment,
    run,
    pending,
    open,
    refreshTree,
    toggleFolder,
    save,
    select,
    createRequest,
    createFolder,
    rename,
    duplicate,
    move,
    remove,
    send,
    cancelSend,
    proposeAssertions,
    selectEnvironment,
    saveEnvironment,
    deleteEnvironment,
    reorderEnvironments,
    setLocal,
    runFolder,
    cancelRun,
    retryFailures,
    confirmPending,
    dismissPending,
    closeRun,
    reset,
  };
});
