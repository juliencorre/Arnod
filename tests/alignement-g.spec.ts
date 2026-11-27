/**
 * Alignement sur la maquette, lot G (docs/plan-alignement-maquette.md, T18 et T19) : page des
 * environnements et portée d'une extraction. Chaque test est la preuve d'un écart `aligner: code`
 * de docs/maquette-etat.yml.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";

import { newRequest } from "./fakeWorkbench";
import type { AppWrapper } from "./mountApp";
import { click, openProject } from "./workbench/helpers";

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

describe("portée d'une extraction (AM44, 4.2-b bis)", () => {
  it("« le test ▾ » se change en « l'environnement », écrit dans la requête", async () => {
    const opened = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/creer.yml",
        newRequest("Créer", {
          method: "POST",
          extractions: [{ name: "client.id", source: "body.id", enabled: true, scope: "runtime" }],
        }),
      );
    });
    app = opened.app;
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Variables");

    const scope = app.get<HTMLSelectElement>("select[aria-label='Portée de client.id']");
    expect(scope.element.value).toBe("runtime");
    expect(scope.findAll("option").map((o) => o.text())).toEqual(["le test", "l'environnement"]);
    await scope.setValue("environment");
    await waitAutosave();
    expect(opened.core.workbench.requests.get("Clients/creer.yml")?.extractions).toEqual([
      { name: "client.id", source: "body.id", enabled: true, scope: "environment" },
    ]);
  });

  it("une portée écrite à la main (« collection ») reste montrée et gardée", async () => {
    const opened = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/creer.yml",
        newRequest("Créer", {
          method: "POST",
          extractions: [
            { name: "lieu", source: "headers.Location", enabled: true, scope: "collection" },
          ],
        }),
      );
    });
    app = opened.app;
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Variables");
    const scope = app.get<HTMLSelectElement>("select[aria-label='Portée de lieu']");
    expect(scope.element.value).toBe("collection");
    expect(scope.findAll("option").map((o) => o.text())).toEqual([
      "le test",
      "l'environnement",
      "le projet (comme le test)",
    ]);
  });
});
