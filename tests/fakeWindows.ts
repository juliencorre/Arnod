/**
 * Faux cœur des fenêtres et zones (docs/plan-fenetres-zones.md) : en mémoire,
 * avec une version simplifiée des règles Z1 à Z6 (le vrai comportement est
 * testé côté Rust, `layout/tests.rs`).
 */
import type {
  CommandError,
  ContentRef,
  ContentTab,
  LayoutNode,
  LayoutPreset,
  Side,
  WindowLayout,
  WindowView,
  WindowsSnapshot,
  ZoneView,
} from "@/types/ipc";

type ZoneNode = { kind: "zone" } & ZoneView;

export interface FakeWindows {
  /** Fenêtre servie par l'interface testée (`main` par défaut). */
  label: string;
  windows: WindowView[];
  /** Fenêtres les plus récemment utilisées, la plus récente d'abord (Z5). */
  recent: string[];
}

const MAX_ZONES = 4;
let counter = 0;
const nextId = (prefix: string): string => `${prefix}-${String(++counter)}`;

const fail = (kind: CommandError["kind"], message: string): never => {
  throw { kind, message } satisfies CommandError;
};

/** Copie profonde tolérante aux proxys réactifs de Vue. */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function newZone(tabs: ContentTab[] = []): ZoneNode {
  return {
    kind: "zone",
    id: nextId("zone"),
    tabs,
    activeTab: tabs[0]?.id ?? null,
    placement: "auto",
  };
}

export function newWindow(kind: WindowView["kind"], tabs: ContentTab[] = []): WindowView {
  const zone = newZone(tabs);
  return {
    label: kind === "main" ? "main" : nextId("fenetre"),
    kind,
    layout: kind === "sidebar" ? null : { root: zone, activeZone: zone.id },
    follow: true,
    sidebarWidth: 260,
    sidebarDetached: false,
    bounds: null,
  };
}

export function createFakeWindows(): FakeWindows {
  return { label: "main", windows: [newWindow("main")], recent: ["main"] };
}

export function snapshotOf(fw: FakeWindows): WindowsSnapshot {
  return clone({ version: 1, windows: fw.windows });
}

export function zonesIn(node: LayoutNode): ZoneNode[] {
  return node.kind === "zone" ? [node] : [...zonesIn(node.first), ...zonesIn(node.second)];
}

export function tab(content: ContentRef): ContentTab {
  return { id: nextId("onglet"), content };
}

function same(a: ContentRef, b: ContentRef): boolean {
  if (a.kind === "request" && b.kind === "request") {
    return a.projectId === b.projectId && a.path === b.path;
  }
  return a.kind === b.kind && a.projectId === b.projectId;
}

function windowOf(fw: FakeWindows, label: string): WindowView {
  return (
    fw.windows.find((w) => w.label === label) ?? fail("notFound", "Cette fenêtre n'existe plus.")
  );
}

function layoutOf(fw: FakeWindows, label: string): WindowLayout {
  return (
    windowOf(fw, label).layout ?? fail("invalidRequest", "La barre latérale n'a pas de zones.")
  );
}

function zone(layout: WindowLayout, id: string): ZoneNode {
  return (
    zonesIn(layout.root).find((z) => z.id === id) ?? fail("notFound", "Cette zone n'existe plus.")
  );
}

function zoneOfTab(layout: WindowLayout, id: string): ZoneNode {
  return (
    zonesIn(layout.root).find((z) => z.tabs.some((t) => t.id === id)) ??
    fail("notFound", "Cet onglet n'existe plus.")
  );
}

function activate(layout: WindowLayout, zoneId: string, tabId: string): void {
  zone(layout, zoneId).activeTab = tabId;
  layout.activeZone = zoneId;
}

/** Range un onglet ; un contenu déjà ouvert est seulement activé (Z3). */
function insert(
  layout: WindowLayout,
  zoneId: string,
  added: ContentTab,
  index: number | null,
): void {
  const target = zone(layout, zoneId);
  for (const z of zonesIn(layout.root)) {
    const existing = z.tabs.find((t) => same(t.content, added.content));
    if (existing) {
      activate(layout, z.id, existing.id);
      return;
    }
  }
  target.tabs.splice(index ?? target.tabs.length, 0, added);
  activate(layout, zoneId, added.id);
}

/** Retire la zone `id` de l'arbre ; renvoie le nouvel arbre, ou `null` si c'était la seule. */
function without(node: LayoutNode, id: string): LayoutNode | null {
  if (node.kind === "zone") return node.id === id ? null : node;
  const first = without(node.first, id);
  const second = without(node.second, id);
  if (first === null) return second;
  if (second === null) return first;
  return { ...node, first, second };
}

function removeZone(layout: WindowLayout, id: string): boolean {
  const root = without(layout.root, id);
  if (root === null) return false;
  layout.root = root;
  if (layout.activeZone === id) layout.activeZone = zonesIn(root)[0]?.id ?? id;
  return true;
}

/** Retire un onglet ; une zone vidée disparaît, sauf la dernière (Z2). */
function take(layout: WindowLayout, id: string): ContentTab {
  const source = zoneOfTab(layout, id);
  const index = source.tabs.findIndex((t) => t.id === id);
  const [removed] = source.tabs.splice(index, 1);
  if (source.activeTab === id) {
    source.activeTab = source.tabs[Math.min(index, source.tabs.length - 1)]?.id ?? null;
  }
  if (source.tabs.length === 0) removeZone(layout, source.id);
  return removed ?? fail("notFound", "Cet onglet n'existe plus.");
}

function wrap(node: LayoutNode, id: string, side: Side, added: ZoneNode): LayoutNode {
  if (node.kind === "zone") {
    if (node.id !== id) return node;
    return {
      kind: "split",
      id: nextId("separation"),
      axis: side === "right" ? "row" : "column",
      ratio: 0.5,
      first: node,
      second: added,
    };
  }
  return {
    ...node,
    first: wrap(node.first, id, side, added),
    second: wrap(node.second, id, side, added),
  };
}

function findSplit(node: LayoutNode, id: string): (LayoutNode & { kind: "split" }) | null {
  if (node.kind === "zone") return null;
  if (node.id === id) return node;
  return findSplit(node.first, id) ?? findSplit(node.second, id);
}

const split = (axis: "row" | "column", first: LayoutNode, second: LayoutNode): LayoutNode => ({
  kind: "split",
  id: nextId("separation"),
  axis,
  ratio: 0.5,
  first,
  second,
});

function applyPreset(layout: WindowLayout, preset: LayoutPreset): void {
  const counts: Record<LayoutPreset, number> = {
    one: 1,
    twoColumns: 2,
    twoRows: 2,
    three: 3,
    four: 4,
  };
  const count = counts[preset];
  const zones = Array.from({ length: count }, () => newZone());
  zonesIn(layout.root).forEach((old, index) => {
    const target = zones[Math.min(index, count - 1)];
    if (!target) return;
    if (target.tabs.length === 0) {
      target.placement = old.placement;
      target.activeTab = old.activeTab;
    }
    target.tabs.push(...old.tabs);
    target.activeTab ??= target.tabs[0]?.id ?? null;
  });
  const [a, b, c, d] = zones as [ZoneNode, ZoneNode, ZoneNode, ZoneNode];
  layout.activeZone = a.id;
  layout.root =
    preset === "one"
      ? a
      : preset === "twoColumns"
        ? split("row", a, b)
        : preset === "twoRows"
          ? split("column", a, b)
          : preset === "three"
            ? split("row", a, split("column", b, c))
            : split("row", split("column", a, b), split("column", c, d));
}

/** Ferme les fenêtres de contenu restées sans onglet. */
function dropEmpty(fw: FakeWindows): void {
  fw.windows = fw.windows.filter(
    (w) =>
      w.kind !== "content" || zonesIn(w.layout?.root ?? newZone()).some((z) => z.tabs.length > 0),
  );
  fw.recent = fw.recent.filter((label) => fw.windows.some((w) => w.label === label));
}

function add(fw: FakeWindows, window: WindowView): void {
  fw.windows.push(window);
  fw.recent.unshift(window.label);
}

function main(fw: FakeWindows): WindowView {
  return windowOf(fw, "main");
}

function reattach(fw: FakeWindows, label: string): void {
  const window = windowOf(fw, label);
  if (window.kind === "main") fail("invalidRequest", "La fenêtre principale n'est pas détachée.");
  fw.windows = fw.windows.filter((w) => w.label !== label);
  fw.recent = fw.recent.filter((l) => l !== label);
  const host = main(fw);
  if (window.kind === "sidebar") {
    host.sidebarDetached = false;
    return;
  }
  const layout = host.layout;
  if (!layout) return;
  for (const z of zonesIn(window.layout?.root ?? newZone())) {
    for (const t of z.tabs) insert(layout, layout.activeZone, t, null);
  }
}

/** Fenêtre qui reçoit la sélection de la barre latérale (Z5). */
function selectionTarget(fw: FakeWindows): string {
  for (const label of fw.recent) {
    const w = fw.windows.find((x) => x.label === label);
    if (w && (w.kind === "main" || (w.kind === "content" && w.follow))) return w.label;
  }
  return "main";
}

/** Ferme les onglets d'une requête ou d'un contrat supprimé (ou de ce qu'il contenait). */
export function closePath(fw: FakeWindows, projectId: string, path: string): void {
  for (const w of fw.windows) {
    if (!w.layout) continue;
    for (const z of zonesIn(w.layout.root)) {
      for (const t of [...z.tabs]) {
        const c = t.content;
        if (
          c.kind !== "run" &&
          c.projectId === projectId &&
          (c.path === path || c.path.startsWith(`${path}/`))
        ) {
          take(w.layout, t.id);
        }
      }
    }
  }
  dropEmpty(fw);
}

/**
 * Répond aux commandes des fenêtres ; `undefined` si la commande n'est pas une
 * commande des fenêtres.
 */
export function handleWindows(
  fw: FakeWindows,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  const me = fw.label;
  const opt = (key: string): string | null => {
    const value = args[key];
    return typeof value === "string" ? value : null;
  };
  const str = (key: string): string => opt(key) ?? "";
  const done = (): WindowsSnapshot => snapshotOf(fw);
  switch (cmd) {
    case "windows_load":
      return { label: me, windows: snapshotOf(fw) };
    case "layout_open": {
      const layout = layoutOf(fw, me);
      insert(layout, opt("zone") ?? layout.activeZone, tab(args.content as ContentRef), null);
      return done();
    }
    case "layout_open_selection": {
      const layout = layoutOf(fw, selectionTarget(fw));
      insert(layout, layout.activeZone, tab(args.content as ContentRef), null);
      return done();
    }
    case "layout_activate":
      activate(layoutOf(fw, me), str("zone"), str("tab"));
      return done();
    case "layout_close":
      take(layoutOf(fw, me), str("tab"));
      dropEmpty(fw);
      return done();
    case "layout_move": {
      const to = str("toWindow");
      const target = layoutOf(fw, to);
      const zoneId = opt("zone") ?? target.activeZone;
      zone(target, zoneId);
      const index = args.index == null ? null : Number(args.index);
      const source = layoutOf(fw, me);
      if (to === me && zoneOfTab(source, str("tab")).id === zoneId) {
        const z = zone(source, zoneId);
        const from = z.tabs.findIndex((t) => t.id === str("tab"));
        const [moved] = z.tabs.splice(from, 1);
        if (moved) z.tabs.splice(index ?? z.tabs.length, 0, moved);
        activate(source, zoneId, str("tab"));
      } else {
        insert(target, zoneId, take(source, str("tab")), index);
      }
      dropEmpty(fw);
      return done();
    }
    case "layout_split": {
      const layout = layoutOf(fw, me);
      const zoneId = str("zone");
      zone(layout, zoneId);
      if (zonesIn(layout.root).length >= MAX_ZONES) {
        fail(
          "invalidRequest",
          `Au plus ${String(MAX_ZONES)} zones par fenêtre : fermez ou détachez une zone d'abord.`,
        );
      }
      const source = zoneOfTab(layout, str("tab"));
      if (source.id === zoneId && source.tabs.length === 1) {
        fail(
          "invalidRequest",
          "Seul onglet de sa zone : il n'y a rien à diviser. Glissez-le dans une autre zone.",
        );
      }
      const added = newZone([take(layout, str("tab"))]);
      layout.root = wrap(layout.root, zoneId, args.side as Side, added);
      layout.activeZone = added.id;
      return done();
    }
    case "layout_resize": {
      const found =
        findSplit(layoutOf(fw, me).root, str("split")) ??
        fail("notFound", "Cette séparation n'existe plus.");
      found.ratio = Math.min(0.9, Math.max(0.1, Number(args.ratio)));
      return done();
    }
    case "layout_preset":
      applyPreset(layoutOf(fw, me), args.preset as LayoutPreset);
      return done();
    case "layout_set_placement":
      zone(layoutOf(fw, me), str("zone")).placement = args.placement as ZoneView["placement"];
      return done();
    case "layout_set_follow": {
      const window = windowOf(fw, me);
      if (window.kind !== "content") {
        fail("invalidRequest", "Seule une fenêtre de contenu peut être épinglée.");
      }
      window.follow = Boolean(args.follow);
      return done();
    }
    case "layout_set_sidebar_width":
      main(fw).sidebarWidth = Math.min(600, Math.max(200, Number(args.width)));
      return done();
    case "window_detach_zone": {
      const layout = layoutOf(fw, me);
      const z = zone(layout, str("zone"));
      if (z.tabs.length === 0) {
        fail("invalidRequest", "Cette zone est vide : il n'y a rien à ouvrir dans une fenêtre.");
      }
      const tabs = [...z.tabs];
      if (!removeZone(layout, z.id)) {
        z.tabs = [];
        z.activeTab = null;
      }
      add(fw, newWindow("content", tabs));
      dropEmpty(fw);
      return done();
    }
    case "window_detach_tab":
      add(fw, newWindow("content", [take(layoutOf(fw, me), str("tab"))]));
      dropEmpty(fw);
      return done();
    case "window_detach_sidebar":
      if (!fw.windows.some((w) => w.kind === "sidebar")) {
        main(fw).sidebarDetached = true;
        add(fw, newWindow("sidebar"));
      }
      return done();
    case "window_reattach":
      reattach(fw, me);
      return done();
    case "window_reattach_all":
      for (const w of fw.windows.filter((x) => x.kind !== "main")) reattach(fw, w.label);
      return done();
    default:
      return undefined;
  }
}
