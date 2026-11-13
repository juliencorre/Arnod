/**
 * Lot I : faux cœur des fichiers modifiés hors d'Arnod (`request_conflict`,
 * `request_conflict_resolve`). La comparaison est simplifiée (adresse, méthode, nom, corps,
 * en-têtes, vérifications) : les règles complètes sont testées côté Rust (`watch/tests.rs`).
 */
import type {
  ConflictItem,
  ConflictSide,
  FilesChange,
  RequestConflict,
  RequestDoc,
} from "@/types/ipc";

import type { FakeWorkbench } from "./fakeWorkbench";

/** Date de modification renvoyée par le faux cœur (15:04 heure locale, un jour fixe). */
export const FICHIERS_LOT_I_MODIFIED_AT = new Date(2026, 10, 13, 15, 4).getTime();

type Elements = Map<string, { label: string; value: string }>;

function elementsLotI(request: RequestDoc): Elements {
  const out: Elements = new Map();
  out.set("name", { label: "Nom", value: request.name });
  out.set("method", { label: "Méthode", value: request.method });
  out.set("url", { label: "Adresse", value: request.url });
  const content = request.body.variants[request.body.selected]?.content ?? "";
  out.set("body", {
    label: "Corps",
    value: content ? `${request.body.type} · ${content}` : request.body.type,
  });
  for (const header of request.headers) {
    out.set(`headers:${header.name.toLowerCase()}#0`, {
      label: `En-tête ${header.name}`,
      value: header.value,
    });
  }
  for (const assertion of request.assertions) {
    const value =
      assertion.value === undefined || assertion.value === null ? "" : ` ${String(assertion.value)}`;
    out.set(`assertions:${assertion.target}#0`, {
      label: `Vérification ${assertion.target}`,
      value: `${assertion.operator === "equals" ? "égal à" : assertion.operator}${value}`,
    });
  }
  return out;
}

export function compareLotI(
  base: RequestDoc,
  disk: RequestDoc,
  draft: RequestDoc,
): ConflictItem[] {
  const [b, d, m] = [elementsLotI(base), elementsLotI(disk), elementsLotI(draft)];
  const keys = [...new Set([...d.keys(), ...m.keys()])];
  const different: ConflictItem[] = [];
  const same: ConflictItem[] = [];
  for (const key of keys) {
    const onDisk = d.get(key);
    const inDraft = m.get(key);
    const atStart = b.get(key)?.value;
    const item: ConflictItem = {
      key,
      label: onDisk?.label ?? inDraft?.label ?? key,
      disk: onDisk?.value ?? null,
      draft: inDraft?.value ?? null,
      identical: false,
      suggested: null,
    };
    if (item.disk !== item.draft) {
      if (item.draft === (atStart ?? null)) item.suggested = "disk";
      else if (item.disk === (atStart ?? null)) item.suggested = "draft";
      different.push(item);
    } else if (item.disk !== (atStart ?? null)) {
      same.push({ ...item, identical: true });
    }
  }
  return [...different, ...same];
}

/** Fusion simplifiée : champs simples et lignes prises du disque selon les choix. */
function mergeLotI(
  disk: RequestDoc,
  draft: RequestDoc,
  choices: Record<string, ConflictSide>,
): RequestDoc {
  const merged = structuredClone(draft);
  const fromDisk = (key: string): boolean => choices[key] === "disk";
  if (fromDisk("name")) merged.name = disk.name;
  if (fromDisk("method")) merged.method = disk.method;
  if (fromDisk("url")) merged.url = disk.url;
  if (fromDisk("body")) merged.body = structuredClone(disk.body);
  const headerKey = (name: string): string => `headers:${name.toLowerCase()}#0`;
  merged.headers = [
    ...merged.headers.flatMap((h) => {
      if (!fromDisk(headerKey(h.name))) return [h];
      const theirs = disk.headers.find((x) => headerKey(x.name) === headerKey(h.name));
      return theirs ? [theirs] : [];
    }),
    ...disk.headers.filter(
      (h) =>
        fromDisk(headerKey(h.name)) &&
        !draft.headers.some((x) => headerKey(x.name) === headerKey(h.name)),
    ),
  ];
  const checkKey = (target: string): string => `assertions:${target}#0`;
  merged.assertions = [
    ...merged.assertions.flatMap((a) => {
      if (!fromDisk(checkKey(a.target))) return [a];
      const theirs = disk.assertions.find((x) => x.target === a.target);
      return theirs ? [theirs] : [];
    }),
    ...disk.assertions.filter(
      (a) => fromDisk(checkKey(a.target)) && !draft.assertions.some((x) => x.target === a.target),
    ),
  ];
  return merged;
}

/** Commandes du lot I ; `undefined` pour une autre commande. */
export function handleFilesLotI(
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "request_conflict": {
      const disk = wb.requests.get(String(args.path));
      if (!disk) throw { kind: "notFound", message: "La requête n'existe plus." };
      const conflict: RequestConflict = {
        items: compareLotI(args.base as RequestDoc, disk, args.draft as RequestDoc),
        modifiedAt: FICHIERS_LOT_I_MODIFIED_AT,
      };
      return conflict;
    }
    case "request_conflict_resolve": {
      const path = String(args.path);
      const disk = wb.requests.get(path);
      if (!disk) throw { kind: "notFound", message: "La requête n'existe plus." };
      const merged = mergeLotI(
        disk,
        args.draft as RequestDoc,
        args.choices as Record<string, ConflictSide>,
      );
      wb.requests.set(path, structuredClone(merged));
      return structuredClone(merged);
    }
    default:
      return undefined;
  }
}

/** Lot de changements faits hors d'Arnod, tel que le cœur le diffuse. */
export function filesChangeLotI(
  projectId: string,
  change: Partial<Omit<FilesChange, "projectId">>,
): FilesChange {
  return {
    projectId,
    added: [],
    modified: [],
    removed: [],
    renamed: [],
    tree: true,
    environments: false,
    contracts: false,
    ...change,
  };
}
