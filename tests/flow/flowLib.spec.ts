/**
 * Affichage des tests en schéma (docs/plan-v4.md, 4.3-a, 4.3-b) : libellés, enchaînement et
 * disposition, calculés pour l'affichage seulement.
 */
import { describe, expect, it } from "vitest";

import { chain, checkLabel, flowCounts, stepSummary } from "@/lib/flow";
import { layoutFlow } from "@/lib/flowLayout";
import type { Flow, StepView } from "@/types/ipc";

const requests: StepView[] = [
  {
    path: "t/client.yml",
    file: "client.yml",
    name: "Créer un client",
    method: "POST",
    url: "{{baseUrl}}/customers",
    assertionCount: 1,
    extractions: ["customerId"],
    assertions: [],
    extracts: [],
    invalid: false,
  },
  {
    path: "t/refus.yml",
    file: "refus.yml",
    name: "Refus attendu",
    method: "GET",
    url: "{{baseUrl}}/orders/{{order.id}}",
    assertionCount: 0,
    extractions: [],
    assertions: [],
    extracts: [],
    invalid: false,
  },
];

/** Le parcours de 4.3-a, réduit : client, condition, paiement (Fin ✓), refus (Fin en échec). */
const FLOW: Flow = {
  onFailure: "stop",
  steps: [
    { id: "s1", name: "", kind: "request", file: "client.yml" },
    {
      id: "c",
      name: "Commande acceptée ?",
      kind: "condition",
      check: { target: "status", operator: "equals", value: 201 },
      next: "w",
      otherwise: "r",
    },
    {
      id: "w",
      name: "Attendre le paiement",
      kind: "wait",
      check: { target: "body.status", operator: "equals", value: "paid" },
      intervalMs: 2000,
      attempts: 5,
      next: "end",
    },
    { id: "r", name: "", kind: "request", file: "refus.yml", next: "fail" },
  ],
};

/** Étape `index` du parcours. */
function stepAt(index: number): Flow["steps"][number] {
  const step = FLOW.steps[index];
  if (!step) throw new Error(`étape ${String(index)} absente`);
  return step;
}

describe("libellés d'un schéma (4.3-a)", () => {
  it("résume chaque étape comme la colonne « Données »", () => {
    expect(stepSummary(stepAt(0), requests)).toBe("POST /customers");
    expect(checkLabel({ target: "status", operator: "equals", value: 201 })).toBe("status = 201");
    expect(stepSummary(stepAt(2), requests)).toBe("body.status = paid · 2 s × 5 max");
    expect(flowCounts(FLOW)).toBe("4 étapes · 1 condition");
  });

  it("écrit l'enchaînement : « puis → 2 », « si oui → 3 », « sinon → 4 », « Fin en échec »", () => {
    expect(chain(FLOW, 0)).toEqual([{ label: "puis", target: "2" }]);
    expect(chain(FLOW, 1)).toEqual([
      { label: "si oui", target: "3" },
      { label: "sinon", target: "4" },
    ]);
    expect(chain(FLOW, 2)).toEqual([{ label: "puis", target: "Fin ✓" }]);
    expect(chain(FLOW, 3)).toEqual([{ label: "puis", target: "Fin en échec" }]);
  });
});

describe("disposition du schéma (4.3-b)", () => {
  it("garde le chemin principal en colonne, la branche « non » à droite, les fins en bas", () => {
    const layout = layoutFlow(FLOW);
    const node = (id: string) => layout.nodes.find((n) => n.id === id);
    expect(node("s1")?.x).toBe(node("w")?.x);
    expect(node("r")?.x).toBeGreaterThan(node("w")?.x ?? 0);
    expect(node("end")).toBeDefined();
    expect(node("fail")).toBeDefined();
    expect(node("end")?.y).toBe(node("fail")?.y);
    // Un lien par suite : départ, suite du client, oui, non, attente → fin, refus → échec.
    expect(layout.edges.map((e) => `${e.from}→${e.to}`)).toEqual([
      "start→s1",
      "s1→c",
      "c→w",
      "c→r",
      "w→end",
      "r→fail",
    ]);
    const no = layout.edges.find((e) => e.label === "non");
    expect(no?.otherwise).toBe(true);
    expect(no?.after).toBe("c");
  });

  it("dessine le retour d'une boucle, sans « + » dessus", () => {
    const loop: Flow = {
      onFailure: "stop",
      steps: [
        { id: "l", name: "Pour chaque", kind: "loop", list: "{{ids}}", item: "id", until: "s1" },
        { id: "s1", name: "", kind: "request", file: "client.yml" },
      ],
    };
    const back = layoutFlow(loop).edges.find((e) => e.back);
    expect(back?.from).toBe("s1");
    expect(back?.to).toBe("l");
    expect(back?.label).toBe("répète");
  });
});
