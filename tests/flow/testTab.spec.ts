/**
 * Tests en schéma (docs/plan-v4.md, lots A et B) : un test en liste (4.3-a), « + Étape ▾ » au
 * clavier (4.3-e), l'étape choisie (4.3-c), le schéma et l'insertion sur un lien (4.3-b bis), le
 * chemin suivi après exécution (4.3-d), sur le faux cœur.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS, useWorkbenchStore } from "@/stores/workbench";
import type { Flow } from "@/types/ipc";

import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "../fakeContracts";
import type { FakeCore } from "../fakeCore";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

/** Application montée par le test en cours. */
function page(): AppWrapper {
  if (!app) throw new Error("application non montée");
  return app;
}

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
    const tab = page().get(".test");
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
    const handle = page().get('[data-step-handle="r:lister.yml"]');
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
    await click(page(), ".test button", "+ Étape ▾");
    const input = page().get(".picker__input");
    expect(document.activeElement).toBe(input.element);
    await input.setValue("scri");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "flow_add_step")?.args).toMatchObject({
      path: "Clients",
      kind: "script",
      placement: { kind: "end" },
    });
    const panel = page().get(".panel");
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
    await click(page(), ".step__name", "Client créé ?");
    const panel = page().get(".panel");
    await panel.get('[aria-label="Valeur"]').setValue("200");
    const selects = panel.findAll("select");
    // Dernier « sinon » : Fin ✓.
    const otherwise = selects.find((s) =>
      s.element.closest("label")?.textContent.includes("sinon"),
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
    await click(page(), ".step__name", "Client créé ?");
    await click(page(), ".panel button", "Supprimer l'étape");
    expect(core.calls.find((c) => c.cmd === "flow_remove_step")?.args).toMatchObject({
      path: "Clients",
      step: "c",
    });
    expect(page().findAll(".step")).toHaveLength(2);
  });
});

describe("test en schéma (4.3-b, 4.3-b bis, 4.3-d)", () => {
  it("montre Départ, les nœuds et les fins, et insère sur un lien au clavier", async () => {
    const core = await openTest();
    const tabs = page().findAll(".test__mode");
    await tabs[0]?.trigger("keydown", { key: "ArrowRight" });
    await tabs[1]?.trigger("click");
    await flushPromises();
    const nodes = page()
      .findAll(".node")
      .map((n) => n.attributes("aria-label"));
    expect(nodes).toEqual(["Étape 1 : Créer", "Étape 2 : Client créé ?", "Étape 3 : Lister"]);
    expect(
      page()
        .findAll(".cap")
        .map((c) => c.text()),
    ).toEqual(["▶ Départ", "✓ Fin", "✕ Fin en échec"]);
    const plus = page()
      .findAll(".edge__plus")
      .find(
        (b) =>
          b.attributes("aria-label") === "Insérer une étape entre « Créer » et « Client créé ? »",
      );
    expect(plus).toBeDefined();
    await plus?.trigger("click");
    const input = page().get(".board__picker .picker__input");
    await input.setValue("vari");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "flow_add_step")?.args).toMatchObject({
      kind: "variable",
      placement: { kind: "link", after: "r:creer.yml", otherwise: false },
    });
    expect(page().findAll(".node")).toHaveLength(4);
    // L'étape insérée reste choisie, « à configurer », et son insertion s'annule (4.3-b quater).
    const panel = page().get(".panel");
    expect(panel.text()).toContain("à configurer");
    await click(page(), ".panel button", "Annuler l'insertion");
    expect(core.calls.some((c) => c.cmd === "flow_remove_step")).toBe(true);
    expect(page().findAll(".node")).toHaveLength(3);
  });

  it("une requête insérée propose le service, l'opération et les données disponibles (4.3-b quater)", async () => {
    const core = await openTest((c) => {
      c.contracts.files.set("contracts/petstore.yaml", {
        name: "Petstore",
        version: "1.0.0",
        source: PETSTORE_SOURCE,
        operations: structuredClone(PETSTORE_OPERATIONS),
      });
      const created = c.workbench.requests.get("Clients/creer.yml");
      if (created)
        created.extractions = [
          { name: "client.id", source: "body.id", enabled: true, scope: "runtime" },
        ];
    });
    await page().findAll(".test__mode")[1]?.trigger("click");
    await flushPromises();
    await page()
      .findAll(".edge__plus")
      .find((b) => b.attributes("aria-label")?.includes("« Créer » et « Client créé ? »"))
      ?.trigger("click");
    const input = page().get(".board__picker .picker__input");
    await input.setValue("rest");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    const panel = page().get(".panel");
    expect(panel.text()).toContain("à configurer");
    expect(panel.text()).toContain("Après « Créer », puis vers « Client créé ? ».");
    // Données disponibles à ce point : ce que « Créer » extrait.
    expect(panel.text()).toContain("client.id de « Créer »");
    const [service, operation] = panel.findAll("select.panel__select");
    expect(operation?.attributes("disabled")).toBeDefined();
    await service?.setValue("contracts/petstore.yaml");
    expect(
      operation
        ?.findAll("option")
        .map((o) => o.text())
        .slice(1),
    ).toEqual([
      "GET /pets · Lister les animaux",
      "POST /pets · Créer un animal",
      "GET /health · Santé",
    ]);
    await operation?.setValue("GET /pets");
    await flushPromises();
    const call = core.calls.find((c) => c.cmd === "flow_step_operation");
    expect(call?.args).toMatchObject({
      path: "Clients",
      contract: "contracts/petstore.yaml",
      operation: "GET /pets",
    });
    expect(page().get(".panel").text()).toContain("{{baseUrl}}/pets");
  });

  it("« Parcourir le catalogue… » de la palette ouvre le catalogue des étapes", async () => {
    await openTest();
    await page().findAll(".test__mode")[1]?.trigger("click");
    await flushPromises();
    await click(page(), ".palette button", "Parcourir le catalogue…");
    expect(page().get("h1").text()).toBe("Catalogue des étapes");
  });

  it("un test arrêté dit où, et ouvre la requête de l'étape (4.3-d)", async () => {
    await openTest();
    await click(page(), ".test button", "Exécuter le test");
    await click(page(), ".zone__tab-button", "Clients");
    // Fin du test telle que le cœur la donne : arrêté par « Créer ».
    const run = Object.values(useWorkbenchStore().runs)[0];
    if (!run) throw new Error("run attendu");
    run.ends = [{ test: "Clients", passed: false, reason: null, failedStep: "r:creer.yml" }];
    await page().findAll(".test__mode")[1]?.trigger("click");
    await flushPromises();
    const stopped = page().get(".diagram__stopped");
    expect(stopped.text()).toContain("✕ Arrêtée à « Créer »");
    await click(page(), ".diagram__stopped button", "Ouvrir la requête");
    expect(
      page()
        .findAll(".zone__tab-button")
        .some((t) => t.text().includes("Créer")),
    ).toBe(true);
  });

  it("après exécution, marque chaque étape et suit le chemin", async () => {
    await openTest();
    await click(page(), ".test button", "Exécuter le test");
    await click(page(), ".zone__tab-button", "Clients");
    await page().findAll(".test__mode")[1]?.trigger("click");
    await flushPromises();
    const marks = page()
      .findAll(".node__mark")
      .map((m) => m.text());
    expect(marks[0]).toMatch(/^✓/);
    expect(page().findAll(".links__path--taken").length).toBeGreaterThan(0);
  });
});
