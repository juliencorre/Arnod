import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";

import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";

/** Onglets de l'écran de gestion (2d, 2e). */
export type ManageTab = "workspaces" | "registry";

/** Vue affichée dans la zone principale. */
export type View =
  | { kind: "firstRun" }
  | { kind: "workspace"; workspaceId: string }
  | { kind: "project"; projectId: string; workspaceId: string }
  | { kind: "manage"; tab: ManageTab };

/** Sujet du titre de fenêtre pour les onglets de gestion. */
const MANAGE_TITLES: Record<ManageTab, string> = {
  workspaces: "Workspaces",
  registry: "Référentiel des projets",
};

/**
 * Navigation entre les vues, sans routeur. Tient aussi le titre de la fenêtre
 * (« Arnod — <workspace ou projet> ») à jour.
 */
export const useNavigationStore = defineStore("navigation", () => {
  const catalog = useCatalogStore();
  const view = ref<View>({ kind: "firstRun" });

  /** Workspace affiché ou contenant le projet ouvert, pour la barre latérale. */
  const currentWorkspaceId = computed<string | null>(() => {
    const current = view.value;
    if (current.kind === "workspace" || current.kind === "project") return current.workspaceId;
    return catalog.snapshot?.lastWorkspaceId ?? null;
  });

  /** Sujet du titre de la fenêtre ; `null` pour « Arnod » seul. */
  const titleSubject = computed<string | null>(() => {
    const current = view.value;
    switch (current.kind) {
      case "firstRun":
        return null;
      case "workspace":
        return catalog.workspaceById(current.workspaceId)?.name ?? null;
      case "project":
        return catalog.projectById(current.projectId)?.name ?? null;
      case "manage":
        return MANAGE_TITLES[current.tab];
    }
  });

  /**
   * Première vue après le chargement : premier lancement s'il n'y a aucun projet,
   * sinon le dernier workspace ouvert (W2, R7).
   */
  function start(): void {
    const last = catalog.snapshot?.lastWorkspaceId;
    view.value =
      catalog.isFirstRun || last === undefined
        ? { kind: "firstRun" }
        : { kind: "workspace", workspaceId: last };
  }

  /** Ouvre un workspace et le mémorise pour le prochain démarrage. */
  async function showWorkspace(workspaceId: string): Promise<void> {
    await catalog.openWorkspace(workspaceId);
    view.value = { kind: "workspace", workspaceId };
  }

  /** Affiche un projet, dans le workspace courant (P5). */
  function showProject(projectId: string, workspaceId: string): void {
    view.value = { kind: "project", projectId, workspaceId };
  }

  /** Affiche l'écran de gestion sur l'onglet voulu (2d, 2e). */
  function showManage(tab: ManageTab): void {
    view.value = { kind: "manage", tab };
  }

  // Si le workspace affiché disparaît (suppression, R8), revenir au workspace
  // que le cœur a choisi comme dernier ouvert.
  watch(
    () => catalog.snapshot,
    (snapshot) => {
      const current = view.value;
      if (snapshot === null || current.kind !== "workspace") return;
      if (catalog.workspaceById(current.workspaceId) === null) {
        view.value = { kind: "workspace", workspaceId: snapshot.lastWorkspaceId };
      }
    },
  );

  watch(
    titleSubject,
    (subject) => {
      // Titre purement cosmétique : un échec n'empêche pas de travailler.
      ipc.windowSetTitle(subject).catch(() => undefined);
    },
    { immediate: true },
  );

  return { view, currentWorkspaceId, titleSubject, start, showWorkspace, showProject, showManage };
});
