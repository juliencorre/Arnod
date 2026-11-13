/**
 * Lot E : tests et étapes (docs/plan-mvp-e-tests-etapes.md) — onglet du test, nouveau test,
 * « + Étape », onglet « Variables », « Envoyé » et « Garder », « modifié, pas encore envoyé ».
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";
import type { RequestDoc } from "@/types/ipc";

import { newRequest, okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function waitAutosave(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
  await flushPromises();
}

describe("test = dossier d'étapes (4.2-a)", () => {
  it("ouvre l'onglet du test, ses étapes numérotées, et l'exécute", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      const creer = c.workbench.requests.get("Clients/creer.yml");
      if (creer) creer.extractions = [{ name: "client.id", source: "body.id", enabled: true }];
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    // La fiole remplace le dossier dans la barre latérale.
    const row = app.findAll('[role="treeitem"]').find((r) => r.text().includes("Clients"));
    expect(row?.find(".row__flask").exists()).toBe(true);
    await click(app, '[role="treeitem"]', "Clients");

    const tab = app.get(".test");
    expect(tab.get(".test__title").text()).toBe("Clients");
    expect(tab.get(".zone__badge, .test__badge").text()).toBe("TEST");
    const steps = tab.findAll(".step").map((s) => s.text().replace(/\s+/g, " "));
    expect(steps[0]).toContain("1POSTCréer");
    expect(steps[0]).toContain("→ client.id");
    expect(steps[1]).toContain("2GETLister");
    expect(app.findAll(".zone__badge").map((b) => b.text())).toContain("TEST");

    await click(app, ".test button", "Exécuter le test");
    expect(core.calls.find((c) => c.cmd === "run_start")?.args).toMatchObject({
      folder: "Clients",
    });
    // Le run s'ouvre ; revenu au test, l'étape 2 est « non jouée ».
    await click(app, ".zone__tab-button", "Clients");
    const marks = app.findAll(".step__mark").map((m) => m.text());
    expect(marks[0]).toMatch(/^✕ 500/);
    expect(marks[1]).toBe("non jouée");
    expect(app.get(".test__summary").text()).toContain("en échec à l'étape 1");
  });

  it("« + Étape » ajoute l'étape suivante et l'ouvre, curseur dans l'adresse", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Clients");
    await click(app, ".test button", "+ Étape");
    expect(core.calls.find((c) => c.cmd === "tree_add_step")?.args).toMatchObject({
      test: "Clients",
    });
    expect((app.get(".editor .meta__input").element as HTMLInputElement).value).toBe("Étape 3");
    expect(document.activeElement).toBe(app.get(".editor__url").element);
    // La barre de l'étape montre les étapes du test, celle-ci en évidence.
    const chips = app.findAll(".steps__chip");
    expect(chips.map((c) => c.text())).toEqual(["1 · Créer", "2 · Lister", "3 · Étape 3"]);
    expect(app.get('.steps__chip[aria-current="step"]').text()).toBe("3 · Étape 3");
  });
});

describe("nouveau test (4.1-b)", () => {
  it("crée le dossier du test et son étape 1, ouverte, au clavier", async () => {
    let core;
    ({ app, core } = await openProject());
    const add = app.get('[aria-label="Ajouter au référentiel de tests de boutique-tests"]');
    await add.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const items = app.findAll('[role="menuitem"]');
    expect(items[2]?.text()).toBe("Nouveau test");
    await items[0]?.trigger("keydown", { key: "ArrowDown" });
    await items[1]?.trigger("keydown", { key: "ArrowDown" });
    await items[2]?.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_test")?.args).toMatchObject({
      parent: "",
    });
    const names = app.findAll('[role="treeitem"]').map((i) => i.text());
    expect(names).toContain("▾Nouveau test1");
    expect(names).toContain("GET1Étape 1");
    expect(document.activeElement).toBe(app.get(".editor__url").element);
    expect(app.get(".steps__hint").text()).toBe(
      "Un test est un dossier d'étapes ; celle-ci est la première.",
    );
  });
});

describe("extractions (4.2-b bis)", () => {
  it("ajoute une variable dans l'onglet « Variables » et l'enregistre", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/lire.yml",
        newRequest("Lire", { url: "{{baseUrl}}/clients/{{client.id}}" }),
      );
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Variables");
    await click(app, ".vars__button", "+ Variable");
    await app.get(".vars__name").setValue("client.id");
    await app.findAll(".vars__input")[1]?.setValue("body.id");
    await waitAutosave();
    const saved = core.workbench.requests.get("Clients/creer.yml");
    expect(saved?.extractions).toEqual([{ name: "client.id", source: "body.id", enabled: true }]);
    expect(app.get(".editor__tab[aria-selected='true']").text()).toBe("Variables 1");
    // « Utilisée par » liste les requêtes qui lisent la variable.
    await flushPromises();
    expect(app.get(".vars__users").text()).toContain("Lire");
  });

  it("montre la dernière valeur extraite et ce qui est parti dans « Envoyé »", async () => {
    ({ app } = await openProject((c) => {
      const creer = c.workbench.requests.get("Clients/creer.yml");
      if (creer) creer.extractions = [{ name: "client.id", source: "body.id", enabled: true }];
      c.workbench.nextSend = okOutcome({
        sent: {
          method: "POST",
          url: "https://api.test/clients",
          headers: [["Content-Type", "application/json"]],
          body: '{"ref":"7c1e2b04-5d3a-4f6e-9b21-0c8d4e7a91f3"}',
          drawn: [
            {
              name: "$randomUUID",
              value: "7c1e2b04-5d3a-4f6e-9b21-0c8d4e7a91f3",
              source: "request.body.ref",
            },
            { name: "$timestamp", value: "1794495731", source: null },
          ],
          used: [{ name: "baseUrl", value: "https://api.test", runtime: false }],
          extracted: [{ name: "client.id", source: "body.id", value: "cus_7", message: null }],
        },
      });
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    await click(app, ".editor__tab", "Variables");
    expect(app.get(".vars__value").text()).toBe("cus_7");

    await click(app, ".response__tab", "Envoyé");
    const sent = app.get(".sent");
    expect(sent.get(".sent__url").text()).toBe("https://api.test/clients");
    expect(sent.get(".sent__body").text()).toContain('"ref": "7c1e2b04');
    expect(sent.text()).toContain("Valeurs tirées à cet envoi · 2");
    expect(sent.text()).toContain("Variables utilisées · 1");
    // « Garder » : une extraction de la valeur envoyée, nommée d'après sa place (VD3).
    await click(app, ".sent__keep");
    expect(app.get(".sent__kept").text()).toBe("gardée : ref");
    expect(app.findAll(".sent__keep")).toHaveLength(0);
    await click(app, ".editor__tab", "Variables");
    expect(app.findAll(".vars__name").map((i) => (i.element as HTMLInputElement).value)).toEqual([
      "client.id",
      "ref",
    ]);
  });
});

describe("modifié, pas encore envoyé (écart 4.2-b)", () => {
  it("signale une modification après l'envoi et revient à la version envoyée", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    expect(app.find(".editor__unsent").exists()).toBe(false);
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    expect(app.find(".editor__unsent").exists()).toBe(false);

    await app.get(".editor__url").setValue("{{baseUrl}}/autre");
    expect(app.get(".editor__unsent").text()).toContain("modifié, pas encore envoyé");
    await click(app, ".editor__saved button", "Revenir à la version envoyée");
    expect((app.get(".editor__url").element as HTMLInputElement).value).toBe("{{baseUrl}}/clients");
    expect(app.find(".editor__unsent").exists()).toBe(false);
    await waitAutosave();
    expect((core.workbench.requests.get("Clients/lister.yml") as RequestDoc).url).toBe(
      "{{baseUrl}}/clients",
    );
  });
});
