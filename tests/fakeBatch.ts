/**
 * Faux cœur de la sélection multiple (écran 4.1-a quater), en mémoire. Les règles (éléments pris
 * une fois, annulation complète en cas d'échec, dossiers dupliqués) sont testées côté Rust
 * (`tree/batch_tests.rs`).
 */
import type { BatchAction, BatchResult, RequestDoc } from "@/types/ipc";

import { type FakeWorkbench, runOf } from "./fakeWorkbench";

interface Snapshot {
  requests: [string, RequestDoc][];
  folders: string[];
}

const undoBy = new Map<string, Snapshot>();

function snapshot(wb: FakeWorkbench): Snapshot {
  return {
    requests: [...wb.requests.entries()].map(([p, r]) => [p, structuredClone(r)]),
    folders: [...wb.folders],
  };
}

function baseName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** Chemins distincts, sans ceux qu'un dossier choisi contient (règle du cœur). */
function normalize(paths: string[]): string[] {
  return [...new Set(paths)].filter((p) => !paths.some((o) => p.startsWith(`${o}/`)));
}

function batch(wb: FakeWorkbench, paths: string[], action: BatchAction): BatchResult {
  const before = snapshot(wb);
  const items: [string, string | null][] = [];
  for (const path of normalize(paths)) {
    const inside = [...wb.requests.keys()].filter((k) => k === path || k.startsWith(`${path}/`));
    if (action.kind === "delete") {
      for (const key of inside) wb.requests.delete(key);
      wb.folders.delete(path);
      items.push([path, null]);
    } else if (action.kind === "move") {
      const to = action.target === "" ? baseName(path) : `${action.target}/${baseName(path)}`;
      for (const key of inside) {
        const request = wb.requests.get(key);
        wb.requests.delete(key);
        if (request) wb.requests.set(to + key.slice(path.length), request);
      }
      if (wb.folders.delete(path)) wb.folders.add(to);
      items.push([path, to]);
    } else {
      const to = wb.folders.has(path) ? `${path} (copie)` : path.replace(".yml", "-copie.yml");
      for (const key of inside) {
        const request = wb.requests.get(key);
        if (request) wb.requests.set(to + key.slice(path.length), structuredClone(request));
      }
      if (wb.folders.has(path)) wb.folders.add(to);
      items.push([path, to]);
    }
  }
  const token = `selection-${String(undoBy.size + 1)}`;
  undoBy.set(token, before);
  return { items, undoToken: token };
}

/** Commandes de la sélection multiple ; `undefined` si `cmd` n'en est pas une. */
export function handleBatch(
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "tree_batch":
      return batch(wb, args.paths as string[], args.action as BatchAction);
    case "tree_batch_undo": {
      const before = undoBy.get(String(args.undoToken));
      if (!before) throw { kind: "undoExpired", message: "Ne peut plus être annulé." };
      undoBy.delete(String(args.undoToken));
      wb.requests = new Map(before.requests);
      wb.folders = new Set(before.folders);
      return null;
    }
    case "run_selection": {
      const chosen = normalize(args.paths as string[]);
      const only = [...wb.requests.keys()].filter((k) =>
        chosen.some((p) => k === p || k.startsWith(`${p}/`)),
      );
      const run = runOf(wb, "", only);
      wb.lastRun = run;
      return structuredClone(run);
    }
    default:
      return undefined;
  }
}
