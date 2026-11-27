/**
 * Faux cœur pour l'arborescence, les requêtes, les environnements et les runs :
 * en mémoire, sans les règles métier (testées côté Rust).
 */
import { emptyOAuth2 } from "@/lib/auth";
import type {
  Assertion,
  DataTable,
  Example,
  Flow,
  FlowStep,
  FlowStepInfo,
  FolderHooks,
  TemplateSummary,
  TemplateView,
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
  RunTest,
  SendOutcome,
  StepView,
  TestView,
  TreeNode,
  VariableUser,
  NewEnvironment,
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
      oauth2: emptyOAuth2(),
    },
    body: { kind: "none", variants: [], selected: 0, other: "" },
    settings: { timeoutMs: null, followRedirects: null, maxRedirects: null, encodeUrl: null },
    docs: "",
    assertions: [
      {
        id: "a0",
        target: "responseTime",
        operator: "lessThan",
        value: 500,
        origin: "default",
        enabled: true,
      },
    ],
    extractions: [],
    examples: [],
    preserved: [],
    inherited: {
      auth: null,
      headers: [],
      settings: { timeoutMs: 30000, followRedirects: true, maxRedirects: 5, encodeUrl: true },
      variables: [],
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
      timeline: {
        dnsMs: null,
        connectMs: null,
        secure: false,
        serverMs: 40,
        receiveMs: 5,
        totalMs: 45,
      },
    },
    assertions: [
      {
        id: "a0",
        target: "responseTime",
        passed: true,
        message: null,
        actual: "45",
        ignored: false,
      },
    ],
    passed: true,
    slow: false,
    sent: null,
    ...overrides,
  };
}

export interface FakeWorkbench {
  /** Requêtes par chemin ; les dossiers se déduisent des chemins et de `folders`. */
  requests: Map<string, RequestDoc>;
  folders: Set<string>;
  /** Étiquettes des dossiers (`folder.yml`, V2 lot B), par chemin. */
  folderTags: Map<string, string[]>;
  /** Rang d'un dossier (`info.seq` de son `folder.yml`, AM38). */
  folderSeq: Map<string, number>;
  /** Fichiers de requête illisibles : listés sans méthode, comme `tree::read` (A10). */
  unreadable: Set<string>;
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
  /** Heure des statuts écrits par un run (aperçu : l'heure réelle), sinon 1. */
  now?: number;
  // V4 (docs/plan-v4.md) : schémas, Avant / Après, propriétés, jeux de données, templates.
  /** Schéma écrit de chaque test (`extensions.arnod.tests`). */
  flows: Map<string, Flow>;
  /** Avant / Après de chaque dossier (`extensions.arnod.folders`). */
  hooks: Map<string, FolderHooks>;
  /** Propriétés de chaque dossier (`request.variables` de son `folder.yml`). */
  folderVars: Map<string, [string, string][]>;
  /** Tableau du jeu de données de chaque requête. */
  datasets: Map<string, DataTable>;
  /** Templates (`templates/…`) : leur vue. */
  templates: TemplateView[];
  /** Utilisations : dossier → template. */
  instances: Map<string, string>;
  /** Sections affichées depuis « Plus ». */
  sections: string[];
}

export function createFakeWorkbench(): FakeWorkbench {
  return {
    requests: new Map(),
    folders: new Set(),
    folderTags: new Map(),
    folderSeq: new Map(),
    unreadable: new Set(),
    envs: [
      {
        name: "Local",
        color: "gray",
        protected: false,
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
    flows: new Map(),
    hooks: new Map(),
    folderVars: new Map(),
    datasets: new Map(),
    templates: [],
    instances: new Map(),
    sections: [],
  };
}

/** Copie profonde tolérante aux proxys réactifs de Vue (comme la sérialisation JSON de l'IPC). */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const fail = (kind: CommandError["kind"], message: string): never => {
  throw { kind, message } satisfies CommandError;
};

/** Étiquettes effectives (V2, R4) : les siennes, puis celles des dossiers parents, sans doublon. */
function effectiveTags(wb: FakeWorkbench, path: string, own: string[]): string[] {
  const out = [...own];
  const parts = path.split("/").slice(0, -1);
  for (let i = parts.length; i > 0; i--) {
    for (const tag of wb.folderTags.get(parts.slice(0, i).join("/")) ?? []) {
      if (!out.some((t) => t.toLowerCase() === tag.toLowerCase())) out.push(tag);
    }
  }
  return out;
}

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
        // Test : des requêtes sans contrat et aucun sous-dossier (lot E, R1).
        test:
          children.some((c) => c.kind === "request" && c.contract === null) &&
          !children.some((c) => c.kind === "folder"),
        tags: wb.folderTags.get(path) ?? [],
        template: wb.instances.get(path) ?? null,
      };
    });
    // Rang (`seq`) d'abord, puis le nom sans casse, comme `tree::read` (lot E, R1) ; un fichier
    // illisible n'a pas de rang.
    const here = (path: string): boolean =>
      path.startsWith(prefix) && !path.slice(prefix.length).includes("/");
    const requests: { seq: number; node: TreeNode }[] = [];
    const owned = ownedFiles(wb);
    for (const [path, request] of wb.requests) {
      if (!here(path) || owned.has(path)) continue;
      requests.push({
        seq: request.seq ?? Number.POSITIVE_INFINITY,
        node: {
          kind: "request",
          name: request.name,
          path,
          method: request.method,
          assertionCount: request.assertions.length,
          invalid: false,
          contract: request.contract?.file ?? null,
          url: request.url,
          tags: effectiveTags(wb, path, request.tags),
        },
      });
    }
    for (const path of wb.unreadable) {
      if (!here(path)) continue;
      requests.push({
        seq: Number.POSITIVE_INFINITY,
        node: {
          kind: "request",
          name: (path.split("/").at(-1) ?? path).replace(/\.yml$/, ""),
          path,
          method: null,
          assertionCount: 0,
          invalid: true,
          contract: null,
          url: "",
          tags: [],
        },
      });
    }
    // Comme `tree::read` (AM38) : rang d'abord, puis les dossiers avant les requêtes, puis le nom.
    const key = (node: TreeNode): string => node.name.toLowerCase();
    const all = [
      ...nodes.map((node) => ({
        seq: wb.folderSeq.get(node.path) ?? Number.POSITIVE_INFINITY,
        kind: 0,
        node,
      })),
      ...requests.map((r) => ({ ...r, kind: 1 })),
    ];
    all.sort((x, y) => {
      if (x.seq !== y.seq) return x.seq < y.seq ? -1 : 1;
      if (x.kind !== y.kind) return x.kind - y.kind;
      const a = key(x.node);
      const b = key(y.node);
      return a < b ? -1 : a > b ? 1 : 0;
    });
    return all.map((entry) => entry.node);
  };
  return build("");
}

/** Requêtes propres à une étape (Avant / Après, attente) : absentes de l'arborescence (R14). */
export function ownedFiles(wb: FakeWorkbench): Set<string> {
  const owned = new Set<string>();
  for (const [folder, hooks] of wb.hooks) {
    for (const step of [
      ...hooks.beforeAll,
      ...hooks.beforeEach,
      ...hooks.afterEach,
      ...hooks.afterAll,
    ]) {
      if ("file" in step && step.file) owned.add(`${folder}/${step.file}`);
    }
  }
  for (const [test, flow] of wb.flows) {
    for (const step of flow.steps) {
      if (step.kind === "wait" && step.file) owned.add(`${test}/${step.file}`);
    }
  }
  return owned;
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

/** Requêtes de l'arborescence, dans son ordre, avec leur test et leur numéro d'étape. */
function orderedRequests(wb: FakeWorkbench): { path: string; test: string | null; step: number }[] {
  const out: { path: string; test: string | null; step: number }[] = [];
  const walk = (nodes: TreeNode[], test: string | null): void => {
    let step = 0;
    for (const node of nodes) {
      if (node.kind === "folder") walk(node.children, node.test ? node.path : null);
      else out.push({ path: node.path, test, step: test === null ? 0 : (step += 1) });
    }
  };
  walk(buildTree(wb), null);
  return out;
}

/** Verdict des tests, comme `run_tests` du cœur (lot E, R4). */
export function testsOf(items: RunItem[]): RunTest[] {
  const tests: RunTest[] = [];
  for (const item of items) {
    if (item.test === null) continue;
    let test = tests.find((t) => t.path === item.test);
    if (!test) {
      test = {
        path: item.test,
        name: item.test.split("/").at(-1) ?? item.test,
        status: "passed",
        steps: 0,
        passed: 0,
        failedStep: null,
      };
      tests.push(test);
    }
    test.steps += 1;
    if (item.status === "passed") test.passed += 1;
    if ((item.status === "failed" || item.status === "error") && test.failedStep === null) {
      test.failedStep = item.step;
      test.status = "failed";
    }
  }
  return tests;
}

export function runOf(wb: FakeWorkbench, folder: string, only?: string[]): RunView {
  const prefix = folder === "" ? "" : `${folder}/`;
  const entries = orderedRequests(wb).filter(
    (r) => (folder === "" || r.path.startsWith(prefix)) && (!only || only.includes(r.path)),
  );
  wb.runNumber += 1;
  const stopped = new Set<string>();
  const items: RunItem[] = entries.map(({ path, test, step }) => {
    const request = wb.requests.get(path) as RequestDoc;
    // Une étape après un échec de son test n'est pas jouée (lot E, R3).
    const status =
      test !== null && stopped.has(test) ? "skipped" : (wb.runStatus.get(path) ?? "passed");
    if (test !== null && (status === "failed" || status === "error")) stopped.add(test);
    return {
      path,
      name: request.name,
      method: request.method,
      status,
      httpStatus:
        status === "error" || status === "skipped" ? null : status === "failed" ? 500 : 200,
      durationMs: status === "skipped" ? null : 12,
      slow: false,
      assertions: structuredClone(wb.runChecks.get(path) ?? []),
      error: status === "error" ? "Variable « x » absente de l'environnement Local." : null,
      test,
      step: test === null ? null : step,
      extracted: [],
      ...runItemFlow(),
      stepId: test === null ? null : stepIdOf(wb, test, path),
      // Une requête à jeu de données compte une fois, avec son détail par cas (R12).
      cases: casesOf(wb, path, status),
    };
  });
  for (const item of items) {
    if (item.status !== "skipped")
      wb.statuses[item.path] = { passed: item.status === "passed", at: wb.now ?? 1 };
  }
  return {
    id: `run-${String(entries.length)}-${Math.random().toString(36).slice(2, 7)}`,
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
    tests: testsOf(items),
    ends: [],
  };
}

/** Cas d'une requête à jeu de données dans un run : réussis, sauf le premier si elle échoue. */
function casesOf(wb: FakeWorkbench, path: string, status: RunItem["status"]): RunItem["cases"] {
  const table = wb.datasets.get(path);
  if (!table) return [];
  return table.rows.map((row, index) => {
    const failed = index === 0 && (status === "failed" || status === "error");
    return {
      number: index + 1,
      values: table.columns.map((name, i): [string, string] => [name, row[i] ?? ""]),
      status: failed ? "failed" : "passed",
      httpStatus: failed ? 400 : 200,
      durationMs: 12,
      assertions: failed
        ? [
            {
              id: "s",
              target: "status",
              passed: false,
              message: "attendu 200, reçu 400",
              actual: "400",
              ignored: false,
            },
          ]
        : [],
      error: null,
    };
  });
}

/** Identifiant de l'étape requête d'un fichier dans le schéma de son test (R2). */
function stepIdOf(wb: FakeWorkbench, test: string, path: string): string {
  const file = path.split("/").at(-1) ?? path;
  const stored = wb.flows.get(test)?.steps.find((s) => s.kind === "request" && s.file === file);
  return stored?.id ?? `r:${file}`;
}

/** Champs V4 d'une ligne de run : une requête, sans schéma. */
export function runItemFlow(): Pick<
  RunItem,
  "kind" | "stepId" | "detail" | "branch" | "passes" | "hook" | "folder" | "cases"
> {
  return {
    kind: "request",
    stepId: null,
    detail: null,
    branch: null,
    passes: 1,
    hook: null,
    folder: null,
    cases: [],
  };
}

/** Onglet d'un test, comme `test_read` du cœur (lot E, TE2). */
export function testView(wb: FakeWorkbench, path: string): TestView {
  const find = (nodes: TreeNode[]): TreeNode | null => {
    for (const node of nodes) {
      if (node.kind !== "folder") continue;
      if (node.path === path) return node;
      const found = find(node.children);
      if (found) return found;
    }
    return null;
  };
  const folder = find(buildTree(wb));
  if (folder?.kind !== "folder") return fail("notFound", "Ce test n'existe plus.");
  const steps: StepView[] = folder.children.flatMap((node) => {
    if (node.kind !== "request") return [];
    const request = wb.requests.get(node.path) as RequestDoc;
    return [stepViewOf(node.path, request)];
  });
  const flow = folder.test ? flowOf(wb, path, steps) : { onFailure: "stop" as const, steps: [] };
  // Requêtes des attentes : absentes de l'arborescence (R14), montrées dans le test.
  for (const step of flow.steps) {
    if (step.kind !== "wait" || step.file === undefined) continue;
    const file = `${path}/${step.file}`;
    const request = wb.requests.get(file);
    if (request && !steps.some((s) => s.path === file)) steps.push(stepViewOf(file, request));
  }
  return { path, name: folder.name, test: folder.test, steps, flow, info: infoOf(flow, steps) };
}

/** Étape d'un fichier de requête, comme `tree::step_view`. */
export function stepViewOf(path: string, request: RequestDoc): StepView {
  return {
    path,
    file: path.split("/").at(-1) ?? path,
    name: request.name,
    method: request.method,
    url: request.url,
    assertionCount: request.assertions.length,
    extractions: request.extractions.filter((e) => e.enabled).map((e) => e.name),
    assertions: request.assertions.filter((a) => a.enabled),
    extracts: request.extractions,
    invalid: false,
  };
}

/** Schéma d'un test : celui écrit, sinon la liste de ses requêtes (R2, sans les retraits). */
export function flowOf(wb: FakeWorkbench, path: string, steps: StepView[]): Flow {
  const stored = wb.flows.get(path);
  if (stored) {
    const known = new Set(
      stored.steps.map((s) => (s.kind === "request" || s.kind === "wait" ? s.file : null)),
    );
    const added = steps
      .filter((s) => !known.has(s.file))
      .map((s): FlowStep => ({ id: `r:${s.file}`, name: "", kind: "request", file: s.file }));
    return { ...structuredClone(stored), steps: [...structuredClone(stored.steps), ...added] };
  }
  return {
    onFailure: "stop",
    steps: steps.map((s) => ({ id: `r:${s.file}`, name: "", kind: "request", file: s.file })),
  };
}

/** Ce que lit et extrait chaque étape, comme `flow::edit::info` (sans les entrées lues). */
export function infoOf(flow: Flow, steps: StepView[]): FlowStepInfo[] {
  return flow.steps.map((step) => {
    const request = "file" in step ? steps.find((s) => s.file === step.file) : undefined;
    const extracts = request?.extractions ?? (step.kind === "variable" ? [step.variable] : []);
    return { id: step.id, inputs: [], extracts };
  });
}

/** Écrit une étape « name » de rang `seq` dans le dossier `folder`. */
function writeStep(wb: FakeWorkbench, folder: string, name: string, seq: number): string {
  const slug = name
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replaceAll(" ", "-");
  const path = `${folder}/${slug}.yml`;
  wb.requests.set(path, newRequest(name, { seq, assertions: [] }));
  return path;
}

/** Environnements supprimés, à remettre par « Annuler » (ER1), par jeton. */
const deletedEnvs = new Map<string, FakeWorkbench["envs"]>();

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
      return {
        nodes,
        tests: nodes,
        collapsed: wb.collapsed,
        templates: wb.templates.map(summaryOf),
        sections: [...wb.sections],
      };
    }
    case "tree_set_collapsed":
      wb.collapsed = args.collapsed as string[];
      return null;
    case "tree_set_folder_tags": {
      const tags: string[] = [];
      for (const tag of args.tags as string[]) {
        const clean = tag.trim();
        if (clean !== "" && !tags.some((t) => t.toLowerCase() === clean.toLowerCase()))
          tags.push(clean);
      }
      wb.folderTags.set(String(args.path), tags);
      return null;
    }
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
          request: clone(args.request as RequestDoc),
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
    case "environment_delete": {
      const removed = wb.envs.filter((e) => e.name === args.name);
      const token = `environnement-${String(deletedEnvs.size + 1)}`;
      deletedEnvs.set(token, removed);
      wb.envs = wb.envs.filter((e) => e.name !== args.name);
      return { view: envView(wb), undoToken: token };
    }
    case "environment_create": {
      // Règles du cœur (environment::create) : variables partagées et noms des locales repris,
      // `baseUrl` posée, place dans la chaîne.
      const created = args.environment as NewEnvironment;
      if (wb.envs.some((e) => e.name.toLowerCase() === created.name.toLowerCase()))
        fail("nameTaken", `Un environnement « ${created.name} » existe déjà dans ce projet.`);
      const source = wb.envs.find((e) => e.name === created.copyFrom);
      const variables = (source?.variables ?? []).map((v) => ({ ...v }));
      if (created.baseUrl !== "") {
        const base = variables.find((v) => v.name === "baseUrl");
        if (base) base.value = created.baseUrl;
        else variables.unshift({ name: "baseUrl", value: created.baseUrl });
      }
      const env = {
        name: created.name,
        color: "gray",
        protected: created.protected,
        variables,
        locals: [...(source?.locals ?? [])],
      };
      const at =
        created.after === null ? 0 : wb.envs.findIndex((e) => e.name === created.after) + 1;
      wb.envs.splice(at, 0, env);
      return envView(wb);
    }
    case "environment_copy": {
      // Règles du cœur (environment::copy_from) : un nom pris reçoit un numéro, en fin de chaîne.
      // Le faux cœur n'a qu'une liste d'environnements : l'autre projet a les mêmes.
      const source = clone(wb.envs);
      for (const wanted of args.names as string[]) {
        const env =
          source.find((e) => e.name.toLowerCase() === wanted.toLowerCase()) ??
          fail("invalidRequest", `L'environnement « ${wanted} » n'existe plus.`);
        let name = env.name;
        let n = 2;
        while (wb.envs.some((e) => e.name.toLowerCase() === name.toLowerCase())) {
          name = `${env.name} ${String(n)}`;
          n += 1;
        }
        wb.envs.push({ ...clone(env), name });
      }
      return envView(wb);
    }
    case "environment_usage": {
      // Dernier run gardé, s'il y en a un (Workbench::environment_usage).
      const run = wb.lastRun;
      return run
        ? [
            {
              environment: run.environment,
              number: run.number,
              folder: run.folder,
              contract: run.contract,
              startedAt: run.startedAt,
            },
          ]
        : [];
    }
    case "environment_delete_undo": {
      const removed =
        deletedEnvs.get(String(args.undoToken)) ??
        fail("undoExpired", "Cette suppression ne peut plus être annulée.");
      deletedEnvs.delete(String(args.undoToken));
      wb.envs = [...wb.envs, ...removed];
      return envView(wb);
    }
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
      const bad = (previous?.items ?? []).filter(
        (i) => i.status === "failed" || i.status === "error",
      );
      // Un test en échec est rejoué en entier (lot E, TE5).
      const tests = new Set(bad.map((i) => i.test).filter((t) => t !== null));
      const failed = (previous?.items ?? [])
        .filter((i) => bad.includes(i) || (i.test !== null && tests.has(i.test)))
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
    // Lot E : tests et étapes.
    case "test_read":
      return testView(wb, String(args.path));
    case "tree_reorder": {
      // Règles du cœur (tree::reorder) : rang de 1 à n, dossiers compris (AM38).
      const order = args.order as string[];
      order.forEach((path, index) => {
        const request = wb.requests.get(path);
        if (request) request.seq = index + 1;
        else if (wb.folders.has(path)) wb.folderSeq.set(path, index + 1);
      });
      return null;
    }
    case "tree_create_test": {
      const parent = String(args.parent);
      // AM37 : un nom saisi sur place ; pris, il est refusé (tree::create_test_named).
      const asked = typeof args.name === "string" ? args.name.trim() : "";
      let name = asked === "" ? "Nouveau test" : asked;
      if (asked !== "" && wb.folders.has(parent === "" ? name : `${parent}/${name}`))
        fail("nameTaken", `« ${name} » existe déjà dans ce dossier.`);
      for (let n = 2; wb.folders.has(parent === "" ? name : `${parent}/${name}`); n += 1)
        name = `Nouveau test ${String(n)}`;
      const folder = parent === "" ? name : `${parent}/${name}`;
      wb.folders.add(folder);
      return { folder, step: writeStep(wb, folder, "Étape 1", 1) };
    }
    case "tree_add_step": {
      const test = String(args.test);
      const steps = testView(wb, test).steps;
      // Les étapes sans rang reçoivent le leur, dans l'ordre actuel (R11).
      steps.forEach((step, index) => {
        const request = wb.requests.get(step.path);
        if (request && request.seq === undefined) request.seq = index + 1;
      });
      return writeStep(wb, test, `Étape ${String(steps.length + 1)}`, steps.length + 1);
    }
    case "request_variable_users": {
      const names = args.names as string[];
      const users: VariableUser[] = [];
      for (const [path, request] of wb.requests) {
        if (path === args.path) continue;
        const text = JSON.stringify([request.url, request.headers, request.body, request.params]);
        const found = names.filter((n) => text.replaceAll(" ", "").includes(`{{${n}}}`));
        if (found.length > 0)
          users.push({
            path,
            name: request.name,
            method: request.method,
            url: request.url,
            variables: found,
          });
      }
      return users;
    }
    default:
      return undefined;
  }
}

/** Résumé d'un template, comme `template::list`. */
export function summaryOf(view: TemplateView): TemplateSummary {
  return {
    path: view.path,
    name: view.name,
    kind: view.kind,
    category: view.category,
    params: view.parameters.length,
    folders: view.folders,
    tests: view.tests,
    assertions: view.assertions,
  };
}
