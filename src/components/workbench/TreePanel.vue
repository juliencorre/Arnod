<script setup lang="ts">
/**
 * Arborescence d'un projet (A1 à A8), sous son nom dans la barre latérale :
 * dossiers, requêtes et actions. Une requête choisie s'ouvre en onglet, dans
 * la fenêtre qui suit la sélection (Z5).
 */
import { computed, ref } from "vue";

import NameDialog from "@/components/workbench/NameDialog.vue";
import TreeRow from "@/components/workbench/TreeRow.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
const nodes = computed(() => workbench.trees[props.projectId]?.nodes ?? []);
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const toasts = useToastStore();

type NameAction =
  | { kind: "new-folder"; parent: string }
  | { kind: "new-request"; folder: string }
  | { kind: "rename"; node: TreeNode };

const nameAction = ref<NameAction | null>(null);
const nameError = ref<string | null>(null);

/** Tous les dossiers, pour « Déplacer vers ». */
const folders = computed(() => {
  const found: { path: string; name: string }[] = [];
  const walk = (nodes: TreeNode[]): void => {
    for (const node of nodes) {
      if (node.kind === "folder") {
        found.push({ path: node.path, name: node.name });
        walk(node.children);
      }
    }
  };
  walk(nodes.value);
  return found;
});

const dialog = computed(() => {
  const action = nameAction.value;
  if (action === null) return { title: "", label: "", initial: "", submit: "" };
  switch (action.kind) {
    case "new-folder":
      return { title: "Nouveau dossier", label: "Nom du dossier", initial: "", submit: "Créer" };
    case "new-request":
      return {
        title: "Nouvelle requête",
        label: "Nom de la requête",
        initial: "",
        submit: "Créer",
      };
    case "rename":
      return {
        title: "Renommer",
        label: "Nouveau nom",
        initial: action.node.name,
        submit: "Renommer",
      };
  }
});

function openName(action: NameAction): void {
  nameError.value = null;
  nameAction.value = action;
}

async function guard(work: () => Promise<unknown>): Promise<void> {
  try {
    await work();
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

async function onAction(id: string, node: TreeNode, target?: string): Promise<void> {
  switch (id) {
    case "new-request":
      openName({ kind: "new-request", folder: node.path });
      break;
    case "new-folder":
      openName({ kind: "new-folder", parent: node.path });
      break;
    case "rename":
      openName({ kind: "rename", node });
      break;
    case "duplicate":
      await guard(() => workbench.duplicate(props.projectId, node.path));
      break;
    case "move":
      await guard(() => workbench.move(props.projectId, node.path, target ?? ""));
      break;
    case "run":
      await guard(() => workbench.runFolder(props.projectId, node.path));
      break;
    case "delete":
      await guard(async () => {
        await workbench.remove(props.projectId, node.path, false);
        toasts.show(`« ${node.name} » mis à la corbeille.`);
      });
      break;
  }
}

async function submitName(name: string): Promise<void> {
  const action = nameAction.value;
  if (action === null) return;
  try {
    if (action.kind === "new-folder")
      await workbench.createFolder(props.projectId, action.parent, name);
    else if (action.kind === "new-request")
      await workbench.createRequest(props.projectId, action.folder, name);
    else await workbench.rename(props.projectId, action.node.path, name);
    nameAction.value = null;
  } catch (caught) {
    nameError.value = caught instanceof Error ? caught.message : String(caught);
  }
}
</script>

<template>
  <div class="tree">
    <div class="tree__head">
      <button
        type="button"
        class="tree__tool"
        @click="openName({ kind: 'new-request', folder: '' })"
      >
        + Requête
      </button>
      <button
        type="button"
        class="tree__tool"
        @click="openName({ kind: 'new-folder', parent: '' })"
      >
        + Dossier
      </button>
      <button
        type="button"
        class="tree__tool"
        :aria-label="`Tout exécuter dans ${projectName}`"
        @click="guard(() => workbench.runFolder(projectId, ''))"
      >
        ▶ Tout exécuter
      </button>
    </div>
    <ul
      v-if="nodes.length > 0"
      role="tree"
      class="tree__list"
      :aria-label="`Requêtes de ${projectName}`"
    >
      <TreeRow
        v-for="node in nodes"
        :key="node.path"
        :node="node"
        :depth="0"
        :folders="folders"
        :project-id="projectId"
        @action="onAction"
      />
    </ul>
    <p v-else class="tree__empty">Aucune requête : « + Requête » crée la première.</p>
    <NameDialog
      :open="nameAction !== null"
      :title="dialog.title"
      :label="dialog.label"
      :initial="dialog.initial"
      :submit-label="dialog.submit"
      :error="nameError"
      @close="nameAction = null"
      @submit="submitName"
    />
  </div>
</template>

<style scoped>
.tree {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.tree__head {
  display: flex;
  flex-wrap: wrap;
  gap: 0.125rem 0.625rem;
  padding: 0 0.25rem;
}

.tree__tool {
  padding: 0.125rem 0;
  border: none;
  font: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.tree__tool:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
  border-radius: 4px;
}

.tree__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.tree__empty {
  margin: 0.25rem 0.25rem 0.5rem;
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
