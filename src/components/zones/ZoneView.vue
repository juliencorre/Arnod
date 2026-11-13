<script setup lang="ts">
/**
 * Zone (F1 à F3, F8, F14) : ses onglets de contenu, ses outils (◫ diviser,
 * ⧉ ouvrir dans une fenêtre, ⤢ agrandir) et l'onglet actif.
 *
 * Un onglet glissé sur la zone l'ouvre à droite (bord droit), en dessous
 * (bord bas) ou dans la zone (centre) ; glissé hors de la fenêtre, il devient
 * une fenêtre à l'endroit du dépôt. Clavier : flèches entre les onglets,
 * Entrée pour l'ouvrir, Suppr pour le fermer, Maj+F10 pour son menu.
 */
import { computed, nextTick, ref, useTemplateRef } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import ContractTab from "@/components/contracts/ContractTab.vue";
import RunPanel from "@/components/workbench/RunPanel.vue";
import ProjectWelcome from "@/components/zones/ProjectWelcome.vue";
import RequestTab from "@/components/zones/RequestTab.vue";
import ResultTab from "@/components/zones/ResultTab.vue";
import { useTabActions } from "@/composables/useTabActions";
import {
  type DropRegion,
  droppedTab,
  dropRegion,
  endedOutside,
  isTabDrag,
  startTabDrag,
} from "@/lib/dnd";
import { useCatalogStore } from "@/stores/catalog";
import { contractKey, useContractsStore } from "@/stores/contracts";
import { useWindowsStore, zonesOf } from "@/stores/windows";
import { findRequest, requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { ContentTab, ZoneView } from "@/types/ipc";

const props = defineProps<{ zone: ZoneView; projectId: string | null }>();

const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const catalog = useCatalogStore();
const contracts = useContractsStore();
const actions = useTabActions();
const strip = useTemplateRef<HTMLElement>("strip");
const body = useTemplateRef<HTMLElement>("body");
const menu = useTemplateRef<InstanceType<typeof BaseMenu>>("menu");

const index = computed(() => windows.zones.findIndex((z) => z.id === props.zone.id));
const isActiveZone = computed(() => windows.layout?.activeZone === props.zone.id);
const activeTab = computed(
  () => props.zone.tabs.find((tab) => tab.id === props.zone.activeTab) ?? null,
);
const full = computed(() => windows.zones.length >= 4);
const single = computed(() => windows.zones.length === 1);

/** Libellé, méthode et état d'un onglet. */
function info(tab: ContentTab): { name: string; badge: string; dirty: boolean } {
  const content = tab.content;
  if (content.kind === "result") return { name: "Résultat", badge: "RES", dirty: false };
  if (content.kind === "run") {
    return {
      name: `Run · ${catalog.projectById(content.projectId)?.name ?? "projet"}`,
      badge: "RUN",
      dirty: false,
    };
  }
  if (content.kind === "contract") {
    const key = contractKey(content.projectId, content.path);
    const summary = contracts.lists[content.projectId]?.find((c) => c.path === content.path);
    return {
      name:
        contracts.open[key]?.view?.name ??
        summary?.name ??
        content.path.split("/").at(-1) ??
        content.path,
      badge: "API",
      dirty: contracts.isDirty(key),
    };
  }
  const key = requestKey(content.projectId, content.path);
  const draft = workbench.editors[key]?.draft;
  const node = findRequest(workbench.trees[content.projectId]?.nodes ?? [], content.path);
  const fallback =
    content.path
      .split("/")
      .at(-1)
      ?.replace(/\.yml$/, "") ?? content.path;
  return {
    name: draft?.name ?? node?.name ?? fallback,
    badge: draft?.method ?? (node?.kind === "request" ? node.method : "GET"),
    dirty: workbench.isDirty(key),
  };
}

function activate(tab: ContentTab): void {
  void windows.activate(props.zone.id, tab.id);
}

/** Un clic dans une zone inactive la rend active (Ctrl+W, menu Fenêtre). */
function focusZone(): void {
  if (!isActiveZone.value && props.zone.activeTab !== null) {
    void windows.activate(props.zone.id, props.zone.activeTab);
  }
}

// --- Clavier sur les onglets --------------------------------------------------

async function focusTab(id: string | undefined): Promise<void> {
  if (id === undefined) return;
  await nextTick();
  strip.value?.querySelector<HTMLElement>(`[data-tab="${id}"]`)?.focus();
}

function onTabKeydown(event: KeyboardEvent, tab: ContentTab): void {
  const list = props.zone.tabs;
  const at = list.findIndex((t) => t.id === tab.id);
  const go = (target: ContentTab | undefined): void => {
    event.preventDefault();
    void focusTab(target?.id);
  };
  switch (event.key) {
    case "ArrowRight":
      go(list[(at + 1) % list.length]);
      break;
    case "ArrowLeft":
      go(list[(at - 1 + list.length) % list.length]);
      break;
    case "Home":
      go(list[0]);
      break;
    case "End":
      go(list.at(-1));
      break;
    case "Delete":
      event.preventDefault();
      void actions.close(tab.id);
      break;
    case "ContextMenu":
      event.preventDefault();
      openMenu(tab);
      break;
    case "F10":
      if (event.shiftKey) {
        event.preventDefault();
        openMenu(tab);
      }
      break;
  }
}

// --- Menu d'un onglet (F3, F8) ------------------------------------------------

const menuTab = ref<ContentTab | null>(null);
const menuPosition = ref({ left: 0, top: 0 });

/** Nom d'une zone de destination : sa place et son onglet actif. */
function zoneLabel(zone: ZoneView, position: number): string {
  const active = zone.tabs.find((t) => t.id === zone.activeTab);
  return `Zone ${String(position + 1)}${active ? ` · ${info(active).name}` : " · vide"}`;
}

const menuItems = computed<MenuItem[]>(() => {
  const tab = menuTab.value;
  if (!tab) return [];
  const targets: MenuItem[] = windows.zones
    .map((zone, position) => ({ zone, position }))
    .filter(({ zone }) => zone.id !== props.zone.id)
    .map(({ zone, position }) => ({
      id: `move:${windows.label}:${zone.id}`,
      label: zoneLabel(zone, position),
    }));
  for (const other of windows.others) {
    const zones = other.layout ? zonesOf(other.layout.root) : [];
    const first = zones.flatMap((z) => z.tabs)[0];
    targets.push({
      id: `move:${other.label}:`,
      label:
        other.kind === "main"
          ? "Fenêtre principale"
          : `Fenêtre${first ? ` · ${info(first).name}` : ""}`,
      separatorBefore: targets.length > 0 && other === windows.others[0],
    });
  }
  return [
    { id: "detach-zone", label: "Ouvrir la zone dans une fenêtre", shortcut: "Ctrl+Maj+O" },
    { id: "detach-tab", label: "Ouvrir cet onglet seul dans une fenêtre" },
    targets.length > 0
      ? { id: "move", label: "Déplacer vers", children: targets }
      : { id: "move", label: "Déplacer vers", disabled: true },
    {
      id: "split-right",
      label: "Diviser à droite",
      separatorBefore: true,
      disabled: full.value || props.zone.tabs.length < 2,
    },
    {
      id: "split-bottom",
      label: "Diviser en dessous",
      disabled: full.value || props.zone.tabs.length < 2,
    },
    { id: "close", label: "Fermer", shortcut: "Ctrl+W", separatorBefore: true },
  ];
});

function openMenu(tab: ContentTab, event?: MouseEvent): void {
  const element = strip.value?.querySelector<HTMLElement>(`[data-tab="${tab.id}"]`);
  const rect = element?.getBoundingClientRect();
  menuTab.value = tab;
  menuPosition.value = event
    ? { left: event.clientX, top: event.clientY }
    : { left: rect?.left ?? 0, top: rect?.bottom ?? 0 };
  element?.focus();
  menu.value?.close(false);
  void nextTick(() => menu.value?.open());
}

async function onMenuSelect(id: string): Promise<void> {
  const tab = menuTab.value;
  if (!tab) return;
  if (id === "detach-zone") await actions.detachZone(props.zone.id);
  else if (id === "detach-tab") await actions.detach(tab.id);
  else if (id === "split-right") await actions.split(props.zone.id, tab.id, "right");
  else if (id === "split-bottom") await actions.split(props.zone.id, tab.id, "bottom");
  else if (id === "close") await actions.close(tab.id);
  else if (id.startsWith("move:")) {
    const [, toWindow = "", zone = ""] = id.split(":");
    await actions.saveTab(tab.id);
    await windows.move(tab.id, toWindow, zone === "" ? null : zone);
  }
}

// --- Glisser-déposer (F2, F8) -------------------------------------------------

const region = ref<DropRegion | null>(null);
const draggingFrom = ref<string | null>(null);

function onDragStart(event: DragEvent, tab: ContentTab): void {
  startTabDrag(event, { window: windows.label, zone: props.zone.id, tab: tab.id });
  draggingFrom.value = tab.id;
}

async function onDragEnd(event: DragEvent, tab: ContentTab): Promise<void> {
  draggingFrom.value = null;
  if (endedOutside(event)) {
    await actions.detach(tab.id, { x: event.screenX - 80, y: event.screenY - 20 });
  }
}

function onDragOver(event: DragEvent): void {
  if (!isTabDrag(event)) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  const rect = body.value?.getBoundingClientRect();
  if (rect) region.value = dropRegion(rect, event.clientX, event.clientY);
}

function onDragLeave(event: DragEvent): void {
  const next = event.relatedTarget;
  if (next instanceof Node && body.value?.contains(next)) return;
  region.value = null;
}

async function onDrop(event: DragEvent): Promise<void> {
  const where = region.value;
  region.value = null;
  const dragged = droppedTab(event);
  if (!dragged || dragged.window !== windows.label || where === null) return;
  event.preventDefault();
  if (where === "center") {
    if (dragged.zone !== props.zone.id)
      await windows.move(dragged.tab, windows.label, props.zone.id);
  } else {
    await actions.split(props.zone.id, dragged.tab, where === "right" ? "right" : "bottom");
  }
}

/** Dépôt sur la barre d'onglets : l'onglet rejoint la zone. */
async function onStripDrop(event: DragEvent): Promise<void> {
  const dragged = droppedTab(event);
  if (!dragged || dragged.window !== windows.label || dragged.zone === props.zone.id) return;
  event.preventDefault();
  await windows.move(dragged.tab, windows.label, props.zone.id);
}

const regionLabel = computed(() => {
  switch (region.value) {
    case "right":
      return "Déposer ici : ouvrir à droite";
    case "bottom":
      return "Déposer ici : ouvrir en dessous";
    case "center":
      return "Déposer ici : dans cette zone";
    default:
      return "";
  }
});

/** Projet accueilli quand la fenêtre principale n'a aucun onglet. */
const welcome = computed(
  () =>
    props.projectId !== null &&
    windows.kind === "main" &&
    single.value &&
    props.zone.tabs.length === 0,
);
</script>

<template>
  <section
    class="zone"
    :class="{ 'zone--active': isActiveZone && !single }"
    :aria-label="`Zone ${index + 1}`"
    @pointerdown.capture="focusZone"
  >
    <header v-if="zone.tabs.length > 0" class="zone__bar">
      <div
        ref="strip"
        class="zone__tabs"
        role="tablist"
        :aria-label="`Onglets de la zone ${index + 1}`"
        @dragover="isTabDrag($event) && $event.preventDefault()"
        @drop="onStripDrop"
      >
        <div
          v-for="tab in zone.tabs"
          :key="tab.id"
          class="zone__tab"
          :class="{
            'zone__tab--active': tab.id === zone.activeTab,
            'zone__tab--dragging': draggingFrom === tab.id,
          }"
        >
          <button
            type="button"
            role="tab"
            class="zone__tab-button"
            :data-tab="tab.id"
            :aria-selected="tab.id === zone.activeTab"
            :tabindex="tab.id === zone.activeTab ? 0 : -1"
            :title="info(tab).name"
            aria-haspopup="menu"
            draggable="true"
            @click="activate(tab)"
            @keydown="onTabKeydown($event, tab)"
            @contextmenu.prevent="openMenu(tab, $event)"
            @dragstart="onDragStart($event, tab)"
            @dragend="onDragEnd($event, tab)"
          >
            <span class="zone__badge" :class="`zone__badge--${info(tab).badge.toLowerCase()}`">{{
              info(tab).badge
            }}</span>
            <span class="zone__name">{{ info(tab).name }}</span>
            <span v-if="info(tab).dirty" class="zone__dirty" aria-label="modifiée">●</span>
          </button>
          <button
            type="button"
            class="zone__close"
            tabindex="-1"
            :aria-label="`Fermer ${info(tab).name}`"
            @click="actions.close(tab.id)"
          >
            ×
          </button>
        </div>
      </div>
      <div class="zone__tools">
        <button
          type="button"
          class="zone__tool"
          aria-label="Diviser à droite"
          title="Diviser à droite"
          :disabled="full || zone.tabs.length < 2 || activeTab === null"
          @click="activeTab && actions.split(zone.id, activeTab.id, 'right')"
        >
          ◫
        </button>
        <button
          type="button"
          class="zone__tool"
          aria-label="Ouvrir la zone dans une fenêtre"
          title="Ouvrir la zone dans une fenêtre (Ctrl+Maj+O)"
          @click="actions.detachZone(zone.id)"
        >
          ⧉
        </button>
        <button
          v-if="!single || windows.maximizedZone === zone.id"
          type="button"
          class="zone__tool"
          :aria-label="windows.maximizedZone === zone.id ? 'Rétablir la zone' : 'Agrandir la zone'"
          :aria-pressed="windows.maximizedZone === zone.id"
          :title="windows.maximizedZone === zone.id ? 'Rétablir la zone' : 'Agrandir la zone'"
          @click="windows.toggleMaximize(zone.id)"
        >
          ⤢
        </button>
      </div>
    </header>

    <div
      ref="body"
      class="zone__body"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <template v-if="activeTab">
        <RequestTab
          v-if="activeTab.content.kind === 'request'"
          :key="activeTab.id"
          :tab-id="activeTab.id"
          :zone-id="zone.id"
          :project-id="activeTab.content.projectId"
          :path="activeTab.content.path"
          :placement="zone.placement"
        />
        <ContractTab
          v-else-if="activeTab.content.kind === 'contract'"
          :key="activeTab.id"
          :project-id="activeTab.content.projectId"
          :path="activeTab.content.path"
        />
        <ResultTab v-else-if="activeTab.content.kind === 'result'" :key="activeTab.id" />
        <div v-else class="zone__run">
          <RunPanel :project-id="activeTab.content.projectId" />
        </div>
      </template>
      <ProjectWelcome v-else-if="welcome && projectId" :project-id="projectId" />
      <p v-else class="zone__empty">
        Zone vide : ouvrez une requête dans la barre latérale, ou glissez un onglet ici.
      </p>

      <div v-if="region" class="zone__drop" :class="`zone__drop--${region}`" aria-hidden="true">
        <span class="zone__drop-label">{{ regionLabel }}</span>
      </div>
      <p v-if="region" class="zone__drop-hints" aria-hidden="true">
        <span>Bord droit : à droite</span><span>Bord bas : en dessous</span
        ><span>Centre : même zone</span>
      </p>
    </div>

    <div
      class="zone__context"
      :style="{ left: `${menuPosition.left}px`, top: `${menuPosition.top}px` }"
    >
      <BaseMenu
        ref="menu"
        :items="menuItems"
        :label="`Actions de l'onglet ${menuTab ? info(menuTab).name : ''}`"
        @select="onMenuSelect"
      >
        <template #trigger><span /></template>
      </BaseMenu>
    </div>
  </section>
</template>

<style scoped>
.zone {
  position: relative;
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  background: var(--color-bg);
}

.zone--active .zone__bar {
  box-shadow: inset 0 2px 0 var(--color-accent);
}

.zone__bar {
  display: flex;
  flex: none;
  align-items: stretch;
  height: 2.375rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.zone__tabs {
  display: flex;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}

.zone__tab {
  display: flex;
  flex: none;
  align-items: center;
  max-width: 16rem;
  border-right: 1px solid var(--color-border);
}

.zone__tab--active {
  background: var(--color-bg);
  box-shadow: inset 0 -1px 0 var(--color-bg);
}

.zone__tab--dragging {
  opacity: 0.5;
}

.zone__tab-button {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  min-width: 0;
  height: 100%;
  padding: 0 0.25rem 0 0.75rem;
  border: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.zone__tab--active .zone__tab-button {
  color: var(--color-text);
  font-weight: 600;
}

.zone__tab-button:focus-visible,
.zone__tool:focus-visible,
.zone__close:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.zone__badge {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.zone__badge--get {
  color: var(--color-success);
}

.zone__badge--post,
.zone__badge--put,
.zone__badge--patch {
  color: var(--color-link);
}

.zone__badge--delete {
  color: var(--color-danger);
}

.zone__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.zone__dirty {
  font-size: 0.625rem;
  color: var(--color-accent);
}

.zone__close {
  padding: 0 0.5rem 0 0.25rem;
  border: none;
  font: inherit;
  font-size: 1rem;
  color: var(--color-subtle);
  background: none;
  cursor: pointer;
}

.zone__close:hover {
  color: var(--color-text);
}

.zone__tools {
  display: flex;
  flex: none;
  align-items: center;
  gap: 0.125rem;
  padding: 0 0.375rem;
}

.zone__tool {
  display: grid;
  place-items: center;
  width: 1.625rem;
  height: 1.625rem;
  border: none;
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.zone__tool:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-chip);
}

.zone__tool:disabled {
  cursor: default;
  color: var(--color-subtle);
}

.zone__tool[aria-pressed="true"] {
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.zone__body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.zone__run {
  flex: 1;
  padding: 1rem 1.25rem;
}

.zone__empty {
  margin: auto;
  padding: 2rem;
  max-width: 22rem;
  text-align: center;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.zone__drop {
  position: absolute;
  z-index: 3;
  display: grid;
  place-items: center;
  border: 2px dashed var(--color-accent);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-accent-soft) 80%, transparent);
  pointer-events: none;
}

.zone__drop--center {
  inset: 0.5rem;
}

.zone__drop--right {
  inset: 0.5rem 0.5rem 0.5rem 50%;
}

.zone__drop--bottom {
  inset: 50% 0.5rem 0.5rem;
}

.zone__drop-label {
  padding: 0.375rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: var(--text-body);
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
}

.zone__drop-hints {
  position: absolute;
  z-index: 3;
  bottom: 0.875rem;
  left: 0.875rem;
  display: flex;
  gap: 0.5rem;
  margin: 0;
  pointer-events: none;
}

.zone__drop-hints span {
  padding: 0.1875rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  color: var(--color-muted);
  background: var(--color-surface);
}

.zone__context {
  position: fixed;
  z-index: 30;
  width: 0;
  height: 0;
}

.zone__context :deep(.menu__popover) {
  min-width: 17rem;
}
</style>
