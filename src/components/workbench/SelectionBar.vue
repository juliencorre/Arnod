<script setup lang="ts">
/**
 * Barre de la sélection multiple (écran 4.1-a quater) : en bas de la fenêtre dès que des
 * éléments de l'arborescence sont choisis (Ctrl+clic, Maj+clic). Déplacer vers…, Dupliquer,
 * Exécuter, Supprimer ; × ou Échap vide la sélection. « Déplacer vers… » choisit le dossier dans
 * une fenêtre. Chaque action se défait par « Annuler » dans la notification.
 */
import { computed, onBeforeUnmount, onMounted, ref, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useSelectionStore } from "@/stores/selection";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { BatchAction, TreeNode } from "@/types/ipc";

const selection = useSelectionStore();
const workbench = useWorkbenchStore();
const catalog = useCatalogStore();
const toasts = useToastStore();
const folderId = useId();
const target = ref("");

const projectName = computed(() =>
  selection.projectId === null ? "" : (catalog.projectById(selection.projectId)?.name ?? ""),
);

/** Dossiers du projet où déplacer (hors des dossiers choisis eux-mêmes). */
const folders = computed(() => {
  const project = selection.projectId;
  const found: string[] = [""];
  const walk = (nodes: TreeNode[]): void => {
    for (const node of nodes) {
      if (node.kind !== "folder") continue;
      const inside = selection.paths.some((p) => node.path === p || node.path.startsWith(`${p}/`));
      if (!inside) found.push(node.path);
      walk(node.children);
    }
  };
  walk(project === null ? [] : (workbench.trees[project]?.nodes ?? []));
  return found;
});

async function act(action: BatchAction): Promise<void> {
  try {
    await selection.apply(action);
  } catch (caught) {
    toasts.showError(caught);
  }
}

function openMove(): void {
  target.value = folders.value[0] ?? "";
  selection.moving = true;
}

async function move(): Promise<void> {
  selection.moving = false;
  await act({ kind: "move", target: target.value });
}

async function run(): Promise<void> {
  try {
    await selection.run();
  } catch (caught) {
    toasts.showError(caught);
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && selection.count > 0 && !selection.moving) selection.clear();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    v-if="selection.count > 0"
    class="selection"
    role="toolbar"
    aria-label="Éléments sélectionnés"
  >
    <strong class="selection__count" aria-live="polite">
      {{ selection.count }} sélectionné{{ selection.count > 1 ? "s" : "" }}
    </strong>
    <button type="button" class="selection__action" :disabled="selection.busy" @click="openMove">
      Déplacer vers…
    </button>
    <button
      type="button"
      class="selection__action"
      :disabled="selection.busy"
      @click="act({ kind: 'duplicate' })"
    >
      Dupliquer
    </button>
    <button type="button" class="selection__action" :disabled="selection.busy" @click="run">
      Exécuter
    </button>
    <button
      type="button"
      class="selection__action selection__action--danger"
      :disabled="selection.busy"
      @click="act({ kind: 'delete', permanent: false })"
    >
      Supprimer
    </button>
    <button
      type="button"
      class="selection__close"
      aria-label="Vider la sélection (Échap)"
      @click="selection.clear()"
    >
      ×
    </button>
  </div>
  <BaseDialog
    :open="selection.moving"
    :title="`Déplacer ${String(selection.count)} élément${selection.count > 1 ? 's' : ''}`"
    :description="projectName"
    @close="selection.moving = false"
  >
    <label :for="folderId" class="selection__label">Vers le dossier</label>
    <select :id="folderId" v-model="target" class="selection__select">
      <option v-for="folder in folders" :key="folder" :value="folder">
        {{ folder === "" ? `Racine de ${projectName}` : folder.replaceAll("/", " / ") }}
      </option>
    </select>
    <template #footer>
      <BaseButton @click="selection.moving = false">Annuler</BaseButton>
      <BaseButton variant="primary" @click="move">Déplacer</BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
/* Barre sombre centrée en bas de la fenêtre, comme la maquette. */
.selection {
  position: fixed;
  bottom: 3rem;
  left: 50%;
  z-index: 15;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.375rem 0.5rem 0.375rem 1rem;
  border-radius: var(--radius-lg);
  background: var(--color-text);
  color: var(--color-bg);
  box-shadow: var(--shadow-popover);
  transform: translateX(-50%);
}

.selection__count {
  margin-right: 0.75rem;
  font-weight: 600;
}

.selection__action,
.selection__close {
  min-height: var(--control-md);
  padding: 0 0.75rem;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: inherit;
  cursor: pointer;
}

.selection__action:hover,
.selection__close:hover {
  background: color-mix(in srgb, var(--color-bg) 15%, transparent);
}

.selection__action:focus-visible,
.selection__close:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.selection__action--danger {
  color: var(--color-accent);
}

.selection__close {
  padding: 0 0.5rem;
  opacity: 0.7;
}

.selection__label {
  display: block;
  margin-bottom: 0.375rem;
  font-weight: 600;
}

.selection__select {
  width: 100%;
  min-height: var(--control-md);
}
</style>
