<script setup lang="ts">
/**
 * Zone (F1 à F3, F8, F14) : ses onglets de contenu, ses outils (◫ diviser,
 * ⧉ ouvrir dans une fenêtre, ⤢ agrandir) et l'onglet actif.
 *
 * Un onglet glissé sur la zone l'ouvre à droite (bord droit), en dessous
 * (bord bas) ou dans la zone (centre) ; glissé hors de la fenêtre, il devient
 * une fenêtre à l'endroit du dépôt. Clavier : flèches entre les onglets,
 * Entrée pour l'ouvrir, Suppr pour le fermer, Maj+F10 pour son menu.
 *
 * Lot F (docs/plan-mvp-f-onglets-clavier.md) : l'onglet d'aperçu a son titre en
 * italique et devient permanent au double-clic, à la modification ou au glisser
 * (1.3-a) ; ◐ pendant une exécution ; les onglets qui ne tiennent pas dans la
 * barre passent dans le menu « +N », avec « Fermer les autres » et « Fermer
 * tout » (1.3-b) ; la liste de Ctrl+Tab s'affiche sur la zone active (1.3-c).
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import ContractTab from "@/components/contracts/ContractTab.vue";
import ImportReportTab from "@/components/import/ImportReportTab.vue";
import RunPanel from "@/components/workbench/RunPanel.vue";
import ProjectWelcome from "@/components/zones/ProjectWelcome.vue";
import RequestTab from "@/components/zones/RequestTab.vue";
import ResultTab from "@/components/zones/ResultTab.vue";
import TestTab from "@/components/zones/TestTab.vue";
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
import { useImportStore } from "@/stores/imports";
import { byUse, fitTabs, isRunning } from "@/lib/tabs";
import { useWindowsStore, zonesOf } from "@/stores/windows";
import { findRequest, requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { ContentTab, ZoneView } from "@/types/ipc";

const props = defineProps<{ zone: ZoneView; projectId: string | null }>();

const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const catalog = useCatalogStore();
const contracts = useContractsStore();
const imports = useImportStore();
const actions = useTabActions();
const strip = useTemplateRef<HTMLElement>("strip");
const body = useTemplateRef<HTMLElement>("body");
const menu = useTemplateRef<InstanceType<typeof BaseMenu>>("menu");
const wrap = useTemplateRef<HTMLElement>("wrap");

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
  if (content.kind === "importReport") {
    const report = imports.reports[content.projectId];
    return {
      name: report ? `Rapport d'import · ${report.collection}` : "Rapport d'import",
      badge: "⇩",
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
  if (content.kind === "test") {
    return {
      name: content.path.split("/").at(-1) ?? content.path,
      badge: "TEST",
      dirty: false,
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

// --- Aperçu et exécution (1.3-a) ------------------------------------------------

/** ◐ : le contenu de l'onglet s'exécute (envoi, run du test, du dossier ou du projet). */
function running(tab: ContentTab): boolean {
  return isRunning(tab.content, {
    sending: (projectId, path) => workbench.editors[requestKey(projectId, path)]?.sending ?? false,
    run: (projectId) => workbench.runs[projectId],
  });
}

/** Double-clic, glisser : l'aperçu devient permanent (R3). */
function pin(tab: ContentTab): void {
  if (tab.preview) void windows.pin(tab.id);
}

// Une modification rend l'aperçu permanent : il ne sera pas remplacé (R3).
watch(
  () => props.zone.tabs.filter((tab) => tab.preview && info(tab).dirty).map((tab) => tab.id),
  (modified) => {
    for (const id of modified) void windows.pin(id);
  },
  { immediate: true },
);

/** Sous-titre d'un onglet : son dossier, ou son projet à la racine. */
function where(tab: ContentTab): string {
  const content = tab.content;
  if (content.kind === "result") return "Requête active";
  const project = catalog.projectById(content.projectId)?.name ?? "";
  if (content.kind === "request" || content.kind === "test") {
    const folder = content.path.split("/").slice(0, -1).join(" / ");
    return folder === "" ? project : folder;
  }
  if (content.kind === "contract") return "Contrats de service";
  return project;
}

// --- Onglets en trop (1.3-b) ----------------------------------------------------

/**
 * Onglets cachés sous « +N », par identifiant. Un onglet que la dernière mesure ne connaît pas
 * encore est visible : il sera rangé à la mesure suivante.
 */
const hiddenIds = ref<string[]>([]);
/** Largeur du bouton « +N », mesurée quand il est affiché. */
const moreWidth = ref(56);

const hiddenTabs = computed(() =>
  props.zone.tabs.filter((tab) => hiddenIds.value.includes(tab.id)),
);
const shownTabs = computed(() =>
  props.zone.tabs.filter((tab) => !hiddenIds.value.includes(tab.id)),
);

function isHidden(tab: ContentTab): boolean {
  return hiddenIds.value.includes(tab.id);
}

/** Mesure les onglets (les cachés restent mis en page, invisibles) et garde ceux qui tiennent. */
function measure(): void {
  const container = wrap.value;
  if (!container) {
    hiddenIds.value = [];
    return;
  }
  const more = container.querySelector<HTMLElement>(".zone__more");
  if (more && more.offsetWidth > 0) moreWidth.value = more.offsetWidth;
  const widths = props.zone.tabs.map(
    (tab) => container.querySelector<HTMLElement>(`[data-tab-item="${tab.id}"]`)?.offsetWidth ?? 0,
  );
  const active = props.zone.tabs.findIndex((tab) => tab.id === props.zone.activeTab);
  const fit = new Set(fitTabs(widths, container.clientWidth, moreWidth.value, active));
  hiddenIds.value = props.zone.tabs.filter((_, index) => !fit.has(index)).map((tab) => tab.id);
}

// La barre disparaît avec le dernier onglet et revient avec le suivant : on suit la nouvelle.
let observer: ResizeObserver | null = null;
watch(
  wrap,
  (element) => {
    observer?.disconnect();
    observer = null;
    if (element && typeof ResizeObserver !== "undefined") {
      observer = new ResizeObserver(() => {
        measure();
      });
      observer.observe(element);
    }
    measure();
  },
  { flush: "post" },
);
onMounted(measure);
onBeforeUnmount(() => observer?.disconnect());

watch(
  () =>
    props.zone.tabs.map(
      (tab) =>
        `${tab.id}:${info(tab).name}:${String(tab.preview)}:${String(info(tab).dirty)}:${String(running(tab))}:${String(tab.id === props.zone.activeTab)}`,
    ),
  () => {
    void nextTick(measure);
  },
  { flush: "post" },
);

const overflowItems = computed<MenuItem[]>(() => [
  ...hiddenTabs.value.map((tab) => ({
    id: `tab:${tab.id}`,
    label: info(tab).name,
    badge: info(tab).badge,
    hint: where(tab),
    tag: running(tab) ? "◐ en cours" : info(tab).dirty ? "● modifié" : undefined,
  })),
  {
    id: "close-others",
    label: "Fermer les autres onglets",
    separatorBefore: hiddenTabs.value.length > 0,
    disabled: props.zone.tabs.length < 2,
  },
  { id: "close-all", label: "Fermer tous les onglets" },
]);

/** Fermer les autres onglets de la zone que `keep` (1.3-b, R6). */
function closeOthers(keep: string | null): Promise<void> {
  return actions.closeMany(props.zone.tabs.filter((t) => t.id !== keep).map((t) => t.id));
}

async function onOverflowSelect(id: string): Promise<void> {
  if (id.startsWith("tab:")) {
    await windows.activate(props.zone.id, id.slice(4));
    await focusTab(id.slice(4));
  } else if (id === "close-others") await closeOthers(props.zone.activeTab);
  else if (id === "close-all") await closeOthers(null);
}

// --- Ctrl+Tab (1.3-c) -------------------------------------------------------------

/** Liste de Ctrl+Tab ouverte sur cette zone : onglets dans l'ordre d'utilisation. */
const switcher = computed(() =>
  windows.tabSwitcher?.zone === props.zone.id ? windows.tabSwitcher : null,
);
const switcherTabs = computed(() => byUse(props.zone.tabs, props.zone.recent));

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
  const list = shownTabs.value;
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
    { id: "close-others", label: "Fermer les autres", disabled: props.zone.tabs.length < 2 },
    { id: "close-all", label: "Fermer tout" },
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
  else if (id === "close-others") await closeOthers(tab.id);
  else if (id === "close-all") await closeOthers(null);
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
  pin(tab);
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
      <div ref="wrap" class="zone__strip">
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
            :data-tab-item="tab.id"
            :aria-hidden="isHidden(tab) ? 'true' : undefined"
            :class="{
              'zone__tab--active': tab.id === zone.activeTab,
              'zone__tab--dragging': draggingFrom === tab.id,
              'zone__tab--preview': tab.preview,
              'zone__tab--hidden': isHidden(tab),
            }"
          >
            <button
              type="button"
              role="tab"
              class="zone__tab-button"
              :data-tab="tab.id"
              :aria-selected="tab.id === zone.activeTab"
              :tabindex="tab.id === zone.activeTab && !isHidden(tab) ? 0 : -1"
              :title="
                tab.preview
                  ? `${info(tab).name} (aperçu : double-clic pour le garder)`
                  : info(tab).name
              "
              aria-haspopup="menu"
              draggable="true"
              @click="activate(tab)"
              @dblclick="pin(tab)"
              @keydown="onTabKeydown($event, tab)"
              @contextmenu.prevent="openMenu(tab, $event)"
              @dragstart="onDragStart($event, tab)"
              @dragend="onDragEnd($event, tab)"
            >
              <span class="zone__badge" :class="`zone__badge--${info(tab).badge.toLowerCase()}`">{{
                info(tab).badge
              }}</span>
              <span class="zone__name">{{ info(tab).name }}</span>
              <span v-if="tab.preview" class="zone__sr">(aperçu)</span>
              <span v-if="running(tab)" class="zone__running" aria-label="en cours" title="En cours"
                >◐</span
              >
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
        <BaseMenu
          v-if="hiddenTabs.length > 0"
          :items="overflowItems"
          :label="`Autres onglets de la zone ${index + 1}`"
          :heading="`Autres onglets ouverts · ${hiddenTabs.length}`"
          align="end"
          class="zone__more-menu"
          @select="onOverflowSelect"
        >
          <template #trigger="{ attrs }">
            <button
              type="button"
              class="zone__more"
              v-bind="attrs"
              :aria-label="`${hiddenTabs.length} autres onglets`"
              :title="`${hiddenTabs.length} autres onglets ouverts`"
            >
              +{{ hiddenTabs.length }} <span aria-hidden="true">▾</span>
            </button>
          </template>
        </BaseMenu>
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
        <TestTab
          v-else-if="activeTab.content.kind === 'test'"
          :key="activeTab.id"
          :project-id="activeTab.content.projectId"
          :path="activeTab.content.path"
        />
        <ImportReportTab
          v-else-if="activeTab.content.kind === 'importReport'"
          :key="activeTab.id"
          :project-id="activeTab.content.projectId"
        />
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
      v-if="switcher"
      class="zone__switcher"
      role="dialog"
      aria-modal="false"
      aria-label="Onglets dans l'ordre d'utilisation"
    >
      <p class="zone__switcher-head">
        <span>Onglets · dans l'ordre d'utilisation</span><kbd>Ctrl+Tab</kbd>
      </p>
      <ul class="zone__switcher-list" role="listbox" aria-label="Onglets">
        <li
          v-for="(tab, position) in switcherTabs"
          :key="tab.id"
          role="option"
          class="zone__switcher-item"
          :class="{ 'zone__switcher-item--active': position === switcher.index }"
          :aria-selected="position === switcher.index"
          @pointerdown.prevent
          @click="windows.commitTabSwitcher(tab.id)"
        >
          <span class="zone__badge" :class="`zone__badge--${info(tab).badge.toLowerCase()}`">{{
            info(tab).badge
          }}</span>
          <span class="zone__switcher-text">
            <span
              class="zone__switcher-name"
              :class="{ 'zone__switcher-name--preview': tab.preview }"
              >{{ info(tab).name }}</span
            >
            <span class="zone__switcher-where"
              >{{ where(tab) }}{{ running(tab) ? " · en cours" : "" }}</span
            >
          </span>
          <span v-if="running(tab)" class="zone__running" aria-hidden="true">◐</span>
          <span v-if="info(tab).dirty" class="zone__dirty" aria-label="modifiée">●</span>
        </li>
      </ul>
      <p class="zone__switcher-foot">Relâcher Ctrl pour ouvrir · Ctrl+Maj+Tab dans l'autre sens</p>
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

.zone__strip {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
}

.zone__tabs {
  position: relative;
  display: flex;
  flex: 0 1 auto;
  align-self: stretch;
  min-width: 0;
  overflow: hidden;
}

.zone__tab {
  display: flex;
  flex: none;
  align-items: center;
  max-width: 12rem;
  border-right: 1px solid var(--color-border);
}

/* Onglet en trop (1.3-b) : mis en page pour être mesuré, invisible, listé dans « +N ». */
.zone__tab--hidden {
  position: absolute;
  top: 0;
  left: 0;
  width: max-content;
  height: 100%;
  visibility: hidden;
  pointer-events: none;
}

/* Aperçu (1.3-a) : titre en italique, remplacé par le prochain aperçu. */
.zone__tab--preview .zone__name {
  font-style: italic;
}

.zone__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.zone__running {
  font-size: var(--text-small);
  color: var(--color-link);
}

.zone__more {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.25rem;
  height: var(--control-sm);
  margin: 0 0.375rem;
  padding: 0 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-small);
  color: var(--color-muted);
  background: var(--color-surface);
  cursor: pointer;
}

.zone__more:hover,
.zone__more[aria-expanded="true"] {
  color: var(--color-text);
  border-color: var(--color-border-strong);
}

.zone__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.zone__more-menu {
  flex: none;
}

/* Liste de Ctrl+Tab (1.3-c), sur la zone active. */
.zone__switcher {
  position: absolute;
  z-index: 25;
  top: 3.25rem;
  left: 50%;
  width: min(27rem, calc(100% - 2rem));
  padding: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
  transform: translateX(-50%);
}

.zone__switcher-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 0.25rem 0.625rem 0.375rem;
  font: 600 var(--text-caption) / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.zone__switcher-head kbd {
  font-family: inherit;
  font-size: var(--text-small);
  letter-spacing: 0;
  text-transform: none;
}

.zone__switcher-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.zone__switcher-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.3125rem 0.625rem;
  border-radius: var(--radius-md);
  cursor: pointer;
}

.zone__switcher-item .zone__badge {
  min-width: 2.25rem;
}

.zone__switcher-item--active {
  background: var(--color-accent-soft);
}

.zone__switcher-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.zone__switcher-name,
.zone__switcher-where {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.zone__switcher-name--preview {
  font-style: italic;
}

.zone__switcher-where {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.zone__switcher-foot {
  margin: 0.375rem 0.625rem 0.25rem;
  font-size: var(--text-small);
  color: var(--color-muted);
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

.zone__badge--test {
  color: var(--color-family-data-text);
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
