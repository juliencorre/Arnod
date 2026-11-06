import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { ipc } from "@/lib/ipc";
import type {
  CatalogSnapshot,
  CommandError,
  ProjectCreateInput,
  ProjectView,
  WorkspaceColor,
  WorkspaceView,
} from "@/types/ipc";

/**
 * Catalogue des projets et workspaces, tel que renvoyé par le cœur Rust.
 *
 * Aucune règle métier ici : chaque action appelle une commande, et l'instantané
 * qu'elle renvoie remplace l'état. Les recherches (`workspaceById`, `projectsOf`)
 * ne servent qu'à l'affichage. Les erreurs (`IpcError`) remontent à l'appelant,
 * qui les affiche au bon endroit.
 */
export const useCatalogStore = defineStore("catalog", () => {
  const snapshot = ref<CatalogSnapshot | null>(null);

  const projects = computed<ProjectView[]>(() => snapshot.value?.projects ?? []);
  const workspaces = computed<WorkspaceView[]>(() => snapshot.value?.workspaces ?? []);
  const projectsById = computed(() => new Map(projects.value.map((p) => [p.id, p])));
  const workspacesById = computed(() => new Map(workspaces.value.map((w) => [w.id, w])));

  /** Aucun projet encore : l'application affiche le premier lancement (1a). */
  const isFirstRun = computed(() => snapshot.value !== null && projects.value.length === 0);

  const defaultWorkspace = computed(() =>
    snapshot.value === null ? null : workspaceById(snapshot.value.defaultWorkspaceId),
  );

  function workspaceById(id: string): WorkspaceView | null {
    return workspacesById.value.get(id) ?? null;
  }

  function projectById(id: string): ProjectView | null {
    return projectsById.value.get(id) ?? null;
  }

  /** Projets d'un workspace, dans l'ordre du workspace. */
  function projectsOf(workspaceId: string): ProjectView[] {
    return (workspaceById(workspaceId)?.projectIds ?? [])
      .map((id) => projectById(id))
      .filter((project): project is ProjectView => project !== null);
  }

  function apply(next: CatalogSnapshot): void {
    snapshot.value = next;
  }

  /** Charge le catalogue au démarrage ; renvoie l'avertissement éventuel à notifier. */
  async function load(): Promise<CommandError | null> {
    const { snapshot: next, warning } = await ipc.catalogLoad();
    apply(next);
    return warning;
  }

  async function openWorkspace(workspaceId: string): Promise<void> {
    apply(await ipc.workspaceOpen(workspaceId));
  }

  /** Crée un workspace et renvoie son identifiant. */
  async function createWorkspace(name: string): Promise<string> {
    const created = await ipc.workspaceCreate(name);
    apply(created.snapshot);
    return created.workspaceId;
  }

  async function renameWorkspace(workspaceId: string, name: string): Promise<void> {
    apply(await ipc.workspaceRename(workspaceId, name));
  }

  async function setWorkspaceColor(workspaceId: string, color: WorkspaceColor): Promise<void> {
    apply(await ipc.workspaceSetColor(workspaceId, color));
  }

  async function setDefaultWorkspace(workspaceId: string): Promise<void> {
    apply(await ipc.workspaceSetDefault(workspaceId));
  }

  async function addProjects(workspaceId: string, projectIds: string[]): Promise<void> {
    apply(await ipc.workspaceAddProjects(workspaceId, projectIds));
  }

  /** Retire un projet d'un workspace ; renvoie le jeton d'annulation. */
  async function removeProject(workspaceId: string, projectId: string): Promise<string> {
    const removed = await ipc.workspaceRemoveProject(workspaceId, projectId);
    apply(removed.snapshot);
    return removed.undoToken;
  }

  /** Supprime un workspace ; renvoie le jeton d'annulation. */
  async function deleteWorkspace(workspaceId: string): Promise<string> {
    const deleted = await ipc.workspaceDelete(workspaceId);
    apply(deleted.snapshot);
    return deleted.undoToken;
  }

  async function undo(undoToken: string): Promise<void> {
    apply(await ipc.undo(undoToken));
  }

  /** Crée un projet et renvoie son identifiant. */
  async function createProject(input: ProjectCreateInput): Promise<string> {
    const created = await ipc.projectCreate(input);
    apply(created.snapshot);
    return created.projectId;
  }

  /** Inscrit un dossier projet existant et renvoie son identifiant (P12). */
  async function addExistingProject(path: string, workspaceId: string): Promise<string> {
    const added = await ipc.projectAddExisting(path, workspaceId);
    apply(added.snapshot);
    return added.projectId;
  }

  /** Oublie un projet ; renvoie le jeton d'annulation (P11). */
  async function forgetProject(projectId: string): Promise<string> {
    const forgotten = await ipc.projectForget(projectId);
    apply(forgotten.snapshot);
    return forgotten.undoToken;
  }

  /** Vérifie qu'un projet peut s'ouvrir (dossier présent) et le renvoie. */
  function openProject(projectId: string): Promise<ProjectView> {
    return ipc.projectOpen(projectId);
  }

  /** Dossier proposé `<emplacement>/<nom>` (aperçu calculé par le cœur). */
  function suggestFolder(name: string, location: string | null): Promise<string> {
    return ipc.projectSuggestFolder(name, location);
  }

  /** Sélecteur de dossier natif ; `null` si l'utilisateur annule. */
  function pickFolder(location: string | null): Promise<string | null> {
    return ipc.projectPickFolder(location);
  }

  return {
    snapshot,
    projects,
    workspaces,
    isFirstRun,
    defaultWorkspace,
    workspaceById,
    projectById,
    projectsOf,
    load,
    openWorkspace,
    createWorkspace,
    renameWorkspace,
    setWorkspaceColor,
    setDefaultWorkspace,
    addProjects,
    removeProject,
    deleteWorkspace,
    undo,
    createProject,
    addExistingProject,
    forgetProject,
    openProject,
    suggestFolder,
    pickFolder,
  };
});
