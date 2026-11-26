<script setup lang="ts">
/**
 * Ligne de l'arborescence : dossier (repliable) ou requête, avec son menu d'actions. Toutes
 * les lignes suivent la grille de la barre latérale (`--tree-*`, défini par `AppSidebar`) :
 * une colonne de tête (chevron et dossier, ou méthode), puis le nom ; chaque profondeur
 * décale de la largeur du chevron, si bien qu'un enfant s'aligne sur l'icône de son dossier.
 * Pendant un filtre (docs/plan-mvp-d-barre-laterale.md, F2, F3), les dossiers sont dépliés sans
 * changer leur état enregistré, la partie trouvée du nom est surlignée et une requête trouvée
 * par son adresse le dit.
 *
 * Lot H (AC1) : une ligne n'est un arrêt de tabulation que si l'arbre la désigne
 * (`TREE_TAB_STOP`) ; ⋯ n'en est pas un (Maj+F10 ou la touche Menu l'ouvrent). Droite déplie ou
 * va au premier enfant, Gauche replie ou remonte au parent, F2 renomme, Suppr met à la corbeille.
 * DE5 : un nom coupé a son infobulle.
 *
 * Lot E de l'alignement (AM36, AM39, 4.1-a, 5.2-d) : le menu ⋯ d'un dossier suit la maquette
 * (« Dans le dossier « … » », raccourcis, « Dupliquer un test… », « Déplacer vers… ») ; au
 * survol d'un dossier, ▶ lance son run et « + » y crée un test, une requête ou un sous-dossier.
 */
import { computed, inject, ref, useTemplateRef } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import StatusCount from "@/components/workbench/StatusCount.vue";
import StatusMark from "@/components/workbench/StatusMark.vue";
import { SECTION_ICONS } from "@/components/ui/sectionIcons";
import { countStatuses, requestPaths } from "@/lib/status";
import { highlight, matchedByUrl, type TreeFilter } from "@/lib/treeFilter";
import { REQUEST_DRAG, dragged, startRequestDrag } from "@/lib/requestDrag";
import { firstChildItem, parentItem, TREE_TAB_STOP } from "@/lib/treeKeyboard";
import { vTitleIfCut } from "@/lib/truncate";
import { useToastStore } from "@/stores/toasts";
import { useSelectionStore } from "@/stores/selection";
import { useTransferStore } from "@/stores/transfer";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { FolderNode } from "@/lib/tree";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{
  node: TreeNode;
  depth: number;
  folders: { path: string; name: string }[];
  projectId: string;
  /** Numéro de l'étape, pour une requête d'un test (lot E, R1). */
  step?: number;
  /** Filtre actif du référentiel (lot D), ou `null`. */
  filter?: TreeFilter | null;
}>();

/** Fiole d'un test (lot E) : l'icône de la section « Référentiel de tests ». */
const FLASK = SECTION_ICONS.tests;
const isTest = computed(() => props.node.kind === "folder" && props.node.test);
const emit = defineEmits<{ action: [id: string, node: TreeNode, target?: string] }>();

const workbench = useWorkbenchStore();
const windows = useWindowsStore();
const tabStop = inject(TREE_TAB_STOP, null);
/** Seul arrêt de tabulation de l'arbre (hors arbre, chaque ligne en est un). */
const isTabStop = computed(() => tabStop === null || tabStop.value === props.node.path);
const menu = useTemplateRef<InstanceType<typeof BaseMenu>>("menu");

const isOpen = computed(
  () =>
    (props.filter ?? null) !== null ||
    !(workbench.trees[props.projectId]?.collapsed ?? []).includes(props.node.path),
);
/** Nom découpé pour le surlignage du filtre (F2). */
const nameParts = computed(() => highlight(props.node.name, props.filter ?? null));
/** Requête retenue par son adresse seulement (F2). */
const byUrl = computed(
  () =>
    props.node.kind === "request" &&
    (props.filter ?? null) !== null &&
    matchedByUrl(props.node, props.filter as TreeFilter),
);
/** Requête, test ou dossier affiché dans l'onglet actif de cette fenêtre. */
const isSelected = computed(() => {
  const content = windows.activeTab?.content;
  return (
    (content?.kind === "request" || content?.kind === "test" || content?.kind === "folder") &&
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
      // Une requête nommée s'ajoute aussi, comme dans un dossier (R4 : libellés gardés).
      { id: "new-request", label: "Nouvelle requête" },
      { id: "run", label: "Exécuter le test", separatorBefore: true },
      { id: "rename", label: "Renommer", separatorBefore: true },
      move,
      remove,
    ];
  }
  if (props.node.kind === "folder") {
    // AM36 (4.1-a) : les entrées de la maquette, avec leurs raccourcis.
    const tests = props.node.children.filter(
      (child): child is FolderNode => child.kind === "folder" && child.test,
    );
    return [
      { id: "new-test", label: "Nouveau test", shortcut: "Ctrl+N" },
      { id: "new-folder", label: "Nouveau dossier", shortcut: "Ctrl+Maj+N" },
      {
        id: "duplicate-test",
        label: "Dupliquer un test…",
        disabled: tests.length === 0,
        children: tests.map((test) => ({ id: `duplicate-test:${test.path}`, label: test.name })),
      },
      { id: "rename", label: "Renommer", shortcut: "F2", separatorBefore: true },
      { id: "move-to", label: "Déplacer vers…" },
      { id: "run", label: "Exécuter les tests du dossier", separatorBefore: true },
      { id: "delete", label: "Supprimer le dossier", tone: "danger", separatorBefore: true },
    ];
  }
  return [
    { id: "rename", label: "Renommer" },
    { id: "duplicate", label: "Dupliquer" },
    // Lot J (4.1-g) : vers un dossier, un test, une opération ou un autre projet.
    { id: "copy-move", label: "Copier ou déplacer…" },
    move,
    remove,
  ];
});

function onSelect(id: string): void {
  if (id.startsWith("move:")) emit("action", "move", props.node, id.slice(5));
  else if (id.startsWith("duplicate-test:"))
    emit("action", "duplicate-test", props.node, id.slice(15));
  else emit("action", id, props.node);
}

/** « + » au survol d'un dossier (AM36) : ce qui se crée dedans. */
const ADD_ITEMS: MenuItem[] = [
  { id: "new-test", label: "Nouveau test", shortcut: "Ctrl+N" },
  { id: "new-request", label: "Requête libre" },
  { id: "new-folder", label: "Nouveau dossier", shortcut: "Ctrl+Maj+N" },
];

// --- Lot J : copier, couper, coller, annuler et glisser ----------------------------

const transfer = useTransferStore();
const toasts = useToastStore();

function parentOf(path: string): string {
  const index = path.lastIndexOf("/");
  return index < 0 ? "" : path.slice(0, index);
}

/** Colle ici : dans le dossier ou le test, ou après cette requête dans son dossier. */
async function pasteHere(mode?: "copy" | "move"): Promise<void> {
  const node = props.node;
  try {
    if (node.kind === "folder") {
      await transfer.paste(props.projectId, node.path, node.test, mode);
    } else if (props.step !== undefined) {
      await transfer.paste(props.projectId, parentOf(node.path), true, mode, props.step);
    } else {
      await transfer.paste(props.projectId, parentOf(node.path), false, mode);
    }
  } catch (caught) {
    toasts.showError(caught);
  }
}

/**
 * Touches de la ligne (AC1) : Droite, Gauche, F2, Suppr, Maj+F10 et la touche Menu ; puis
 * Ctrl+C, Ctrl+X, Ctrl+V et Ctrl+Z (code de la touche, pour l'AZERTY).
 */
function onKeydown(event: KeyboardEvent): void {
  if (onTreeKey(event)) {
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  onClipboardKey(event);
}

/** Touches de l'arbre propres à la ligne ; `true` si la touche est prise. */
function onTreeKey(event: KeyboardEvent): boolean {
  // AM36 : sur un dossier, Ctrl+N et Ctrl+Maj+N créent dedans (code de la touche, pour l'AZERTY).
  if (
    (event.ctrlKey || event.metaKey) &&
    !event.altKey &&
    event.code === "KeyN" &&
    props.node.kind === "folder" &&
    !isTest.value
  ) {
    emit("action", event.shiftKey ? "new-folder" : "new-test", props.node);
    return true;
  }
  if (event.altKey || event.ctrlKey || event.metaKey) return false;
  const item = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
  if (!item) return false;
  const folder = props.node.kind === "folder";
  // Pendant un filtre, les dossiers restent dépliés (F3) : Droite et Gauche ne font que naviguer.
  const foldable = folder && (props.filter ?? null) === null;
  if ((event.shiftKey && event.key === "F10") || event.key === "ContextMenu") {
    menu.value?.open();
    return true;
  }
  if (event.shiftKey) return false;
  switch (event.key) {
    case "ArrowRight":
      if (foldable && !isOpen.value) void workbench.toggleFolder(props.projectId, props.node.path);
      else if (folder) firstChildItem(item)?.focus();
      return true;
    case "ArrowLeft":
      if (foldable && isOpen.value) void workbench.toggleFolder(props.projectId, props.node.path);
      else parentItem(item)?.focus();
      return true;
    case "F2":
      emit("action", "rename", props.node);
      return true;
    case "Delete":
      emit("action", "delete", props.node);
      return true;
    default:
      return false;
  }
}

function onClipboardKey(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
  const node = props.node;
  const source = { projectId: props.projectId, path: node.path, name: node.name };
  if ((event.code === "KeyC" || event.code === "KeyX") && node.kind === "request") {
    transfer.copy(source, event.code === "KeyC" ? "copy" : "move");
  } else if (event.code === "KeyV" && transfer.clipboard) {
    void pasteHere();
  } else if (event.code === "KeyZ" && transfer.last) {
    void transfer.undo().catch(toasts.showError);
  } else return;
  event.preventDefault();
  event.stopPropagation();
}

function onDragStart(event: DragEvent): void {
  if (props.node.kind !== "request") return;
  startRequestDrag(event, {
    projectId: props.projectId,
    path: props.node.path,
    name: props.node.name,
  });
}

/** Une requête glissée se dépose sur un dossier, un test, ou une étape (après elle). */
const dropTarget = computed(() => props.node.kind === "folder" || props.step !== undefined);
const dropping = ref(false);

function onDragOver(event: DragEvent): void {
  if (!dropTarget.value || !event.dataTransfer?.types.includes(REQUEST_DRAG)) return;
  event.preventDefault();
  event.dataTransfer.dropEffect = event.ctrlKey ? "copy" : "move";
  dropping.value = true;
}

async function onDrop(event: DragEvent): Promise<void> {
  dropping.value = false;
  const source = dragged(event);
  if (!dropTarget.value || !source || source.path === props.node.path) return;
  event.preventDefault();
  const mode = event.ctrlKey ? "copy" : "move";
  transfer.clipboard = { ...source, mode };
  await pasteHere(mode);
}

/**
 * Simple clic : la requête ou le test s'ouvre en aperçu ; double-clic : l'onglet reste
 * (lot F, 1.3-a).
 */
// --- Sélection multiple (4.1-a quater) --------------------------------------------

const selection = useSelectionStore();
const picked = computed(() => selection.has(props.projectId, props.node.path));

/** Élément ouvert dans l'onglet actif, qui entre dans la sélection avec le premier Ctrl+clic. */
function currentPath(): string | null {
  const content = windows.activeTab?.content;
  return (content?.kind === "request" || content?.kind === "test") &&
    content.projectId === props.projectId
    ? content.path
    : null;
}

/** Ordre affiché des éléments de l'arborescence, pour une plage (Maj+clic). */
function shownOrder(element: Element): string[] {
  const tree = element.closest('[role="tree"]');
  return Array.from(tree?.querySelectorAll<HTMLElement>("[data-path]") ?? []).map(
    (item) => item.dataset.path ?? "",
  );
}

/** Ctrl+clic ajoute ou retire, Maj+clic prend la plage ; un clic simple vide la sélection. */
function onClick(event: MouseEvent): void {
  if (event.ctrlKey || event.metaKey) {
    selection.toggle(props.projectId, props.node.path, currentPath());
    return;
  }
  if (event.shiftKey && event.currentTarget instanceof Element) {
    selection.range(
      props.projectId,
      props.node.path,
      shownOrder(event.currentTarget),
      currentPath(),
    );
    return;
  }
  if (selection.count > 0) selection.clear();
  activate();
}

/** Au clavier : Ctrl+Espace ajoute ou retire, Maj+Espace prend la plage. */
function onSpace(event: KeyboardEvent): void {
  if (event.ctrlKey || event.metaKey) {
    event.preventDefault();
    selection.toggle(props.projectId, props.node.path, currentPath());
  } else if (event.shiftKey && event.currentTarget instanceof Element) {
    event.preventDefault();
    selection.range(
      props.projectId,
      props.node.path,
      shownOrder(event.currentTarget),
      currentPath(),
    );
  }
}

function activate(preview = true): void {
  // Un test s'ouvre dans son onglet ; un dossier, dans sa vue (AM37, Q4) ; leur chevron plie.
  if (isTest.value) void workbench.openTest(props.projectId, props.node.path, preview);
  else if (props.node.kind === "folder")
    void workbench.openFolder(props.projectId, props.node.path, preview);
  else void workbench.openRequest(props.projectId, props.node.path, preview);
}

/** Le chevron plie ou déplie, sauf pendant un filtre, qui garde les dossiers dépliés (F3). */
function toggleFromChevron(): void {
  if ((props.filter ?? null) === null)
    void workbench.toggleFolder(props.projectId, props.node.path);
}
</script>

<template>
  <li role="presentation">
    <div
      class="row"
      :class="{ 'row--selected': isSelected, 'row--drop': dropping, 'row--picked': picked }"
      :style="{ '--depth': depth + 1 }"
      @dragover="onDragOver"
      @dragleave="dropping = false"
      @drop="onDrop"
    >
      <button
        type="button"
        class="row__main"
        role="treeitem"
        :aria-expanded="node.kind === 'folder' ? isOpen : undefined"
        :aria-selected="isSelected || picked"
        :aria-level="depth + 1"
        :tabindex="isTabStop ? 0 : -1"
        :data-path="node.path"
        :draggable="node.kind === 'request'"
        @dragstart="onDragStart"
        @keydown="onKeydown"
        @click="onClick"
        @keydown.space="onSpace"
        @dblclick="activate(false)"
      >
        <template v-if="node.kind === 'folder'">
          <span class="row__lead" aria-hidden="true">
            <span class="row__chevron" @click.stop="toggleFromChevron">{{
              isOpen ? "▾" : "▸"
            }}</span>
            <svg v-if="isTest" class="row__flask" width="14" height="14" viewBox="0 0 16 16">
              <path v-for="(d, index) in FLASK" :key="index" :d="d" />
            </svg>
            <svg v-else class="row__folder" width="14" height="14" viewBox="0 0 14 14">
              <path d="M1.5 3.5h4l1.2 1.4h5.8v6.6h-11z" />
            </svg>
          </span>
          <span v-title-if-cut class="row__name">{{ node.name }}</span>
          <StatusCount class="row__count" :count="folderCount" :fallback="node.requestCount" />
        </template>
        <template v-else>
          <span
            class="row__lead row__method"
            :class="node.method && `row__method--${node.method.toLowerCase()}`"
          >
            {{ node.method }}
          </span>
          <span v-if="step" class="row__step" :aria-label="`Étape ${step}`">{{ step }}</span>
          <span v-title-if-cut class="row__name"
            ><template v-for="(part, index) in nameParts" :key="index"
              ><mark v-if="part.hit" class="row__hit">{{ part.text }}</mark
              ><template v-else>{{ part.text }}</template></template
            ></span
          >
          <span v-if="byUrl" class="row__via">adresse</span>
          <span class="row__end">
            <span v-if="dirty" class="row__dirty" title="Modifiée, non enregistrée">●</span>
            <span v-if="node.invalid" class="row__invalid" title="Fichier illisible">⚠</span>
            <StatusMark v-if="status" class="row__status" :status="status" />
          </span>
        </template>
      </button>
      <!-- AM39 (5.2-d) et AM36 : ▶ et « + » au survol d'un dossier ; au clavier, le menu ⋯. -->
      <template v-if="node.kind === 'folder' && !isTest">
        <button
          type="button"
          class="row__tool"
          tabindex="-1"
          :aria-label="`Exécuter les tests de ${node.name}`"
          :title="`Exécuter les tests de ${node.name}`"
          @click="emit('action', 'run', node)"
        >
          ▶
        </button>
        <BaseMenu
          :items="ADD_ITEMS"
          :label="`Ajouter dans ${node.name}`"
          align="end"
          @select="(id) => emit('action', id, node)"
        >
          <template #trigger="{ attrs }">
            <button
              type="button"
              class="row__tool"
              v-bind="attrs"
              tabindex="-1"
              :aria-label="`Ajouter dans ${node.name}`"
              :title="`Ajouter dans ${node.name}`"
            >
              +
            </button>
          </template>
        </BaseMenu>
      </template>
      <BaseMenu
        ref="menu"
        :items="items"
        :label="`Actions pour ${node.name}`"
        :heading="
          node.kind === 'folder' && !isTest ? `Dans le dossier « ${node.name} »` : undefined
        "
        align="end"
        @select="onSelect"
      >
        <template #trigger="{ attrs }">
          <!-- Hors de l'ordre de tabulation (AC1) : Maj+F10 ou la touche Menu sur la ligne. -->
          <button
            type="button"
            class="row__more"
            v-bind="attrs"
            tabindex="-1"
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
        :step="node.test && child.kind === 'request' && !filter ? index + 1 : undefined"
        :depth="depth + 1"
        :folders="folders"
        :project-id="projectId"
        :filter="filter"
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

/* 4.1-a quater : élément de la sélection multiple. */
.row--picked {
  background: var(--color-accent-soft);
  box-shadow: inset 2px 0 0 var(--color-accent);
}

/* Lot J : une requête glissée sera déposée ici. */
.row--drop {
  box-shadow: inset 0 0 0 1px var(--color-accent);
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

.row__hit {
  border-radius: var(--radius-sm);
  font-weight: 600;
  color: inherit;
  background: none;
  box-shadow: inset 0 -2px var(--color-accent);
}

/* Requête trouvée par son adresse seulement (F2) : le nom cède sa place en premier. */
.row__via {
  flex: 0 0.2 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 0 0.375rem;
  border-radius: var(--radius-sm);
  font-size: var(--text-caption);
  color: var(--color-muted);
  background: var(--color-chip);
}

/* Couleur du compteur : StatusCount (vert, rouge, sinon neutre) ; ne pas la fixer ici. */
.row__count {
  margin-left: auto;
  font-size: var(--text-small);
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

/*
 * ⋯ au survol, au focus de la ligne et sur la ligne ouverte (DE5, 1.2-a) ; caché, il rend sa
 * largeur au nom.
 */
.row__tool,
.row__more {
  flex: none;
  width: 0;
  overflow: hidden;
  border: 0;
  background: none;
  padding: 0.125rem 0;
  color: var(--color-muted);
  cursor: pointer;
  opacity: 0;
}

.row:hover .row__more,
.row:hover .row__tool,
.row:focus-within .row__more,
.row:focus-within .row__tool,
.row--selected .row__more,
.row--selected .row__tool,
.row__more[aria-expanded="true"],
.row__tool[aria-expanded="true"] {
  width: auto;
  padding-inline: 0.5rem;
  opacity: 1;
}

.children {
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
