/**
 * Affichage des tests en schéma (docs/plan-v4.md, 4.3-a à 4.3-e) : libellés des natures, résumé
 * d'une étape, suite écrite « puis → 2 », palette. Aucune règle : le cœur calcule le schéma et
 * le joue ; ce module ne fait que nommer et compter pour l'affichage.
 */
import { OPERATORS } from "@/lib/assertions";
import type { Flow, FlowCheck, FlowStep, RunItem, StepKind, StepView } from "@/types/ipc";

/** Libellé d'une nature, en capitales sur la ligne (4.3-a « REQUÊTE », « CONDITION »). */
export const KIND_LABELS: Record<StepKind, string> = {
  request: "Requête",
  condition: "Condition",
  loop: "Boucle",
  wait: "Attente",
  variable: "Variable",
  script: "Script",
  test: "Test existant",
  template: "Template",
};

/** Raison d'être de chaque nature (4.3-e). */
export const KIND_HINTS: Record<StepKind, string> = {
  request: "Une requête HTTP, comme un test du référentiel",
  condition: "Si … aller à une étape, sinon à une autre",
  loop: "Répéter, ou pour chaque élément d'une liste",
  wait: "Jusqu'à ce qu'une condition soit vraie",
  variable: "Définir ou calculer une valeur pour les étapes suivantes",
  script: "Quelques lignes de JavaScript, isolées",
  test: "Un test du référentiel, appelé tel quel",
  template: "Avec ses paramètres",
};

/** Famille de couleur d'une nature (jetons `--color-family-*`). */
export const KIND_FAMILY: Record<StepKind, string> = {
  request: "calls",
  condition: "jobs",
  loop: "jobs",
  wait: "jobs",
  variable: "data",
  script: "code",
  test: "messages",
  template: "files",
};

/** Groupes de la palette et du sélecteur (4.3-b, 4.3-e). */
export const PALETTE: { title: string; kinds: StepKind[] }[] = [
  { title: "Appels", kinds: ["request"] },
  { title: "Logique", kinds: ["condition", "loop", "wait"] },
  { title: "Données et code", kinds: ["variable", "script"] },
  { title: "Réutiliser", kinds: ["test", "template"] },
];

/** Libellé d'une nature dans la palette : « REST » pour une requête (4.3-e). */
export function paletteLabel(kind: StepKind): string {
  return kind === "request" ? "REST" : KIND_LABELS[kind];
}

/** Fichier de requête que joue une étape (requête, attente). */
export function stepFile(step: FlowStep): string | null {
  if (step.kind === "request") return step.file;
  if (step.kind === "wait") return step.file ?? null;
  return null;
}

/** Requête d'une étape, d'après son fichier. */
export function stepRequest(step: FlowStep, requests: StepView[]): StepView | null {
  const file = stepFile(step);
  return file === null ? null : (requests.find((r) => r.file === file) ?? null);
}

/** Nom affiché d'une étape : celui de sa requête, sinon le sien, sinon sa nature. */
export function stepName(step: FlowStep, requests: StepView[]): string {
  if (step.kind === "request") return stepRequest(step, requests)?.name ?? step.file;
  if (step.name.trim() !== "") return step.name;
  if (step.kind === "test") return step.test.split("/").at(-1) ?? KIND_LABELS.test;
  if (step.kind === "template") return step.template.split("/").at(-1) ?? KIND_LABELS.template;
  return KIND_LABELS[step.kind];
}

/** Valeur d'une condition, en texte. */
function valueText(check: FlowCheck): string {
  const value = check.value;
  if (value === undefined || value === null) return "";
  return typeof value === "string" ? value : String(value);
}

/** Condition en clair, compacte : « status = 201 », « body.status = paid ». */
export function checkLabel(check: FlowCheck): string {
  const value = valueText(check);
  const short: Partial<Record<FlowCheck["operator"], string>> = {
    equals: "=",
    notEquals: "≠",
    lessThan: "<",
    lessOrEqual: "≤",
    greaterThan: ">",
    greaterOrEqual: "≥",
  };
  const op = short[check.operator];
  if (op !== undefined) return `${check.target} ${op} ${value}`.trim();
  const label = OPERATORS.find((o) => o.id === check.operator)?.label ?? check.operator;
  return `${check.target} ${label} ${value}`.trim();
}

/** Durée en secondes, à la française (« 2 s », « 0,5 s »). */
export function seconds(ms: number): string {
  return `${String(ms / 1000).replace(".", ",")} s`;
}

/** Ce que porte une étape (colonne « Données » de 4.3-a, ligne 2 d'un nœud de 4.3-b). */
export function stepSummary(step: FlowStep, requests: StepView[]): string {
  switch (step.kind) {
    case "request": {
      const request = stepRequest(step, requests);
      if (!request) return step.file;
      const path = request.url.replace(/^\{\{\s*baseUrl\s*\}\}/, "") || request.url;
      return `${request.method ?? "?"} ${path}`;
    }
    case "condition":
      return checkLabel(step.check);
    case "loop":
      return step.list !== undefined && step.list.trim() !== ""
        ? `pour chaque ${step.item || "item"} de ${step.list}`
        : `${String(step.count ?? 0)} fois`;
    case "wait":
      return `${checkLabel(step.check)} · ${seconds(step.intervalMs)} × ${String(step.attempts)} max`;
    case "variable":
      return step.variable === "" ? "à nommer" : `${step.variable} = ${step.value}`;
    case "script": {
      const lines = step.code.split("\n").filter((l) => l.trim() !== "").length;
      return `JavaScript · ${String(lines)} ligne${lines > 1 ? "s" : ""}`;
    }
    case "test":
      return step.test === "" ? "test à choisir" : step.test;
    case "template": {
      const names = Object.keys(step.values);
      return step.template === ""
        ? "template à choisir"
        : names.length === 0
          ? "template"
          : `template · ${names.join(", ")}`;
    }
  }
}

/** Position (à partir de 1) de l'étape `id`, ou la fin. */
export function targetLabel(flow: Flow, target: string): string {
  if (target === "end") return "Fin ✓";
  if (target === "fail") return "Fin en échec";
  const index = flow.steps.findIndex((s) => s.id === target);
  return index < 0 ? "étape suivante" : String(index + 1);
}

/** Suite effective de l'étape `index` : son renvoi, sinon l'étape suivante, sinon la fin. */
export function following(flow: Flow, index: number): string {
  return flow.steps[index]?.next ?? flow.steps[index + 1]?.id ?? "end";
}

/** Enchaînement d'une étape (4.3-a) : [libellé, cible] ; une condition en a deux. */
export function chain(flow: Flow, index: number): { label: string; target: string }[] {
  const step = flow.steps[index];
  if (!step) return [];
  const next = following(flow, index);
  if (step.kind === "condition") {
    return [
      { label: "si oui", target: targetLabel(flow, next) },
      { label: "sinon", target: targetLabel(flow, step.otherwise ?? next) },
    ];
  }
  if (step.kind === "loop") {
    const until = step.until ?? flow.steps[index + 1]?.id;
    return [
      { label: "répète jusqu'à", target: until === undefined ? "—" : targetLabel(flow, until) },
    ];
  }
  return [{ label: "puis", target: targetLabel(flow, next) }];
}

/** « 6 étapes · 1 condition » (en-tête de 4.3-a). */
export function flowCounts(flow: Flow): string {
  const steps = flow.steps.length;
  const conditions = flow.steps.filter((s) => s.kind === "condition").length;
  const kinds = new Set(flow.steps.map((s) => s.kind)).size;
  const parts = [`${String(steps)} étape${steps > 1 ? "s" : ""}`];
  if (conditions > 0) parts.push(`${String(conditions)} condition${conditions > 1 ? "s" : ""}`);
  else if (kinds > 1) parts.push(`${String(kinds)} natures`);
  return parts.join(" · ");
}

/** Ligne du run d'une étape du schéma. */
export function stepItem(items: RunItem[] | undefined, test: string, id: string): RunItem | null {
  return items?.find((i) => i.test === test && i.stepId === id) ?? null;
}

/** Marque d'une ligne de run (4.3-d) : « ✓ 52 ms », « ✓ oui », « ✕ 9/12 », « non jouée ». */
export function runMark(
  item: RunItem | null,
): { text: string; tone: "ok" | "ko" | "muted" } | null {
  if (item === null) return null;
  const time = item.durationMs === null ? "" : ` ${formatMs(item.durationMs)}`;
  const detail = item.detail === null ? "" : ` ${item.detail}`;
  switch (item.status) {
    case "passed":
      return { text: `✓${item.kind === "request" ? time : detail || time}`, tone: "ok" };
    case "failed":
    case "error":
      return {
        text: `✕${item.kind === "request" && item.httpStatus !== null ? ` ${String(item.httpStatus)}` : detail}`,
        tone: "ko",
      };
    case "skipped":
      return { text: "non jouée", tone: "muted" };
    case "running":
      return { text: "en cours…", tone: "muted" };
    case "pending":
      return { text: "en attente", tone: "muted" };
    case "cancelled":
      return { text: "annulé", tone: "muted" };
  }
}

/** Millisecondes : « 52 ms », « 2,3 s ». */
export function formatMs(ms: number): string {
  return ms < 1000 ? `${String(ms)} ms` : seconds(Math.round(ms / 100) * 100);
}
