<script setup lang="ts">
/**
 * Référentiel de tests d'un projet (A1 à A8, RC6 de docs/plan-requetes-dans-contrats.md), sous
 * ses contrats dans la barre latérale : dossiers, requêtes sans contrat et actions ; les requêtes
 * liées à un contrat sont rangées sous ce contrat (R1, R5). Une requête choisie s'ouvre en
 * onglet, dans la fenêtre qui suit la sélection (Z5). Les créations partent du « + » de la ligne
 * du projet ou de l'en-tête de la section (`newRequest`, `newFolder`) ; un référentiel vide montre
 * la ligne « Nouvelle requête » (plan de la barre latérale du projet, B4).
 */
import { computed, ref } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import SectionHeading from "@/components/ui/SectionHeading.vue";
import NameDialog from "@/components/workbench/NameDialog.vue";
import TreeRow from "@/components/workbench/TreeRow.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
/** Arborescence sans les requêtes liées à un contrat, calculée par le cœur (R5). */
const nodes = computed(() => workbench.trees[props.projectId]?.tests ?? []);
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const toasts = useToastStore();

type NameAction =
  | { kind: "new-folder"; parent: string }
  | { kind: "new-request"; folder: string }
  | { kind: "rename"; node: TreeNode };

const nameAction = ref<NameAction | null>(null);
const nameError = ref<string | null>(null);

/** Nombre de requêtes du référentiel, pour l'en-tête de la section (I1). */
const requestCount = computed(() => {
  const count = (list: TreeNode[]): number =>
    list.reduce((sum, node) => sum + (node.kind === "folder" ? count(node.children) : 1), 0);
  return count(nodes.value);
});

const ADD_ITEMS: MenuItem[] = [
  { id: "request", label: "Nouvelle requête", shortcut: "Ctrl+N" },
  { id: "folder", label: "Nouveau dossier" },
];

function onAdd(id: string): void {
  if (id === "request") newRequest();
  else newFolder();
}

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

/** Nouvelle requête ou nouveau dossier à la racine du projet (menu « + » du projet). */
function newRequest(): void {
  openName({ kind: "new-request", folder: "" });
}

function newFolder(): void {
  openName({ kind: "new-folder", parent: "" });
}

defineExpose({ newRequest, newFolder });

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
    <SectionHeading icon="tests" title="Référentiel de tests" :count="requestCount">
      <BaseMenu
        :items="ADD_ITEMS"
        :label="`Ajouter au référentiel de tests de ${projectName}`"
        align="end"
        @select="onAdd"
      >
        <template #trigger="{ attrs }">
          <button
            type="button"
            class="tree__tool"
            v-bind="attrs"
            :aria-label="`Ajouter au référentiel de tests de ${projectName}`"
            :title="`Ajouter au référentiel de tests de ${projectName}`"
          >
            <svg width="14" height="14" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M6 2.5v7M2.5 6h7" />
            </svg>
          </button>
        </template>
      </BaseMenu>
    </SectionHeading>
    <ul
      v-if="nodes.length > 0"
      role="tree"
      class="tree__list"
      :aria-label="`Référentiel de tests de ${projectName}`"
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
    <button v-else type="button" class="tree__ghost" @click="newRequest">
      <span class="tree__lead" aria-hidden="true">+</span>
      Nouvelle requête
    </button>
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
  min-width: 0;
}

.tree__tool {
  display: grid;
  place-items: center;
  width: 1.625rem;
  height: 1.625rem;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: none;
  cursor: pointer;
}

.tree__tool path {
  stroke: var(--color-text);
  stroke-width: 1.6;
  stroke-linecap: round;
}

.tree__tool:hover,
.tree__tool[aria-expanded="true"] {
  background: var(--color-chip);
}

.tree__tool:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.tree__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

/* Ligne d'ajout, sur la grille des lignes de l'arborescence. */
.tree__ghost {
  display: flex;
  align-items: center;
  gap: var(--tree-gap);
  width: 100%;
  min-height: 1.75rem;
  padding: 0 var(--tree-pad) 0 calc(var(--tree-pad) + var(--tree-indent));
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: left;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.tree__ghost:hover {
  background: var(--color-accent-soft);
}

.tree__ghost:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.tree__lead {
  box-sizing: border-box;
  flex: none;
  width: var(--tree-lead);
  padding-left: 0.125rem;
}
</style>
