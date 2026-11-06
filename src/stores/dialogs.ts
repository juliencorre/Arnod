import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * Fenêtres modales de l'application, ouvertes depuis plusieurs endroits
 * (barre latérale, vue workspace, référentiel, notifications) et rendues une
 * seule fois par la coquille. `null` : fenêtre fermée.
 */
export const useDialogStore = defineStore("dialogs", () => {
  /** « Nouveau projet » (2a bis), workspace prérempli. */
  const newProject = ref<{ workspaceId: string } | null>(null);
  /** « Ajouter à des workspaces » pour un projet (P4, P6). */
  const addToWorkspaces = ref<{ projectId: string } | null>(null);
  /** « Ajouter des projets à un workspace » (2c). */
  const addProjects = ref<{ workspaceId: string } | null>(null);
  /** « Nouveau workspace » (2b, 2d) ; `open` : ouvrir le workspace créé. */
  const newWorkspace = ref<{ open: boolean } | null>(null);

  function openNewProject(workspaceId: string): void {
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
