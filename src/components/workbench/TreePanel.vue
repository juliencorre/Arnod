<script setup lang="ts">
/** Arborescence du projet (A1 à A8) : dossiers, requêtes et actions. */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import NameDialog from "@/components/workbench/NameDialog.vue";
import TreeRow from "@/components/workbench/TreeRow.vue";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const workbench = useWorkbenchStore();
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
  walk(workbench.nodes);
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
      await guard(() => workbench.duplicate(node.path));
      break;
    case "move":
      await guard(() => workbench.move(node.path, target ?? ""));
      break;
    case "run":
      await guard(() => workbench.runFolder(node.path));
      break;
    case "delete":
      await guard(async () => {
        await workbench.remove(node.path, false);
        toasts.show(`« ${node.name} » mis à la corbeille.`);
      });
      break;
  }
}

async function submitName(name: string): Promise<void> {
  const action = nameAction.value;
  if (action === null) return;
  try {
    if (action.kind === "new-folder") await workbench.createFolder(action.parent, name);
    else if (action.kind === "new-request") await workbench.createRequest(action.folder, name);
    else await workbench.rename(action.node.path, name);
    nameAction.value = null;
  } catch (caught) {
    nameError.value = caught instanceof Error ? caught.message : String(caught);
  }
}
</script>

<template>
  <nav class="tree" aria-label="Requêtes du projet">
    <div class="tree__head">
      <BaseButton @click="openName({ kind: 'new-request', folder: '' })">+ Requête</BaseButton>
      <BaseButton @click="openName({ kind: 'new-folder', parent: '' })">+ Dossier</BaseButton>
      <BaseButton variant="link" @click="workbench.runFolder('')">Tout exécuter</BaseButton>
    </div>
    <ul v-if="workbench.nodes.length > 0" role="tree" class="tree__list">
      <TreeRow
        v-for="node in workbench.nodes"
        :key="node.path"
        :node="node"
        :depth="0"
        :folders="folders"
        @action="onAction"
      />
    </ul>
    <p v-else class="tree__empty">Aucune requête. Créez la première avec « + Requête ».</p>
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
  </nav>
</template>

<style scoped>
.tree {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.tree__head {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
}

.tree__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.tree__empty {
  margin: 0.5rem 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}
</style>
