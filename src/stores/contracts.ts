import { defineStore } from "pinia";
import { reactive, ref, watch } from "vue";

import { events, ipc } from "@/lib/ipc";
import { contentKey, useWindowsStore } from "@/stores/windows";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  ContractSource,
  ContractSummary,
  ContractView,
  FiledTrials,
  GenerateReport,
  ProjectChange,
} from "@/types/ipc";

/** Façon d'ajouter un contrat dans le dialogue (C2 à C4). */
export type ContractDialogMode = "text" | "url" | "template";

/** Contrat ouvert dans un onglet : vue du cœur et source en cours de modification. */
export interface OpenContract {
  projectId: string;
  path: string;
  view: ContractView | null;
  /** Lecture impossible (contrat supprimé). */
  loadError: string | null;
  /** Source modifiée dans la vue « Source » (C9). */
  draft: string;
  saveError: string | null;
  /** Clés des opérations cochées pour « Créer les requêtes » (C6). */
  selected: string[];
  busy: boolean;
  /** Vue affichée dans l'onglet. */
  tab: "operations" | "source";
}

/** Clé d'un contrat ouvert. */
export function contractKey(projectId: string, path: string): string {
  return contentKey({ kind: "contract", projectId, path });
}

/**
 * Nœud dépliable d'un contrat dans la barre latérale (docs/plan-requetes-dans-contrats.md, RC1) :
 * le contrat, une ressource, une opération ou le groupe « Hors contrat ».
 */
export type ContractNode =
  | { kind: "contract"; path: string }
  | { kind: "resource"; path: string; resource: string }
  | { kind: "operation"; path: string; operation: string }
  | { kind: "unplaced"; path: string };

/** Clé d'un nœud déplié, par projet. */
export function nodeKey(projectId: string, node: ContractNode): string {
  const part =
    node.kind === "resource" ? node.resource : node.kind === "operation" ? node.operation : "";
  return [projectId, node.kind, node.path, part].join("\u0000");
}

/** Message d'une erreur, prêt à afficher. */
function messageOf(caught: unknown): string {
  return caught instanceof Error ? caught.message : String(caught);
}

/**
 * Contrats de services (docs/plan-contrats-services.md) : contrats de chaque
 * projet et contrats ouverts dans les onglets de cette fenêtre. Le cœur lit
 * les contrats, crée les requêtes et vérifie la conformité ; le store affiche.
 */
export const useContractsStore = defineStore("contracts", () => {
  const windows = useWindowsStore();
  const workbench = useWorkbenchStore();
  const toasts = useToastStore();
  const lists = ref<Record<string, ContractSummary[]>>({});
  const open = reactive<Record<string, OpenContract>>({});
  /** Contrats à ouvrir sur leur source : créés vides (C4). */
  const sourceFirst = new Set<string>();
  /** Nœuds dépliés dans la barre latérale de cette fenêtre ; repliés par défaut (RC1). */
  const expanded = ref<string[]>([]);
  /** Projets dont l'arborescence est relue avec leurs contrats : l'observateur n'y revient pas. */
  const syncing = new Set<string>();
  let subscribed = false;

  async function loadList(projectId: string): Promise<void> {
    lists.value[projectId] = await ipc.contractList(projectId);
  }

  /**
   * Relit les contrats et l'arborescence d'un projet chargé : le référentiel de tests ne garde que
   * les requêtes sans contrat (R5, R6), les contrats montrent leurs requêtes (R1).
   */
  async function sync(projectId: string): Promise<void> {
    await Promise.all([
      lists.value[projectId] ? loadList(projectId) : Promise.resolve(),
      refreshTree(projectId),
    ]);
  }

  /** Relit l'arborescence d'un projet chargé, sans que l'observateur relise ses contrats. */
  async function refreshTree(projectId: string): Promise<void> {
    if (!workbench.trees[projectId]) return;
    syncing.add(projectId);
    try {
      await workbench.refreshTree(projectId);
    } catch (caught) {
      syncing.delete(projectId);
      throw caught;
    }
  }

  // Arborescence relue (requête créée, renommée, déplacée, supprimée, ici ou ailleurs) : les
  // requêtes rangées sous les contrats aussi (RC9).
  watch(
    () => ({ ...workbench.trees }),
    (now, before) => {
      for (const [projectId, tree] of Object.entries(now)) {
        const previous = before[projectId];
        if (previous === undefined || previous === tree) continue;
        if (syncing.delete(projectId) || !lists.value[projectId]) continue;
        loadList(projectId).catch((caught: unknown) => {
          toasts.showError(caught);
        });
      }
    },
  );

  function isExpanded(projectId: string, node: ContractNode): boolean {
    return expanded.value.includes(nodeKey(projectId, node));
  }

  function setExpanded(projectId: string, node: ContractNode, open: boolean): void {
    const key = nodeKey(projectId, node);
    const present = expanded.value.includes(key);
    if (open && !present) expanded.value = [...expanded.value, key];
    else if (!open && present) expanded.value = expanded.value.filter((k) => k !== key);
  }

  function toggleExpanded(projectId: string, node: ContractNode): void {
    setExpanded(projectId, node, !isExpanded(projectId, node));
  }

  /**
   * Déplie le chemin d'une requête liée (RC4) : son contrat, sa ressource et, si elle n'y est pas
   * seule, son opération. `false` si elle n'est sous aucun contrat de la liste.
   */
  function reveal(projectId: string, request: string): boolean {
    for (const contract of lists.value[projectId] ?? []) {
      const path = contract.path;
      if (contract.unplaced.some((r) => r.path === request)) {
        setExpanded(projectId, { kind: "contract", path }, true);
        setExpanded(projectId, { kind: "unplaced", path }, true);
        return true;
      }
      for (const resource of contract.resources) {
        const operation = resource.operations.find((o) =>
          o.requests.some((r) => r.path === request),
        );
        if (!operation) continue;
        setExpanded(projectId, { kind: "contract", path }, true);
        setExpanded(projectId, { kind: "resource", path, resource: resource.path }, true);
        if (operation.requests.length > 1) {
          setExpanded(projectId, { kind: "operation", path, operation: operation.key }, true);
        }
        return true;
      }
    }
    return false;
  }

  /** Ouvre un contrat en onglet, dans la fenêtre qui suit la sélection (Z5) ; en aperçu si `preview`. */
  function show(projectId: string, path: string, preview = false): Promise<boolean> {
    return windows.openSelection({ kind: "contract", projectId, path }, preview);
  }

  function isDirty(key: string): boolean {
    const entry = open[key];
    return !!entry?.view && entry.draft !== entry.view.source;
  }

  /** Nouvelle vue du cœur ; une source modifiée ici est gardée. */
  function apply(key: string, view: ContractView): void {
    const entry = open[key];
    if (!entry) return;
    const keepDraft = isDirty(key);
    const before = entry.view;
    entry.view = view;
    entry.loadError = null;
    if (!keepDraft) entry.draft = view.source;
    // Cochées par défaut : les opérations sans requête (C6).
    const creatable = view.operations
      .filter((o) => o.supported && o.request === null)
      .map((o) => o.key);
    entry.selected =
      before === null
        ? creatable
        : entry.selected
            .filter((k) => creatable.includes(k))
            .concat(creatable.filter((k) => !before.operations.some((o) => o.key === k)));
  }

  /** Contrat d'un onglet, lu une fois. */
  async function openTab(projectId: string, path: string): Promise<OpenContract> {
    const key = contractKey(projectId, path);
    const existing = open[key];
    if (existing) return existing;
    open[key] = {
      projectId,
      path,
      view: null,
      loadError: null,
      draft: "",
      saveError: null,
      selected: [],
      busy: false,
      tab: sourceFirst.delete(key) ? "source" : "operations",
    };
    await reload(key);
    const entry = open[key];
    if (entry.view?.error) entry.tab = "source";
    return entry;
  }

  async function reload(key: string): Promise<void> {
    const entry = open[key];
    if (!entry) return;
    try {
      apply(key, await ipc.contractRead(entry.projectId, entry.path));
    } catch (caught) {
      entry.loadError = messageOf(caught);
    }
  }

  /** Oublie un contrat qui n'est plus ouvert dans cette fenêtre. */
  function release(key: string): void {
    if (!isDirty(key)) Reflect.deleteProperty(open, key);
  }

  /** Ajoute un contrat collé, lu à une adresse ou vide (C2 à C4), puis l'ouvre. */
  async function add(projectId: string, source: ContractSource): Promise<ContractView> {
    const view = await ipc.contractAdd(projectId, source);
    if (source.kind === "template") sourceFirst.add(contractKey(projectId, view.path));
    await sync(projectId);
    await show(projectId, view.path);
    return view;
  }

  /** Importe un fichier choisi dans le sélecteur natif (C1). */
  async function importFile(projectId: string): Promise<void> {
    try {
      const view = await ipc.contractImportFile(projectId, null);
      if (view === null) return;
      await sync(projectId);
      await show(projectId, view.path);
      toasts.show(`Contrat « ${view.name} » ajouté.`);
    } catch (caught) {
      toasts.showError(caught);
    }
  }

  /** Remplace la source par un fichier (C9). */
  async function replaceFromFile(key: string): Promise<void> {
    const entry = open[key];
    if (!entry) return;
    try {
      const view = await ipc.contractImportFile(entry.projectId, entry.path);
      if (view === null) return;
      entry.draft = view.source;
      apply(key, view);
      await sync(entry.projectId);
      toasts.show(`Contrat « ${view.name} » mis à jour.`);
    } catch (caught) {
      toasts.showError(caught);
    }
  }

  /** Enregistre la source modifiée ; refusée si illisible (R10). */
  async function save(key: string): Promise<boolean> {
    const entry = open[key];
    if (!entry || !isDirty(key)) return true;
    entry.saveError = null;
    try {
      const view = await ipc.contractSave(entry.projectId, entry.path, entry.draft);
      entry.draft = view.source;
      apply(key, view);
      await sync(entry.projectId);
      return true;
    } catch (caught) {
      entry.saveError = messageOf(caught);
      return false;
    }
  }

  /**
   * Met un contrat à la corbeille (C11) ; ses onglets se ferment partout et ses requêtes reviennent
   * dans le référentiel de tests (R6).
   */
  async function remove(projectId: string, path: string, name: string): Promise<void> {
    await ipc.contractDelete(projectId, path, false);
    await windows.load();
    Reflect.deleteProperty(open, contractKey(projectId, path));
    await sync(projectId);
    toasts.show(`Contrat « ${name} » mis à la corbeille.`);
  }

  function toggle(key: string, operation: string): void {
    const entry = open[key];
    if (!entry) return;
    entry.selected = entry.selected.includes(operation)
      ? entry.selected.filter((k) => k !== operation)
      : [...entry.selected, operation];
  }

  function setSelection(key: string, operations: string[]): void {
    const entry = open[key];
    if (entry) entry.selected = operations;
  }

  /**
   * Crée les requêtes d'opérations du contrat, relit le projet et montre les requêtes créées sous
   * le contrat, qui se déplie (RC7) : le bilan, ou `null` si le cœur refuse.
   */
  async function createRequests(
    projectId: string,
    path: string,
    operations: string[],
  ): Promise<GenerateReport> {
    const report = await ipc.contractGenerate(
      projectId,
      path,
      operations,
      workbench.activeEnvironment(projectId),
    );
    const key = contractKey(projectId, path);
    await Promise.all([
      open[key] ? reload(key) : Promise.resolve(),
      refreshTree(projectId),
      loadList(projectId),
      report.baseUrl && workbench.environments[projectId]
        ? workbench.refreshEnvironments(projectId)
        : Promise.resolve(),
    ]);
    await workbench.expand(projectId);
    setExpanded(projectId, { kind: "contract", path }, true);
    const contract = lists.value[projectId]?.find((c) => c.path === path);
    for (const resource of contract?.resources ?? []) {
      const created = resource.operations.some((o) =>
        o.requests.some((r) => report.created.includes(r.path)),
      );
      if (created)
        setExpanded(projectId, { kind: "resource", path, resource: resource.path }, true);
    }
    return report;
  }

  /** Crée les requêtes des opérations cochées (C6) et en fait le bilan. */
  async function generate(key: string): Promise<GenerateReport | null> {
    const entry = open[key];
    if (!entry?.view || entry.busy || entry.selected.length === 0) return null;
    entry.busy = true;
    try {
      const report = await createRequests(entry.projectId, entry.path, entry.selected);
      const count = report.created.length;
      let message =
        count === 0
          ? "Aucune requête créée."
          : `${String(count)} requête${count > 1 ? "s" : ""} créée${count > 1 ? "s" : ""} sous le contrat « ${entry.view.name} ».`;
      if (report.baseUrl) {
        message += ` baseUrl de « ${report.baseUrl.environment} » : ${report.baseUrl.value}.`;
      }
      toasts.show(message);
      return report;
    } catch (caught) {
      toasts.showError(caught);
      return null;
    } finally {
      entry.busy = false;
    }
  }

  /** Opération sans requête cliquée dans la barre latérale : crée sa requête et l'ouvre (RC3). */
  async function createForOperation(
    projectId: string,
    path: string,
    operation: string,
  ): Promise<void> {
    try {
      const report = await createRequests(projectId, path, [operation]);
      const created = report.created[0];
      if (created === undefined) {
        toasts.show(report.skipped[0]?.reason ?? "Aucune requête créée.", { tone: "error" });
        return;
      }
      reveal(projectId, created);
      await workbench.openRequest(projectId, created);
    } catch (caught) {
      toasts.showError(caught);
    }
  }

  /** Exécute les requêtes du contrat (C10). */
  async function run(key: string): Promise<void> {
    const entry = open[key];
    if (!entry) return;
    try {
      await workbench.runContract(entry.projectId, entry.path);
    } catch (caught) {
      toasts.showError(caught);
    }
  }

  /**
   * Modification faite dans une autre fenêtre (F11). Une arborescence relue relit les contrats
   * (observateur ci-dessus) ; un contrat ajouté ou supprimé relit aussi l'arborescence (RC9).
   */
  async function onProjectChange(change: ProjectChange): Promise<void> {
    if (change.origin === windows.label) return;
    if (change.scope !== "contracts" && change.scope !== "tree") return;
    const { projectId } = change;
    if (change.scope === "contracts") await sync(projectId);
    // Arborescence ou contrat changé : les liens des onglets ouverts aussi.
    for (const [key, entry] of Object.entries(open)) {
      if (entry.projectId === projectId) await reload(key);
    }
  }

  async function subscribe(): Promise<void> {
    if (subscribed) return;
    subscribed = true;
    await events.onProject((change) => {
      void onProjectChange(change);
    });
  }

  // --- Essais rangés dans un projet (2.2-d) ------------------------------------

  /** Range les essais du contrat déduit `contract` de « Non classés » dans le projet `target`. */
  async function fileTrials(
    inbox: string,
    contract: string,
    target: string,
    into: string | null,
  ): Promise<FiledTrials> {
    const filed = await ipc.trialFile(inbox, contract, target, into);
    await Promise.all([sync(inbox), workbench.trees[target] ? sync(target) : Promise.resolve()]);
    return filed;
  }

  /** « Annuler » un rangement : les essais reviennent dans « Non classés ». */
  async function undoFiling(inbox: string, target: string, token: string): Promise<void> {
    await ipc.trialFileUndo(token);
    await Promise.all([sync(inbox), workbench.trees[target] ? sync(target) : Promise.resolve()]);
  }

  return {
    lists,
    open,
    expanded,
    loadList,
    sync,
    isExpanded,
    setExpanded,
    toggleExpanded,
    reveal,
    show,
    isDirty,
    openTab,
    reload,
    release,
    add,
    importFile,
    replaceFromFile,
    save,
    remove,
    toggle,
    setSelection,
    generate,
    createForOperation,
    run,
    onProjectChange,
    subscribe,
    // Essais rangés dans un projet (2.2-d).
    fileTrials,
    undoFiling,
  };
});
