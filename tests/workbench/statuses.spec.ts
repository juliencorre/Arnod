/**
 * Résultat de la dernière exécution dans la barre latérale
 * (`docs/plan-fin-premier-usage.md`, ST1 à ST4 ; règles R4 à R6 testées dans le cœur).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { countStatuses, countTitle } from "@/lib/status";

import { okOutcome } from "../fakeWorkbench";
import { BOUTIQUE_TESTS } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function row(name: string) {
  const found = app?.findAll('[role="treeitem"]').find((r) => r.text().includes(name));
  if (!found) throw new Error(`ligne introuvable : ${name}`);
  return found;
}

describe("comptage des statuts (R6)", () => {
  it("compte réussies et exécutées parmi les requêtes", () => {
    const count = countStatuses(["a", "b", "c"], {
      a: { passed: true, at: 1 },
      b: { passed: false, at: 1 },
      z: { passed: true, at: 1 },
    });
    expect(count).toEqual({ passed: 1, executed: 2, total: 3 });
    expect(countTitle(count)).toBe("1 réussie sur 2 exécutées (3 requêtes)");
    expect(countStatuses(["a"], undefined)).toEqual({ passed: 0, executed: 0, total: 1 });
  });
});

describe("statuts dans la barre latérale (ST1 à ST4)", () => {
  it("relit les statuts gardés et compte les dossiers (ST2, ST4)", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.statuses = {
        "Clients/lister.yml": { passed: true, at: 1 },
        "Clients/creer.yml": { passed: false, at: 2 },
      };
    }));
    expect(row("Lister").get(".row__status").text()).toContain("✓");
    expect(row("Créer").get(".row__status").classes()).toContain("status-mark--ko");
    expect(row("Santé").find(".row__status").exists()).toBe(false);
    const count = row("Clients").get(".row__count");
    expect(count.get('[aria-hidden="true"]').text()).toBe("1/2");
    expect(count.classes()).toContain("status-count--ko");
    expect(count.attributes("title")).toBe("1 réussie sur 2 exécutées (2 requêtes)");
  });

  it("un dossier jamais exécuté garde son nombre de requêtes", async () => {
    ({ app } = await openProject());
    expect(row("Clients").get(".row__count").text()).toBe("2");
  });

  it("un envoi met à jour la ligne de la requête (ST1, ST3)", async () => {
    ({ app } = await openProject((c) => {
      c.workbench.nextSend = okOutcome({ passed: false });
    }));
    await click(app, '[role="treeitem"]', "Lister");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    expect(row("Lister").get(".row__status").text()).toContain("✕");
    expect(row("Clients").get(".row__count").attributes("title")).toBe(
      "0 réussie sur 1 exécutée (2 requêtes)",
    );
  });

  it("suit les statuts diffusés par le cœur (ST3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await core.emit("arnod://statuts", {
      projectId: BOUTIQUE_TESTS,
      statuses: { "sante.yml": { passed: true, at: 3 } },
    });
    await flushPromises();
    expect(row("Santé").get(".row__status").text()).toContain("✓");
  });
});
