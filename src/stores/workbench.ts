import { defineStore } from "pinia";
import { computed, reactive, ref, watch, type ComputedRef } from "vue";

import { events, ipc, IpcError } from "@/lib/ipc";
import { contentKey, MAIN_WINDOW, useWindowsStore } from "@/stores/windows";
import { useToastStore } from "@/stores/toasts";
import type {
  ConflictSide,
  FilesChange,
  RequestConflict,
  DrawnValue,
  EnvironmentDoc,
  EnvironmentsView,
  ProjectChange,
  RequestDoc,
  RequestResult,
  RequestStatuses,
  RunView,
  SendOutcome,
  TestView,
  TreeNode,
  TreeView,
  VariableUser,
} from "@/types/ipc";

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
  /**
   * Lot I : fichier modifié sur le disque (hors d'Arnod ou dans une autre fenêtre) alors que
   * l'onglet a un brouillon. Tant qu'il n'est pas réglé, rien n'est enregistré (R9).
   */
  conflict: RequestConflict | null;
  /** Version choisie pour chaque élément en conflit (« disque » ou « brouillon »). */
  choices: Record<string, ConflictSide>;
  /** Lot I : fichier supprimé hors d'Arnod alors que l'onglet a un brouillon (R4). */
  removedOnDisk: boolean;
  /** Lot I : relue depuis le disque il y a quelques secondes (R3). */
  reloaded: boolean;
  /** Lot I : heure du début du brouillon, montrée dans le conflit. */
  draftSince: Date | null;
  execId: string | null;
  saveState: SaveState;
  /** Raison d'un enregistrement impossible (B7). */
  saveError: string | null;
  /**
   * Requête telle qu'envoyée la dernière fois, en JSON aux clés triées (lot E, R12) : de quoi
   * dire « modifié, pas encore envoyé » et y revenir. `null` avant le premier envoi.
   */
  sent: string | null;
}

/** JSON aux clés triées : deux requêtes égales ont le même texte, quel que soit l'ordre. */
function canonical(value: unknown): string {
  return JSON.stringify(value, (_key, item: unknown) =>
    item !== null && typeof item === "object" && !Array.isArray(item)
      ? Object.fromEntries(
          Object.entries(item as Record<string, unknown>).sort(([a], [b]) => a.localeCompare(b)),
        )
      : item,
  );
}

/** Action en attente de la confirmation d'un environnement protégé (R8). */
export type PendingConfirmation =
  | { kind: "send"; key: string }
  | { kind: "run"; projectId: string; folder: string }
  | { kind: "contractRun"; projectId: string; path: string }
  | { kind: "retry"; projectId: string };

/** Lot I : résumé d'un lot de changements faits hors d'Arnod (« 4 requêtes modifiées, 1 ajoutée »). */
export function filesSummary(change: FilesChange): string {
  const parts = (
    [
      [change.modified.length, "modifiée", "modifiées"],
      [change.added.length, "ajoutée", "ajoutées"],
      [change.removed.length, "supprimée", "supprimées"],
      [change.renamed.length, "renommée", "renommées"],
    ] as const
  ).filter(([n]) => n > 0);
  return parts
    .map(([n, one, many], index) => {
      const noun = index === 0 ? (n === 1 ? "requête " : "requêtes ") : "";
      return `${String(n)} ${noun}${n === 1 ? one : many}`;
    })
    .join(", ");
}

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
  /** Dernière exécution de chaque requête, par projet (`docs/plan-fin-premier-usage.md`, R5). */
  const statuses = ref<Record<string, RequestStatuses>>({});
  const editors = reactive<Record<string, Editor>>({});
  /** Projets dépliés dans la barre latérale. */
  const expanded = ref<string[]>([]);
  const pending = ref<PendingConfirmation | null>(null);
  const loading = new Map<string, Promise<void>>();
  let subscribed = false;

  // --- Projets ---------------------------------------------------------------

  /** Charge un projet une fois : arborescence, environnements, dernier run, statuts. */
  function loadProject(projectId: string): Promise<void> {
    if (trees.value[projectId]) return Promise.resolve();
    const existing = loading.get(projectId);
    if (existing) return existing;
    const load = (async () => {
      const [tree, envs, last, known] = await Promise.all([
        ipc.treeLoad(projectId),
        ipc.environmentsLoad(projectId),
        ipc.runLast(projectId),
        ipc.requestStatuses(projectId),
      ]);
      trees.value[projectId] = tree;
      environments.value[projectId] = envs;
      if (!(projectId in runs.value)) runs.value[projectId] = last;
      if (!(projectId in statuses.value)) statuses.value[projectId] = known;
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
      conflict: null,
      choices: {},
      removedOnDisk: false,
      reloaded: false,
      draftSince: null,
      execId: null,
      saveState: "saved",
      saveError: null,
      sent: null,
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
    if (result.request !== null) editor.sent = canonical(result.request);
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
      editor.conflict = null;
      editor.choices = {};
      editor.removedOnDisk = false;
      editor.draftSince = null;
    } catch (caught) {
      editor.draft = null;
      editor.loadError = caught instanceof Error ? caught.message : String(caught);
    }
  }

  /**
   * Brouillon sérialisé de chaque requête ouverte. Calculé une fois par modification, quel que
   * soit le nombre de lecteurs (enregistrement automatique, pastilles des onglets et de l'arbre).
   */
  const serialized = new Map<string, ComputedRef<string | null>>();

  function draftText(key: string): string | null {
    let text = serialized.get(key);
    if (!text) {
      text = computed(() => {
        const draft = editors[key]?.draft;
        return draft ? JSON.stringify(draft) : null;
      });
      serialized.set(key, text);
    }
    return text.value;
  }

  function isDirty(key: string): boolean {
    const editor = editors[key];
    // Sans requête ouverte, rien à comparer (ni à mémoriser : une ligne de l'arbre par requête).
    if (!editor?.draft) return false;
    return draftText(key) !== editor.saved;
  }

  /** Brouillon aux clés triées, calculé une fois par modification et seulement après un envoi. */
  const canonicalDrafts = new Map<string, ComputedRef<string | null>>();

  /** La requête a changé depuis son dernier envoi (lot E, R12, écart 4.2-b). */
  function isChangedSinceSent(key: string): boolean {
    const editor = editors[key];
    if (!editor?.draft || editor.sent === null) return false;
    let text = canonicalDrafts.get(key);
    if (!text) {
      text = computed(() => {
        const draft = editors[key]?.draft;
        return draft ? canonical(draft) : null;
      });
      canonicalDrafts.set(key, text);
    }
    return text.value !== editor.sent;
  }

  /** Reprend la requête telle qu'envoyée ; elle s'enregistre seule (R12). */
  function revertToSent(key: string): void {
    const editor = editors[key];
    if (!editor || editor.sent === null) return;
    editor.draft = JSON.parse(editor.sent) as RequestDoc;
  }

  const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const saving = new Map<string, Promise<void>>();

  /** Oublie une requête ouverte : brouillon, texte mémorisé et enregistrement programmé. */
  function forget(key: string): void {
    clearSaveTimer(key);
    serialized.delete(key);
    canonicalDrafts.delete(key);
    Reflect.deleteProperty(editors, key);
  }

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

  // Toute modification d'un brouillon déclenche son enregistrement automatique : le sien
  // seulement, pour qu'une frappe dans une requête ne retarde pas celui des autres.
  watch(
    () => Object.keys(editors).map((key) => [key, draftText(key)] as const),
    (now, before) => {
      const previous = new Map(before);
      for (const [key, text] of now) {
        if (text === null || text === previous.get(key)) continue;
        const editor = editors[key];
        if (!editor || text === editor.saved) continue;
        editor.draftSince ??= new Date();
        // Conflit ou fichier supprimé : rien n'est enregistré avant le choix (lot I, R9).
        if (editor.conflict || editor.removedOnDisk) continue;
        scheduleSave(key);
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
    if (editor && (editor.conflict || editor.removedOnDisk)) return;
    const snapshot = editor?.draft ? draftText(key) : null;
    if (!editor || snapshot === null || snapshot === editor.saved) {
      if (editor && editor.saveState !== "error") editor.saveState = "saved";
      return;
    }
    clearSaveTimer(key);
    editor.saveState = "saving";
    const write = (async () => {
      try {
        await ipc.requestSave(editor.projectId, editor.path, JSON.parse(snapshot) as RequestDoc);
        editor.saved = snapshot;
        editor.saveError = null;
        editor.saveState = isDirty(key) ? "pending" : "saved";
        if (!isDirty(key)) editor.draftSince = null;
      } catch (caught) {
        // Fichier modifié hors d'Arnod depuis sa lecture : conflit, rien d'écrasé (lot I, R8).
        if (caught instanceof IpcError && caught.kind === "changedOnDisk") {
          editor.saveState = "pending";
          await detectConflict(key);
          return;
        }
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
    forget(key);
  }

  /** Envoie la requête (E1) ; un environnement protégé demande confirmation (R8). */
  async function send(key: string, confirmed = false): Promise<void> {
    const editor = editors[key];
    const environment = editor ? activeEnvironment(editor.projectId) : null;
    if (!editor?.draft || environment === null || editor.sending) return;
    // Marqué dès maintenant : un second envoi demandé pendant l'enregistrement ne part pas.
    editor.sending = true;
    editor.sendError = null;
    try {
      // L'envoi part du brouillon (E1) : un enregistrement impossible, signalé par l'éditeur
      // (« Non enregistré · Réessayer »), ne le bloque pas.
      await save(key).catch(() => undefined);
      editor.execId = crypto.randomUUID();
      const sending = canonical(editor.draft);
      editor.outcome = await ipc.requestSend(
        editor.projectId,
        editor.execId,
        environment,
        editor.path,
        editor.draft,
        confirmed,
      );
      editor.receivedAt = new Date();
      editor.sent = sending;
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

  // --- Tests et étapes (lot E, docs/plan-mvp-e-tests-etapes.md) -------------------

  /** Onglets de test ouverts, par clé de contenu. */
  const tests = ref<Record<string, TestView>>({});
  /** Requête dont l'adresse doit recevoir le curseur à l'ouverture (NT1). */
  const focusUrl = ref<string | null>(null);

  function testKey(projectId: string, path: string): string {
    return contentKey({ kind: "test", projectId, path });
  }

  /** Lit (ou relit) un test ouvert dans un onglet. */
  async function loadTest(projectId: string, path: string): Promise<TestView> {
    const view = await ipc.testRead(projectId, path);
    tests.value[testKey(projectId, path)] = view;
    return view;
  }

  /** Ouvre l'onglet d'un test (TE1). */
  function openTest(projectId: string, path: string): Promise<boolean> {
    return windows.openSelection({ kind: "test", projectId, path });
  }

  /** Déplie un dossier replié de la barre latérale. */
  async function unfold(projectId: string, path: string): Promise<void> {
    if (trees.value[projectId]?.collapsed.includes(path)) await toggleFolder(projectId, path);
  }

  /** « Nouveau test » (NT1) : dossier et étape 1 créés, étape ouverte, curseur dans l'adresse. */
  async function createTest(projectId: string, parent: string): Promise<string> {
    const created = await ipc.treeCreateTest(projectId, parent);
    await unfold(projectId, parent);
    await refreshTree(projectId);
    focusUrl.value = requestKey(projectId, created.step);
    await openRequest(projectId, created.step);
    return created.step;
  }

  /** « + Étape » (NT2) : étape suivante du test, ouverte, curseur dans l'adresse. */
  async function addStep(projectId: string, test: string): Promise<string> {
    const path = await ipc.treeAddStep(projectId, test);
    await unfold(projectId, test);
    await refreshTree(projectId);
    focusUrl.value = requestKey(projectId, path);
    await openRequest(projectId, path);
    return path;
  }

  /**
   * « Garder » une valeur tirée (VD3) : une extraction de la valeur envoyée, nommée d'après sa
   * place (`externalRef`), range la valeur dans une variable du test.
   */
  function keepDrawn(key: string, drawn: DrawnValue): string | null {
    return drawn.source === null ? null : addExtraction(key, drawn.source);
  }

  /**
   * Ajoute l'extraction de `source` (`body.items[0].sku`) au brouillon, nommée d'après sa place
   * (`sku`) ; renvoie le nom de la variable, celle qui existait déjà pour cette source.
   */
  function addExtraction(key: string, source: string): string | null {
    const draft = editors[key]?.draft;
    if (!draft) return null;
    const existing = draft.extractions.find((e) => e.source === source);
    if (existing) return existing.name;
    const base =
      source
        .split(/[.[\]]/)
        .filter(Boolean)
        .at(-1) ?? "valeur";
    let name = base;
    for (let n = 2; draft.extractions.some((e) => e.name === name); n += 1) {
      name = `${base}${String(n)}`;
    }
    draft.extractions.push({ name, source, enabled: true });
    return name;
  }

  /** Requêtes du projet qui lisent l'une des variables `names` de la requête `path` (EX5). */
  function variableUsers(
    projectId: string,
    path: string,
    names: string[],
  ): Promise<VariableUser[]> {
    return ipc.requestVariableUsers(projectId, path, names);
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
      forget(key);
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
        forget(key);
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

  /**
   * Montre un run : démarré ici, diffusé par une autre fenêtre ou avancé par le cœur. Une
   * révision antérieure du run montré, ou un run plus ancien, est ignoré (R3) : l'avancement
   * peut arriver avant la réponse de la commande qui a lancé le run.
   */
  function follow(projectId: string, view: RunView): void {
    const current = runs.value[projectId];
    if (current && current.id === view.id && current.revision >= view.revision) return;
    if (current && current.id !== view.id && current.startedAt > view.startedAt) return;
    runs.value[projectId] = view;
  }

  /** Ignore l'échec d'une vérification dans une ligne du run montré (R7). */
  async function ignoreInRun(projectId: string, path: string, check: string): Promise<void> {
    const current = runs.value[projectId];
    if (!current) return;
    follow(projectId, await ipc.runIgnoreCheck(projectId, current.id, path, check));
  }

  // --- Vérifications en échec (VT1 à VT3) --------------------------------------

  /** Ignore l'échec d'une vérification dans la dernière réponse de la requête (R7). */
  async function ignoreCheck(key: string, check: string): Promise<void> {
    const editor = editors[key];
    if (!editor?.outcome) return;
    applyResult(await ipc.resultIgnoreCheck(editor.projectId, editor.path, check));
  }

  /**
   * Retire une vérification de la requête (enregistrée comme toute modification) et l'ignore
   * dans la réponse affichée (R8).
   */
  async function removeCheck(key: string, check: string): Promise<void> {
    const editor = editors[key];
    if (!editor?.draft) return;
    editor.draft.assertions = editor.draft.assertions.filter((a) => a.id !== check);
    await ignoreCheck(key, check);
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
      const editor = editors[key];
      if (!editor) return;
      // Lot I : avec un brouillon, le conflit se règle élément par élément (R9) ; sans, la
      // requête est relue et l'onglet le dit quelques secondes (R3).
      if (isDirty(key) || editor.conflict) await detectConflict(key);
      else {
        await reload(key);
        if (change.origin === "") flashReloaded(key);
      }
    }
  }

  // --- Fichiers modifiés hors d'Arnod (lot I, docs/plan-mvp-i-fichiers-externes.md) ---

  /** Durée de « Rechargé depuis le disque » dans l'onglet. */
  const RELOADED_MS = 4000;

  function flashReloaded(key: string): void {
    const editor = editors[key];
    if (!editor) return;
    editor.reloaded = true;
    setTimeout(() => {
      const current = editors[key];
      if (current) current.reloaded = false;
    }, RELOADED_MS);
  }

  /**
   * Compare le brouillon au fichier (le cœur le fait, élément par élément) : un conflit à
   * régler, ou rien si le disque a rejoint le brouillon. Fichier supprimé : `removedOnDisk` ;
   * illisible : l'erreur est montrée et rien n'est écrit.
   */
  async function detectConflict(key: string): Promise<void> {
    const editor = editors[key];
    if (!editor?.draft) return;
    clearSaveTimer(key);
    try {
      const base = JSON.parse(editor.saved) as RequestDoc;
      const found = await ipc.requestConflict(editor.projectId, editor.path, base, editor.draft);
      if (found.items.every((item) => item.identical)) {
        await reload(key);
        return;
      }
      const choices: Record<string, ConflictSide> = {};
      for (const item of found.items) {
        if (item.identical) continue;
        const choice = editor.choices[item.key] ?? item.suggested;
        if (choice) choices[item.key] = choice;
      }
      editor.conflict = found;
      editor.choices = choices;
      editor.saveState = "pending";
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "notFound") {
        editor.removedOnDisk = true;
        editor.saveState = "pending";
      } else {
        editor.saveState = "error";
        editor.saveError = caught instanceof Error ? caught.message : String(caught);
      }
    }
  }

  /** Choisit la version d'un élément en conflit, ou de tous (« Tout prendre du disque »). */
  function chooseConflict(key: string, item: string | null, side: ConflictSide): void {
    const editor = editors[key];
    if (!editor?.conflict) return;
    if (item !== null) {
      editor.choices = { ...editor.choices, [item]: side };
      return;
    }
    editor.choices = Object.fromEntries(
      editor.conflict.items.filter((i) => !i.identical).map((i) => [i.key, side]),
    );
  }

  /** Chaque élément qui diffère a une version choisie. */
  function conflictReady(key: string): boolean {
    const editor = editors[key];
    if (!editor?.conflict) return false;
    return editor.conflict.items.every((item) => item.identical || item.key in editor.choices);
  }

  /** « Appliquer et enregistrer » : le cœur fusionne et écrit le fichier une seule fois (R11). */
  async function resolveConflict(key: string): Promise<void> {
    const editor = editors[key];
    if (!editor?.draft || !editor.conflict || !conflictReady(key)) return;
    try {
      const merged = await ipc.requestConflictResolve(
        editor.projectId,
        editor.path,
        JSON.parse(JSON.stringify(editor.draft)) as RequestDoc,
        { ...editor.choices },
      );
      editor.draft = merged;
      editor.saved = JSON.stringify(merged);
      editor.conflict = null;
      editor.choices = {};
      editor.draftSince = null;
      editor.saveError = null;
      editor.saveState = "saved";
      if (trees.value[editor.projectId]) await refreshTree(editor.projectId);
    } catch (caught) {
      // Le fichier a encore changé : les différences sont recalculées.
      if (caught instanceof IpcError && caught.kind === "changedOnDisk") await detectConflict(key);
      else throw caught;
    }
  }

  /** Fichier supprimé hors d'Arnod, brouillon gardé : « Restaurer » le réécrit (R4). */
  async function restoreRemoved(key: string): Promise<void> {
    const editor = editors[key];
    if (!editor?.removedOnDisk) return;
    editor.removedOnDisk = false;
    editor.saved = "";
    await save(key);
  }

  /** « Fermer l'onglet » d'une requête supprimée hors d'Arnod : le brouillon est abandonné. */
  function discardRemoved(key: string): void {
    if (editors[key]?.removedOnDisk) forget(key);
  }

  /**
   * Changements faits hors d'Arnod (R4 à R6) : les brouillons suivent les renommages ; l'onglet
   * d'une requête supprimée se ferme, sauf s'il a un brouillon (« Restaurer » ou « Fermer ») ;
   * la fenêtre principale résume le lot dans une notification.
   */
  async function onFiles(change: FilesChange): Promise<void> {
    const { projectId } = change;
    for (const renamed of change.renamed) moveEditors(projectId, renamed.from, renamed.to);
    const closed: string[] = [];
    for (const path of change.removed) {
      const key = requestKey(projectId, path);
      const editor = editors[key];
      if (!editor) continue;
      if (isDirty(key)) {
        editor.removedOnDisk = true;
        clearSaveTimer(key);
        editor.saveState = "pending";
        continue;
      }
      const tab = windows.tabFor({ kind: "request", projectId, path });
      if (tab && (await windows.close(tab.id))) closed.push(editor.draft?.name ?? path);
      forget(key);
    }
    const toasts = useToastStore();
    if (closed.length > 0) {
      toasts.show(
        closed.length === 1
          ? `« ${closed[0] ?? ""} » a été supprimée hors d'Arnod : son onglet est fermé.`
          : `${String(closed.length)} requêtes supprimées hors d'Arnod : leurs onglets sont fermés.`,
      );
    } else if (windows.label === MAIN_WINDOW) {
      const summary = filesSummary(change);
      if (summary) toasts.show(`Rechargé depuis le disque : ${summary}`);
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
    // Run démarré dans une autre fenêtre ou avancé par le cœur : suivi ici aussi (R7, R3).
    await events.onRun((change) => {
      follow(change.projectId, change.run);
    });
    // Statuts des requêtes, pour la barre latérale (R4).
    await events.onStatuses((change) => {
      statuses.value[change.projectId] = change.statuses;
    });
    // Lot I : fichiers modifiés hors d'Arnod.
    await events.onFiles((change) => {
      void onFiles(change);
    });
  }

  return {
    trees,
    environments,
    runs,
    statuses,
    ignoreInRun,
    ignoreCheck,
    removeCheck,
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
    // Lot E : tests et étapes.
    tests,
    focusUrl,
    loadTest,
    openTest,
    createTest,
    addStep,
    keepDrawn,
    addExtraction,
    variableUsers,
    isChangedSinceSent,
    revertToSent,
    // Lot I : fichiers modifiés hors d'Arnod.
    detectConflict,
    chooseConflict,
    conflictReady,
    resolveConflict,
    restoreRemoved,
    discardRemoved,
    onFiles,
  };
});
