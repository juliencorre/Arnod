<script setup lang="ts">
/**
 * Onglets des workspaces (2b, O2 à O7) : « Tous les projets » épinglé en tête,
 * puis les workspaces dans l'ordre de création ; ils défilent, avec ‹ › quand
 * ils débordent. « + » crée un workspace.
 *
 * Clic droit, Maj+F10 ou touche Menu : renommer, changer la couleur, gérer,
 * supprimer. Clavier : flèches, Début et Fin entre les onglets, Entrée ou Espace
 * pour ouvrir, F2 pour renommer, Suppr pour supprimer (avec « Annuler »).
 * Un projet glissé depuis « Tous les projets » se dépose sur un onglet.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";

import NewWorkspacePopover from "@/components/layout/NewWorkspacePopover.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { droppedProject, isProjectDrag } from "@/lib/dnd";
import { COLOR_LABELS } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import type { WorkspaceColor } from "@/types/ipc";

/** Clé d'un onglet : l'identifiant du workspace, ou `null` pour « Tous les projets ». */
type TabKey = string | null;

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const actions = useProjectActions();

const bar = useTemplateRef<HTMLElement>("bar");
const scroller = useTemplateRef<HTMLElement>("scroller");
const plusButton = useTemplateRef<HTMLButtonElement>("plusButton");
const contextMenu = useTemplateRef<InstanceType<typeof BaseMenu>>("contextMenu");
const renameInput = useTemplateRef<HTMLInputElement[]>("renameInput");

/** Ordre des onglets pour le clavier. */
const keys = computed<TabKey[]>(() => [null, ...catalog.workspaces.map((w) => w.id)]);
const active = computed<TabKey>(() => navigation.currentWorkspaceId);

// --- Défilement ---------------------------------------------------------------

const overflow = ref({ left: false, right: false });

function measure(): void {
  const el = scroller.value;
  if (!el) return;
  overflow.value = {
    left: el.scrollLeft > 1,
    right: el.scrollLeft + el.clientWidth < el.scrollWidth - 1,
  };
}

function scrollBy(delta: number): void {
  scroller.value?.scrollBy({ left: delta, behavior: "smooth" });
}

let observer: ResizeObserver | null = null;
onMounted(() => {
  measure();
  if (typeof ResizeObserver !== "undefined" && scroller.value) {
    observer = new ResizeObserver(measure);
    observer.observe(scroller.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());
watch(
  () => catalog.workspaces.length,
  async () => {
    await nextTick();
    measure();
  },
);

// L'onglet actif reste visible.
watch(active, async (key) => {
  await nextTick();
  tabElement(key)?.scrollIntoView({ block: "nearest", inline: "nearest" });
});

// --- Focus et clavier -----------------------------------------------------------

function tabElement(key: TabKey): HTMLElement | null {
  return bar.value?.querySelector<HTMLElement>(`[data-tab="${key ?? "all"}"]`) ?? null;
}

async function focusTab(key: TabKey): Promise<void> {
  await nextTick();
  const el = tabElement(key);
  el?.focus();
  el?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

async function activate(key: TabKey): Promise<void> {
  try {
    await navigation.showTab(key);
  } catch (error) {
    toasts.showError(error);
  }
}

function onKeydown(event: KeyboardEvent, key: TabKey): void {
  const list = keys.value;
  const index = list.indexOf(key);
  const move = (target: TabKey | undefined): void => {
    event.preventDefault();
    if (target !== undefined) void focusTab(target);
  };
  switch (event.key) {
    case "ArrowRight":
      move(list[(index + 1) % list.length]);
      break;
    case "ArrowLeft":
      move(list[(index - 1 + list.length) % list.length]);
      break;
    case "Home":
      move(list[0]);
      break;
    case "End":
      move(list.at(-1));
      break;
    case "F2":
      if (key !== null) {
        event.preventDefault();
        void startRename(key);
      }
      break;
    case "Delete":
      if (key !== null) {
        event.preventDefault();
        void remove(key);
      }
      break;
    case "ContextMenu":
      event.preventDefault();
      openMenu(key);
      break;
    case "F10":
      if (event.shiftKey) {
        event.preventDefault();
        openMenu(key);
      }
      break;
  }
}

// --- Menu contextuel ------------------------------------------------------------

const menuTarget = ref<string | null>(null);
const menuPosition = ref({ left: 0, top: 0 });

const menuItems = computed<MenuItem[]>(() => {
  const workspace = menuTarget.value === null ? null : catalog.workspaceById(menuTarget.value);
  if (!workspace) return [];
  return [
    { id: "rename", label: "Renommer", shortcut: "F2" },
    {
      id: "color",
      label: "Changer la couleur",
      children: (Object.keys(COLOR_LABELS) as WorkspaceColor[]).map((color) => ({
        id: `color:${color}`,
        label: COLOR_LABELS[color],
        checked: workspace.color === color,
      })),
    },
    { id: "manage", label: "Gérer les workspaces…", separatorBefore: true },
    {
      id: "delete",
      label: `Supprimer « ${workspace.name} »`,
      hint: `Les projets restent dans « ${ALL_PROJECTS} ».`,
      tone: "danger",
      shortcut: "Suppr",
      separatorBefore: true,
    },
  ];
});

/** Ouvre le menu d'un onglet de workspace, sous l'onglet ou au pointeur. */
function openMenu(key: TabKey, event?: MouseEvent): void {
  if (key === null) return;
  const tab = tabElement(key);
  const rect = tab?.getBoundingClientRect();
  menuTarget.value = key;
  menuPosition.value = event
    ? { left: event.clientX, top: event.clientY }
    : { left: rect?.left ?? 0, top: rect?.bottom ?? 0 };
  tab?.focus();
  contextMenu.value?.close(false);
  void nextTick(() => contextMenu.value?.open());
}

async function onMenuSelect(id: string): Promise<void> {
  const key = menuTarget.value;
  if (key === null) return;
  if (id === "rename") await startRename(key);
  else if (id === "manage") navigation.showManage(key);
  else if (id === "delete") await remove(key);
  else if (id.startsWith("color:")) {
    try {
      await catalog.setWorkspaceColor(key, id.slice("color:".length) as WorkspaceColor);
    } catch (error) {
      toasts.showError(error);
    }
  }
}

async function remove(key: string): Promise<void> {
  const index = keys.value.indexOf(key);
  await actions.deleteWorkspace(key);
  // Le focus passe à l'onglet voisin, ou à « Tous les projets ».
  if (catalog.workspaceById(key) === null) await focusTab(keys.value[index - 1] ?? null);
}

// --- Renommage dans l'onglet ----------------------------------------------------

const renaming = ref<string | null>(null);
const newName = ref("");
let saving = false;

async function startRename(key: string): Promise<void> {
  newName.value = catalog.workspaceById(key)?.name ?? "";
  renaming.value = key;
  await nextTick();
  const input = renameInput.value?.[0];
  input?.focus();
  input?.select();
}

async function saveRename(): Promise<void> {
  const key = renaming.value;
  if (key === null || saving) return;
  if (newName.value.trim() === catalog.workspaceById(key)?.name) {
    cancelRename();
    return;
  }
  saving = true;
  try {
    await catalog.renameWorkspace(key, newName.value);
    renaming.value = null;
    await focusTab(key);
  } catch (error) {
    toasts.showError(error);
    renameInput.value?.[0]?.focus();
  } finally {
    saving = false;
  }
}

function cancelRename(): void {
  const key = renaming.value;
  renaming.value = null;
  if (key !== null) void focusTab(key);
}

// --- Dépôt d'un projet ----------------------------------------------------------

const dropTarget = ref<string | null>(null);
const dropHint = ref({ left: 0, top: 0 });

function onDragOver(event: DragEvent, key: string): void {
  if (!isProjectDrag(event)) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
  if (dropTarget.value !== key) {
    const rect = tabElement(key)?.getBoundingClientRect();
    dropHint.value = { left: rect?.left ?? 0, top: (rect?.bottom ?? 0) + 6 };
    dropTarget.value = key;
  }
}

function onDragLeave(key: string): void {
  if (dropTarget.value === key) dropTarget.value = null;
}

async function onDrop(event: DragEvent, key: string): Promise<void> {
  dropTarget.value = null;
  const projectId = droppedProject(event);
  if (projectId === null) return;
  event.preventDefault();
  await actions.addToWorkspaces(projectId, [key]);
}

const dropLabel = computed(() =>
  dropTarget.value === null
    ? ""
    : `Ajouter à « ${catalog.workspaceById(dropTarget.value)?.name ?? ""} »`,
);
</script>

<template>
  <header ref="bar" class="tabs">
    <div class="tabs__list" role="tablist" aria-label="Workspaces">
      <button
        type="button"
        role="tab"
        class="tab tab--pinned"
        data-tab="all"
        :aria-selected="active === null"
        :tabindex="active === null ? 0 : -1"
        @click="activate(null)"
        @keydown="onKeydown($event, null)"
      >
        {{ ALL_PROJECTS }}
      </button>
      <span v-if="catalog.workspaces.length > 0" class="tabs__separator" aria-hidden="true" />
      <div ref="scroller" class="tabs__scroller" @scroll="measure">
        <template v-for="workspace in catalog.workspaces" :key="workspace.id">
          <form
            v-if="renaming === workspace.id"
            class="tab tab--editing"
            novalidate
            @submit.prevent="saveRename"
          >
            <ColorDot :color="workspace.color" size="sm" />
            <input
              ref="renameInput"
              v-model="newName"
              class="tab__input"
              type="text"
              autocomplete="off"
              :aria-label="`Nouveau nom de « ${workspace.name} »`"
              @keydown.esc.prevent="cancelRename"
              @blur="saveRename"
            />
          </form>
          <button
            v-else
            type="button"
            role="tab"
            class="tab"
            :class="{ 'tab--drop': dropTarget === workspace.id }"
            :data-tab="workspace.id"
            :title="workspace.name"
            :aria-selected="active === workspace.id"
            :tabindex="active === workspace.id ? 0 : -1"
            aria-haspopup="menu"
            @click="activate(workspace.id)"
            @keydown="onKeydown($event, workspace.id)"
            @contextmenu.prevent="openMenu(workspace.id, $event)"
            @dragover="onDragOver($event, workspace.id)"
            @dragleave="onDragLeave(workspace.id)"
            @drop="onDrop($event, workspace.id)"
          >
            <ColorDot :color="workspace.color" size="sm" />
            <span class="tab__name">{{ workspace.name }}</span>
          </button>
        </template>
      </div>
    </div>
    <div class="tabs__tools">
      <template v-if="overflow.left || overflow.right">
        <button
          type="button"
          class="tabs__tool"
          aria-label="Faire défiler les onglets vers la gauche"
          :disabled="!overflow.left"
          @click="scrollBy(-240)"
        >
          ‹
        </button>
        <button
          type="button"
          class="tabs__tool"
          aria-label="Faire défiler les onglets vers la droite"
          :disabled="!overflow.right"
          @click="scrollBy(240)"
        >
          ›
        </button>
      </template>
      <button
        ref="plusButton"
        type="button"
        class="tabs__tool tabs__tool--plus"
        aria-label="Nouveau workspace"
        title="Nouveau workspace"
        :aria-expanded="dialogs.newWorkspace !== null"
        @click="dialogs.openNewWorkspace({ open: true })"
      >
        +
      </button>
    </div>

    <div
      class="tabs__context"
      :style="{ left: `${menuPosition.left}px`, top: `${menuPosition.top}px` }"
    >
      <BaseMenu
        ref="contextMenu"
        :items="menuItems"
        :label="`Actions du workspace ${catalog.workspaceById(menuTarget ?? '')?.name ?? ''}`"
        @select="onMenuSelect"
      >
        <template #trigger><span /></template>
      </BaseMenu>
    </div>

    <p
      v-if="dropTarget !== null"
      class="tabs__drop-hint"
      :style="{ left: `${dropHint.left}px`, top: `${dropHint.top}px` }"
      aria-hidden="true"
    >
      {{ dropLabel }}
    </p>

    <NewWorkspacePopover :anchor="plusButton" />
  </header>
</template>

<style scoped>
.tabs {
  box-sizing: border-box;
  display: flex;
  flex: none;
  align-items: flex-end;
  gap: 0.5rem;
  height: 2.75rem;
  padding: 0 0.5rem 0 0.75rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.tabs__list {
  display: flex;
  flex: 1;
  align-items: flex-end;
  min-width: 0;
  height: 100%;
}

.tabs__separator {
  flex: none;
  align-self: center;
  width: 1px;
  height: 1.125rem;
  margin: 0 0.375rem;
  background: var(--color-border-strong);
}

.tabs__scroller {
  display: flex;
  align-items: flex-end;
  gap: 0.125rem;
  min-width: 0;
  height: 100%;
  overflow-x: auto;
  scrollbar-width: none;
}

.tabs__scroller::-webkit-scrollbar {
  display: none;
}

.tab {
  box-sizing: border-box;
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.5rem;
  max-width: 15rem;
  height: 2.25rem;
  padding: 0 0.875rem;
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: 8px 8px 0 0;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
  white-space: nowrap;
}

.tab:hover {
  color: var(--color-text);
}

.tab[aria-selected="true"] {
  /* L'onglet actif se fond dans la zone principale. */
  margin-bottom: -1px;
  height: calc(2.25rem + 1px);
  border-color: var(--color-border);
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-bg);
}

.tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -3px;
}

.tab--pinned {
  flex: none;
}

.tab--drop {
  outline: 2px dashed var(--color-accent);
  outline-offset: -3px;
  color: var(--color-text);
  background: var(--color-accent-soft);
}

.tab__name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab--editing {
  background: var(--color-bg);
  border-color: var(--color-border);
}

.tab__input {
  width: 10rem;
  padding: 0.25rem 0.375rem;
  border: 1px solid var(--color-focus);
  border-radius: 4px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.tab__input:focus-visible {
  outline: none;
}

.tabs__tools {
  display: flex;
  flex: none;
  align-items: center;
  gap: 0.125rem;
  align-self: center;
}

.tabs__tool {
  display: grid;
  place-items: center;
  width: 1.75rem;
  height: 1.75rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 1.0625rem;
  line-height: 1;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.tabs__tool:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-chip);
}

.tabs__tool:disabled {
  cursor: default;
  color: var(--color-subtle);
}

.tabs__tool:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.tabs__context {
  position: fixed;
  z-index: 30;
  width: 0;
  height: 0;
}

/* Le menu d'un onglet porte des libellés longs (« Supprimer « … » » et son explication). */
.tabs__context :deep(.menu__popover) {
  min-width: 19rem;
}

.tabs__drop-hint {
  position: fixed;
  z-index: 30;
  margin: 0;
  padding: 0.3125rem 0.625rem;
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
  pointer-events: none;
}
</style>
