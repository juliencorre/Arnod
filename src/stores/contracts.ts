import { defineStore } from "pinia";
import { reactive, ref } from "vue";

import { events, ipc } from "@/lib/ipc";
import { contentKey, useWindowsStore } from "@/stores/windows";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  ContractSource,
  ContractSummary,
  ContractView,
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
  let subscribed = false;

  async function loadList(projectId: string): Promise<void> {
    lists.value[projectId] = await ipc.contractList(projectId);
  }

  /** Ouvre un contrat en onglet, dans la fenêtre qui suit la sélection (Z5). */
  function show(projectId: string, path: string): Promise<boolean> {
    return windows.openSelection({ kind: "contract", projectId, path });
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
    await loadList(projectId);
    await show(projectId, view.path);
    return view;
  }

  /** Importe un fichier choisi dans le sélecteur natif (C1). */
  async function importFile(projectId: string): Promise<void> {
    try {
      const view = await ipc.contractImportFile(projectId, null);
      if (view === null) return;
      await loadList(projectId);
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
      await loadList(entry.projectId);
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
      await loadList(entry.projectId);
      return true;
    } catch (caught) {
      entry.saveError = messageOf(caught);
      return false;
    }
  }

  /** Met un contrat à la corbeille (C11) ; ses onglets se ferment partout. */
  async function remove(projectId: string, path: string, name: string): Promise<void> {
    await ipc.contractDelete(projectId, path, false);
    await windows.load();
    Reflect.deleteProperty(open, contractKey(projectId, path));
    await loadList(projectId);
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

  /** Crée les requêtes des opérations cochées (C6) et en fait le bilan. */
  async function generate(key: string): Promise<GenerateReport | null> {
    const entry = open[key];
    if (!entry?.view || entry.busy || entry.selected.length === 0) return null;
    entry.busy = true;
    try {
      const { projectId } = entry;
      const report = await ipc.contractGenerate(
        projectId,
        entry.path,
        entry.selected,
        workbench.activeEnvironment(projectId),
      );
      await Promise.all([
        reload(key),
        workbench.trees[projectId] ? workbench.refreshTree(projectId) : Promise.resolve(),
        report.baseUrl && workbench.environments[projectId]
          ? workbench.refreshEnvironments(projectId)
          : Promise.resolve(),
      ]);
      await workbench.expand(projectId);
      const count = report.created.length;
      let message =
        count === 0
          ? "Aucune requête créée."
          : `${String(count)} requête${count > 1 ? "s" : ""} créée${count > 1 ? "s" : ""} dans « ${report.folder} ».`;
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

  /** Modification faite dans une autre fenêtre (F11). */
  async function onProjectChange(change: ProjectChange): Promise<void> {
    if (change.origin === windows.label) return;
    if (change.scope !== "contracts" && change.scope !== "tree") return;
    const { projectId } = change;
    if (change.scope === "contracts" && lists.value[projectId]) await loadList(projectId);
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

  return {
    lists,
    open,
    loadList,
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
    run,
    onProjectChange,
    subscribe,
  };
});
