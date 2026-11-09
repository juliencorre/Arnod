/**
 * Faux cœur des contrats de services : en mémoire, sans lecture OpenAPI (les
 * règles sont testées côté Rust, `contract/tests.rs`). Un texte est un contrat
 * s'il contient `openapi:` ; son titre est lu sur la ligne `title:`.
 */
import type {
  CommandError,
  ContractSource,
  ContractSummary,
  ContractView,
  GenerateReport,
  OperationView,
} from "@/types/ipc";

import { newRequest, runOf, type FakeWorkbench } from "./fakeWorkbench";

/** Opération d'un faux contrat. */
export type FakeOperation = Pick<OperationView, "key" | "method" | "path" | "summary" | "tag">;

export interface FakeContract {
  name: string;
  version: string;
  source: string;
  operations: FakeOperation[];
}

export interface FakeContracts {
  /** Contrats par chemin (`contracts/petstore.yaml`). */
  files: Map<string, FakeContract>;
  /** Réponse du sélecteur de fichier : texte du fichier choisi, ou `null` (annulé). */
  pickedFile: string | null;
}

/** Opérations du contrat de démonstration. */
export const PETSTORE_OPERATIONS: FakeOperation[] = [
  { key: "GET /pets", method: "GET", path: "/pets", summary: "Lister les animaux", tag: "animaux" },
  { key: "POST /pets", method: "POST", path: "/pets", summary: "Créer un animal", tag: "animaux" },
  { key: "GET /health", method: "GET", path: "/health", summary: "Santé", tag: null },
];

export const PETSTORE_SOURCE = "openapi: 3.0.3\ninfo:\n  title: Petstore\n  version: 1.0.0\n";

export function createFakeContracts(): FakeContracts {
  return { files: new Map(), pickedFile: null };
}

const fail = (kind: CommandError["kind"], message: string): never => {
  throw { kind, message } satisfies CommandError;
};

function slug(name: string): string {
  return (
    name
      .toLowerCase()
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "contrat"
  );
}

/** Lecture simplifiée : `title:` et `version:` du texte. */
function parse(text: string): FakeContract {
  if (!text.includes("openapi:")) {
    fail(
      "invalidRequest",
      "Ce texte n'est pas un contrat lisible : ce n'est pas un contrat OpenAPI : champ « openapi » ou « swagger » absent.",
    );
  }
  const read = (field: string): string =>
    new RegExp(`^\\s*${field}:\\s*"?([^"\\n]+)"?`, "m").exec(text)?.[1]?.trim() ?? "";
  return {
    name: read("title") || "Contrat sans titre",
    version: read("version"),
    source: text,
    operations: structuredClone(PETSTORE_OPERATIONS),
  };
}

function linked(wb: FakeWorkbench, path: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const [requestPath, request] of wb.requests) {
    if (request.contract?.file === path) found.set(request.contract.operation, requestPath);
  }
  return found;
}

function view(fc: FakeContracts, wb: FakeWorkbench, path: string): ContractView {
  const contract = fc.files.get(path) ?? fail("notFound", "Ce contrat n'existe plus.");
  const links = linked(wb, path);
  const operations: OperationView[] = contract.operations.map((operation) => ({
    ...operation,
    operationId: null,
    deprecated: false,
    supported: true,
    externalRefs: false,
    request: links.get(operation.key) ?? null,
  }));
  const orphans = [...links]
    .filter(([key]) => !contract.operations.some((o) => o.key === key))
    .map(([operation, requestPath]) => ({
      operation,
      path: requestPath,
      name: wb.requests.get(requestPath)?.name ?? requestPath,
    }));
  return {
    path,
    name: contract.name,
    version: contract.version,
    kind: "openApi3",
    specVersion: "3.0.3",
    description: null,
    servers: ["http://127.0.0.1:8080"],
    source: contract.source,
    error: null,
    operations,
    orphans,
  };
}

function summaries(fc: FakeContracts): ContractSummary[] {
  return [...fc.files]
    .map(([path, contract]) => ({
      path,
      name: contract.name,
      version: contract.version,
      operationCount: contract.operations.length,
      error: null,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function add(fc: FakeContracts, contract: FakeContract): string {
  const base = slug(contract.name);
  let path = `contracts/${base}.yaml`;
  for (let n = 2; fc.files.has(path); n++) path = `contracts/${base}-${String(n)}.yaml`;
  fc.files.set(path, contract);
  return path;
}

function template(name: string): FakeContract {
  return {
    name,
    version: "1.0.0",
    source: `openapi: 3.0.3\ninfo:\n  title: "${name}"\n  version: 1.0.0\n`,
    operations: [
      { key: "GET /sante", method: "GET", path: "/sante", summary: "État du service", tag: null },
    ],
  };
}

function generate(
  fc: FakeContracts,
  wb: FakeWorkbench,
  path: string,
  keys: string[],
  environment: string | null,
): GenerateReport {
  const contract = fc.files.get(path) ?? fail("notFound", "Ce contrat n'existe plus.");
  const links = linked(wb, path);
  const report: GenerateReport = { folder: contract.name, created: [], skipped: [], baseUrl: null };
  for (const operation of contract.operations.filter((o) => keys.includes(o.key))) {
    if (links.has(operation.key)) {
      report.skipped.push({
        operation: operation.key,
        reason: "une requête existe déjà pour cette opération",
      });
      continue;
    }
    const folder = operation.tag ? `${contract.name}/${operation.tag}` : contract.name;
    wb.folders.add(contract.name);
    wb.folders.add(folder);
    const name = operation.summary ?? operation.key;
    const requestPath = `${folder}/${slug(name)}.yml`;
    wb.requests.set(
      requestPath,
      newRequest(name, {
        method: operation.method as "GET",
        url: `{{baseUrl}}${operation.path}`,
        contract: { file: path, operation: operation.key },
        assertions: [
          {
            id: "duree",
            target: "responseTime",
            operator: "lessThan",
            value: 500,
            origin: "default",
          },
          { id: "statut", target: "status", operator: "equals", value: 200, origin: "default" },
          { id: "contrat", target: "response", operator: "matchesContract", origin: "default" },
        ],
      }),
    );
    report.created.push(requestPath);
  }
  const env = wb.envs.find((e) => e.name === environment);
  const baseUrl = env?.variables.find((v) => v.name === "baseUrl");
  if (report.created.length > 0 && env && baseUrl?.value === "") {
    baseUrl.value = "http://127.0.0.1:8080";
    report.baseUrl = { environment: env.name, value: baseUrl.value };
  }
  return report;
}

/** Répond aux commandes des contrats ; `undefined` pour une autre commande. */
export function handleContracts(
  fc: FakeContracts,
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  const path = String(args.path);
  switch (cmd) {
    case "contract_list":
      return summaries(fc);
    case "contract_read":
      return view(fc, wb, path);
    case "contract_add": {
      const source = args.source as ContractSource;
      if (source.kind === "url") {
        return fail(
          "network",
          "Impossible de joindre exemple.invalid. Vérifiez l'adresse et la connexion.",
        );
      }
      const contract = source.kind === "template" ? template(source.name) : parse(source.text);
      return view(fc, wb, add(fc, contract));
    }
    case "contract_import_file": {
      if (fc.pickedFile === null) return null;
      const contract = parse(fc.pickedFile);
      const replace = args.replace as string | null;
      if (replace !== null) {
        fc.files.set(replace, contract);
        return view(fc, wb, replace);
      }
      return view(fc, wb, add(fc, contract));
    }
    case "contract_save": {
      const contract = parse(String(args.text));
      const current = fc.files.get(path) ?? fail("notFound", "Ce contrat n'existe plus.");
      fc.files.set(path, { ...contract, operations: current.operations });
      return view(fc, wb, path);
    }
    case "contract_delete":
      fc.files.delete(path);
      return null;
    case "contract_generate":
      return generate(
        fc,
        wb,
        path,
        args.operations as string[],
        (args.environment as string | null) ?? null,
      );
    case "contract_run": {
      const paths = [...linked(wb, path).values()];
      const run = { ...runOf(wb, "", paths), contract: fc.files.get(path)?.name ?? path };
      wb.lastRun = run;
      return run;
    }
    default:
      return undefined;
  }
}
