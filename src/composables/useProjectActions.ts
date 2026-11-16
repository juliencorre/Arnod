/**
 * Actions sur les projets et workspaces partagées entre les vues : ouverture,
 * ajout et retrait de workspaces, annonce d'une création, suppression d'un
 * workspace. Chaque action notifie son résultat ; les erreurs du cœur sont
 * affichées telles quelles.
 */
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";

/** Libellé de l'onglet épinglé, qui remplace l'ancien « référentiel ». */
export const ALL_PROJECTS = "Tous les projets";

/** « A », « A et B », « A, B et C ». */
export function joinNames(names: string[]): string {
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} et ${names.at(-1) ?? ""}`;
}

/** « « A » », « « A » et « B » ». */
function quoted(names: string[]): string {
  return joinNames(names.map((name) => `« ${name} »`));
}

export function useProjectActions() {
  const catalog = useCatalogStore();
  const navigation = useNavigationStore();
  const toasts = useToastStore();
  const dialogs = useDialogStore();

  function projectName(projectId: string): string {
    return catalog.projectById(projectId)?.name ?? "Le projet";
  }

  function workspaceName(workspaceId: string): string {
    return catalog.workspaceById(workspaceId)?.name ?? "ce workspace";
  }

  /** Ouvre un projet dans l'onglet courant (P5) ; un dossier introuvable est notifié (P10). */
  async function openProject(projectId: string, workspaceId: string | null): Promise<void> {
    try {
      await catalog.openProject(projectId);
      navigation.showProject(projectId, workspaceId);
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Après une création ou un import (P1, P2, P12) : ouvre le projet et l'annonce,
   * avec « Ranger dans un workspace… » ou « Ajouter à un autre… » (P4).
   */
  function announceCreated(projectId: string, workspaceId: string | null): void {
    navigation.showProject(projectId, workspaceId);
    const workspaces = (catalog.projectById(projectId)?.workspaceIds ?? []).map(workspaceName);
    const name = projectName(projectId);
    const message =
      workspaces.length === 0
        ? `✓ « ${name} » créé · visible dans « ${ALL_PROJECTS} ».`
        : `✓ « ${name} » créé dans ${quoted(workspaces)} · visible dans « ${ALL_PROJECTS} ».`;
    toasts.show(message, {
      action: {
        label: workspaces.length === 0 ? "Ranger dans un workspace…" : "Ajouter à un autre…",
        run: () => {
          dialogs.openAddToWorkspaces(projectId);
        },
      },
    });
  }

  /** Ajoute un projet à des workspaces (P6, O7) ; ceux qui le contiennent déjà sont signalés. */
  async function addToWorkspaces(projectId: string, workspaceIds: string[]): Promise<void> {
    const present = catalog.projectById(projectId)?.workspaceIds ?? [];
    const targets = workspaceIds.filter((id) => !present.includes(id));
    if (targets.length === 0) {
      toasts.show(
        `« ${projectName(projectId)} » est déjà dans ${quoted(workspaceIds.map(workspaceName))}.`,
      );
      return;
    }
    try {
      for (const workspaceId of targets) {
        await catalog.addProjects(workspaceId, [projectId]);
      }
      toasts.show(`« ${projectName(projectId)} » ajouté à ${quoted(targets.map(workspaceName))}.`);
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Inscrit un dossier projet existant choisi dans le sélecteur natif (P12),
   * dans « Tous les projets » et `workspaceId` s'il est donné.
   */
  async function addExistingProject(workspaceId: string | null): Promise<void> {
    try {
      const path = await catalog.pickFolder(null);
      if (path === null) return;
      const projectId = await catalog.addExistingProject(path, workspaceId);
      const where =
        workspaceId === null
          ? `« ${ALL_PROJECTS} »`
          : `« ${ALL_PROJECTS} » et à « ${workspaceName(workspaceId)} »`;
      toasts.show(`« ${projectName(projectId)} » ajouté à ${where}.`);
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Oublie un projet : il quitte « Tous les projets » et ses workspaces, ses
   * fichiers restent sur le disque ; « Annuler » le réinscrit (P11, R6).
   */
  async function forgetProject(projectId: string): Promise<void> {
    const name = projectName(projectId);
    try {
      const undoToken = await catalog.forgetProject(projectId);
      toasts.show(
        `« ${name} » oublié : retiré de « ${ALL_PROJECTS} » et de ses workspaces. Ses fichiers restent sur le disque.`,
        { action: { label: "Annuler", run: () => catalog.undo(undoToken) } },
      );
    } catch (error) {
      toasts.showError(error);
    }
  }

  /** Retire un projet d'un workspace, avec « Annuler » (P7, R5, R6). */
  async function removeFromWorkspace(projectId: string, workspaceId: string): Promise<void> {
    const name = projectName(projectId);
    const from = workspaceName(workspaceId);
    try {
      const undoToken = await catalog.removeProject(workspaceId, projectId);
      toasts.show(`« ${name} » retiré de « ${from} ». Le projet reste dans « ${ALL_PROJECTS} ».`, {
        action: { label: "Annuler", run: () => catalog.undo(undoToken) },
      });
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Supprime un workspace sans confirmation : ses projets restent dans « Tous
   * les projets » ; « Annuler » le rétablit et rouvre son onglet s'il était
   * ouvert (O6, R5, R6, R8).
   */
  async function deleteWorkspace(workspaceId: string): Promise<void> {
    const workspace = catalog.workspaceById(workspaceId);
    if (!workspace) return;
    const kept = countLabel(workspace.projectIds.length, "projet conservé", "projets conservés");
    const wasOpen =
      navigation.currentWorkspaceId === workspaceId && navigation.view.kind !== "manage";
    try {
      const undoToken = await catalog.deleteWorkspace(workspaceId);
      toasts.show(`Workspace « ${workspace.name} » supprimé · ${kept} dans « ${ALL_PROJECTS} ».`, {
        action: {
          label: "Annuler",
          run: async () => {
            await catalog.undo(undoToken);
            if (wasOpen) await navigation.showWorkspace(workspaceId);
            else navigation.selectManagedWorkspace(workspaceId);
          },
        },
      });
    } catch (error) {
      toasts.showError(error);
    }
  }

  return {
    openProject,
    announceCreated,
    addToWorkspaces,
    addExistingProject,
    forgetProject,
    removeFromWorkspace,
    deleteWorkspace,
  };
}
