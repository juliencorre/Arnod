import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * Fenêtres et fenêtres surgissantes partagées, ouvertes depuis plusieurs endroits
 * (sélecteur de workspace, barre latérale, vue workspace, « Tous les projets », notifications)
 * et rendues une seule fois. `null` : fermée.
 */
export const useDialogStore = defineStore("dialogs", () => {
  /** « Nouveau projet » (2a bis), workspace prérempli ou aucun. */
  const newProject = ref<{ workspaceId: string | null } | null>(null);
  /** « Ajouter à des workspaces » pour un projet (P4, P6). */
  const addToWorkspaces = ref<{ projectId: string } | null>(null);
  /** « Ajouter un projet à un workspace » (2c) : créer, importer ou reprendre. */
  const addProjects = ref<{ workspaceId: string } | null>(null);
  /**
   * « Nouveau workspace » (S3), sous le sélecteur de workspace ; `open` : ouvrir
   * le workspace créé (sinon il est détaillé dans la gestion).
   */
  const newWorkspace = ref<{ open: boolean } | null>(null);

  function openNewProject(workspaceId: string | null): void {
    newProject.value = { workspaceId };
  }

  function openAddToWorkspaces(projectId: string): void {
    addToWorkspaces.value = { projectId };
  }

  function openAddProjects(workspaceId: string): void {
    addProjects.value = { workspaceId };
  }

  function openNewWorkspace(options: { open: boolean }): void {
    newWorkspace.value = options;
  }

  return {
    newProject,
    addToWorkspaces,
    addProjects,
    newWorkspace,
    openNewProject,
    openAddToWorkspaces,
    openAddProjects,
    openNewWorkspace,
  };
});
