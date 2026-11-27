/**
 * Faux cœur des préférences d'interface (1.6-f, 1.6-f bis) et de l'historique des envois
 * (5.1-c bis), en mémoire. Les règles (valeurs proposées, combinaisons réservées, conflits,
 * comparaison des envois) sont testées côté Rust (`interface/tests.rs`, `history/tests.rs`) ;
 * le faux cœur en reprend ce dont les parcours d'interface ont besoin.
 */
import { emit } from "@tauri-apps/api/event";

import { EVENTS } from "@/lib/ipc";
import { DEFAULT_SHORTCUTS } from "@/lib/shortcuts";
import type {
  Appearance,
  CommandError,
  HistoryDiff,
  HistoryEntry,
  HistoryView,
  InterfaceView,
  SendOutcome,
  ShortcutChange,
  ShortcutGroup,
  ShortcutScope,
  ShortcutView,
} from "@/types/ipc";

/** Libellé, groupe et portée de chaque action, comme `interface::ACTIONS`. */
const ACTIONS: Record<string, [string, ShortcutGroup, ShortcutScope, boolean]> = {
  send: ["Envoyer", "request", "editor", true],
  copyCurl: ["Copier en cURL", "request", "editor", true],
  filterResponse: ["Filtrer la réponse", "request", "response", true],
  newTest: ["Nouveau test", "request", "global", true],
  newFolder: ["Nouveau dossier", "request", "global", true],
  filterTree: ["Filtrer le référentiel", "request", "sidebar", true],
  saveContract: ["Enregistrer le contrat", "request", "contract", true],
  togglePlacement: ["Résultat dessous ou à côté", "window", "global", true],
  toggleSidebar: ["Afficher ou masquer la barre latérale", "window", "global", true],
  nextTab: ["Onglet suivant, précédent", "window", "global", false],
  closeTab: ["Fermer l'onglet", "window", "global", true],
  environments: ["Environnements du projet", "project", "global", true],
  manageWorkspaces: ["Gérer les workspaces", "project", "global", true],
  workspaces: ["Workspace 1 à 9, Tous les projets", "project", "global", false],
  preferences: ["Préférences", "project", "global", true],
};

const RESERVED = new Set(["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+Y", "Ctrl+A", "Ctrl+Tab"]);

export interface FakeInterface {
  appearance: Appearance;
  /** Combinaisons changées, par action ; `null` : retirée. */
  overrides: Map<string, string | null>;
  /** Historique par « projet|chemin », le plus récent d'abord. */
  history: Map<string, HistoryEntry[]>;
}

export function createFakeInterface(): FakeInterface {
  return {
    appearance: { theme: "system", zoom: 100, editorFont: null, editorFontSize: 13 },
    overrides: new Map(),
    history: new Map(),
  };
}

function fail(kind: CommandError["kind"], message: string): never {
  throw { kind, message } satisfies CommandError;
}

function shortcutsOf(fake: FakeInterface): ShortcutView[] {
  return Object.entries(DEFAULT_SHORTCUTS).map(([id, defaults]) => {
    const [label, group, scope, editable] = ACTIONS[id] ?? ["", "request", "global", false];
    const override = fake.overrides.get(id);
    const keys =
      override === undefined || !editable ? [...defaults] : override === null ? [] : [override];
    return { id, label, group, scope, keys, defaults: [...defaults], editable };
  });
}

export function viewOf(fake: FakeInterface): InterfaceView {
  return {
    appearance: structuredClone(fake.appearance),
    zooms: [80, 90, 100, 110, 125, 150, 175, 200],
    fonts: ["Cascadia Mono", "Consolas", "DejaVu Sans Mono", "Fira Code"],
    sizes: [11, 12, 13, 14, 15, 16, 17, 18],
    shortcuts: shortcutsOf(fake),
  };
}

function commit(fake: FakeInterface): InterfaceView {
  const view = viewOf(fake);
  void emit(EVENTS.interface, structuredClone(view));
  return view;
}

function setShortcut(fake: FakeInterface, args: Record<string, unknown>): ShortcutChange {
  const id = String(args.action);
  const meta = ACTIONS[id] ?? fail("notFound", `Action « ${id} » inconnue.`);
  if (!meta[3]) fail("invalidRequest", `Le raccourci de « ${meta[0]} » ne se change pas.`);
  const keys = typeof args.keys === "string" ? args.keys : null;
  if (keys === null) {
    fake.overrides.set(id, null);
    return { view: commit(fake), conflict: null };
  }
  if (RESERVED.has(keys) || /^Ctrl\+\d$/.test(keys)) fail("invalidRequest", `${keys} est réservé.`);
  const scope = meta[2];
  const holders = shortcutsOf(fake).filter(
    (s) =>
      s.id !== id &&
      s.keys.includes(keys) &&
      (s.scope === scope || s.scope === "global" || scope === "global"),
  );
  const holder = holders[0];
  if (holder && (args.replace !== true || !holder.editable)) {
    return {
      view: viewOf(fake),
      conflict: { keys, action: holder.id, label: holder.label },
    };
  }
  for (const other of holders) fake.overrides.set(other.id, null);
  const defaults = DEFAULT_SHORTCUTS[id as keyof typeof DEFAULT_SHORTCUTS];
  if (defaults.length === 1 && defaults[0] === keys) fake.overrides.delete(id);
  else fake.overrides.set(id, keys);
  return { view: commit(fake), conflict: null };
}

function historyKey(args: Record<string, unknown>): string {
  return `${String(args.project)}|${String(args.path)}`;
}

/** Résumé simplifié de `history::compare` : statut, en-têtes disparus, durée. */
function changesOf(entry: HistoryEntry, previous: HistoryEntry): string[] {
  const out: string[] = [];
  if (entry.status !== previous.status)
    out.push(`statut ${String(entry.status)} au lieu de ${String(previous.status)}`);
  for (const [name] of previous.headers) {
    if (!entry.headers.some(([n]) => n.toLowerCase() === name.toLowerCase()))
      out.push(`l'en-tête ${name} a disparu`);
  }
  if (entry.body !== previous.body) out.push("le corps a changé");
  const delta = entry.totalMs - previous.totalMs;
  if (delta > 0) out.push(`${String(delta)} ms de plus`);
  if (delta < 0) out.push(`${String(-delta)} ms de moins`);
  return out;
}

function historyView(fake: FakeInterface, args: Record<string, unknown>): HistoryView {
  const entries = fake.history.get(historyKey(args)) ?? [];
  return {
    path: String(args.path),
    max: 20,
    items: entries.map((entry, i) => {
      const previous = entries[i + 1];
      return {
        entry: structuredClone(entry),
        previousAt: previous?.at ?? null,
        changes: previous ? changesOf(entry, previous) : [],
      };
    }),
  };
}

let counter = 0;

/** Garde un envoi, comme `request_send` (H1). */
export function recordSend(
  fake: FakeInterface,
  args: Record<string, unknown>,
  outcome: SendOutcome,
): void {
  if (typeof args.path !== "string") return;
  const key = historyKey(args);
  const entries = fake.history.get(key) ?? [];
  entries.unshift({
    id: `envoi-${String(++counter)}`,
    at: Date.now(),
    environment: outcome.sent?.environment ?? null,
    status: outcome.response.status,
    statusText: outcome.response.statusText,
    totalMs: outcome.response.totalMs,
    sizeBytes: outcome.response.sizeBytes,
    passed: outcome.passed,
    assertions: structuredClone(outcome.assertions),
    headers: structuredClone(outcome.response.headers),
    body: outcome.response.body,
    truncated: outcome.response.truncated,
    sent: outcome.sent
      ? {
          method: outcome.sent.method,
          url: outcome.sent.url,
          headers: structuredClone(outcome.sent.headers),
          body: outcome.sent.body,
        }
      : null,
  });
  fake.history.set(key, entries.slice(0, 20));
}

function historyDiff(fake: FakeInterface, args: Record<string, unknown>): HistoryDiff {
  const entries = fake.history.get(historyKey(args)) ?? [];
  const index = entries.findIndex((e) => e.id === args.id);
  const entry = entries[index];
  const previous = entries[index + 1];
  if (!entry || !previous)
    fail("notFound", "Cet envoi n'a pas d'envoi précédent dans l'historique.");
  const lines: HistoryDiff["lines"] = [];
  if (entry.status !== previous.status) {
    lines.push({
      part: "status",
      added: false,
      text: `${String(previous.status)} ${previous.statusText}`,
    });
    lines.push({
      part: "status",
      added: true,
      text: `${String(entry.status)} ${entry.statusText}`,
    });
  }
  for (const [name, value] of previous.headers) {
    if (!entry.headers.some(([n]) => n === name))
      lines.push({ part: "header", added: false, text: `${name}: ${value}` });
  }
  if (entry.body !== previous.body) {
    lines.push({ part: "body", added: false, text: previous.body });
    lines.push({ part: "body", added: true, text: entry.body });
  }
  return { previousId: previous.id, lines, truncated: false };
}

export function handleInterface(
  fake: FakeInterface,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "interface_load":
      return viewOf(fake);
    case "interface_save_appearance":
      fake.appearance = structuredClone(args.appearance as Appearance);
      return commit(fake);
    case "interface_set_shortcut":
      return setShortcut(fake, args);
    case "interface_reset_shortcuts":
      fake.overrides.clear();
      return commit(fake);
    case "history_load":
      return historyView(fake, args);
    case "history_diff":
      return historyDiff(fake, args);
    case "history_clear":
      fake.history.delete(historyKey(args));
      return historyView(fake, args);
    case "history_restore": {
      const key = historyKey(args);
      const kept = fake.history.get(key) ?? [];
      const restored = (args.entries as HistoryEntry[]).filter(
        (e) => !kept.some((k) => k.id === e.id),
      );
      fake.history.set(key, [...kept, ...restored].sort((a, b) => b.at - a.at).slice(0, 20));
      return historyView(fake, args);
    }
    default:
      return undefined;
  }
}
