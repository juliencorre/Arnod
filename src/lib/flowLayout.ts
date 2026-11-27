/**
 * Disposition du schéma d'un test (4.3-b) : où dessiner chaque nœud et chaque lien. Calcul
 * d'affichage seulement (pur, testé) : le cœur décide de l'ordre et des renvois, ce module les
 * place sur une grille. Le chemin principal (suites et « si oui ») tient une colonne ; une
 * branche « sinon » qui n'y est pas part dans la colonne de droite.
 */
import type { Flow, FlowStep } from "@/types/ipc";

/** Largeur et hauteur d'un nœud d'étape, écart entre deux rangées et deux colonnes. */
export const NODE_W = 208;
export const NODE_H = 64;
export const ROW_GAP = 46;
export const COL_GAP = 64;
/** Marge autour du schéma. */
export const PAD = 24;
/** Hauteur des pastilles Départ et Fin. */
export const CAP_H = 30;

export type NodeKind = "start" | "end" | "fail" | "step";

export interface LayoutNode {
  /** `start`, `end`, `fail`, ou l'identifiant de l'étape. */
  id: string;
  kind: NodeKind;
  /** Position de l'étape dans la liste (étapes seulement). */
  index: number | null;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface LayoutEdge {
  /** Identifiant stable : `source→cible:sorte`. */
  key: string;
  from: string;
  to: string;
  /** « oui », « non », « répète », ou rien. */
  label: "oui" | "non" | "répète" | null;
  /** Étape avant laquelle insérer sur ce lien (`null` : depuis le Départ). */
  after: string | null;
  /** Lien « sinon » d'une condition. */
  otherwise: boolean;
  /** Retour d'une boucle : pas d'insertion dessus. */
  back: boolean;
  /** Points du tracé, du départ à l'arrivée. */
  points: [number, number][];
  /** Milieu, pour le « + » et les valeurs passées. */
  mid: [number, number];
}

export interface Layout {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  width: number;
  height: number;
}

/** Suite effective de l'étape `index`. */
function following(flow: Flow, index: number): string {
  return flow.steps[index]?.next ?? flow.steps[index + 1]?.id ?? "end";
}

interface RawEdge {
  from: string;
  to: string;
  label: LayoutEdge["label"];
  after: string | null;
  otherwise: boolean;
  back: boolean;
}

/** Liens du schéma, dans l'ordre de la liste. */
function rawEdges(flow: Flow): RawEdge[] {
  const ids = new Set(flow.steps.map((s) => s.id));
  const safe = (target: string): string =>
    target === "end" || target === "fail" || ids.has(target) ? target : "end";
  const edges: RawEdge[] = [
    {
      from: "start",
      to: flow.steps[0]?.id ?? "end",
      label: null,
      after: null,
      otherwise: false,
      back: false,
    },
  ];
  flow.steps.forEach((step: FlowStep, index) => {
    const next = safe(following(flow, index));
    if (step.kind === "condition") {
      edges.push({
        from: step.id,
        to: next,
        label: "oui",
        after: step.id,
        otherwise: false,
        back: false,
      });
      edges.push({
        from: step.id,
        to: safe(step.otherwise ?? next),
        label: "non",
        after: step.id,
        otherwise: true,
        back: false,
      });
      return;
    }
    if (step.kind === "loop") {
      const body = flow.steps[index + 1]?.id ?? "end";
      edges.push({
        from: step.id,
        to: body,
        label: null,
        after: step.id,
        otherwise: false,
        back: false,
      });
      const until = step.until ?? body;
      if (until !== "end" && ids.has(until))
        edges.push({
          from: until,
          to: step.id,
          label: "répète",
          after: null,
          otherwise: false,
          back: true,
        });
      return;
    }
    edges.push({
      from: step.id,
      to: next,
      label: null,
      after: step.id,
      otherwise: false,
      back: false,
    });
  });
  return edges;
}

/** Place les nœuds et trace les liens du schéma `flow`. */
export function layoutFlow(flow: Flow): Layout {
  const edges = rawEdges(flow);
  // Colonne de chaque étape : le chemin principal (suites, « si oui ») depuis le Départ en 0 ; une
  // étape atteinte seulement par un « sinon » prend la colonne de droite de sa condition.
  const column = new Map<string, number>();
  const visit = (id: string, col: number): void => {
    if (id === "end" || id === "fail" || column.has(id)) return;
    column.set(id, col);
    for (const edge of edges.filter((e) => e.from === id && !e.back && !e.otherwise))
      visit(edge.to, col);
    for (const edge of edges.filter((e) => e.from === id && e.otherwise)) visit(edge.to, col + 1);
  };
  visit(flow.steps[0]?.id ?? "end", 0);
  // Étapes libres (non atteintes) : colonne de droite.
  const maxCol = Math.max(0, ...column.values());
  for (const step of flow.steps) if (!column.has(step.id)) column.set(step.id, maxCol + 1);

  const colX = (col: number): number => PAD + col * (NODE_W + COL_GAP);
  const rowY = (row: number): number => PAD + CAP_H + ROW_GAP + row * (NODE_H + ROW_GAP);
  const nodes: LayoutNode[] = [
    {
      id: "start",
      kind: "start",
      index: null,
      x: colX(0) + NODE_W / 2 - 56,
      y: PAD,
      w: 112,
      h: CAP_H,
    },
  ];
  flow.steps.forEach((step, i) => {
    nodes.push({
      id: step.id,
      kind: "step",
      index: i,
      x: colX(column.get(step.id) ?? 0),
      y: rowY(i),
      w: NODE_W,
      h: NODE_H,
    });
  });
  // Fins : sous la dernière étape qui y mène, dans sa colonne.
  const endRow = flow.steps.length;
  for (const kind of ["end", "fail"] as const) {
    const sources = edges.filter((e) => e.to === kind).map((e) => e.from);
    if (sources.length === 0 && !(kind === "end" && flow.steps.length === 0)) continue;
    const cols = sources.map((s) => (s === "start" ? 0 : (column.get(s) ?? 0)));
    let col = kind === "end" ? Math.min(0, ...cols) : Math.max(...cols, 0);
    if (kind === "fail" && nodes.some((n) => n.kind === "end" && n.x === colX(col))) col += 1;
    nodes.push({
      id: kind,
      kind,
      index: null,
      x: colX(col) + NODE_W / 2 - 64,
      y: rowY(endRow),
      w: 128,
      h: CAP_H,
    });
  }
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const laid: LayoutEdge[] = edges.flatMap((edge) => {
    const a = byId.get(edge.from);
    const b = byId.get(edge.to);
    if (!a || !b) return [];
    const points: [number, number][] = [];
    if (edge.back) {
      // Retour de boucle : par la gauche des nœuds.
      const left = Math.min(a.x, b.x) - 18;
      points.push(
        [a.x, a.y + a.h / 2],
        [left, a.y + a.h / 2],
        [left, b.y + b.h / 2],
        [b.x, b.y + b.h / 2],
      );
    } else {
      const sx = edge.otherwise ? a.x + a.w : a.x + a.w / 2;
      const sy = edge.otherwise ? a.y + a.h / 2 : a.y + a.h;
      const tx = b.x + b.w / 2;
      const ty = b.y;
      if (edge.otherwise) {
        points.push([sx, sy], [tx, sy], [tx, ty]);
      } else if (Math.abs(sx - tx) < 1) {
        points.push([sx, sy], [tx, ty]);
      } else {
        const bend = ty - ROW_GAP / 2;
        points.push([sx, sy], [sx, bend], [tx, bend], [tx, ty]);
      }
    }
    const [x1, y1] = points[Math.floor((points.length - 1) / 2)] ?? [0, 0];
    const [x2, y2] = points[Math.floor((points.length - 1) / 2) + 1] ?? [x1, y1];
    return [
      {
        key: `${edge.from}→${edge.to}:${edge.label ?? ""}`,
        from: edge.from,
        to: edge.to,
        label: edge.label,
        after: edge.after,
        otherwise: edge.otherwise,
        back: edge.back,
        points,
        mid: [(x1 + x2) / 2, (y1 + y2) / 2],
      },
    ];
  });
  const width = Math.max(...nodes.map((n) => n.x + n.w)) + PAD;
  const height = Math.max(...nodes.map((n) => n.y + n.h)) + PAD;
  return { nodes, edges: laid, width, height };
}

/** Chemin SVG d'un lien. */
export function edgePath(edge: LayoutEdge): string {
  return edge.points
    .map(([x, y], i) => `${i === 0 ? "M" : "L"} ${String(x)} ${String(y)}`)
    .join(" ");
}
