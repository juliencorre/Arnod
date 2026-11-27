/**
 * Tests en schéma (docs/plan-v4.md, lots A et B) : brouillon du schéma de chaque onglet de test,
 * étape choisie, vue (Liste ou Schéma), ajout et retrait d'étapes. Le cœur réconcilie, vérifie et
 * enregistre (`flow_save`, R1 à R3) ; ce store ne garde que l'état d'affichage et le brouillon.
 */
import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useToastStore } from "@/stores/toasts";
import { contentKey } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  Flow,
  FlowStep,
  FolderHooks,
  FolderProperty,
  HookPhase,
  HooksView,
  StepKind,
  StepPlacement,
  TestView,
} from "@/types/ipc";

export type FlowMode = "list" | "diagram";

/** Copie profonde (le brouillon ne partage rien avec la vue du cœur). */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export const useFlowsStore = defineStore("flows", () => {
  const workbench = useWorkbenchStore();
  /** Brouillon du schéma de chaque test ouvert, par clé de contenu. */
  const drafts = ref<Record<string, Flow>>({});
  /** Étape choisie (panneau de droite, 4.3-c). */
  const selected = ref<Record<string, string | null>>({});
  /** Vue de chaque onglet : Liste (4.3-a) ou Schéma (4.3-b). */
  const modes = ref<Record<string, FlowMode>>({});
  /** Dernière étape insérée, pour « Annuler l'insertion » (4.3-b quater). */
  const inserted = ref<Record<string, string | null>>({});

  function key(projectId: string, path: string): string {
    return contentKey({ kind: "test", projectId, path });
  }

  /** Reprend le schéma du cœur (lecture, enregistrement, ajout). */
  function adopt(projectId: string, path: string, view: TestView): void {
    workbench.tests[key(projectId, path)] = view;
    drafts.value[key(projectId, path)] = clone(view.flow);
  }

  /** Lit le test et son schéma. */
  async function load(projectId: string, path: string): Promise<TestView> {
    const view = await workbench.loadTest(projectId, path);
    drafts.value[key(projectId, path)] = clone(view.flow);
    return view;
  }

  function draft(projectId: string, path: string): Flow | null {
    return drafts.value[key(projectId, path)] ?? null;
  }

  /** Modifie le brouillon ; l'onglet l'enregistre seul (`useAutosave`). */
  function edit(projectId: string, path: string, change: (flow: Flow) => void): void {
    const flow = drafts.value[key(projectId, path)];
    if (flow) change(flow);
  }

  /** Remplace une étape du brouillon. */
  function setStep(projectId: string, path: string, step: FlowStep): void {
    edit(projectId, path, (flow) => {
      const at = flow.steps.findIndex((s) => s.id === step.id);
      if (at >= 0) flow.steps[at] = step;
    });
  }

  /** Déplace l'étape `id` à la place `to` (⋮⋮, 4.3-a) : les renvois suivent l'étape (R3). */
  function move(projectId: string, path: string, id: string, to: number): void {
    edit(projectId, path, (flow) => {
      const from = flow.steps.findIndex((s) => s.id === id);
      if (from < 0 || to < 0 || to >= flow.steps.length || from === to) return;
      const [step] = flow.steps.splice(from, 1);
      if (step) flow.steps.splice(to, 0, step);
    });
  }

  /** Enregistre le brouillon (R1) ; le cœur rend le schéma vérifié. */
  async function save(projectId: string, path: string): Promise<void> {
    const flow = drafts.value[key(projectId, path)];
    if (!flow) return;
    const view = await ipc.flowSave(projectId, path, clone(flow));
    adopt(projectId, path, view);
  }

  /**
   * Ajoute une étape de nature `kind` sur un lien ou en fin de liste (4.3-e, 4.3-b bis) : le
   * brouillon est d'abord enregistré, puis l'étape est choisie, prête à configurer.
   */
  async function add(
    projectId: string,
    path: string,
    kind: StepKind,
    placement: StepPlacement,
    name: string | null = null,
  ): Promise<string> {
    await save(projectId, path);
    const added = await ipc.flowAddStep(projectId, path, kind, placement, name);
    const view = await workbench.loadTest(projectId, path);
    adopt(projectId, path, view);
    selected.value[key(projectId, path)] = added.step;
    inserted.value[key(projectId, path)] = added.step;
    return added.step;
  }

  /** « Supprimer l'étape » ou « Annuler l'insertion » : le cœur reconnecte ses liens (R3). */
  async function remove(projectId: string, path: string, id: string): Promise<void> {
    await save(projectId, path);
    const view = await ipc.flowRemoveStep(projectId, path, id);
    adopt(projectId, path, view);
    const k = key(projectId, path);
    if (selected.value[k] === id) selected.value[k] = null;
    if (inserted.value[k] === id) inserted.value[k] = null;
  }

  /**
   * « Service » puis « Opération » d'une étape requête insérée (4.3-b quater) : le cœur lui donne
   * la requête de l'opération, sans lien au contrat ; une requête ouverte du test est relue.
   */
  async function fromOperation(
    projectId: string,
    path: string,
    id: string,
    contract: string,
    operation: string,
  ): Promise<void> {
    await save(projectId, path);
    const view = await ipc.flowStepOperation(
      projectId,
      path,
      id,
      contract,
      operation,
      workbench.activeEnvironment(projectId),
    );
    adopt(projectId, path, view);
    await workbench.refreshTree(projectId);
    await workbench.reloadIn(projectId, path);
  }

  /** « Utiliser » une donnée disponible (4.3-b quater) : `{{nom}}`, à coller dans la requête. */
  async function copyVariable(name: string): Promise<void> {
    await navigator.clipboard.writeText(`{{${name}}}`);
    useToastStore().show(`{{${name}}} copiée : collez-la dans la requête.`);
  }

  /** Annule la dernière insertion (Ctrl+Z dans l'onglet du test). */
  async function undoInsert(projectId: string, path: string): Promise<boolean> {
    const id = inserted.value[key(projectId, path)];
    if (id === null || id === undefined) return false;
    await remove(projectId, path, id);
    return true;
  }

  function select(projectId: string, path: string, id: string | null): void {
    selected.value[key(projectId, path)] = id;
  }

  function mode(projectId: string, path: string): FlowMode {
    return modes.value[key(projectId, path)] ?? "list";
  }

  function setMode(projectId: string, path: string, value: FlowMode): void {
    modes.value[key(projectId, path)] = value;
  }

  // --- Avant / Après et propriétés d'un dossier (4.7-c, 4.8-a) ---------------------------

  /** Avant / Après de chaque dossier ouvert, par projet puis chemin. */
  const hooks = ref<Record<string, HooksView>>({});
  /** Propriétés de chaque dossier ouvert (4.8-a). */
  const properties = ref<Record<string, FolderProperty[]>>({});
  const folderKey = (projectId: string, path: string): string => `${projectId}:${path}`;

  async function loadHooks(projectId: string, path: string): Promise<HooksView> {
    const view = await ipc.hooksRead(projectId, path);
    hooks.value[folderKey(projectId, path)] = view;
    return view;
  }

  async function saveHooks(projectId: string, path: string, value: FolderHooks): Promise<void> {
    hooks.value[folderKey(projectId, path)] = await ipc.hooksSave(projectId, path, clone(value));
    await workbench.refreshTree(projectId);
  }

  async function addHook(
    projectId: string,
    path: string,
    phase: HookPhase,
    kind: StepKind,
  ): Promise<void> {
    hooks.value[folderKey(projectId, path)] = await ipc.hooksAddStep(projectId, path, phase, kind);
    await workbench.refreshTree(projectId);
  }

  async function removeHook(
    projectId: string,
    path: string,
    phase: HookPhase,
    id: string,
  ): Promise<void> {
    hooks.value[folderKey(projectId, path)] = await ipc.hooksRemoveStep(projectId, path, phase, id);
    await workbench.refreshTree(projectId);
  }

  async function loadProperties(projectId: string, path: string): Promise<FolderProperty[]> {
    const environment = workbench.activeEnvironment(projectId) ?? "";
    const found = await ipc.folderProperties(projectId, path, environment);
    properties.value[folderKey(projectId, path)] = found;
    return found;
  }

  /** Enregistre les propriétés définies dans le dossier (`request.variables` de `folder.yml`). */
  async function saveProperties(
    projectId: string,
    path: string,
    list: { name: string; value: string }[],
  ): Promise<void> {
    const environment = workbench.activeEnvironment(projectId) ?? "";
    properties.value[folderKey(projectId, path)] = await ipc.folderSetProperties(
      projectId,
      path,
      list,
      environment,
    );
  }

  return {
    hooks,
    properties,
    folderKey,
    loadHooks,
    saveHooks,
    addHook,
    removeHook,
    loadProperties,
    saveProperties,
    drafts,
    selected,
    inserted,
    key,
    load,
    draft,
    edit,
    setStep,
    move,
    save,
    add,
    remove,
    undoInsert,
    fromOperation,
    copyVariable,
    select,
    mode,
    setMode,
  };
});
