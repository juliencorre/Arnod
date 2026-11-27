/**
 * Faux cœur de la V4 (docs/plan-v4.md) : schémas des tests, Avant / Après, propriétés, jeux de
 * données, templates et « Plus ». En mémoire, sans les règles métier (testées côté Rust) : il
 * reproduit la forme des réponses du cœur.
 */
import type {
  CaseOutcome,
  CasesOutcome,
  CommandError,
  DataSet,
  DataTable,
  Flow,
  FlowStep,
  FolderHooks,
  FolderProperty,
  HookPhase,
  HooksView,
  InstanceView,
  Literal,
  ParamValue,
  RequestDoc,
  SendOutcome,
  StepKind,
  StepPlacement,
  TemplateCandidate,
  TemplateParam,
  TemplateView,
  TreeNode,
} from "@/types/ipc";

import {
  buildTree,
  newRequest,
  okOutcome,
  stepViewOf,
  summaryOf,
  testView,
  type FakeWorkbench,
} from "./fakeWorkbench";

const fail = (kind: CommandError["kind"], message: string): never => {
  throw { kind, message } satisfies CommandError;
};

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function slug(name: string): string {
  return name
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

/** Requête neuve au nom libre dans `folder`, comme `tree::create_request`. */
function createRequest(wb: FakeWorkbench, folder: string, base: string): string {
  let name = base;
  for (let n = 2; [...wb.requests.keys()].includes(`${folder}/${slug(name)}.yml`); n += 1)
    name = `${base} ${String(n)}`;
  const path = `${folder}/${slug(name)}.yml`;
  wb.requests.set(path, newRequest(name, { assertions: [] }));
  return path;
}

/** Étape neuve d'une nature, comme `flow::edit::new_step`. */
export function newStep(id: string, kind: StepKind, file: string | null): FlowStep {
  const check = { target: "status", operator: "equals" as const, value: 200 };
  switch (kind) {
    case "request":
      return { id, name: "", kind, file: file ?? "" };
    case "condition":
      return { id, name: "Nouvelle condition", kind, check };
    case "loop":
      return { id, name: "Nouvelle boucle", kind, count: 3, item: "item" };
    case "wait":
      return {
        id,
        name: "Nouvelle attente",
        kind,
        check,
        intervalMs: 2000,
        attempts: 5,
        ...(file === null ? {} : { file }),
      };
    case "variable":
      return { id, name: "Nouvelle variable", kind, variable: "", value: "" };
    case "script":
      return {
        id,
        name: "Nouveau script",
        kind,
        code: '// vars, response et assert() sont à disposition.\nassert(response.status === 200, "statut 200");\n',
      };
    case "test":
      return { id, name: "Test existant", kind, test: "" };
    case "template":
      return { id, name: "Template", kind, template: "", values: {} };
  }
}

function freeId(flow: Flow): string {
  for (let n = 1; ; n += 1) {
    const id = `s${String(n)}`;
    if (!flow.steps.some((s) => s.id === id)) return id;
  }
}

/** Suite effective d'une étape : son renvoi, sinon la suivante, sinon la fin. */
function following(flow: Flow, index: number): string {
  return flow.steps[index]?.next ?? flow.steps[index + 1]?.id ?? "end";
}

/** Insère une étape et reconnecte le lien, comme `flow::edit::insert` (R3). */
export function insertStep(flow: Flow, step: FlowStep, placement: StepPlacement): void {
  if (placement.kind === "start") flow.steps.unshift(step);
  else if (placement.kind === "end") flow.steps.push(step);
  else if (placement.kind === "free") {
    const last = flow.steps.at(-1);
    if (last && last.next === undefined) last.next = "end";
    flow.steps.push({ ...step, next: "end" });
  } else {
    const at = flow.steps.findIndex((s) => s.id === placement.after);
    if (at < 0) {
      flow.steps.push(step);
      return;
    }
    const source = flow.steps[at] as FlowStep;
    const after = following(flow, at);
    if (placement.otherwise && source.kind === "condition") {
      const old = source.otherwise ?? after;
      source.next = after;
      source.otherwise = step.id;
      step.next = old;
    } else {
      if (source.next !== undefined) source.next = step.id;
      step.next = after;
    }
    flow.steps.splice(at + 1, 0, step);
  }
  // Un renvoi vers l'étape suivante ne s'écrit pas (R1).
  flow.steps.forEach((s, i) => {
    if (s.next !== undefined && s.next === (flow.steps[i + 1]?.id ?? "end")) delete s.next;
  });
}

function emptyHooks(): FolderHooks {
  return { beforeAll: [], beforeEach: [], afterEach: [], afterAll: [] };
}

function hooksView(wb: FakeWorkbench, folder: string): HooksView {
  const hooks = clone(wb.hooks.get(folder) ?? emptyHooks());
  const requests = [...hooks.beforeAll, ...hooks.beforeEach, ...hooks.afterEach, ...hooks.afterAll]
    .flatMap((step) => ("file" in step && step.file ? [`${folder}/${step.file}`] : []))
    .flatMap((path) => {
      const request = wb.requests.get(path);
      return request ? [stepViewOf(path, request)] : [];
    });
  return { folder, hooks, requests };
}

function properties(wb: FakeWorkbench, folder: string): FolderProperty[] {
  const out: FolderProperty[] = [];
  const parts = folder.split("/");
  for (let depth = parts.length; depth > 0; depth -= 1) {
    const at = parts.slice(0, depth).join("/");
    for (const [name, value] of wb.folderVars.get(at) ?? []) {
      if (out.some((p) => p.name === name)) continue;
      out.push({
        name,
        value,
        origin: at === folder ? { kind: "here" } : { kind: "folder", folder: at },
      });
    }
  }
  const env = wb.envs.find((e) => e.name === wb.active);
  for (const variable of env?.variables ?? []) {
    if (out.some((p) => p.name === variable.name)) continue;
    out.push({
      name: variable.name,
      value: variable.value,
      origin: { kind: "environment", name: wb.active },
    });
  }
  return out;
}

function dataSet(wb: FakeWorkbench, path: string): DataSet | null {
  const request = wb.requests.get(path);
  const table = wb.datasets.get(path);
  if (!request?.data || !table) return null;
  return { link: clone(request.data), error: null, ...clone(table) };
}

/** Valeurs de chaque cas, comme `dataset::cases`. */
function cases(table: DataTable): Record<string, string>[] {
  return table.rows
    .filter((row) => row.some((cell) => cell.trim() !== ""))
    .map((row) =>
      Object.fromEntries(
        table.columns.map((name, i) => [name, row[i] ?? ""]).filter(([name]) => name !== ""),
      ),
    );
}

function substitute(text: string, values: Record<string, string>): string {
  return text.replace(/\{\{\s*([^}\s]+)\s*\}\}/g, (all, name: string) => values[name] ?? all);
}

function findNode(nodes: TreeNode[], path: string): TreeNode | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.kind === "folder") {
      const found = findNode(node.children, path);
      if (found) return found;
    }
  }
  return null;
}

function templateOf(wb: FakeWorkbench, path: string): TemplateView {
  const view =
    wb.templates.find((t) => t.path === path) ?? fail("notFound", "Ce template n'existe plus.");
  view.uses = [...wb.instances]
    .filter(([, template]) => template === path)
    .map(([use]) => ({
      path: use,
      name: use.split("/").at(-1) ?? use,
      values: Object.fromEntries(wb.folderVars.get(use) ?? []),
    }));
  view.params = view.parameters.length;
  return clone(view);
}

function instanceOf(wb: FakeWorkbench, path: string): InstanceView {
  const templatePath =
    wb.instances.get(path) ?? fail("notFound", "Ce dossier n'utilise pas de template.");
  const template = wb.templates.find((t) => t.path === templatePath) ?? null;
  const here = Object.fromEntries(wb.folderVars.get(path) ?? []);
  const values: ParamValue[] = (template?.parameters ?? []).map((param) => {
    const value = here[param.name] ?? "";
    const origin: ParamValue["origin"] =
      value !== ""
        ? { kind: "here" }
        : param.default !== ""
          ? { kind: "default" }
          : { kind: "missing" };
    return {
      name: param.name,
      value,
      effective: value !== "" ? value : param.default,
      origin,
      default: param.default,
      description: param.description,
    };
  });
  return {
    path,
    name: path.split("/").at(-1) ?? path,
    template: template ? summaryOf(template) : null,
    templatePath,
    values,
    content: clone(template?.content ?? []),
  };
}

/** Valeurs écrites d'une requête, comme `template::literals` (R19). */
function literals(request: RequestDoc): Literal[] {
  const out: Literal[] = [];
  const add = (name: string, value: string, location: string): void => {
    let unique = name;
    for (let n = 2; out.some((l) => l.name === unique); n += 1) unique = `${name}${String(n)}`;
    out.push({ name: unique, value, location });
  };
  const walk = (value: unknown, path: string, key: string): void => {
    if (Array.isArray(value))
      value.forEach((v, i) => {
        walk(v, `${path}[${String(i)}]`, key);
      });
    else if (value !== null && typeof value === "object")
      for (const [k, v] of Object.entries(value)) walk(v, `${path}.${k}`, k);
    else if (typeof value === "string" ? !value.includes("{{") : value !== null)
      add(key, typeof value === "string" ? value : JSON.stringify(value), path);
  };
  if (request.body.kind === "json") {
    try {
      walk(
        JSON.parse(request.body.variants[request.body.selected]?.content ?? "") as unknown,
        "body",
        "",
      );
    } catch {
      // Corps illisible : aucune valeur proposée.
    }
  }
  for (const param of request.params) {
    if (param.kind === "query" && param.enabled && !param.value.includes("{{"))
      add(param.name, param.value, `query.${param.name}`);
  }
  return out;
}

/** Répond aux commandes de la V4 ; `undefined` pour une autre commande. */
export function handleV4(wb: FakeWorkbench, cmd: string, args: Record<string, unknown>): unknown {
  const path = typeof args.path === "string" ? args.path : "";
  switch (cmd) {
    case "flow_save":
      wb.flows.set(path, clone(args.flow as Flow));
      return testView(wb, path);
    case "flow_add_step": {
      const kind = args.kind as StepKind;
      const flow = clone(testView(wb, path).flow);
      const name =
        typeof args.name === "string" && args.name.trim() !== "" ? args.name.trim() : null;
      const request =
        kind === "request" || kind === "wait"
          ? createRequest(
              wb,
              path,
              name ?? (kind === "wait" ? "Requête de l'attente" : "Nouvelle requête"),
            )
          : null;
      const file = request === null ? null : (request.split("/").at(-1) ?? request);
      // Comme `flow::edit::new_step` : une étape requête prend l'identifiant de son fichier.
      const own = file === null ? null : `r:${file}`;
      const id =
        kind === "request" && own !== null && !flow.steps.some((s) => s.id === own)
          ? own
          : freeId(flow);
      const step = newStep(id, kind, file);
      insertStep(flow, step, args.placement as StepPlacement);
      wb.flows.set(path, flow);
      return { step: step.id, request };
    }
    case "flow_remove_step": {
      const flow = clone(testView(wb, path).flow);
      const at = flow.steps.findIndex((s) => s.id === args.step);
      const step = flow.steps[at] ?? fail("notFound", "Cette étape n'existe plus.");
      const after = following(flow, at);
      flow.steps.splice(at, 1);
      for (const s of flow.steps) {
        if (s.next === step.id) s.next = after;
        if (s.kind === "condition" && s.otherwise === step.id) s.otherwise = after;
      }
      if ((step.kind === "request" || step.kind === "wait") && step.file)
        wb.requests.delete(`${path}/${step.file}`);
      wb.flows.set(path, flow);
      return testView(wb, path);
    }
    case "flow_step_operation": {
      // Comme `flow::edit::from_operation` : la requête de l'opération, sans lien au contrat.
      const flow = testView(wb, path).flow;
      const step = flow.steps.find((s) => s.id === args.step);
      if (step?.kind !== "request")
        return fail("notFound", "Cette étape n'est plus une requête du test.");
      const key = String(args.operation);
      const [method = "GET", route = "/"] = key.split(" ");
      const file = `${path}/${step.file}`;
      const current = wb.requests.get(file) ?? fail("notFound", "Cette requête n'existe plus.");
      wb.requests.set(file, {
        ...current,
        name: key,
        method: method as RequestDoc["method"],
        url: `{{baseUrl}}${route.replaceAll(/\{(\w+)\}/g, ":$1")}`,
      });
      return testView(wb, path);
    }
    case "hooks_remove_step": {
      const hooks = wb.hooks.get(path) ?? emptyHooks();
      const phase = args.phase as HookPhase;
      const step = hooks[phase].find((s) => s.id === args.step);
      hooks[phase] = hooks[phase].filter((s) => s.id !== args.step);
      if (step && (step.kind === "request" || step.kind === "wait") && step.file)
        wb.requests.delete(`${path}/${step.file}`);
      wb.hooks.set(path, hooks);
      return hooksView(wb, path);
    }
    case "hooks_read":
      return hooksView(wb, path);
    case "hooks_save":
      wb.hooks.set(path, clone(args.hooks as FolderHooks));
      return hooksView(wb, path);
    case "hooks_add_step": {
      const hooks = wb.hooks.get(path) ?? emptyHooks();
      const kind = args.kind as StepKind;
      const request =
        kind === "request" || kind === "wait"
          ? createRequest(wb, path, typeof args.name === "string" ? args.name : "Nouvelle requête")
          : null;
      const all = [...hooks.beforeAll, ...hooks.beforeEach, ...hooks.afterEach, ...hooks.afterAll];
      let n = 1;
      while (all.some((s) => s.id === `h${String(n)}`)) n += 1;
      hooks[args.phase as HookPhase].push(
        newStep(
          `h${String(n)}`,
          kind,
          request === null ? null : (request.split("/").at(-1) ?? null),
        ),
      );
      wb.hooks.set(path, hooks);
      return hooksView(wb, path);
    }
    case "folder_properties":
      return properties(wb, path);
    case "folder_set_properties":
      wb.folderVars.set(
        path,
        (args.properties as { name: string; value: string }[]).map((p) => [p.name, p.value]),
      );
      return properties(wb, path);
    case "data_read":
      return dataSet(wb, path);
    case "data_save": {
      const request = wb.requests.get(path) ?? fail("notFound", "Introuvable.");
      if (request.data && !request.data.editable)
        fail(
          "invalidRequest",
          "Ce jeu de données vient d'un fichier : modifiez-le dans votre tableur.",
        );
      request.data ??= { file: `données/${slug(request.name)}.csv`, editable: true };
      wb.datasets.set(path, clone(args.table as DataTable));
      return dataSet(wb, path);
    }
    case "data_import": {
      const request = wb.requests.get(path) ?? fail("notFound", "Introuvable.");
      request.data = { file: "données/filtres-commandes.csv", editable: false };
      wb.datasets.set(path, {
        columns: ["statut", "limite"],
        rows: [
          ["pending", "10"],
          ["paid", "10"],
        ],
      });
      return dataSet(wb, path);
    }
    case "data_remove": {
      const request = wb.requests.get(path);
      if (request) delete request.data;
      wb.datasets.delete(path);
      return null;
    }
    case "data_parse":
      return String(args.text)
        .split(/\r?\n/)
        .filter((line) => line.trim() !== "")
        .map((line) => line.split(line.includes("\t") ? "\t" : ","));
    case "data_open":
      return null;
    case "data_case": {
      const request = args.request as RequestDoc;
      const table = wb.datasets.get(path) ?? { columns: [], rows: [] };
      const values = cases(table)[Number(args.case) - 1] ?? {};
      const env = Object.fromEntries(
        (wb.envs.find((e) => e.name === wb.active)?.variables ?? []).map((v) => [v.name, v.value]),
      );
      return {
        method: request.method,
        url: substitute(request.url, { ...env, ...values }),
        values: Object.entries(values),
      };
    }
    case "request_send_cases": {
      const table =
        wb.datasets.get(path) ?? fail("invalidRequest", "Cette requête n'a pas de jeu de données.");
      const only = args.cases as number[] | null;
      const out: CaseOutcome[] = [];
      cases(table).forEach((values, index) => {
        const number = index + 1;
        if (only !== null && !only.includes(number)) return;
        const failed = Object.values(values).includes("cancelled");
        const outcome: SendOutcome = failed
          ? okOutcome({
              passed: false,
              response: {
                ...okOutcome().response,
                status: 400,
                statusText: "Bad Request",
                body: '{"error":"statut inconnu"}',
              },
            })
          : okOutcome();
        out.push({ number, values: Object.entries(values), outcome, error: null });
      });
      const result: CasesOutcome = {
        cases: out,
        passed: out.every((c) => c.outcome?.passed === true),
      };
      wb.statuses[path] = { passed: result.passed, at: 1 };
      return result;
    }
    case "template_list":
      return wb.templates.map(summaryOf);
    case "template_categories":
      return [...new Set(wb.templates.map((t) => t.category).filter((c) => c !== ""))];
    case "template_read":
      return templateOf(wb, path);
    case "template_save_params": {
      const view =
        wb.templates.find((t) => t.path === path) ?? fail("notFound", "Ce template n'existe plus.");
      view.parameters = clone(args.params as TemplateParam[]);
      return templateOf(wb, path);
    }
    case "template_candidates":
      return properties(wb, String(args.folder)).map((p): TemplateCandidate => ({
        name: p.name,
        value: p.value,
        origin:
          p.origin.kind === "here"
            ? "propriété du dossier"
            : p.origin.kind === "folder"
              ? `hérité de ${p.origin.folder.split("/").at(-1) ?? ""}`
              : p.origin.kind === "environment"
                ? `environnement ${p.origin.name}`
                : "variable du projet",
        checked: p.origin.kind !== "environment",
      }));
    case "template_create": {
      const folder = String(args.folder);
      const name = String(args.name);
      const category = String(args.category);
      const target = ["templates", category, name].filter((p) => p !== "").join("/");
      if (wb.templates.some((t) => t.path === target))
        fail("nameTaken", `« ${name} » existe déjà à cet endroit : choisissez un autre nom.`);
      const node = findNode(buildTree(wb), folder);
      const content = node?.kind === "folder" ? clone(node.children) : [];
      wb.templates.push({
        path: target,
        name,
        kind: node?.kind === "folder" && node.test ? "test" : "folder",
        category,
        params: (args.params as TemplateParam[]).length,
        folders: 0,
        tests: content.length,
        assertions: 0,
        parameters: clone(args.params as TemplateParam[]),
        content,
        uses: [],
      });
      if (args.replace === true) {
        for (const key of [...wb.requests.keys()])
          if (key.startsWith(`${folder}/`)) wb.requests.delete(key);
        wb.instances.set(folder, target);
      }
      return templateOf(wb, target);
    }
    case "template_use": {
      const folder = String(args.folder);
      const name = String(args.name);
      const use = folder === "" ? name : `${folder}/${name}`;
      if (wb.folders.has(use)) fail("nameTaken", `« ${name} » existe déjà à cet endroit.`);
      wb.folders.add(use);
      wb.instances.set(use, String(args.template));
      return use;
    }
    case "template_instance":
      return instanceOf(wb, path);
    case "template_set_values":
      wb.folderVars.set(
        path,
        Object.entries(args.values as Record<string, string>).filter(([, v]) => v.trim() !== ""),
      );
      return instanceOf(wb, path);
    case "template_literals":
      return literals(wb.requests.get(path) ?? fail("notFound", "Introuvable."));
    case "template_from_request": {
      const request = wb.requests.get(path) ?? fail("notFound", "Introuvable.");
      const chosen = args.params as string[];
      const target = ["templates", String(args.category), String(args.name)]
        .filter((p) => p !== "")
        .join("/");
      const params = literals(request)
        .filter((l) => chosen.includes(l.name))
        .map((l) => ({ name: l.name, default: l.value, description: "" }));
      wb.requests.delete(path);
      wb.templates.push({
        path: target,
        name: String(args.name),
        kind: "test",
        category: String(args.category),
        params: params.length,
        folders: 0,
        tests: 1,
        assertions: request.assertions.length,
        parameters: params,
        content: [],
        uses: [],
      });
      return templateOf(wb, target);
    }
    case "sections_show": {
      const section = String(args.section);
      wb.sections = wb.sections.filter((s) => s !== section);
      if (args.shown === true) wb.sections.push(section);
      return [...wb.sections];
    }
    default:
      return undefined;
  }
}
