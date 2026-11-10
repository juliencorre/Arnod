<script setup lang="ts">
/**
 * Sélecteur de workspace (docs/plan-selecteur-workspace.md, S1 à S7) : le workspace
 * ouvert (pastille, nom, ▾) en tête de la barre latérale. Son menu liste « Tous les
 * projets » puis les workspaces (couleur, nombre de projets, Ctrl+1…9), et finit par
 * « + Nouveau workspace » et « Gérer les workspaces… ».
 *
 * Clavier : ↓ ou ↑ ouvre ; flèches, Début, Fin, Entrée ou Espace ; Échap rend le focus
 * au sélecteur. Sur un workspace : F2 renomme, Suppr supprime (avec « Annuler »), Maj+F10
 * ou la touche Menu ouvre ses actions, comme le clic droit ou « ⋯ » au survol. Partout :
 * Ctrl+0 à Ctrl+9 et Ctrl+Maj+W. Un projet glissé sur le sélecteur l'ouvre : on le dépose
 * sur un workspace.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useId, useTemplateRef } from "vue";

import NewWorkspacePopover from "@/components/layout/NewWorkspacePopover.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import NameDialog from "@/components/workbench/NameDialog.vue";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { droppedProject, isProjectDrag } from "@/lib/dnd";
import { COLOR_LABELS, countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import type { WorkspaceColor } from "@/types/ipc";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const actions = useProjectActions();

const menuId = useId();
const root = useTemplateRef<HTMLElement>("root");
const trigger = useTemplateRef<HTMLButtonElement>("trigger");
const actionsMenu = useTemplateRef<InstanceType<typeof BaseMenu>>("actionsMenu");

/** Entrée du menu, dans l'ordre du clavier. */
type Entry =
  { kind: "all" } | { kind: "workspace"; id: string } | { kind: "new" } | { kind: "manage" };

const entries = computed<Entry[]>(() => [
  { kind: "all" },
  ...catalog.workspaces.map((w): Entry => ({ kind: "workspace", id: w.id })),
  { kind: "new" },
  { kind: "manage" },
]);

/** Workspace ouvert : son identifiant, ou `null` pour « Tous les projets ». */
const current = computed(() => navigation.currentWorkspaceId);
const currentWorkspace = computed(() =>
  current.value === null ? null : catalog.workspaceById(current.value),
);

function shortcut(index: number): string | null {
  return index < 9 ? `Ctrl+${String(index + 1)}` : null;
}

function projectCount(count: number): string {
  return countLabel(count, "projet", "projets");
}

// --- Ouverture et focus ----------------------------------------------------------

const isOpen = ref(false);
/** Entrée active du menu (-1 : aucune). */
const active = ref(-1);
/** Menu ouvert par un projet glissé : il se ferme à la fin du glisser. */
const byDrag = ref(false);

function indexOfCurrent(): number {
  const index = entries.value.findIndex((e) =>
    current.value === null ? e.kind === "all" : e.kind === "workspace" && e.id === current.value,
  );
  return Math.max(index, 0);
}

async function focusActive(): Promise<void> {
  await nextTick();
  root.value?.querySelector<HTMLElement>(`[data-index="${String(active.value)}"]`)?.focus();
}

function onOutside(event: PointerEvent): void {
  if (!(event.target instanceof Node) || !root.value?.contains(event.target)) close(false);
}

function onDragEnd(): void {
  if (byDrag.value) close(false);
}

function open(at?: number): void {
  if (isOpen.value) return;
  isOpen.value = true;
  active.value = at ?? indexOfCurrent();
  void focusActive();
  document.addEventListener("pointerdown", onOutside, true);
  document.addEventListener("dragend", onDragEnd, true);
}

/** Ferme le menu ; `restore` rend le focus au sélecteur. */
function close(restore = true): void {
  if (!isOpen.value) return;
  isOpen.value = false;
  active.value = -1;
  byDrag.value = false;
  dropTarget.value = null;
  actionsMenu.value?.close(false);
  document.removeEventListener("pointerdown", onOutside, true);
  document.removeEventListener("dragend", onDragEnd, true);
  if (restore) trigger.value?.focus();
}

function toggle(): void {
  if (isOpen.value) close();
  else open();
}

function onTriggerKeydown(event: KeyboardEvent): void {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  event.preventDefault();
  open(event.key === "ArrowUp" ? entries.value.length - 1 : undefined);
}

// --- Choix -------------------------------------------------------------------------

async function show(workspaceId: string | null): Promise<void> {
  try {
    await navigation.showTab(workspaceId);
  } catch (error) {
    toasts.showError(error);
  }
}

async function activate(index: number): Promise<void> {
  const entry = entries.value[index];
  if (!entry) return;
  close();
  if (entry.kind === "all") await show(null);
  else if (entry.kind === "workspace") await show(entry.id);
  else if (entry.kind === "new") dialogs.openNewWorkspace({ open: true });
  else navigation.showManage(current.value ?? undefined);
}

function hover(index: number): void {
  if (actionTarget.value !== null) return;
  active.value = index;
  void focusActive();
}

function onMenuKeydown(event: KeyboardEvent): void {
  const count = entries.value.length;
  const entry = entries.value[active.value];
  const workspaceId = entry?.kind === "workspace" ? entry.id : null;
  const move = (index: number): void => {
    active.value = (index + count) % count;
    void focusActive();
  };
  switch (event.key) {
    case "ArrowDown":
      move(active.value + 1);
      break;
    case "ArrowUp":
      move(active.value - 1);
      break;
    case "Home":
      move(0);
      break;
    case "End":
      move(count - 1);
      break;
    case "Enter":
    case " ":
      void activate(active.value);
      break;
    case "Escape":
      close();
      break;
    case "Tab":
      close(false);
      return;
    case "F2":
      if (workspaceId === null) return;
      startRename(workspaceId);
      break;
    case "Delete":
      if (workspaceId === null) return;
      void remove(workspaceId);
      break;
    case "ContextMenu":
      if (workspaceId === null) return;
      openActions(workspaceId);
      break;
    case "F10":
      if (!event.shiftKey || workspaceId === null) return;
      openActions(workspaceId);
      break;
    default:
      return;
  }
  event.preventDefault();
  event.stopPropagation();
}

// --- Actions d'un workspace (S4) ------------------------------------------------------

const actionTarget = ref<string | null>(null);
const actionsPosition = ref({ left: 0, top: 0 });

const actionItems = computed<MenuItem[]>(() => {
  const workspace = actionTarget.value === null ? null : catalog.workspaceById(actionTarget.value);
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

function rowOf(workspaceId: string): HTMLElement | null {
  return root.value?.querySelector<HTMLElement>(`[data-workspace="${workspaceId}"]`) ?? null;
}

/** Ouvre les actions d'un workspace, à droite de sa ligne ou au pointeur. */
function openActions(workspaceId: string, event?: MouseEvent): void {
  const row = rowOf(workspaceId);
  const index = entries.value.findIndex((e) => e.kind === "workspace" && e.id === workspaceId);
  active.value = index;
  row?.focus();
  const rect = row?.getBoundingClientRect();
  actionsPosition.value = event
    ? { left: event.clientX, top: event.clientY }
    : { left: (rect?.right ?? 0) - 8, top: (rect?.top ?? 0) - 4 };
  actionsMenu.value?.close(false);
  actionTarget.value = workspaceId;
  void nextTick(() => actionsMenu.value?.open());
}

async function onAction(id: string): Promise<void> {
  const workspaceId = actionTarget.value;
  actionTarget.value = null;
  if (workspaceId === null) return;
  if (id === "rename") startRename(workspaceId);
  else if (id === "manage") {
    close(false);
    navigation.showManage(workspaceId);
  } else if (id === "delete") await remove(workspaceId);
  else if (id.startsWith("color:")) {
    try {
      await catalog.setWorkspaceColor(workspaceId, id.slice("color:".length) as WorkspaceColor);
    } catch (error) {
      toasts.showError(error);
    }
  }
}

/** Le menu d'actions s'est fermé sans choix (Échap, clic ailleurs). */
function onActionsClosed(): void {
  actionTarget.value = null;
}

async function remove(workspaceId: string): Promise<void> {
  close();
  await actions.deleteWorkspace(workspaceId);
}

// --- Renommer (S4) ---------------------------------------------------------------------

const renaming = ref<string | null>(null);
const renameError = ref<string | null>(null);
const renameInitial = computed(() =>
  renaming.value === null ? "" : (catalog.workspaceById(renaming.value)?.name ?? ""),
);

function startRename(workspaceId: string): void {
  close(false);
  renameError.value = null;
  renaming.value = workspaceId;
}

async function submitRename(name: string): Promise<void> {
  const workspaceId = renaming.value;
  if (workspaceId === null) return;
  if (name.trim() === renameInitial.value) {
    endRename();
    return;
  }
  try {
    await catalog.renameWorkspace(workspaceId, name);
    endRename();
  } catch (error) {
    renameError.value = error instanceof Error ? error.message : String(error);
  }
}

function endRename(): void {
  renaming.value = null;
  renameError.value = null;
  void nextTick(() => trigger.value?.focus());
}

// --- Glisser un projet (S6) --------------------------------------------------------------

const dropTarget = ref<string | null>(null);

function onTriggerDrag(event: DragEvent): void {
  if (!isProjectDrag(event)) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "none";
  if (!isOpen.value) {
    open();
    byDrag.value = true;
  }
}

function onDragOver(event: DragEvent, workspaceId: string): void {
  if (!isProjectDrag(event)) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
  dropTarget.value = workspaceId;
}

function onDragLeave(workspaceId: string): void {
  if (dropTarget.value === workspaceId) dropTarget.value = null;
}

async function onDrop(event: DragEvent, workspaceId: string): Promise<void> {
  const projectId = droppedProject(event);
  if (projectId === null) return;
  event.preventDefault();
  close(false);
  await actions.addToWorkspaces(projectId, [workspaceId]);
}

const dropHint = computed(() => {
  const workspace = dropTarget.value === null ? null : catalog.workspaceById(dropTarget.value);
  return workspace ? `Ajouter à « ${workspace.name} »` : "Déposez le projet sur un workspace.";
});

// --- Raccourcis globaux (S2, S5) ----------------------------------------------------------

function onGlobalKeydown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
  if (event.shiftKey) {
    if (event.key.toLowerCase() !== "w") return;
    event.preventDefault();
    close(false);
    navigation.showManage(current.value ?? undefined);
    return;
  }
  // Le code de la touche : Ctrl+1 donne « & » sur un clavier AZERTY.
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(event.code)?.[1];
  if (digit === undefined) return;
  const target = digit === "0" ? null : catalog.workspaces[Number(digit) - 1]?.id;
  if (target === undefined) return;
  event.preventDefault();
  close(false);
  void show(target);
}

onMounted(() => {
  window.addEventListener("keydown", onGlobalKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  document.removeEventListener("pointerdown", onOutside, true);
  document.removeEventListener("dragend", onDragEnd, true);
});
</script>

<template>
  <div ref="root" class="switcher">
    <button
      ref="trigger"
      type="button"
      class="switcher__trigger"
      aria-haspopup="menu"
      :aria-expanded="isOpen"
      :aria-controls="isOpen ? menuId : undefined"
      title="Changer de workspace (Ctrl+0…9)"
      @click="toggle"
      @keydown="onTriggerKeydown"
      @dragenter="onTriggerDrag"
      @dragover="onTriggerDrag"
    >
      <span class="switcher__lead" aria-hidden="true">
        <ColorDot v-if="currentWorkspace" :color="currentWorkspace.color" size="sm" />
        <svg v-else class="switcher__grid" width="12" height="12" viewBox="0 0 12 12">
          <rect x="1.5" y="1.5" width="3.6" height="3.6" rx="0.8" />
          <rect x="6.9" y="1.5" width="3.6" height="3.6" rx="0.8" />
          <rect x="1.5" y="6.9" width="3.6" height="3.6" rx="0.8" />
          <rect x="6.9" y="6.9" width="3.6" height="3.6" rx="0.8" />
        </svg>
      </span>
      <span class="visually-hidden">Workspace : </span>
      <span class="switcher__label">{{ currentWorkspace?.name ?? ALL_PROJECTS }}</span>
      <span class="switcher__caret" aria-hidden="true">▾</span>
    </button>

    <div v-if="isOpen" class="switcher__popover" @keydown="onMenuKeydown">
      <ul :id="menuId" role="menu" aria-label="Workspaces" class="switcher__list">
        <li
          role="menuitemradio"
          tabindex="-1"
          class="switcher__item"
          :class="{ 'switcher__item--active': active === 0 }"
          data-index="0"
          :aria-checked="current === null"
          @click="activate(0)"
          @mouseenter="hover(0)"
        >
          <span class="switcher__lead" aria-hidden="true">
            <svg class="switcher__grid" width="12" height="12" viewBox="0 0 12 12">
              <rect x="1.5" y="1.5" width="3.6" height="3.6" rx="0.8" />
              <rect x="6.9" y="1.5" width="3.6" height="3.6" rx="0.8" />
              <rect x="1.5" y="6.9" width="3.6" height="3.6" rx="0.8" />
              <rect x="6.9" y="6.9" width="3.6" height="3.6" rx="0.8" />
            </svg>
          </span>
          <span class="switcher__name">{{ ALL_PROJECTS }}</span>
          <span v-if="current === null" class="switcher__check" aria-hidden="true">✓</span>
          <span class="switcher__count">{{ projectCount(catalog.projects.length) }}</span>
          <kbd class="switcher__key">Ctrl+0</kbd>
        </li>
        <li v-if="catalog.workspaces.length > 0" role="separator" class="switcher__separator" />
        <li v-if="catalog.workspaces.length > 0" role="presentation">
          <p class="switcher__heading" aria-hidden="true">
            Workspaces · {{ catalog.workspaces.length }}
          </p>
          <ul role="group" aria-label="Workspaces" class="switcher__list">
            <li
              v-for="(workspace, index) in catalog.workspaces"
              :key="workspace.id"
              role="menuitemradio"
              tabindex="-1"
              class="switcher__item"
              :class="{
                'switcher__item--active': active === index + 1,
                'switcher__item--drop': dropTarget === workspace.id,
              }"
              :data-index="index + 1"
              :data-workspace="workspace.id"
              :aria-checked="current === workspace.id"
              @click="activate(index + 1)"
              @mouseenter="hover(index + 1)"
              @contextmenu.prevent="openActions(workspace.id, $event)"
              @dragover="onDragOver($event, workspace.id)"
              @dragleave="onDragLeave(workspace.id)"
              @drop="onDrop($event, workspace.id)"
            >
              <span class="switcher__lead" aria-hidden="true">
                <ColorDot :color="workspace.color" size="sm" />
              </span>
              <span class="switcher__name">{{ workspace.name }}</span>
              <span v-if="current === workspace.id" class="switcher__check" aria-hidden="true"
                >✓</span
              >
              <span class="switcher__count">{{ projectCount(workspace.projectIds.length) }}</span>
              <span
                class="switcher__more"
                aria-hidden="true"
                title="Renommer, changer la couleur, supprimer (Maj+F10)"
                @click.stop="openActions(workspace.id)"
                >⋯</span
              >
              <kbd v-if="shortcut(index)" class="switcher__key">{{ shortcut(index) }}</kbd>
            </li>
          </ul>
        </li>
        <li role="separator" class="switcher__separator" />
        <li
          role="menuitem"
          tabindex="-1"
          class="switcher__item switcher__item--link"
          :class="{ 'switcher__item--active': active === entries.length - 2 }"
          :data-index="entries.length - 2"
          @click="activate(entries.length - 2)"
          @mouseenter="hover(entries.length - 2)"
        >
          <span class="switcher__lead" aria-hidden="true">+</span>
          <span class="switcher__name">Nouveau workspace</span>
        </li>
        <li
          role="menuitem"
          tabindex="-1"
          class="switcher__item"
          :class="{ 'switcher__item--active': active === entries.length - 1 }"
          :data-index="entries.length - 1"
          @click="activate(entries.length - 1)"
          @mouseenter="hover(entries.length - 1)"
        >
          <span class="switcher__lead" aria-hidden="true" />
          <span class="switcher__name">Gérer les workspaces…</span>
          <span class="switcher__count" />
          <kbd class="switcher__key">Ctrl+Maj+W</kbd>
        </li>
      </ul>
      <p v-if="byDrag" class="switcher__hint" aria-live="polite">{{ dropHint }}</p>
    </div>

    <div
      class="switcher__context"
      :style="{ left: `${actionsPosition.left}px`, top: `${actionsPosition.top}px` }"
    >
      <BaseMenu
        ref="actionsMenu"
        :items="actionItems"
        :label="`Actions du workspace ${catalog.workspaceById(actionTarget ?? '')?.name ?? ''}`"
        @select="onAction"
        @closed="onActionsClosed"
      >
        <template #trigger><span /></template>
      </BaseMenu>
    </div>

    <NameDialog
      :open="renaming !== null"
      title="Renommer le workspace"
      label="Nom du workspace"
      :initial="renameInitial"
      submit-label="Renommer"
      :error="renameError"
      @close="endRename"
      @submit="submitRename"
    />

    <NewWorkspacePopover :anchor="trigger" />
  </div>
</template>

<style scoped>
.switcher {
  position: relative;
  display: flex;
  min-width: 0;
}

.switcher__trigger {
  box-sizing: border-box;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
  max-width: 100%;
  height: 1.75rem;
  padding: 0 0.5rem 0 0.5625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.switcher__trigger:hover,
.switcher__trigger[aria-expanded="true"] {
  border-color: var(--color-muted);
}

.switcher__trigger:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.switcher__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.switcher__caret {
  flex: none;
  font-size: 0.625rem;
  color: var(--color-muted);
}

.switcher__lead {
  display: inline-grid;
  flex: none;
  place-items: center;
  width: 0.875rem;
  color: var(--color-muted);
}

.switcher__grid {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
}

.switcher__popover {
  position: absolute;
  z-index: 30;
  top: calc(100% + 0.375rem);
  left: 0;
  box-sizing: border-box;
  width: 21rem;
  max-width: calc(100vw - 1rem);
  max-height: calc(100vh - 4rem);
  overflow-y: auto;
  padding: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.switcher__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.switcher__item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  min-height: 2rem;
  padding: 0 0.625rem;
  border-radius: 6px;
  font-size: 0.8125rem;
  color: var(--color-text);
  cursor: pointer;
  outline: none;
}

.switcher__item--active {
  background: var(--color-accent-soft);
}

.switcher__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.switcher__item[aria-checked="true"] .switcher__name {
  font-weight: 600;
}

.switcher__item--drop {
  outline: 2px dashed var(--color-accent);
  outline-offset: -2px;
  background: var(--color-accent-soft);
}

.switcher__item--link {
  font-weight: 500;
  color: var(--color-link);
}

.switcher__item--link .switcher__lead {
  color: var(--color-link);
}

.switcher__name {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.switcher__check {
  flex: none;
  font-weight: 700;
  color: var(--color-success);
}

.switcher__count {
  flex: none;
  margin-left: auto;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.switcher__key {
  flex: none;
  min-width: 3.25rem;
  font: 0.6875rem var(--font-mono);
  text-align: right;
  color: var(--color-muted);
}

/* « ⋯ » remplace le raccourci sur la ligne survolée ou active. */
.switcher__more {
  display: none;
  flex: none;
  place-items: center;
  width: 1.375rem;
  height: 1.375rem;
  margin-left: calc(3.25rem - 1.375rem);
  border-radius: 5px;
  color: var(--color-text);
  background: var(--color-chip);
}

.switcher__item--active .switcher__more {
  display: grid;
}

.switcher__item--active .switcher__more + .switcher__key {
  display: none;
}

.switcher__separator {
  height: 1px;
  margin: 0.375rem 0.5rem;
  background: var(--color-border);
}

.switcher__heading {
  margin: 0.25rem 0.625rem 0.25rem;
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.switcher__hint {
  margin: 0.375rem 0.625rem 0.25rem;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.switcher__context {
  position: fixed;
  z-index: 31;
  width: 0;
  height: 0;
}

/* Le menu d'actions porte des libellés longs (« Supprimer « … » » et son explication). */
.switcher__context :deep(.menu__popover) {
  min-width: 19rem;
}
</style>
