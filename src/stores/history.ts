import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import type { HistoryDiff, HistoryEntry, HistoryView } from "@/types/ipc";

/** Clé d'une requête : projet et chemin. */
function keyOf(projectId: string, path: string): string {
  return `${projectId}\u0000${path}`;
}

/**
 * Historique des envois (écran 5.1-c bis, version 1, `docs/plan-v1.md`). Le cœur garde les 20
 * derniers envois de chaque requête sur le poste et compare chacun au précédent ; le store garde
 * la vue lue de chaque requête et les différences demandées.
 */
export const useHistoryStore = defineStore("history", () => {
  const views = ref<Record<string, HistoryView>>({});
  /** Différences de chaque envoi demandées (« Voir les différences »), par identifiant. */
  const diffs = ref<Record<string, HistoryDiff>>({});

  function viewOf(projectId: string, path: string): HistoryView | null {
    return views.value[keyOf(projectId, path)] ?? null;
  }

  /** Relit l'historique de la requête (après un envoi, à l'ouverture). */
  async function load(projectId: string, path: string): Promise<HistoryView> {
    const view = await ipc.historyLoad(projectId, path);
    views.value[keyOf(projectId, path)] = view;
    return view;
  }

  async function diff(projectId: string, path: string, id: string): Promise<HistoryDiff> {
    const found = diffs.value[id] ?? (await ipc.historyDiff(projectId, path, id));
    diffs.value[id] = found;
    return found;
  }

  /**
   * « Effacer » : l'historique de la requête est oublié sur ce poste. Renvoie de quoi
   * « Annuler » : les envois effacés.
   */
  async function clear(projectId: string, path: string): Promise<HistoryEntry[]> {
    // Copie en données simples : les envois gardés par le store sont réactifs.
    const cleared = JSON.parse(
      JSON.stringify(viewOf(projectId, path)?.items.map((item) => item.entry) ?? []),
    ) as HistoryEntry[];
    views.value[keyOf(projectId, path)] = await ipc.historyClear(projectId, path);
    return cleared;
  }

  async function restore(projectId: string, path: string, entries: HistoryEntry[]): Promise<void> {
    views.value[keyOf(projectId, path)] = await ipc.historyRestore(projectId, path, entries);
  }

  return { views, diffs, viewOf, load, diff, clear, restore };
});
