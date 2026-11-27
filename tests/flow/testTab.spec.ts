/**
 * Tests en schéma (docs/plan-v4.md, lots A et B) : un test en liste (4.3-a), « + Étape ▾ » au
 * clavier (4.3-e), l'étape choisie (4.3-c), le schéma et l'insertion sur un lien (4.3-b bis), le
 * chemin suivi après exécution (4.3-d), sur le faux cœur.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";
import type { Flow } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

/** « Clients » : créer, puis une condition sur le statut, lister si oui, fin en échec sinon. */
const FLOW: Flow = {
  onFailure: "stop",
  steps: [
    { id: "r:creer.yml", name: "", kind: "request", file: "creer.yml" },
    {
      id: "c",
      name: "Client créé ?",
      kind: "condition",
      check: { target: "status", operator: "equals", value: 201 },
      next: "r:lister.yml",
      otherwise: "fail",
    },
    { id: "r:lister.yml", name: "", kind: "request", file: "lister.yml" },
  ],
};

async function openTest(setup?: (core: FakeCore) => void): Promise<FakeCore> {
  let core: FakeCore;
  ({ app, core } = await openProject((c) => {
    c.workbench.flows.set("Clients", structuredClone(FLOW));
    setup?.(c);
  }));
  await click(app, '[role="treeitem"]', "Clients");
  return core;
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe("test en liste (4.3-a)", () => {
  it("numérote les étapes, nomme leur nature et dit où l'on va ensuite", async () => {
    await openTest();
    const tab = app!.get(".test");
    expect(tab.get(".test__counts").text()).toBe("3 étapes · 1 condition");
    const rows = tab.findAll(".step").map((s) => s.text().replace(/\s+/g, " "));
    expect(rows[0]).toContain("1POSTCréer");
    expect(rows[0]).toContain("puis → 2");
    expect(rows[1]).toContain("Condition");
    expect(rows[1]).toContain("status = 201");
    expect(rows[1]).toContain("si oui → 3");
    expect(rows[1]).toContain("sinon → Fin en échec");
  });

  it("⋮⋮ se déplace au clavier ; le brouillon s'enregistre seul (R3)", async () => {
    const core = await openTest();
    const handle = app!.get('[data-step-handle="r:lister.yml"]');
    await handle.trigger("keydown", { key: "ArrowUp" });
    await wait(AUTOSAVE_MS + 50);
    await flushPromises();
    const saved = core.calls.filter((c) => c.cmd === "flow_save").at(-1)?.args.flow as Flow;
    expect(saved.steps.map((s) => s.id)).toEqual(["r:creer.yml", "r:lister.yml", "c"]);
  });
});

describe("ajouter une étape (4.3-e) et la configurer (4.3-c)", () => {
  it("« + Étape ▾ » cherche une nature au clavier, l'ajoute et ouvre son panneau", async () => {
    const core = await openTest();
    await click(app!, ".test button", "+ Étape ▾");
    const input = app!.get(".picker__input");
    expect(document.activeElement).toBe(input.element);
    await input.setValue("scri");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "flow_add_step")?.args).toMatchObject({
      path: "Clients",
      kind: "script",
      placement: { kind: "end" },
    });
    const panel = app!.get(".panel");
    expect(panel.text()).toContain("Étape · Script");
    expect(panel.text()).toContain("à configurer");
    expect(panel.text()).toContain("vars — variables du test");
    // Le code s'écrit dans le panneau et part à l'enregistrement.
    await panel.get(".panel__code").setValue("assert(response.status === 201);");
    await wait(AUTOSAVE_MS + 50);
    await flushPromises();
    const saved = core.calls.filter((c) => c.cmd === "flow_save").at(-1)?.args.flow as Flow;
    const script = saved.steps.at(-1);
    expect(script?.kind === "script" && script.code).toBe("assert(response.status === 201);");
  });

  it("une condition se règle : cible, valeur, si oui et sinon", async () => {
    const core = await openTest();
    await click(app!, ".step__name", "Client créé ?");
    const panel = app!.get(".panel");
    await panel.get('[aria-label="Valeur"]').setValue("200");
    const selects = panel.findAll("select");
    // Dernier « sinon » : Fin ✓.
    const otherwise = selects.find((s) =>
      s.element.closest("label")?.textContent?.includes("sinon"),
    );
    await otherwise?.setValue("end");
    await wait(AUTOSAVE_MS + 50);
    await flushPromises();
    const saved = core.calls.filter((c) => c.cmd === "flow_save").at(-1)?.args.flow as Flow;
    const condition = saved.steps[1];
    expect(condition?.kind === "condition" && condition.check.value).toBe(200);
    expect(condition?.kind === "condition" && condition.otherwise).toBe("end");
  });

  it("« Supprimer l'étape » passe par le cœur, qui reconnecte les liens", async () => {
    const core = await openTest();
    await click(app!, ".step__name", "Client créé ?");
    await click(app!, ".panel button", "Supprimer l'étape");
    expect(core.calls.find((c) => c.cmd === "flow_remove_step")?.args).toMatchObject({
      path: "Clients",
      step: "c",
    });
    expect(app!.findAll(".step")).toHaveLength(2);
  });
});

describe("test en schéma (4.3-b, 4.3-b bis, 4.3-d)", () => {
  it("montre Départ, les nœuds et les fins, et insère sur un lien au clavier", async () => {
    const core = await openTest();
    const tabs = app!.findAll(".test__mode");
    await tabs[0]?.trigger("keydown", { key: "ArrowRight" });
    await tabs[1]?.trigger("click");
    await flushPromises();
    const nodes = app!.findAll(".node").map((n) => n.attributes("aria-label"));
    expect(nodes).toEqual(["Étape 1 : Créer", "Étape 2 : Client créé ?", "Étape 3 : Lister"]);
    expect(app!.findAll(".cap").map((c) => c.text())).toEqual([
      "▶ Départ",
      "✓ Fin",
      "✕ Fin en échec",
    ]);
    const plus = app!
      .findAll(".edge__plus")
      .find(
        (b) =>
          b.attributes("aria-label") === "Insérer une étape entre « Créer » et « Client créé ? »",
      );
    expect(plus).toBeDefined();
    await plus?.trigger("click");
    const input = app!.get(".board__picker .picker__input");
    await input.setValue("vari");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "flow_add_step")?.args).toMatchObject({
      kind: "variable",
      placement: { kind: "link", after: "r:creer.yml", otherwise: false },
    });
    expect(app!.findAll(".node")).toHaveLength(4);
  });

  it("après exécution, marque chaque étape et suit le chemin", async () => {
    await openTest();
    await click(app!, ".test button", "Exécuter le test");
    await click(app!, ".zone__tab-button", "Clients");
    await app!.findAll(".test__mode")[1]?.trigger("click");
    await flushPromises();
    const marks = app!.findAll(".node__mark").map((m) => m.text());
    expect(marks[0]).toMatch(/^✓/);
    expect(app!.findAll(".links__path--taken").length).toBeGreaterThan(0);
  });
});
