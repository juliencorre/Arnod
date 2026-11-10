import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { EVENTS } from "@/lib/ipc";
import type { RequestResult, ResultChange, RunChange } from "@/types/ipc";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { newWindow, tab } from "../fakeWindows";
import { newRequest, okOutcome, runOf } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

/** Résultat détachable (docs/plan-resultat-detachable.md, D1 à D7). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const LISTER = "Clients/lister.yml";
const lister = { kind: "request", projectId: BOUTIQUE_TESTS, path: LISTER } as const;

function result(overrides: Partial<RequestResult> = {}): RequestResult {
  return {
    projectId: BOUTIQUE_TESTS,
    path: LISTER,
    sending: false,
    outcome: okOutcome(),
    error: null,
    receivedAt: 1_700_000_000_000,
    ...overrides,
  };
}

function shared(overrides: Partial<RequestResult> = {}): ResultChange {
  return { result: result(overrides), origin: "fenetre-autre" };
}

/** Faux cœur dont l'interface sert la fenêtre de résultat ; `lister` est ouverte dans la principale. */
function resultWindow(): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  core.workbench.folders.add("Clients");
  core.workbench.requests.set(LISTER, newRequest("Lister", { url: "{{baseUrl}}/clients" }));
  core.workbench.requests.set("sante.yml", newRequest("Santé"));
  const main = core.windows.windows[0];
  if (main?.layout?.root.kind === "zone") {
    const opened = tab(lister);
    main.layout.root.tabs.push(opened);
    main.layout.root.activeTab = opened.id;
  }
  const window = newWindow("content", [tab({ kind: "result" })]);
  window.follow = false;
  core.windows.windows.push(window);
  core.windows.label = window.label;
  return core;
}

describe("résultat détachable", () => {
  it("⧉ ouvre le résultat dans sa fenêtre ; l'onglet de la requête garde l'éditeur seul (D1, R3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    expect(app.find(".request-tab__response").exists()).toBe(true);

    const detach = app.get('button[aria-label="Ouvrir le résultat dans une fenêtre"]');
    (detach.element as HTMLButtonElement).focus();
    expect(document.activeElement).toBe(detach.element);
    await detach.trigger("click");
    await flushPromises();

    expect(core.calls.some((c) => c.cmd === "window_detach_result")).toBe(true);
    const created = core.windows.windows.find((w) => w.kind === "content");
    expect(created?.follow).toBe(false);
    expect(app.find(".request-tab__response").exists()).toBe(false);
    expect(app.get(".request-tab").classes()).toContain("request-tab--alone");
    expect(app.find(".editor__url").exists()).toBe(true);
  });

  it("la fenêtre de résultat montre la requête active et « Suit la requête active » (D2)", async () => {
    resultWindow();
    app = await mountApp();

    expect(app.get(".content-window__follow").text()).toBe("⇆ Suit la requête active");
    expect(app.find("button.content-window__follow").exists()).toBe(false);
    expect(app.get('nav[aria-label="Emplacement"]').text()).toBe("Résultat");
    expect(app.get(".result-tab__name").text()).toBe("Lister");
    expect(app.get(".result-tab__where").text()).toBe("boutique-tests › Clients");
    expect(app.get(".response").text()).toContain("Envoyez la requête pour voir la réponse.");
  });

  it("suit la requête active diffusée par le cœur, et reprend son dernier envoi (D2, R5)", async () => {
    const core = resultWindow();
    core.workbench.results.set("sante.yml", result({ path: "sante.yml" }));
    app = await mountApp();

    await core.emit(EVENTS.activeRequest, { projectId: BOUTIQUE_TESTS, path: "sante.yml" });
    await flushPromises();

    expect(app.get(".result-tab__name").text()).toBe("Santé");
    expect(app.get(".response").text()).toContain("200 OK");

    await core.emit(EVENTS.activeRequest, null);
    await flushPromises();
    expect(app.text()).toContain("Ouvrez une requête : son résultat s'affiche ici");
  });

  it("un envoi fait dans une autre fenêtre s'affiche : en cours, puis la réponse (D3)", async () => {
    const core = resultWindow();
    app = await mountApp();

    await core.emit(EVENTS.result, shared({ sending: true, outcome: null, receivedAt: null }));
    await flushPromises();
    expect(app.get(".response").text()).toContain("Envoi en cours…");

    await core.emit(EVENTS.result, shared());
    await flushPromises();
    expect(app.get(".response").text()).toContain("200 OK");
    expect(app.get(".response").text()).toContain("Réussie");

    await core.emit(EVENTS.result, shared({ outcome: null, error: "Hôte introuvable." }));
    await flushPromises();
    expect(app.get("[role='alert']").text()).toBe("Hôte introuvable.");
  });

  it("ignore l'écho de ses propres envois", async () => {
    const core = resultWindow();
    app = await mountApp();

    await core.emit(EVENTS.result, { result: result(), origin: core.windows.label });
    await flushPromises();

    expect(app.get(".response").text()).toContain("Envoyez la requête pour voir la réponse.");
  });

  it("« Réponses attendues » reste disponible et se parcourt au clavier (D4)", async () => {
    resultWindow();
    app = await mountApp();

    const tabs = app.findAll('[role="tablist"][aria-label="Résultat"] [role="tab"]');
    expect(tabs.map((t) => t.text())).toEqual(["Réponse", "Réponses attendues"]);
    await tabs[1]?.trigger("click");
    expect(app.get('[role="tabpanel"]').attributes("aria-label")).toBe("Réponses attendues");
  });

  it("un run démarré dans une autre fenêtre est suivi ici (D7, R7)", async () => {
    let core;
    ({ app, core } = await openProject());
    // Le cœur n'a pas encore de dernier run pour ce projet au chargement.
    const run = runOf(core.workbench, "");
    const change: RunChange = { projectId: BOUTIQUE_TESTS, run, origin: "fenetre-autre" };

    await core.emit(EVENTS.run, change);
    const main = core.windows.windows[0];
    if (main?.layout?.root.kind === "zone") {
      const opened = tab({ kind: "run", projectId: BOUTIQUE_TESTS });
      main.layout.root.tabs.push(opened);
      main.layout.root.activeTab = opened.id;
    }
    await core.emitWindows();
    await flushPromises();

    expect(app.text()).not.toContain("Aucun run pour l'instant");
    expect(app.find('section[aria-label="Run"]').exists()).toBe(true);
  });
});
