<script setup lang="ts">
/**
 * Ligne de l'arborescence : dossier (repliable) ou requête, avec son menu d'actions. Toutes
 * les lignes suivent la grille de la barre latérale (`--tree-*`, défini par `AppSidebar`) :
 * une colonne de tête (chevron et dossier, ou méthode), puis le nom ; chaque profondeur
 * décale de la largeur du chevron, si bien qu'un enfant s'aligne sur l'icône de son dossier.
 */
import { computed } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import StatusCount from "@/components/workbench/StatusCount.vue";
import StatusMark from "@/components/workbench/StatusMark.vue";
import { SECTION_ICONS } from "@/components/ui/sectionIcons";
import { countStatuses, requestPaths } from "@/lib/status";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{
  node: TreeNode;
  depth: number;
  folders: { path: string; name: string }[];
  projectId: string;
  /** Numéro de l'étape, pour une requête d'un test (lot E, R1). */
  step?: number;
}>();

/** Fiole d'un test (lot E) : l'icône de la section « Référentiel de tests ». */
const FLASK = SECTION_ICONS.tests;
const isTest = computed(() => props.node.kind === "folder" && props.node.test);
const emit = defineEmits<{ action: [id: string, node: TreeNode, target?: string] }>();

const workbench = useWorkbenchStore();
const windows = useWindowsStore();

const isOpen = computed(
  () => !(workbench.trees[props.projectId]?.collapsed ?? []).includes(props.node.path),
);
/** Requête (ou test) affiché dans l'onglet actif de cette fenêtre. */
const isSelected = computed(() => {
  const content = windows.activeTab?.content;
  return (
    (content?.kind === "request" || content?.kind === "test") &&
    content.projectId === props.projectId &&
    content.path === props.node.path
  );
});
/** Dernière exécution de la requête, ou des requêtes du dossier (ST1, ST2). */
const status = computed(() =>
  props.node.kind === "request"
    ? (workbench.statuses[props.projectId]?.[props.node.path] ?? null)
    : null,
);
const folderCount = computed(() =>
  countStatuses(requestPaths(props.node), workbench.statuses[props.projectId]),
);
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
  if (isTest.value) {
    return [
      { id: "add-step", label: "Ajouter une étape" },
      { id: "run", label: "Exécuter le test", separatorBefore: true },
      { id: "rename", label: "Renommer", separatorBefore: true },
      move,
      remove,
    ];
  }
  if (props.node.kind === "folder") {
    return [
      { id: "new-test", label: "Nouveau test" },
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
  // Un test s'ouvre dans son onglet ; son chevron le replie (lot E, TE1).
  if (isTest.value) void workbench.openTest(props.projectId, props.node.path);
  else if (props.node.kind === "folder")
    void workbench.toggleFolder(props.projectId, props.node.path);
  else void workbench.openRequest(props.projectId, props.node.path);
}
</script>

<template>
  <li role="presentation">
    <div class="row" :class="{ 'row--selected': isSelected }" :style="{ '--depth': depth + 1 }">
      <button
        type="button"
        class="row__main"
        role="treeitem"
        :aria-expanded="node.kind === 'folder' ? isOpen : undefined"
        :aria-selected="isSelected"
        :aria-level="depth + 1"
        @click="activate"
        @keydown.right="isTest && !isOpen && workbench.toggleFolder(projectId, node.path)"
        @keydown.left="isTest && isOpen && workbench.toggleFolder(projectId, node.path)"
      >
        <template v-if="node.kind === 'folder'">
          <span class="row__lead" aria-hidden="true">
            <span
              class="row__chevron"
              @click.stop="isTest && workbench.toggleFolder(projectId, node.path)"
              >{{ isOpen ? "▾" : "▸" }}</span
            >
            <svg v-if="isTest" class="row__flask" width="14" height="14" viewBox="0 0 16 16">
              <path v-for="(d, index) in FLASK" :key="index" :d="d" />
            </svg>
            <svg v-else class="row__folder" width="14" height="14" viewBox="0 0 14 14">
              <path d="M1.5 3.5h4l1.2 1.4h5.8v6.6h-11z" />
            </svg>
          </span>
          <span class="row__name">{{ node.name }}</span>
          <StatusCount class="row__count" :count="folderCount" :fallback="node.requestCount" />
        </template>
        <template v-else>
          <span class="row__lead row__method" :class="`row__method--${node.method.toLowerCase()}`">
            {{ node.method }}
          </span>
          <span v-if="step" class="row__step" :aria-label="`Étape ${step}`">{{ step }}</span>
          <span class="row__name">{{ node.name }}</span>
          <span class="row__end">
            <span v-if="dirty" class="row__dirty" title="Modifiée, non enregistrée">●</span>
            <span v-if="node.invalid" class="row__invalid" title="Fichier illisible">⚠</span>
            <StatusMark v-if="status" class="row__status" :status="status" />
          </span>
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
        v-for="(child, index) in node.children"
        :key="child.path"
        :node="child"
        :step="node.test && child.kind === 'request' ? index + 1 : undefined"
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
  padding-left: calc(var(--tree-pad) + var(--tree-indent) * var(--depth));
  border-radius: var(--radius-md);
}

.row:hover,
.row--selected {
  background: var(--color-accent-soft);
}

.row__main {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--tree-gap);
  min-width: 0;
  min-height: 1.75rem;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.row__main:focus-visible,
.row__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: var(--radius-md);
}

/* Colonne de tête : chevron et dossier, ou méthode. */
.row__lead {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.375rem;
  width: var(--tree-lead);
}

.row__chevron {
  width: 0.875rem;
  font-size: var(--text-small);
  text-align: center;
  color: var(--color-muted);
}

.row__folder {
  flex: none;
}

.row__folder path {
  fill: var(--color-chip);
  stroke: var(--color-subtle);
  stroke-width: 1;
}

.row__flask {
  flex: none;
  fill: none;
  stroke: var(--color-subtle);
  stroke-width: 1.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.row__step {
  flex: none;
  min-width: 0.75rem;
  font: var(--text-caption) var(--font-mono);
  text-align: right;
  color: var(--color-muted);
}

.row__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__count {
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.row__method {
  font: 700 var(--text-caption) var(--font-mono);
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

.row__end {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.25rem;
  margin-left: auto;
  padding-right: 0.25rem;
}

.row__dirty {
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
