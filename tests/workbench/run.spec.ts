import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

describe("run d'un dossier", () => {
  it("exécute le dossier et affiche le résultat par requête", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    const row = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Clients"));
    await row?.get(".row__more").trigger("click");
    await click(app, '[role="menuitem"]', "Exécuter le dossier");

    expect(core.calls.find((c) => c.cmd === "run_start")?.args).toMatchObject({
      folder: "Clients",
      environment: "Local",
    });
    const run = app.get(".run");
    expect(run.get(".run__progress").text().replace(/\s+/g, " ")).toBe("1 réussi, 1 en échec");
    expect(run.get(".run__meta").text()).toBe("run #1 · Local · 8 en même temps · 4,1 s");
    // « Échecs » est montré d'abord ; « Tous » liste chaque requête.
    expect(run.findAll(".run__item")).toHaveLength(1);
    await click(app, ".run__tab", "Tous");
    const items = run.findAll(".run__item").map((i) => i.text().replace(/\s+/g, " "));
    expect(items[0]).toContain("Lister");
    expect(items[0]).toContain("Réussie");
    expect(items[1]).toContain("Créer");
    expect(items[1]).toContain("Échec");
  });

  it("relance seulement les échecs", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    await click(app, ".run button", "Relancer les échecs");
    expect(core.commands()).toContain("run_retry_failures");
    await click(app, ".run__tab", "Tous");
    expect(app.findAll(".run__item")).toHaveLength(1);
    expect(app.get(".run__item").text()).toContain("Créer");
    expect(app.get(".run__meta").text()).toContain("run #2");
  });

  it("indique l'erreur de variable d'une requête", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.runStatus.set("sante.yml", "error");
    }));
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    expect(app.get(".run").text()).toContain("Variable « x » absente de l'environnement Local.");
  });
});

describe("écran du run (maquette 1f)", () => {
  const amountCheck = {
    id: "montant",
    target: "body.amount",
    passed: false,
    message: 'attendu number, reçu "129.90"',
    actual: '"129.90"',
    ignored: false,
  };

  it("détaille un échec et l'ignore pour ce run (RU3, VT2)", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
      c.workbench.runChecks.set("Clients/creer.yml", [amountCheck]);
    }));
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    const failed = app.get(".run__item--failed");
    expect(failed.get(".run__check").text()).toBe("— body.amount");
    expect(failed.get(".run__failure").text()).toContain(
      'body.amount : attendu number, reçu "129.90"',
    );

    await click(app, ".run__link", "Ignorer pour ce run");

    expect(core.calls.find((c) => c.cmd === "run_ignore_check")?.args).toMatchObject({
      path: "Clients/creer.yml",
      check: "montant",
    });
    expect(app.get(".run__progress").text()).toContain("3 réussis · tout est réussi");
    await click(app, ".run__tab", "Tous");
    expect(app.get(".run__ignored").text()).toBe("body.amount : ignorée pour ce run");
    // Le statut de la requête suit dans la barre latérale (R7).
    const row = app.findAll('[role="treeitem"]').find((r) => r.text().includes("Créer"));
    expect(row?.get(".row__status").attributes("title")).toMatch(/^Réussie le /);
    expect(row?.get(".row__status").text()).toBe("✓, réussie");
  });

  it("filtre par dossier et compte les lents (RU2, RU4)", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    const groups = app
      .findAll(".run__group")
      .map((g) => `${g.get(".run__group-name").text()} ${g.get(".run__group-count").text()}`);
    expect(groups).toEqual(["Clients 1/2", "Projet 1/1"]);
    expect(app.findAll(".run__tab").map((t) => t.text().replace(/\s+/g, " "))).toEqual([
      "Échecs 1",
      "Tous 3",
      "Lents 0",
    ]);
    await click(app, ".run__tab", "Tous");
    expect(app.findAll(".run__section").map((h) => h.text())).toEqual(["Clients", "Projet"]);

    await click(app, ".run__group", "Projet");
    expect(app.get(".run__group[aria-pressed='true']").text()).toContain("Projet");
    expect(app.findAll(".run__item").map((i) => i.get(".run__name").text())).toEqual(["Santé"]);
    await click(app, ".run__group", "Projet");
    expect(app.findAll(".run__item")).toHaveLength(3);

    await app.get(".run__layout select").setValue("flat");
    expect(app.findAll(".run__section")).toHaveLength(0);
    await click(app, ".run__tab", "Lents");
    expect(app.get(".run__none").text()).toBe("Aucune requête lente.");
  });

  it("change d'onglet au clavier (RU2)", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    const selected = (): string => app?.get('.run__tab[aria-selected="true"]').text() ?? "";
    expect(selected()).toContain("Échecs");
    await app.get('.run__tab[aria-selected="true"]').trigger("keydown", { key: "ArrowRight" });
    expect(selected()).toContain("Tous");
    await app.get('.run__tab[aria-selected="true"]').trigger("keydown", { key: "End" });
    expect(selected()).toContain("Lents");
    await app.get('.run__tab[aria-selected="true"]').trigger("keydown", { key: "ArrowRight" });
    expect(selected()).toContain("Échecs");
  });

  it("suit l'avancement diffusé par le cœur, sans revenir en arrière (RU5, R3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    const run = structuredClone(core.workbench.lastRun);
    if (!run) throw new Error("run absent");
    const older = { ...structuredClone(run), revision: 0, finished: false };
    const projectId = core.calls.find((c) => c.cmd === "run_start")?.args.project;
    await core.emit("arnod://run", { projectId, run: older, origin: "" });
    await flushPromises();
    expect(app.get(".run__progress").text()).toContain("tout est réussi");

    const newer = {
      ...structuredClone(run),
      revision: run.revision + 1,
      finished: false,
      finishedAt: null,
    };
    const first = newer.items[0];
    if (first) first.status = "running";
    await core.emit("arnod://run", { projectId, run: newer, origin: "" });
    await flushPromises();
    expect(app.get(".run__progress").text()).toBe("En cours · 2/3");
    expect(app.get(".run button").text()).toBe("Annuler");
  });

  it("annonce un dossier sans requête (RU6)", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.folders.add("Vide");
    }));
    const row = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Vide"));
    await row?.get(".row__more").trigger("click");
    await click(app, '[role="menuitem"]', "Exécuter le dossier");
    expect(core.commands()).toContain("run_start");
    expect(app.get(".run__empty").text()).toBe("Aucune requête à exécuter dans ce dossier.");
  });
});
