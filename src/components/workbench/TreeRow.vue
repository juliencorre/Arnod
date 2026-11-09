<script setup lang="ts">
/** Ligne de l'arborescence : dossier (repliable) ou requête, avec son menu d'actions. */
import { computed } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{
  node: TreeNode;
  depth: number;
  folders: { path: string; name: string }[];
  projectId: string;
}>();
const emit = defineEmits<{ action: [id: string, node: TreeNode, target?: string] }>();

const workbench = useWorkbenchStore();
const windows = useWindowsStore();

const isOpen = computed(
  () => !(workbench.trees[props.projectId]?.collapsed ?? []).includes(props.node.path),
);
/** Requête affichée dans l'onglet actif de cette fenêtre. */
const isSelected = computed(() => {
  const content = windows.activeTab?.content;
  return (
    content?.kind === "request" &&
    content.projectId === props.projectId &&
    content.path === props.node.path
  );
});
const dirty = computed(
  () =>
    props.node.kind === "request" && workbench.isDirty(`${props.projectId}::${props.node.path}`),
);

const items = computed<MenuItem[]>(() => {
  const move: MenuItem = {
    id: "move",
    label: "Déplacer vers",
    separatorBefore: true,
    children: [
      { id: "move:", label: "Racine du projet" },
      ...props.folders
        .filter((f) => f.path !== props.node.path && !f.path.startsWith(`${props.node.path}/`))
        .map((f) => ({ id: `move:${f.path}`, label: f.path })),
    ],
  };
  const remove: MenuItem = {
    id: "delete",
    label: "Mettre à la corbeille",
    tone: "danger",
    separatorBefore: true,
  };
  if (props.node.kind === "folder") {
    return [
      { id: "new-request", label: "Nouvelle requête" },
      { id: "new-folder", label: "Nouveau dossier" },
      { id: "run", label: "Exécuter le dossier", separatorBefore: true },
      { id: "rename", label: "Renommer", separatorBefore: true },
      move,
      remove,
    ];
  }
  return [
    { id: "rename", label: "Renommer" },
    { id: "duplicate", label: "Dupliquer" },
    move,
    remove,
  ];
});

function onSelect(id: string): void {
  if (id.startsWith("move:")) emit("action", "move", props.node, id.slice(5));
  else emit("action", id, props.node);
}

function activate(): void {
  if (props.node.kind === "folder") void workbench.toggleFolder(props.projectId, props.node.path);
  else void workbench.openRequest(props.projectId, props.node.path);
}
</script>

<template>
  <li role="none">
    <div
      class="row"
      :class="{ 'row--selected': isSelected }"
      :style="{ paddingLeft: `${0.5 + depth * 1}rem` }"
    >
      <button
        type="button"
        class="row__main"
        role="treeitem"
        :aria-expanded="node.kind === 'folder' ? isOpen : undefined"
        :aria-selected="isSelected"
        :aria-level="depth + 1"
        @click="activate"
      >
        <template v-if="node.kind === 'folder'">
          <span class="row__chevron" aria-hidden="true">{{ isOpen ? "▾" : "▸" }}</span>
          <span class="row__name">{{ node.name }}</span>
          <span class="row__count">{{ node.requestCount }}</span>
        </template>
        <template v-else>
          <span class="row__method" :class="`row__method--${node.method.toLowerCase()}`">
            {{ node.method }}
          </span>
          <span class="row__name">{{ node.name }}</span>
          <span v-if="dirty" class="row__dirty" title="Modifiée, non enregistrée">●</span>
          <span v-if="node.invalid" class="row__invalid" title="Fichier illisible">⚠</span>
        </template>
      </button>
      <BaseMenu :items="items" :label="`Actions pour ${node.name}`" align="end" @select="onSelect">
        <template #trigger="{ attrs }">
          <button
            type="button"
            class="row__more"
            v-bind="attrs"
            :aria-label="`Actions pour ${node.name}`"
          >
            ⋯
          </button>
        </template>
      </BaseMenu>
    </div>
    <ul v-if="node.kind === 'folder' && isOpen" role="group" class="children">
      <TreeRow
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :folders="folders"
        :project-id="projectId"
        @action="(id, n, target) => emit('action', id, n, target)"
      />
    </ul>
  </li>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  border-radius: 6px;
}

.row:hover,
.row--selected {
  background: var(--color-accent-soft);
}

.row__main {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 0.375rem;
  min-width: 0;
  padding: 0.3125rem 0.25rem;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.row__main:focus-visible,
.row__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: 6px;
}

.row__chevron {
  width: 0.75rem;
  color: var(--color-muted);
}

.row__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__count {
  margin-left: auto;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.row__method {
  min-width: 2.5rem;
  font: 700 0.6875rem var(--font-mono);
  color: var(--color-muted);
}

.row__method--get {
  color: var(--color-success);
}

.row__method--post,
.row__method--put,
.row__method--patch {
  color: var(--color-link);
}

.row__method--delete {
  color: var(--color-danger);
}

.row__dirty {
  margin-left: auto;
  font-size: 0.625rem;
  color: var(--color-accent);
}

.row__invalid {
  color: var(--color-danger);
}

.row__more {
  border: 0;
  background: none;
  padding: 0.125rem 0.5rem;
  color: var(--color-muted);
  cursor: pointer;
}

.children {
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
