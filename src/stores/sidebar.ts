import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";

/** Section d'un projet dans la barre latérale. */
export type SidebarSection = "contracts" | "tests";

/** Demande d'ouverture d'une réponse attendue (3.2-a3, E3), lue par l'onglet de la requête. */
export interface ExampleFocus {
  /** Clé de l'éditeur de la requête (`requestKey`). */
  key: string;
  /** Position de la réponse dans `examples`. */
  index: number;
}

/**
 * État d'affichage de la barre latérale, en mémoire et propre à la fenêtre
 * (docs/plan-mvp-d-barre-laterale.md) : texte du filtre du référentiel par projet (F1), sections
 * dépliées (R6), réponse attendue à ouvrir (E3) et nombre de contrats des cartes de projet (W1).
 */
export const useSidebarStore = defineStore("sidebar", () => {
  const filters = ref<Record<string, string>>({});
  /** Choix de l'utilisateur : section dépliée ou repliée, par `projet:section`. */
  const sections = ref<Record<string, boolean>>({});
  const exampleFocus = ref<ExampleFocus | null>(null);
  /** Dernière réponse attendue ouverte depuis la barre latérale, mise en évidence (E3). */
  const shownExample = ref<ExampleFocus | null>(null);
  const contractCounts = ref<Record<string, number>>({});

  function filterOf(projectId: string): string {
    return filters.value[projectId] ?? "";
  }

  function setFilter(projectId: string, text: string): void {
    filters.value[projectId] = text;
  }

  /** Section dépliée : le choix de l'utilisateur, sinon dépliée si elle a du contenu (R6). */
  function isSectionOpen(projectId: string, section: SidebarSection, empty: boolean): boolean {
    return sections.value[`${projectId}:${section}`] ?? !empty;
  }

  function setSectionOpen(projectId: string, section: SidebarSection, open: boolean): void {
    sections.value[`${projectId}:${section}`] = open;
  }

  /** Ouvre la requête sur l'onglet « Réponses attendues », la réponse `index` choisie (E3). */
  async function openExample(projectId: string, path: string, index: number): Promise<void> {
    exampleFocus.value = { key: requestKey(projectId, path), index };
    shownExample.value = exampleFocus.value;
    await useWorkbenchStore().openRequest(projectId, path);
  }

  /** La demande a été servie par l'onglet de la requête. */
  function consumeExampleFocus(key: string): void {
    if (exampleFocus.value?.key === key) exampleFocus.value = null;
  }

  /** Lit le nombre de contrats d'un projet, pour sa carte (W1). */
  async function loadContractCount(projectId: string): Promise<void> {
    contractCounts.value[projectId] = await ipc.contractCount(projectId);
  }

  return {
    filters,
    sections,
    exampleFocus,
    shownExample,
    contractCounts,
    filterOf,
    setFilter,
    isSectionOpen,
    setSectionOpen,
    openExample,
    consumeExampleFocus,
    loadContractCount,
  };
});
