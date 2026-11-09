import { clearMocks } from "@tauri-apps/api/mocks";
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
      .findAll('li[role="none"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Clients"));
    await row?.get(".row__more").trigger("click");
    await click(app, '[role="menuitem"]', "Exécuter le dossier");

    expect(core.calls.find((c) => c.cmd === "run_start")?.args).toMatchObject({
      folder: "Clients",
      environment: "Local",
    });
    const run = app.get(".run");
    expect(run.get(".run__progress").text()).toContain("2/2");
    expect(run.get(".run__progress").text()).toContain("1 en échec");
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
    await click(app, ".tree__head button", "Tout exécuter");
    await click(app, ".run button", "Relancer les échecs");
    expect(core.commands()).toContain("run_retry_failures");
    expect(app.findAll(".run__item")).toHaveLength(1);
    expect(app.get(".run__item").text()).toContain("Créer");
  });

  it("indique l'erreur de variable d'une requête", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.runStatus.set("sante.yml", "error");
    }));
    await click(app, ".tree__head button", "Tout exécuter");
    expect(app.get(".run").text()).toContain("Variable « x » absente de l'environnement Local.");
  });
});
