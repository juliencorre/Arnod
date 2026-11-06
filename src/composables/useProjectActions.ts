/**
 * Actions sur les projets partagées entre les vues : ouverture, ajout et
 * retrait de workspaces, annonce d'une création. Chaque action notifie son
 * résultat ; les erreurs du cœur sont affichées telles quelles.
 */
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";

/** « A », « A et B », « A, B et C ». */
export function joinNames(names: string[]): string {
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} et ${names.at(-1) ?? ""}`;
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

  /** Ouvre un projet (P5) ; un dossier introuvable est notifié (P10). */
  async function openProject(projectId: string, workspaceId: string): Promise<void> {
    try {
      await catalog.openProject(projectId);
      navigation.showProject(projectId, workspaceId);
    } catch (error) {
      toasts.showError(error);
    }
  }

  /**
   * Après une création (P1, P2) : ouvre le projet et l'annonce, avec
   * « Ajouter à un autre… » (P4).
   */
  function announceCreated(projectId: string, workspaceId: string): void {
    navigation.showProject(projectId, workspaceId);
    const workspaces = (catalog.projectById(projectId)?.workspaceIds ?? [])
      .map((id) => catalog.workspaceById(id))
      .filter((workspace) => workspace !== null)
      .map((workspace) =>
        workspace.isDefault ? `${workspace.name} (par défaut)` : workspace.name,
      );
    const where =
      workspaces.length > 1
        ? `dans les workspaces ${joinNames(workspaces)}`
        : `dans le workspace ${joinNames(workspaces)}`;
    toasts.show(`✓ « ${projectName(projectId)} » créé ${where}.`, {
      action: {
        label: "Ajouter à un autre…",
        run: () => {
          dialogs.openAddToWorkspaces(projectId);
        },
      },
    });
  }

  /** Ajoute un projet à des workspaces (P6). */
  async function addToWorkspaces(projectId: string, workspaceIds: string[]): Promise<void> {
    try {
      for (const workspaceId of workspaceIds) {
        await catalog.addProjects(workspaceId, [projectId]);
      }
      toasts.show(
        `« ${projectName(projectId)} » ajouté à ${joinNames(workspaceIds.map(workspaceName))}.`,
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
      toasts.show(`« ${name} » retiré de « ${from} ». Le projet reste dans le référentiel.`, {
        action: { label: "Annuler", run: () => catalog.undo(undoToken) },
      });
    } catch (error) {
      toasts.showError(error);
    }
  }

  return { openProject, announceCreated, addToWorkspaces, removeFromWorkspace };
}
