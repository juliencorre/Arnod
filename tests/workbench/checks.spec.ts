/**
 * Vérifications en échec sous la réponse (`docs/plan-fin-premier-usage.md`, VT1 à VT4 ; règles R7
 * et R8 testées dans le cœur).
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

/** « Créer » vérifie l'en-tête Location, que la réponse ne renvoie pas. */
async function sendFailing() {
  const opened = await openProject((c) => {
    const base = newRequest("Créer", { method: "POST" });
    c.workbench.requests.set(
      "Clients/creer.yml",
      newRequest("Créer", {
        method: "POST",
        assertions: [
          ...base.assertions,
          {
            id: "location",
            target: "headers.Location",
            operator: "exists",
            value: null,
            origin: "manual",
            enabled: true,
          },
        ],
      }),
    );
    c.workbench.nextSend = okOutcome({
      passed: false,
      assertions: [
        ...okOutcome().assertions,
        {
          id: "location",
          target: "headers.Location",
          passed: false,
          message: "en-tête absent de la réponse",
          actual: null,
          ignored: false,
        },
      ],
    });
  });
  app = opened.app;
  await click(app, '[role="treeitem"]', "Créer");
  await app.get(".editor__line").trigger("submit");
  await flushPromises();
  return opened;
}

describe("vérifications en échec (VT1 à VT4)", () => {
  it("détaille l'échec sous la réponse (VT1)", async () => {
    await sendFailing();
    const failure = app?.get(".failure");
    expect(failure?.get(".failure__title").text()).toBe("✕ headers.Location");
    expect(failure?.get(".failure__message").text()).toBe("en-tête absent de la réponse");
    expect(app?.get(".response__verdict").text()).toBe("Échec · 1 vérification(s)");
  });

  it("ignore l'échec pour ce run, sans toucher à la requête (VT2)", async () => {
    const { core } = await sendFailing();
    await click(app as AppWrapper, ".failure__link", "Ignorer pour ce run");
    expect(core.calls.find((c) => c.cmd === "result_ignore_check")?.args).toMatchObject({
      path: "Clients/creer.yml",
      check: "location",
    });
    expect(app?.find(".failure").exists()).toBe(false);
    expect(app?.get(".response__verdict").text()).toBe("Réussie");
    const saved = core.workbench.requests.get("Clients/creer.yml");
    expect(saved?.assertions.some((a) => a.target === "headers.Location")).toBe(true);
    // Le statut de la requête suit dans la barre latérale.
    const row = app?.findAll('[role="treeitem"]').find((r) => r.text().includes("Créer"));
    expect(row?.get(".row__status").text()).toContain("✓");
  });

  it("retire la vérification de la requête (VT3)", async () => {
    const { core } = await sendFailing();
    await click(app as AppWrapper, ".failure__link", "Retirer la vérification");
    expect(app?.find(".failure").exists()).toBe(false);
    await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
    await flushPromises();
    const saves = core.calls.filter((c) => c.cmd === "request_save");
    const last = saves.at(-1)?.args.request as RequestDoc | undefined;
    expect(last?.assertions.map((a) => a.target)).toEqual(["responseTime"]);
  });

  it("le bouton se déclenche au clavier et l'origine s'affiche (VT4)", async () => {
    await sendFailing();
    const ignore = app?.findAll(".failure__link").find((b) => b.text() === "Ignorer pour ce run");
    expect(ignore?.element.tagName).toBe("BUTTON");
    expect(ignore?.attributes("aria-label")).toBe("Ignorer headers.Location pour ce run");
    await click(app as AppWrapper, '.editor [role="tab"]', "Vérifications");
    const origins = app?.findAll(".assertion__origin").map((o) => o.text());
    expect(origins).toEqual(["par défaut", "manuelle"]);
  });
});
