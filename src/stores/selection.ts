import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { BatchAction } from "@/types/ipc";

/**
 * Sélection multiple de l'arborescence (écran 4.1-a quater) : Ctrl+clic ajoute ou retire un
 * élément, Maj+clic prend la plage depuis le dernier choisi. La barre de sélection déplace,
 * duplique, exécute ou met à la corbeille d'un coup ; le cœur fait tout (`tree_batch`) et rend un
 * jeton pour « Annuler ». Propre à la fenêtre ; une sélection ne vaut que dans un projet.
 */
export const useSelectionStore = defineStore("selection", () => {
  const projectId = ref<string | null>(null);
  const paths = ref<string[]>([]);
  /** Dernier élément choisi, point de départ d'une plage (Maj+clic). */
  const anchor = ref<string | null>(null);
  const busy = ref(false);
  /** Fenêtre « Déplacer vers… » ouverte. */
  const moving = ref(false);

  const count = computed(() => paths.value.length);

  function has(project: string, path: string): boolean {
    return projectId.value === project && paths.value.includes(path);
  }

  function clear(): void {
    projectId.value = null;
    paths.value = [];
    anchor.value = null;
    moving.value = false;
  }

  /** Ctrl+clic : ajoute ou retire `path` ; `current` (élément ouvert) entre avec le premier. */
  function toggle(project: string, path: string, current?: string | null): void {
    if (projectId.value !== project) {
      clear();
      projectId.value = project;
      if (current && current !== path) paths.value = [current];
    }
    paths.value = paths.value.includes(path)
      ? paths.value.filter((p) => p !== path)
      : [...paths.value, path];
    anchor.value = path;
    if (paths.value.length === 0) clear();
  }

  /** Maj+clic : la plage de `order` (ordre affiché) entre le dernier choisi et `path`. */
  function range(project: string, path: string, order: string[], current?: string | null): void {
    const from = projectId.value === project ? (anchor.value ?? current) : current;
    const start = from ? order.indexOf(from) : -1;
    const end = order.indexOf(path);
    if (start < 0 || end < 0) {
      toggle(project, path, current);
      return;
    }
    const [low, high] = start < end ? [start, end] : [end, start];
    projectId.value = project;
    paths.value = order.slice(low, high + 1);
    anchor.value = from ?? path;
  }

  /** Relit l'arborescence et les contrats du projet. */
  async function refresh(project: string): Promise<void> {
    await useContractsStore().sync(project);
  }

  const DONE: Record<BatchAction["kind"], [string, string]> = {
    move: ["déplacé", "déplacés"],
    duplicate: ["dupliqué", "dupliqués"],
    delete: ["mis à la corbeille", "mis à la corbeille"],
  };

  /** Applique `action` à la sélection, avec « Annuler » dans la notification. */
  async function apply(action: BatchAction): Promise<void> {
    const project = projectId.value;
    if (project === null || paths.value.length === 0 || busy.value) return;
    const workbench = useWorkbenchStore();
    const toasts = useToastStore();
    busy.value = true;
    try {
      await workbench.saveAll();
      const result = await ipc.treeBatch(project, paths.value, action);
      for (const [from, to] of result.items) {
        if (action.kind === "move" && to !== null) workbench.followTransfer(project, from, to);
        if (to === null) workbench.followTransfer(project, from, null);
      }
      const n = result.items.length;
      clear();
      await refresh(project);
      const [one, many] = DONE[action.kind];
      toasts.show(`${String(n)} élément${n > 1 ? "s" : ""} ${n > 1 ? many : one}.`, {
        action: {
          label: "Annuler",
          run: () => {
            void ipc
              .treeBatchUndo(result.undoToken)
              .then(() => refresh(project))
              .catch((caught: unknown) => {
                toasts.showError(caught);
              });
          },
        },
      });
    } finally {
      busy.value = false;
    }
  }

  /** Exécute la sélection dans l'environnement actif du projet. */
  async function run(): Promise<void> {
    const project = projectId.value;
    if (project === null || paths.value.length === 0) return;
    const chosen = [...paths.value];
    clear();
    await useWorkbenchStore().runSelection(project, chosen);
  }

  return {
    projectId,
    paths,
    anchor,
    busy,
    moving,
    count,
    has,
    clear,
    toggle,
    range,
    apply,
    run,
  };
});
