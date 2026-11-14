/**
 * Faux cœur de « Copier ou déplacer » (lot J, écrans 4.1-f et 4.1-g), en mémoire. Les règles
 * (noms libres, rangs d'étapes, lien au contrat, « Ce qui change ») sont testées côté Rust
 * (`tree/transfer_tests.rs`) ; ici, le faux établi est commun aux projets.
 */
import type {
  RequestDoc,
  TransferChange,
  TransferDone,
  TransferTarget,
  TransferTargets,
} from "@/types/ipc";

import type { FakeContracts } from "./fakeContracts";
import type { FakeWorkbench } from "./fakeWorkbench";

/** Fichiers à remettre (« Annuler »), par jeton : chemin et requête d'avant (`null` : créée). */
const undoBy = new Map<string, [string, RequestDoc | null][]>();

function parentOf(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

function fileOf(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** Dossiers connus : déclarés ou parents d'une requête. */
function folders(wb: FakeWorkbench): string[] {
  const all = new Set(wb.folders);
  for (const path of wb.requests.keys()) {
    let p = parentOf(path);
    while (p !== "") {
      all.add(p);
      p = parentOf(p);
    }
  }
  return [...all].sort((a, b) => a.localeCompare(b));
}

/** Étapes d'un test, dans l'ordre (`seq`, puis le chemin). */
function steps(wb: FakeWorkbench, test: string): string[] {
  return [...wb.requests.entries()]
    .filter(([p, r]) => parentOf(p) === test && !r.contract)
    .sort(([a, ra], [b, rb]) => (ra.seq ?? 0) - (rb.seq ?? 0) || a.localeCompare(b))
    .map(([p]) => p);
}

function isTest(wb: FakeWorkbench, folder: string): boolean {
  const all = folders(wb);
  return !all.some((f) => parentOf(f) === folder) && steps(wb, folder).length > 0;
}

function targets(
  wb: FakeWorkbench,
  contracts: FakeContracts,
  path: string,
  same: boolean,
): TransferTargets {
  const request = wb.requests.get(path);
  const link = same ? request?.contract : undefined;
  const method = request?.method ?? "GET";
  const operations = [...contracts.files.entries()].flatMap(([file, c]) =>
    c.operations
      .filter((o) => o.method === method)
      .map((o) => ({
        contract: file,
        contractName: c.deduced ? c.name : `${c.name} v${c.version}`,
        operation: o.key,
        current: link?.file === file && link.operation === o.key,
      })),
  );
  const here = same && !request?.contract ? parentOf(path) : null;
  const all = folders(wb).filter((f) => !f.startsWith("contracts"));
  return {
    method,
    operations,
    folders: [
      { path: "", current: here === "" },
      ...all.filter((f) => !isTest(wb, f)).map((f) => ({ path: f, current: here === f })),
    ],
    tests: all
      .filter((f) => isTest(wb, f))
      .map((f) => {
        const list = steps(wb, f);
        const index = list.indexOf(path);
        return {
          path: f,
          name: fileOf(f),
          steps: list.map((p) => wb.requests.get(p)?.name ?? p),
          current: index < 0 ? null : index,
        };
      }),
  };
}

function preview(
  wb: FakeWorkbench,
  path: string,
  destination: TransferTarget,
  mode: string,
): TransferChange[] {
  const request = wb.requests.get(path);
  const out: TransferChange[] = [];
  if (destination.kind === "step") {
    const count = steps(wb, destination.test).filter((p) => p !== path).length;
    const rank = Math.min(destination.position, count) + 1;
    out.push({
      title: `Devient l'étape ${String(rank)} de « ${fileOf(destination.test)} »`,
      detail:
        rank <= count
          ? `L'actuelle étape ${String(rank)} devient la ${String(rank + 1)}.`
          : "Dernière étape du test.",
      warning: false,
    });
  } else if (destination.kind === "folder") {
    out.push({
      title: `Rangée dans ${destination.folder === "" ? "la racine" : `« ${destination.folder} »`}`,
      detail: "Une requête du référentiel de tests, hors de tout test.",
      warning: false,
    });
  } else {
    out.push({
      title: `Liée à ${destination.operation}`,
      detail: "Conformité vérifiée à chaque envoi.",
      warning: false,
    });
  }
  if (request?.contract && destination.kind !== "operation") {
    out.push({
      title: `N'est plus liée à ${request.contract.operation}`,
      detail:
        "Une étape ou une requête du référentiel n'est jamais liée à un contrat : la conformité n'est plus vérifiée.",
      warning: true,
    });
  }
  out.push({
    title: mode === "copy" ? "Copie" : "Déplacement",
    detail: `« ${request?.name ?? path} » ${mode === "copy" ? "reste aussi à sa place" : "quitte sa place"}.`,
    warning: false,
  });
  return out;
}

function apply(
  wb: FakeWorkbench,
  contracts: FakeContracts,
  path: string,
  destination: TransferTarget,
  mode: string,
): TransferDone {
  const request = wb.requests.get(path);
  if (!request) throw { kind: "notFound", message: "Cette requête n'existe plus." };
  const touched: [string, RequestDoc | null][] = [];
  const folder =
    destination.kind === "folder"
      ? destination.folder
      : destination.kind === "step"
        ? destination.test
        : (contracts.files.get(destination.contract)?.name ?? "Contrat");
  const beside = parentOf(path) === folder;
  const inPlace = mode === "move" && beside;
  let name = request.name;
  let to = path;
  if (!inPlace) {
    name = beside ? `${request.name} (copie)` : request.name;
    const base = fileOf(path).replace(/\.yml$/, "");
    to = `${folder === "" ? "" : `${folder}/`}${beside ? `${base}-copie` : base}.yml`;
    let n = 2;
    while (wb.requests.has(to)) to = to.replace(/(-\d+)?\.yml$/, `-${String(n++)}.yml`);
  }
  const moved: RequestDoc = structuredClone(request);
  moved.name = name;
  if (destination.kind === "operation")
    moved.contract = { file: destination.contract, operation: destination.operation };
  else delete moved.contract;
  delete moved.seq;
  touched.push([to, inPlace ? structuredClone(request) : null]);
  wb.requests.set(to, moved);
  if (destination.kind === "step") {
    const order = steps(wb, destination.test).filter((p) => p !== to);
    order.splice(Math.min(destination.position, order.length), 0, to);
    order.forEach((p, i) => {
      const step = wb.requests.get(p);
      if (!step) return;
      if (p !== to) touched.push([p, structuredClone(step)]);
      step.seq = i + 1;
    });
  }
  if (mode === "move" && !inPlace) {
    touched.push([path, structuredClone(request)]);
    wb.requests.delete(path);
  }
  const token = `transfert-${String(undoBy.size + 1)}`;
  undoBy.set(token, touched);
  return { projectId: "", path: to, name, undoToken: token };
}

/** Commandes de « Copier ou déplacer » ; `undefined` si `cmd` n'en est pas une. */
export function handleTransfer(
  wb: FakeWorkbench,
  contracts: FakeContracts,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  const path = String(args.path);
  switch (cmd) {
    case "transfer_targets":
      return targets(wb, contracts, path, args.project === args.target);
    case "transfer_preview":
      return preview(wb, path, args.destination as TransferTarget, String(args.mode));
    case "transfer_apply":
      return {
        ...apply(wb, contracts, path, args.destination as TransferTarget, String(args.mode)),
        projectId: String(args.target),
      };
    case "transfer_undo": {
      const entries = undoBy.get(String(args.undoToken));
      if (!entries) throw { kind: "undoExpired", message: "Ne peut plus être annulé." };
      undoBy.delete(String(args.undoToken));
      for (const [p, before] of [...entries].reverse()) {
        if (before === null) wb.requests.delete(p);
        else wb.requests.set(p, before);
      }
      return null;
    }
    default:
      return undefined;
  }
}
