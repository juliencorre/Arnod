/**
 * Jeux de données (docs/plan-v4.md, lot D) : l'onglet « Données » d'une requête (4.7-a), son
 * envoi par cas (4.7-b) et la ligne du run qui compte une fois (4.7-b bis), sur le faux cœur.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";

import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe("onglet « Données » (4.7-a)", () => {
  it("commence un tableau d'après les variables de l'adresse et l'enregistre seul", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "commandes.yml",
        newRequest("Lister les commandes", {
          url: "{{baseUrl}}/orders?status={{statut}}&limit={{limite}}",
        }),
      );
    }));
    await click(app, '[role="treeitem"]', "Lister les commandes");
    await click(app, ".editor__tab", "Données");
    await click(app, ".data button", "+ Tableau");
    const heads = app
      .findAll(".data__input--head")
      .map((i) => (i.element as HTMLInputElement).value);
    expect(heads).toEqual(["statut", "limite"]);
    const cells = app.findAll("tbody .data__input");
    await cells[0]?.setValue("pending");
    await cells[1]?.setValue("10");
    await click(app, ".data button", "+ Cas");
    const more = app.findAll("tbody .data__input");
    await more[2]?.setValue("cancelled");
    await more[3]?.setValue("10");
    await wait(AUTOSAVE_MS + 50);
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "data_save").at(-1)?.args).toMatchObject({
      path: "commandes.yml",
      table: {
        columns: ["statut", "limite"],
        rows: [
          ["pending", "10"],
          ["cancelled", "10"],
        ],
      },
    });
    // La requête du cas choisi se lit résolue.
    await flushPromises();
    expect(app.get(".data__case-view").text()).toContain("Cas 1 sur 2");
  });

  it("« Envoyer » joue chaque cas ; la réponse montrée est celle du cas choisi (4.7-b)", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "commandes.yml",
        newRequest("Lister les commandes", {
          url: "{{baseUrl}}/orders?status={{statut}}",
          data: { file: "données/lister.csv", editable: true },
        }),
      );
      c.workbench.datasets.set("commandes.yml", {
        columns: ["statut"],
        rows: [["pending"], ["cancelled"], ["paid"]],
      });
    }));
    await click(app, '[role="treeitem"]', "Lister les commandes");
    await click(app, "button", "Envoyer");
    expect(core.calls.some((c) => c.cmd === "request_send_cases")).toBe(true);
    const bar = app.get(".cases");
    expect(bar.text()).toContain("Cas 1 sur 3");
    expect(bar.text()).toContain("2 ✓");
    expect(bar.text()).toContain("1 ✕");
    await click(app, ".cases__chip", "2");
    expect(app.get(".cases__failure").text()).toContain("✕ Cas 2");
    await click(app, ".cases__failure button", "Ignorer ce cas pour ce résultat");
    expect(app.find(".cases__failure").exists()).toBe(false);
  });
});

describe("dans un run (4.7-b bis)", () => {
  it("la requête compte une fois, avec son détail par cas", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.requests.set(
        "commandes.yml",
        newRequest("Lister les commandes", { data: { file: "données/l.csv", editable: true } }),
      );
      c.workbench.datasets.set("commandes.yml", {
        columns: ["statut"],
        rows: [["cancelled"], ["paid"]],
      });
      c.workbench.runStatus.set("commandes.yml", "failed");
    }));
    const run = app
      .findAll("button")
      .find((b) => b.attributes("aria-label")?.startsWith("Exécuter 4 requêtes"));
    await run?.trigger("click");
    await flushPromises();
    const row = app.findAll(".run__item").find((r) => r.text().includes("Lister les commandes"));
    expect(row?.text()).toContain("2 cas");
    expect(row?.text()).toContain("✕ cas 1");
    await row
      ?.findAll("button")
      .find((b) => b.text().includes("Voir les 2 cas"))
      ?.trigger("click");
    expect(row?.findAll(".run__case")).toHaveLength(2);
  });
});
