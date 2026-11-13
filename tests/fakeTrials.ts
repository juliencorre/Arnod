/**
 * Faux cœur des essais rangés (écrans 2.2-b et 2.2-c) : chemin généralisé et transfert au
 * référentiel de tests, en mémoire. Les règles sont testées côté Rust (`trial/generalize.rs`,
 * `trial/tests.rs`) ; ici, un segment fait de chiffres suffit à proposer `{id}`.
 */
import type { PathGeneralization, TransferredTrial } from "@/types/ipc";

import type { FakeContracts } from "./fakeContracts";
import type { FakeWorkbench } from "./fakeWorkbench";

/** Opérations gardées telles quelles (« Garder tel quel »), par faux cœur. */
const keptBy = new WeakMap<FakeWorkbench, Set<string>>();

function keptOf(wb: FakeWorkbench): Set<string> {
  const set = keptBy.get(wb) ?? new Set<string>();
  keptBy.set(wb, set);
  return set;
}

function suggestion(wb: FakeWorkbench, path: string): PathGeneralization | null {
  const kept = keptOf(wb);
  const link = wb.requests.get(path)?.contract;
  if (!link || kept.has(`${link.file}#${link.operation}`)) return null;
  const route = link.operation.slice(link.operation.indexOf(" ") + 1);
  const segments = route.split("/");
  let last = -1;
  segments.forEach((s, i) => {
    if (/^\d+$/.test(s)) last = i;
  });
  if (last < 0) return null;
  const value = segments[last] ?? "";
  segments[last] = "{id}";
  return {
    path: route,
    template: { path: segments.join("/"), params: [{ name: "id", value }] },
    contract: link.file,
  };
}

/** Commandes des essais rangés ; `undefined` si `cmd` n'en est pas une. */
export function handleTrials(
  wb: FakeWorkbench,
  contracts: FakeContracts,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  const path = String(args.path);
  switch (cmd) {
    case "trial_generalization":
      return suggestion(wb, path);
    case "trial_generalize": {
      const proposed = suggestion(wb, path);
      const request = wb.requests.get(path);
      if (!proposed || !request?.contract) throw new Error("rien à généraliser");
      const method = request.contract.operation.split(" ")[0] ?? "GET";
      const key = `${method} ${proposed.template.path}`;
      const contract = contracts.files.get(proposed.contract);
      const operation = contract?.operations.find((o) => o.key === request.contract?.operation);
      if (operation) {
        operation.key = key;
        operation.path = proposed.template.path;
      }
      request.url = request.url.replace(proposed.path, proposed.template.path);
      request.contract = { file: proposed.contract, operation: key };
      request.params = [
        ...request.params.filter((p) => p.kind !== "path"),
        ...proposed.template.params.map((p) => ({
          name: p.name,
          value: p.value,
          enabled: true,
          kind: "path" as const,
          description: "",
        })),
      ];
      return [path];
    }
    case "trial_keep_path": {
      const link = wb.requests.get(path)?.contract;
      if (link) keptOf(wb).add(`${link.file}#${link.operation}`);
      return null;
    }
    case "trial_transfer": {
      const name = String(args.name);
      const parent = String(args.parent);
      const test = parent === "" ? name : `${parent}/${name}`;
      if (wb.folders.has(test)) {
        throw { kind: "nameTaken", message: `« ${name} » existe déjà à cet endroit.` };
      }
      const request = wb.requests.get(path);
      if (!request) throw { kind: "notFound", message: "Introuvable." };
      const step = `${test}/${path.slice(path.lastIndexOf("/") + 1)}`;
      wb.folders.add(test);
      wb.requests.delete(path);
      delete request.contract;
      request.seq = 1;
      wb.requests.set(step, request);
      return { test, step } satisfies TransferredTrial;
    }
    default:
      return undefined;
  }
}
