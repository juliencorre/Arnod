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
  | {
      kind: "manage";
      tab: ManageTab;
      /** Workspace détaillé dans l'onglet « Workspaces » (2d). */
      workspaceId: string;
      /** Projet mis en évidence dans le référentiel (P8). */
      highlightProjectId: string | null;
    };

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
    if (current.kind !== "firstRun") return current.workspaceId;
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
  function showManage(
    tab: ManageTab,
    options: { workspaceId?: string; highlightProjectId?: string } = {},
  ): void {
    const workspaceId =
      options.workspaceId ?? currentWorkspaceId.value ?? catalog.snapshot?.defaultWorkspaceId ?? "";
    view.value = {
      kind: "manage",
      tab,
      workspaceId,
      highlightProjectId: options.highlightProjectId ?? null,
    };
  }

  /** Change d'onglet dans l'écran de gestion, sans mise en évidence. */
  function selectTab(tab: ManageTab): void {
    if (view.value.kind === "manage") view.value = { ...view.value, tab, highlightProjectId: null };
  }

  /** Choisit le workspace détaillé dans l'écran de gestion (2d). */
  function selectManagedWorkspace(workspaceId: string): void {
    if (view.value.kind === "manage") view.value = { ...view.value, workspaceId };
  }

  /** Quitte l'écran de gestion pour le dernier workspace ouvert. */
  function closeManage(): void {
    const last = catalog.snapshot?.lastWorkspaceId;
    if (last !== undefined) view.value = { kind: "workspace", workspaceId: last };
  }

  // Si le workspace affiché disparaît (suppression, R8), revenir au workspace
  // que le cœur a choisi comme dernier ouvert ; dans l'écran de gestion, au
  // workspace par défaut.
  watch(
    () => catalog.snapshot,
    (snapshot) => {
      const current = view.value;
      if (snapshot === null || current.kind === "firstRun") return;
      if (current.kind === "project") {
        // Projet oublié (P11) : retour à son workspace.
        if (catalog.projectById(current.projectId) === null) {
          view.value = { kind: "workspace", workspaceId: current.workspaceId };
        }
        return;
      }
      if (catalog.workspaceById(current.workspaceId) !== null) return;
      view.value =
        current.kind === "workspace"
          ? { kind: "workspace", workspaceId: snapshot.lastWorkspaceId }
          : { ...current, workspaceId: snapshot.defaultWorkspaceId };
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

  return {
    view,
    currentWorkspaceId,
    titleSubject,
    start,
    showWorkspace,
    showProject,
    showManage,
    selectTab,
    selectManagedWorkspace,
    closeManage,
  };
});
