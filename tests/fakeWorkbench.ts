/**
 * Faux cœur pour l'arborescence, les requêtes, les environnements et les runs :
 * en mémoire, sans les règles métier (testées côté Rust).
 */
import type {
  Assertion,
  Example,
  CommandError,
  EnvironmentDoc,
  EnvironmentView,
  EnvironmentsView,
  ProjectLastRun,
  RequestDoc,
  RequestResult,
  RequestStatuses,
  RunGroup,
  RunItem,
  RunView,
  SendOutcome,
  TreeNode,
} from "@/types/ipc";

export function newRequest(name: string, overrides: Partial<RequestDoc> = {}): RequestDoc {
  return {
    name,
    description: "",
    tags: [],
    method: "GET",
    url: "{{baseUrl}}/",
    params: [],
    headers: [],
    auth: {
      type: "none",
      username: "",
      password: "",
      token: "",
      name: "",
      value: "",
      location: "header",
      other: "",
    },
    body: { kind: "none", variants: [], selected: 0, other: "" },
    settings: { timeoutMs: null, followRedirects: null, maxRedirects: null, encodeUrl: null },
    docs: "",
    assertions: [
      {
        id: "duree",
        target: "responseTime",
        operator: "lessThan",
        value: 500,
        origin: "default",
        enabled: true,
      },
    ],
    examples: [],
    preserved: [],
    inherited: {
      auth: null,
      headers: [],
      settings: { timeoutMs: 30000, followRedirects: true, maxRedirects: 5, encodeUrl: true },
    },
    ...overrides,
  };
}

export function okOutcome(overrides: Partial<SendOutcome> = {}): SendOutcome {
  return {
    response: {
      status: 200,
      statusText: "OK",
      headers: [["content-type", "application/json"]],
      body: '{"id":1,"name":"Alice"}',
      truncated: false,
      sizeBytes: 23,
      waitMs: 40,
      receiveMs: 5,
      totalMs: 45,
    },
    assertions: [
      {
        id: "duree",
        target: "responseTime",
        passed: true,
        message: null,
        actual: "45",
        ignored: false,
      },
    ],
    passed: true,
    slow: false,
    ...overrides,
  };
}

export interface FakeWorkbench {
  /** Requêtes par chemin ; les dossiers se déduisent des chemins et de `folders`. */
  requests: Map<string, RequestDoc>;
  folders: Set<string>;
  envs: EnvironmentDoc[];
  active: string;
  collapsed: string[];
  /** Réponse du prochain envoi ; une erreur est levée telle quelle. */
  nextSend: SendOutcome | CommandError;
  lastRun: RunView | null;
  /** Résultat des requêtes d'un run, par chemin (réussie par défaut). */
  runStatus: Map<string, "passed" | "failed" | "error">;
  /** Réponse de `project_last_runs`. */
  projectLastRuns: ProjectLastRun[];
  /** Derniers résultats gardés par le cœur, par chemin (`request_result`, R5). */
  results: Map<string, RequestResult>;
  /** Statut de la dernière exécution de chaque requête (`request_statuses`). */
  statuses: RequestStatuses;
  /** Vérifications en échec des lignes d'un run, par chemin. */
  runChecks: Map<string, RunItem["assertions"]>;
  /** Numéro du dernier run. */
  runNumber: number;
}

export function createFakeWorkbench(): FakeWorkbench {
  return {
    requests: new Map(),
    folders: new Set(),
    envs: [
      {
        name: "Local",
        color: "gray",
        variables: [{ name: "baseUrl", value: "" }],
        locals: [],
      },
    ],
    active: "Local",
    collapsed: [],
    nextSend: okOutcome(),
    results: new Map(),
    lastRun: null,
    runStatus: new Map(),
    projectLastRuns: [],
    statuses: {},
    runChecks: new Map(),
    runNumber: 0,
  };
}

/** Copie profonde tolérante aux proxys réactifs de Vue (comme la sérialisation JSON de l'IPC). */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const fail = (kind: CommandError["kind"], message: string): never => {
  throw { kind, message } satisfies CommandError;
};

export function buildTree(wb: FakeWorkbench): TreeNode[] {
  const build = (parent: string): TreeNode[] => {
    const prefix = parent === "" ? "" : `${parent}/`;
    const folders = [...wb.folders]
      .filter((f) => f.startsWith(prefix) && !f.slice(prefix.length).includes("/"))
      .sort((a, b) => a.localeCompare(b));
    const nodes: TreeNode[] = folders.map((path) => {
      const children = build(path);
      const count = [...wb.requests.keys()].filter((r) => r.startsWith(`${path}/`)).length;
      return {
        kind: "folder",
        name: path.split("/").at(-1) ?? path,
        path,
        children,
        requestCount: count,
      };
    });
    for (const [path, request] of [...wb.requests.entries()].sort(([a], [b]) =>
      a.localeCompare(b),
    )) {
      if (path.startsWith(prefix) && !path.slice(prefix.length).includes("/")) {
        nodes.push({
          kind: "request",
          name: request.name,
          path,
          method: request.method,
          assertionCount: request.assertions.length,
          invalid: false,
          contract: request.contract?.file ?? null,
        });
      }
    }
    return nodes;
  };
  return build("");
}

/** Référentiel de tests : sans les requêtes liées à un contrat présent (R1, R5). */
export function testsView(nodes: TreeNode[], contracts: Set<string>): TreeNode[] {
  const kept: TreeNode[] = [];
  for (const node of nodes) {
    if (node.kind === "request") {
      if (node.contract === null || !contracts.has(node.contract)) kept.push(node);
      continue;
    }
    const children = testsView(node.children, contracts);
    const count = (list: TreeNode[]): number =>
      list.reduce((sum, n) => sum + (n.kind === "folder" ? count(n.children) : 1), 0);
    if (node.requestCount > 0 && children.length === 0) continue;
    kept.push({ ...node, children, requestCount: count(children) });
  }
  return kept;
}

function envView(wb: FakeWorkbench): EnvironmentsView {
  const environments: EnvironmentView[] = wb.envs.map((env) => ({
    ...structuredClone(env),
    localVars: env.locals.map((name) => ({ name, secret: false, value: null, hasValue: false })),
  }));
  return { environments, active: wb.active };
}

/** Résultat par dossier, comme `run_groups` du cœur (R2). */
export function groupsOf(folder: string, items: RunItem[]): RunGroup[] {
  const groups: RunGroup[] = [];
  for (const item of items) {
    const relative = folder === "" ? item.path : item.path.slice(folder.length + 1);
    const slash = relative.indexOf("/");
    const [name, path] =
      slash >= 0
        ? [
            relative.slice(0, slash),
            folder === "" ? relative.slice(0, slash) : `${folder}/${relative.slice(0, slash)}`,
          ]
        : folder === ""
          ? ["Projet", ""]
          : [folder.split("/").at(-1) ?? folder, folder];
    let group = groups.find((g) => g.path === path);
    if (!group) {
      group = { name, path, passed: 0, failed: 0, total: 0 };
      groups.push(group);
    }
    group.total += 1;
    if (item.status === "passed") group.passed += 1;
    if (item.status === "failed" || item.status === "error") group.failed += 1;
  }
  return groups;
}

export function runOf(wb: FakeWorkbench, folder: string, only?: string[]): RunView {
  const prefix = folder === "" ? "" : `${folder}/`;
  const paths = [...wb.requests.keys()].filter(
    (p) => p.startsWith(prefix) && (!only || only.includes(p)),
  );
  wb.runNumber += 1;
  const items: RunItem[] = paths.map((path) => {
    const request = wb.requests.get(path) as RequestDoc;
    const status = wb.runStatus.get(path) ?? "passed";
    return {
      path,
      name: request.name,
      method: request.method,
      status,
      httpStatus: status === "error" ? null : status === "failed" ? 500 : 200,
      durationMs: 12,
      slow: false,
      assertions: structuredClone(wb.runChecks.get(path) ?? []),
      error: status === "error" ? "Variable « x » absente de l'environnement Local." : null,
    };
  });
  for (const item of items) wb.statuses[item.path] = { passed: item.status === "passed", at: 1 };
  return {
    id: `run-${paths.length}-${Math.random().toString(36).slice(2, 7)}`,
    number: wb.runNumber,
    folder,
    environment: wb.active,
    concurrency: 8,
    startedAt: wb.runNumber,
    finishedAt: wb.runNumber + 4100,
    finished: true,
    cancelled: false,
    contract: null,
    items,
    revision: 1,
    groups: groupsOf(folder, items),
  };
}

/** Répond aux commandes du premier usage ; `undefined` pour une commande inconnue. */
export function handleWorkbench(
  wb: FakeWorkbench,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "tree_load": {
      // Sans contrat (le faux cœur des contrats répond quand il y en a) : tout est au référentiel.
      const nodes = buildTree(wb);
      return { nodes, tests: nodes, collapsed: wb.collapsed };
    }
    case "tree_set_collapsed":
      wb.collapsed = args.collapsed as string[];
      return null;
    case "tree_create_folder": {
      const parent = String(args.parent);
      const path = parent === "" ? String(args.name) : `${parent}/${String(args.name)}`;
      if (wb.folders.has(path))
        fail("nameTaken", `Un dossier « ${String(args.name)} » existe déjà.`);
      wb.folders.add(path);
      return path;
    }
    case "tree_create_request": {
      const folder = String(args.folder);
      const name = (args.name as string | null) ?? "Nouvelle requête";
      const path = `${folder === "" ? "" : `${folder}/`}${name.toLowerCase().replaceAll(" ", "-")}.yml`;
      if ([...wb.requests.values()].some((r) => r.name.toLowerCase() === name.toLowerCase()))
        fail("nameTaken", `Une requête « ${name} » existe déjà dans ce dossier.`);
      wb.requests.set(path, newRequest(name));
      return path;
    }
    case "tree_rename": {
      const path = String(args.path);
      const request = wb.requests.get(path);
      if (request) request.name = String(args.name);
      return path;
    }
    case "tree_duplicate": {
      const path = String(args.path);
      const request = wb.requests.get(path) ?? fail("notFound", "Introuvable.");
      const copy = path.replace(".yml", "-copie.yml");
      wb.requests.set(copy, { ...structuredClone(request), name: `${request.name} (copie)` });
      return copy;
    }
    case "tree_delete": {
      const path = String(args.path);
      wb.requests.delete(path);
      wb.folders.delete(path);
      for (const key of [...wb.requests.keys()])
        if (key.startsWith(`${path}/`)) wb.requests.delete(key);
      return null;
    }
    case "tree_move":
      return String(args.path);
    case "request_read":
      return structuredClone(
        wb.requests.get(String(args.path)) ?? fail("notFound", "Introuvable."),
      );
    case "request_save":
      wb.requests.set(String(args.path), clone(args.request as RequestDoc));
      return null;
    case "request_send": {
      const path = args.path as string | null;
      if ("kind" in wb.nextSend) {
        if (path !== null) wb.statuses[path] = { passed: false, at: 1 };
        throw wb.nextSend;
      }
      const outcome = structuredClone(wb.nextSend);
      if (path !== null) {
        wb.statuses[path] = { passed: outcome.passed, at: 1 };
        wb.results.set(path, {
          projectId: String(args.project),
          path,
          sending: false,
          outcome: structuredClone(outcome),
          error: null,
          receivedAt: 1,
        });
      }
      return outcome;
    }
    case "request_statuses":
      return structuredClone(wb.statuses);
    case "result_ignore_check": {
      const path = String(args.path);
      const result = wb.results.get(path) ?? fail("notFound", "Aucun résultat.");
      const outcome = result.outcome ?? fail("notFound", "Aucun résultat.");
      const check =
        outcome.assertions.find((a) => a.id === args.check) ?? fail("notFound", "Absente.");
      check.ignored = true;
      outcome.passed = outcome.assertions.every((a) => a.passed || a.ignored);
      wb.statuses[path] = { passed: outcome.passed, at: 2 };
      return structuredClone(result);
    }
    case "request_cancel":
      return null;
    case "request_result":
      return structuredClone(wb.results.get(String(args.path)) ?? null);
    case "request_example_from_response": {
      const response = args.response as {
        status: number;
        statusText: string;
        headers: [string, string][];
        body: string;
      };
      return {
        name: String(args.name),
        description: "",
        status: response.status,
        statusText: response.statusText,
        headers: response.headers.map(([name, value]) => ({ name, value })),
        bodyKind: response.body === "" ? "none" : "json",
        body: response.body,
      } satisfies Example;
    }
    case "request_propose_assertions":
      return [
        {
          id: "status",
          target: "status",
          operator: "equals",
          value: 200,
          origin: "proposed",
          enabled: true,
        },
      ] satisfies Assertion[];
    case "environments_load":
      return envView(wb);
    case "environment_select":
      wb.active = String(args.name);
      return envView(wb);
    case "environment_save": {
      const env = clone(args.environment as EnvironmentDoc);
      const previous = (args.previous as string | null) ?? null;
      const index = wb.envs.findIndex((e) => e.name === (previous ?? env.name));
      if (index >= 0) wb.envs[index] = env;
      else wb.envs.push(env);
      return envView(wb);
    }
    case "environment_delete":
      wb.envs = wb.envs.filter((e) => e.name !== args.name);
      return envView(wb);
    case "environments_reorder":
      return envView(wb);
    case "local_set":
      return envView(wb);
    case "run_start": {
      const run = runOf(wb, String(args.folder));
      wb.lastRun = run;
      return structuredClone(run);
    }
    case "run_status":
      return structuredClone(wb.lastRun);
    case "run_cancel":
      return null;
    case "run_retry_failures": {
      const previous = wb.lastRun;
      const failed = (previous?.items ?? [])
        .filter((i) => i.status === "failed" || i.status === "error")
        .map((i) => i.path);
      wb.runStatus.clear();
      wb.lastRun = runOf(wb, previous?.folder ?? "", failed);
      return structuredClone(wb.lastRun);
    }
    case "run_last":
      return structuredClone(wb.lastRun);
    case "run_ignore_check": {
      const run = wb.lastRun ?? fail("notFound", "Ce run n'existe plus.");
      const item = run.items.find((i) => i.path === args.path) ?? fail("notFound", "Absente.");
      const check =
        item.assertions.find((a) => a.id === args.check) ?? fail("notFound", "Absente.");
      check.ignored = true;
      if (item.status === "failed" && item.assertions.every((a) => a.passed || a.ignored)) {
        item.status = "passed";
      }
      run.revision += 1;
      run.groups = groupsOf(run.folder, run.items);
      wb.statuses[item.path] = { passed: item.status === "passed", at: 2 };
      return structuredClone(run);
    }
    case "project_last_runs":
      return wb.projectLastRuns;
    default:
      return undefined;
  }
}
