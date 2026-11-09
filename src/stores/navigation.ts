import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";

import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useWindowsStore } from "@/stores/windows";

/** Vue affichée dans la zone principale. */
export type View =
  /** Onglet épinglé « Tous les projets » (2e), ou premier lancement s'il n'y a aucun projet (1a). */
  | { kind: "all"; highlightProjectId: string | null }
  | { kind: "workspace"; workspaceId: string }
  /** Projet ouvert depuis un workspace, ou depuis « Tous les projets » (`workspaceId` à `null`). */
  | { kind: "project"; projectId: string; workspaceId: string | null }
  | {
      kind: "manage";
      /** Workspace détaillé dans « Gérer les workspaces » (2d). */
      workspaceId: string | null;
      /** Onglet à retrouver en quittant la gestion. */
      returnTo: string | null;
    };

/**
 * Navigation entre les vues, sans routeur. Tient l'onglet actif (un workspace, ou
 * « Tous les projets » avec `null`) et le titre de la fenêtre
 * (« Arnod — <onglet ou projet> »).
 */
export const useNavigationStore = defineStore("navigation", () => {
  const catalog = useCatalogStore();
  const windows = useWindowsStore();
  const view = ref<View>({ kind: "all", highlightProjectId: null });

  /** Onglet actif : workspace affiché ou contenant le projet ouvert ; `null` pour « Tous les projets ». */
  const currentWorkspaceId = computed<string | null>(() => {
    const current = view.value;
    switch (current.kind) {
      case "all":
        return null;
      case "workspace":
      case "project":
        return current.workspaceId;
      case "manage":
        return current.returnTo;
    }
  });

  /** Sujet du titre de la fenêtre. */
  const titleSubject = computed<string | null>(() => {
    const current = view.value;
    switch (current.kind) {
      case "all":
        // Avant le chargement et au premier lancement : « Arnod » seul.
        return catalog.snapshot === null || catalog.isFirstRun ? null : "Tous les projets";
      case "workspace":
        return catalog.workspaceById(current.workspaceId)?.name ?? null;
      case "project":
        return catalog.projectById(current.projectId)?.name ?? null;
      case "manage":
        return "Workspaces";
    }
  });

  /** Première vue après le chargement : le dernier onglet ouvert (R7). */
  function start(): void {
    const last = catalog.snapshot?.lastWorkspaceId ?? null;
    view.value =
      last !== null && catalog.workspaceById(last) !== null
        ? { kind: "workspace", workspaceId: last }
        : { kind: "all", highlightProjectId: null };
    // Onglets restaurés dans la fenêtre principale (Z8) : la zone de travail.
    const content = windows.activeTab?.content;
    const project = content ? catalog.projectById(content.projectId) : null;
    if (windows.kind === "main" && project && project.status !== "missing") {
      view.value = { kind: "project", projectId: project.id, workspaceId: last };
    }
  }

  /** Ouvre l'onglet d'un workspace et le mémorise pour le prochain démarrage. */
  async function showWorkspace(workspaceId: string): Promise<void> {
    await catalog.openWorkspace(workspaceId);
    view.value = { kind: "workspace", workspaceId };
  }

  /** Ouvre l'onglet « Tous les projets », en mettant éventuellement un projet en évidence. */
  async function showAll(highlightProjectId: string | null = null): Promise<void> {
    await catalog.openWorkspace(null);
    view.value = { kind: "all", highlightProjectId };
  }

  /** Ouvre l'onglet `workspaceId`, ou « Tous les projets » avec `null`. */
  function showTab(workspaceId: string | null): Promise<void> {
    return workspaceId === null ? showAll() : showWorkspace(workspaceId);
  }

  /** Affiche un projet, dans l'onglet courant (P5). */
  function showProject(projectId: string, workspaceId: string | null): void {
    view.value = { kind: "project", projectId, workspaceId };
  }

  /** Affiche « Gérer les workspaces » (2d), sur `workspaceId` ou le premier. */
  function showManage(workspaceId?: string): void {
    view.value = {
      kind: "manage",
      workspaceId: workspaceId ?? currentWorkspaceId.value ?? catalog.workspaces[0]?.id ?? null,
      returnTo: currentWorkspaceId.value,
    };
  }

  /** Choisit le workspace détaillé dans la gestion (2d). */
  function selectManagedWorkspace(workspaceId: string): void {
    if (view.value.kind === "manage") view.value = { ...view.value, workspaceId };
  }

  /** Quitte la gestion pour l'onglet d'où l'on venait. */
  async function closeManage(): Promise<void> {
    const current = view.value;
    if (current.kind !== "manage") return;
    const back = current.returnTo !== null && catalog.workspaceById(current.returnTo) !== null;
    await showTab(back ? current.returnTo : null);
  }

  // Le catalogue change : un projet oublié ramène à son onglet, un workspace
  // supprimé ramène sur « Tous les projets » (R8), comme le cœur.
  watch(
    () => catalog.snapshot,
    (snapshot) => {
      const current = view.value;
      if (snapshot === null) return;
      if (current.kind === "project") {
        if (catalog.projectById(current.projectId) === null) {
          view.value =
            current.workspaceId !== null && catalog.workspaceById(current.workspaceId) !== null
              ? { kind: "workspace", workspaceId: current.workspaceId }
              : { kind: "all", highlightProjectId: null };
        } else if (
          current.workspaceId !== null &&
          catalog.workspaceById(current.workspaceId) === null
        ) {
          view.value = { ...current, workspaceId: null };
        }
        return;
      }
      if (current.kind === "workspace" && catalog.workspaceById(current.workspaceId) === null) {
        view.value = { kind: "all", highlightProjectId: null };
        return;
      }
      if (current.kind === "manage") {
        const detailed =
          current.workspaceId !== null && catalog.workspaceById(current.workspaceId) !== null
            ? current.workspaceId
            : (catalog.workspaces[0]?.id ?? null);
        const returnTo =
          current.returnTo !== null && catalog.workspaceById(current.returnTo) !== null
            ? current.returnTo
            : null;
        if (detailed !== current.workspaceId || returnTo !== current.returnTo) {
          view.value = { ...current, workspaceId: detailed, returnTo };
        }
      }
    },
  );

  // Fenêtre principale : une requête ouverte depuis la barre latérale (ici ou
  // détachée) affiche les zones (F1, Z5).
  watch(
    () => windows.activeTab?.id ?? null,
    (tab, previous) => {
      const content = windows.activeTab?.content;
      if (windows.kind !== "main" || tab === null || tab === previous || !content) return;
      const current = view.value;
      if (current.kind === "project" && current.projectId === content.projectId) return;
      if (catalog.projectById(content.projectId) === null) return;
      showProject(content.projectId, currentWorkspaceId.value);
    },
  );

  watch(
    titleSubject,
    (subject) => {
      // Une fenêtre de contenu gère son titre ; ailleurs, il suit la vue.
      if (windows.kind === "content") return;
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
    showAll,
    showTab,
    showProject,
    showManage,
    selectManagedWorkspace,
    closeManage,
  };
});
