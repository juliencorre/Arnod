import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { events, ipc } from "@/lib/ipc";
import { useToastStore } from "@/stores/toasts";
import type {
  ActiveRequest,
  ContentRef,
  ContentTab,
  LayoutNode,
  LayoutPreset,
  Placement,
  Side,
  WindowBounds,
  WindowView,
  WindowsSnapshot,
  ZoneView,
} from "@/types/ipc";

/** Étiquette de la fenêtre principale. */
export const MAIN_WINDOW = "main";

/** Zones d'un nœud, dans l'ordre de lecture. */
export function zonesOf(node: LayoutNode): ZoneView[] {
  return node.kind === "zone" ? [node] : [...zonesOf(node.first), ...zonesOf(node.second)];
}

/** Séparation d'axe `axis` la plus proche de la zone `zone`, ou `null`. */
export function splitAround(
  node: LayoutNode,
  zone: string,
  axis: "row" | "column",
): (LayoutNode & { kind: "split" }) | null {
  if (node.kind === "zone") return null;
  const inFirst = zonesOf(node.first).some((z) => z.id === zone);
  const inSecond = !inFirst && zonesOf(node.second).some((z) => z.id === zone);
  if (!inFirst && !inSecond) return null;
  return (
    splitAround(inFirst ? node.first : node.second, zone, axis) ??
    (node.axis === axis ? node : null)
  );
}

/** Projet d'un contenu ; aucun pour le résultat, qui suit la requête active. */
export function contentProject(content: ContentRef): string | null {
  return content.kind === "result" ? null : content.projectId;
}

function isResult(tab: ContentTab): boolean {
  return tab.content.kind === "result";
}

function hasResult(zone: ZoneView): boolean {
  return zone.tabs.some(isResult);
}

/** Même contenu (Z3). */
export function sameContent(a: ContentRef, b: ContentRef): boolean {
  if (a.kind === "result" || b.kind === "result") return a.kind === b.kind;
  if (a.kind === "run" || b.kind === "run") {
    return a.kind === b.kind && a.projectId === b.projectId;
  }
  if (a.kind === "importReport" || b.kind === "importReport") {
    return a.kind === b.kind && a.projectId === b.projectId;
  }
  return a.kind === b.kind && a.projectId === b.projectId && a.path === b.path;
}

/** Clé d'un contenu, pour les brouillons et réponses de l'établi. */
export function contentKey(content: ContentRef): string {
  switch (content.kind) {
    case "request":
      return `${content.projectId}::${content.path}`;
    case "run":
      return `${content.projectId}::run`;
    case "contract":
      return `${content.projectId}::contrat::${content.path}`;
    case "result":
      return "resultat";
    case "importReport":
      return `${content.projectId}::import`;
  }
}

/**
 * Fenêtres et zones (docs/plan-fenetres-zones.md). Le cœur tient la disposition
 * de toutes les fenêtres et la diffuse à chaque changement ; ce store garde
 * l'instantané, sait quelle fenêtre il sert, et relaie les opérations.
 * Seul l'agrandissement d'une zone (F14) reste local à la fenêtre.
 */
export const useWindowsStore = defineStore("windows", () => {
  const toasts = useToastStore();
  const label = ref(MAIN_WINDOW);
  const snapshot = ref<WindowsSnapshot | null>(null);
  /** Requête active, que suit la fenêtre de résultat (D2). */
  const activeRequest = ref<ActiveRequest | null>(null);
  /** Zone agrandie dans cette fenêtre (⤢), ou `null`. */
  const maximizedZone = ref<string | null>(null);
  let subscribed = false;

  const me = computed<WindowView | null>(
    () => snapshot.value?.windows.find((w) => w.label === label.value) ?? null,
  );
  const kind = computed(() => me.value?.kind ?? "main");
  const main = computed(() => snapshot.value?.windows.find((w) => w.kind === "main") ?? null);
  const layout = computed(() => me.value?.layout ?? null);
  const zones = computed(() => (layout.value ? zonesOf(layout.value.root) : []));
  const activeZone = computed(
    () => zones.value.find((z) => z.id === layout.value?.activeZone) ?? zones.value[0] ?? null,
  );
  const activeTab = computed<ContentTab | null>(() => {
    const zone = activeZone.value;
    return zone?.tabs.find((t) => t.id === zone.activeTab) ?? null;
  });
  /** Fenêtres qui suivent la sélection de la barre latérale (Z5). */
  const followers = computed(
    () =>
      snapshot.value?.windows.filter((w) => w.kind === "main" || (w.kind === "content" && w.follow))
        .length ?? 0,
  );
  /** Fenêtre qui porte l'onglet de résultat, s'il existe (R1). */
  const resultWindow = computed(
    () =>
      snapshot.value?.windows.find(
        (w) => w.layout && zonesOf(w.layout.root).some((z) => hasResult(z)),
      ) ?? null,
  );
  /** Le résultat a son onglet : celui des requêtes ne montre que l'éditeur (R3). */
  const resultDetached = computed(() => resultWindow.value !== null);
  /** Cette fenêtre ne montre que le résultat : elle suit la requête active (R2). */
  const resultOnly = computed(
    () => zones.value.length > 0 && zones.value.every((z) => z.tabs.every(isResult)),
  );
  /** Autres fenêtres à zones, pour « Déplacer vers » (F3). */
  const others = computed(
    () => snapshot.value?.windows.filter((w) => w.label !== label.value && w.layout) ?? [],
  );

  function apply(next: WindowsSnapshot): void {
    snapshot.value = next;
    if (maximizedZone.value !== null && !zones.value.some((z) => z.id === maximizedZone.value)) {
      maximizedZone.value = null;
    }
  }

  /** Charge l'état des fenêtres et s'abonne à ses changements. */
  async function load(): Promise<void> {
    const context = await ipc.windowsLoad();
    label.value = context.label;
    apply(context.windows);
    activeRequest.value = context.activeRequest;
    if (!subscribed) {
      subscribed = true;
      await events.onWindows(apply);
      await events.onActiveRequest((active) => {
        activeRequest.value = active;
      });
    }
  }

  /** Relaie une opération du cœur ; ses erreurs sont notifiées. */
  async function run(operation: () => Promise<WindowsSnapshot>): Promise<boolean> {
    try {
      apply(await operation());
      return true;
    } catch (error) {
      toasts.showError(error);
      return false;
    }
  }

  /** Onglet ouvert sur ce contenu dans cette fenêtre (Z3). */
  function tabFor(content: ContentRef): ContentTab | null {
    for (const zone of zones.value) {
      const tab = zone.tabs.find((t) => sameContent(t.content, content));
      if (tab) return tab;
    }
    return null;
  }

  function zoneOf(tab: string): ZoneView | null {
    return zones.value.find((z) => z.tabs.some((t) => t.id === tab)) ?? null;
  }

  /** Séparation à régler depuis la zone `zone` (Alt+Maj+flèches, F4). */
  function splitNear(zone: string, axis: "row" | "column") {
    return layout.value ? splitAround(layout.value.root, zone, axis) : null;
  }

  return {
    label,
    snapshot,
    maximizedZone,
    activeRequest,
    resultWindow,
    resultDetached,
    resultOnly,
    me,
    kind,
    main,
    layout,
    zones,
    activeZone,
    activeTab,
    followers,
    others,
    apply,
    load,
    tabFor,
    zoneOf,
    splitAround: splitNear,
    open: (content: ContentRef, zone: string | null = null) =>
      run(() => ipc.layoutOpen(content, zone)),
    openSelection: (content: ContentRef) => run(() => ipc.layoutOpenSelection(content)),
    activate: (zone: string, tab: string) => run(() => ipc.layoutActivate(zone, tab)),
    close: (tab: string) => run(() => ipc.layoutClose(tab)),
    move: (tab: string, toWindow: string, zone: string | null, index: number | null = null) =>
      run(() => ipc.layoutMove(tab, toWindow, zone, index)),
    split: (zone: string, tab: string, side: Side) => run(() => ipc.layoutSplit(zone, tab, side)),
    resize: (split: string, ratio: number) => run(() => ipc.layoutResize(split, ratio)),
    preset: (preset: LayoutPreset) => run(() => ipc.layoutPreset(preset)),
    setPlacement: (zone: string, placement: Placement) =>
      run(() => ipc.layoutSetPlacement(zone, placement)),
    setFollow: (follow: boolean) => run(() => ipc.layoutSetFollow(follow)),
    setSidebarWidth: (width: number) => run(() => ipc.layoutSetSidebarWidth(width)),
    detachZone: (zone: string, bounds: WindowBounds | null = null) =>
      run(() => ipc.windowDetachZone(zone, bounds)),
    detachTab: (tab: string, bounds: WindowBounds | null = null) =>
      run(() => ipc.windowDetachTab(tab, bounds)),
    detachSidebar: () => run(() => ipc.windowDetachSidebar(null)),
    detachResult: () => run(() => ipc.windowDetachResult(null)),
    reattach: () => run(() => ipc.windowReattach()),
    reattachAll: () => run(() => ipc.windowReattachAll()),
    toggleMaximize(zone: string): void {
      maximizedZone.value = maximizedZone.value === zone ? null : zone;
    },
  };
});
