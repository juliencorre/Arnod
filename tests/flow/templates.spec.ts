/**
 * Dossiers et templates (docs/plan-v4.md, lots E à G) : Avant / Après (4.7-c), propriétés
 * (4.8-a), créer un template depuis un dossier (4.8-b), le template (4.8-c), son utilisation
 * (4.8-d), le transfert d'un endpoint (3.2-d) et « Plus » (1.2-d bis), sur le faux cœur.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
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

/** « v2 » : une requête libre et un sous-dossier, comme 4.7-c. */
async function openFolder(setup?: (core: FakeCore) => void): Promise<FakeCore> {
  let core: FakeCore;
  ({ app, core } = await openProject((c) => {
    c.workbench.folders.add("v2");
    c.workbench.folders.add("v2/cas");
    c.workbench.requests.set("v2/lister.yml", newRequest("Lister les commandes"));
    c.workbench.requests.set("v2/cas/e404.yml", newRequest("Erreur 404"));
    c.workbench.folderVars.set("v2", [["devise", "EUR"]]);
    setup?.(c);
  }));
  await click(app, '[role="treeitem"]', "v2");
  return core;
}

describe("Avant / Après d'un dossier (4.7-c)", () => {
  it("ajoute une requête « Avant le dossier » ; elle quitte l'arborescence (R14)", async () => {
    const core = await openFolder();
    await click(page(), ".folder button", "+ Avant / Après");
    const before = page()
      .findAll(".hooks__phase")
      .find((p) => p.text().includes("Avant le dossier"));
    expect(before?.text()).toContain("une fois, au début de v2");
    await before
      ?.findAll("button")
      .find((b) => b.text().includes("+ Étape"))
      ?.trigger("click");
    await flushPromises();
    await page().get(".hooks__picker .picker__input").trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "hooks_add_step")?.args).toMatchObject({
      path: "v2",
      phase: "beforeAll",
      kind: "request",
    });
    expect(
      page()
        .findAll(".hooks__row")
        .map((r) => r.text()),
    ).toEqual([expect.stringContaining("Nouvelle requête")]);
    // La requête de l'étape n'est plus une ligne du référentiel.
    const rows = page()
      .findAll('[role="treeitem"]')
      .map((r) => r.text());
    expect(rows.some((r) => r.includes("Nouvelle requête"))).toBe(false);
  });
});

describe("propriétés d'un dossier (4.8-a)", () => {
  it("montre leur origine et en ajoute une, définie ici", async () => {
    const core = await openFolder();
    await click(page(), ".folder__view", "Propriétés");
    const table = page().get('[aria-label="Propriétés du dossier"]');
    expect(table.text()).toContain("devise");
    expect(table.text()).toContain("défini ici");
    expect(table.text()).toContain("environnement Local");
    await click(page(), ".props button", "+ Propriété");
    await page().get('[aria-label="Nom de la propriété"]').setValue("montant");
    await page().get('[aria-label="Valeur de la propriété"]').setValue("129.90");
    await click(page(), ".props button", "Ajouter");
    expect(core.calls.filter((c) => c.cmd === "folder_set_properties").at(-1)?.args).toMatchObject({
      path: "v2",
      properties: [
        { name: "devise", value: "EUR" },
        { name: "montant", value: "129.90" },
      ],
    });
  });
});

describe("templates (4.8-b à 4.8-d)", () => {
  it("crée un template du dossier, l'ouvre et remplace le dossier par une utilisation", async () => {
    const core = await openFolder();
    await click(page(), ".folder button", "Créer un template de ce dossier");
    await flushPromises();
    const dialog = page().get(".tpl");
    expect(dialog.text()).toContain("devise");
    expect(dialog.text()).toContain("propriété du dossier");
    await click(page(), ".tpl button", "Créer le template");
    expect(core.calls.find((c) => c.cmd === "template_create")?.args).toMatchObject({
      folder: "v2",
      name: "v2",
      replace: true,
      params: [{ name: "devise", default: "EUR" }],
    });
    // Le template s'ouvre ; la barre latérale gagne la section « Templates » (R21).
    await new Promise((resolve) => setTimeout(resolve, 20));
    await flushPromises();
    expect(page().get(".template__title").text()).toBe("v2");
    expect(page().get(".template").text()).toContain("Utilisations 1");
    expect(page().find(".templates").exists()).toBe(true);
    // Le dossier d'origine est une utilisation : on n'y règle que les valeurs.
    await click(page(), '[role="treeitem"]', "v2");
    expect(page().get(".instance").text()).toContain("Utilise le template v2");
    await page().get('[aria-label="Valeur de devise"]').setValue("USD");
    await page().get('[aria-label="Valeur de devise"]').trigger("change");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "template_set_values")?.args).toMatchObject({
      path: "v2",
      values: { devise: "USD" },
    });
  });

  it("transfère un endpoint vers les templates : les valeurs cochées deviennent des paramètres (3.2-d)", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "commande.yml",
        newRequest("Commande simple", {
          method: "POST",
          contract: { file: "contracts/commandes.yaml", operation: "POST /orders" },
          body: {
            kind: "json",
            variants: [
              {
                title: "",
                content:
                  '{"customerId":"cus_31","items":[{"sku":"TSH","qty":2}],"coupon":"BIENVENUE"}',
                form: [],
              },
            ],
            selected: 0,
            other: "",
          },
        }),
      );
    }));
    await click(app, '[role="treeitem"]', "Commande simple");
    await click(app, ".trial-transfer button", "Transférer vers les templates");
    await flushPromises();
    const rows = app.findAll(".transfer__literal").map((r) => r.text().replace(/\s+/g, ""));
    expect(rows).toEqual([
      "customerIdcus_31devientunparamètre",
      "skuTSHdevientunparamètre",
      "qty2devientunparamètre",
      "couponBIENVENUErestefixe",
    ]);
    await app.get(".transfer-template").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "template_from_request")?.args).toMatchObject({
      path: "commande.yml",
      params: ["customerId", "sku", "qty"],
    });
  });
});

describe("« Plus » (1.2-d bis)", () => {
  it("propose Templates avec sa raison d'être ; « Afficher » garde la section affichée", async () => {
    let core;
    ({ app, core } = await openProject());
    const toggle = app.get('[aria-label="Plus de sections pour boutique-tests : 1"]');
    await toggle.trigger("click");
    expect(app.get(".more__list").text()).toContain("Des modèles de requêtes et de tests");
    await click(app, ".more__list button", "Afficher");
    expect(core.calls.find((c) => c.cmd === "sections_show")?.args).toMatchObject({
      section: "templates",
      shown: true,
    });
    expect(app.find(".templates").exists()).toBe(true);
    expect(app.find(".more").exists()).toBe(false);
  });
});
