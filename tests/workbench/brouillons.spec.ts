/**
 * Brouillons et fenêtres (plan qualité, lot C : IN2, IN3, IN5, IN6, IN9). Tests de preuve repris de
 * l'audit (docs/audit-2026-11-mvp/preuves/interface/) : ils échouaient avant les corrections.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EVENTS } from "@/lib/ipc";
import { useSelectionStore } from "@/stores/selection";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { ProjectChange } from "@/types/ipc";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { newWindow, tab } from "../fakeWindows";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

const lister = { kind: "request", projectId: BOUTIQUE_TESTS, path: "Clients/lister.yml" } as const;

/** Faux cœur dont l'interface sert une fenêtre de contenu détachée sur « Lister ». */
function contentWindow(): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  core.workbench.folders.add("Clients");
  core.workbench.requests.set(
    "Clients/lister.yml",
    newRequest("Lister", { url: "{{baseUrl}}/clients" }),
  );
  const window = newWindow("content", [tab(lister)]);
  core.windows.windows.push(window);
  core.windows.label = window.label;
  return core;
}

async function wait(ms: number): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
  await flushPromises();
}

function saves(core: FakeCore): string[] {
  return core.calls.filter((c) => c.cmd === "request_save").map((c) => String(c.args.path));
}

function fromOther(overrides: Partial<ProjectChange>): ProjectChange {
  return {
    projectId: BOUTIQUE_TESTS,
    scope: "tree",
    path: null,
    origin: "fenetre-autre",
    renamed: [],
    removed: [],
    ...overrides,
  };
}

describe("fenêtre détachée : brouillon perdu", () => {
  it("« Rattacher » enregistre la requête modifiée avant de rendre l'onglet", async () => {
    const core = contentWindow();
    app = await mountApp();
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?actifs=1");

    await click(app, ".content-window__bar button", "Rattacher");
    const seen = { saves: saves(core), reattached: core.commands().includes("window_reattach") };
    // L'enregistrement automatique encore programmé part ici : dans l'application, la fenêtre
    // est déjà détruite.
    await wait(650);

    expect(seen).toEqual({ saves: ["Clients/lister.yml"], reattached: true });
  });

  it("la fermeture ne rattache pas une requête dont l'enregistrement est suspendu (secret)", async () => {
    const core = contentWindow();
    app = await mountApp();
    await click(app, ".editor__tab", "Auth");
    await app.get(".auth select").setValue("bearer");
    await app.get('.auth input[type="text"]').setValue("eyJhbGciOi.xyz");
    await wait(750);
    expect(app.get(".editor__saved").text()).toBe("Non enregistré : choisissez où va le jeton");
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?actifs=1");

    await core.emit(EVENTS.close, null);
    await flushPromises();

    // Le brouillon (adresse modifiée) n'est écrit nulle part : la fenêtre ne doit pas partir.
    expect({ saves: saves(core), reattached: core.commands().includes("window_reattach") }).toEqual(
      { saves: [], reattached: false },
    );
  });
});

describe("sortie d'Arnod : la fenêtre principale enregistre d'abord (IN3)", () => {
  function answers(core: FakeCore): unknown[] {
    return core.calls.filter((c) => c.cmd === "window_quit_ready").map((c) => c.args.saved);
  }

  it("enregistre le brouillon puis accepte de quitter", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?actifs=1");

    await core.emit(EVENTS.quit, null);
    await flushPromises();

    expect({ saves: saves(core), answers: answers(core) }).toEqual({
      saves: ["Clients/lister.yml"],
      answers: [true],
    });
  });

  it("refuse de quitter sur un secret en attente et dit pourquoi", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await click(app, ".editor__tab", "Auth");
    await app.get(".auth select").setValue("bearer");
    await app.get('.auth input[type="text"]').setValue("eyJhbGciOi.xyz");
    await wait(750);

    await core.emit(EVENTS.quit, null);
    await flushPromises();

    expect({
      saves: saves(core),
      answers: answers(core),
      toast: app.get(".toast").text(),
    }).toEqual({
      saves: [],
      answers: [false],
      toast: expect.stringContaining(
        "« Lister » n'est pas enregistrée : choisissez où va le secret saisi en clair.",
      ) as unknown,
    });
  });
});

describe("actions sur l'arborescence et brouillons en attente (R6)", () => {
  /** Requête « Lister » ouverte avec un jeton en clair en attente d'un choix. */
  async function heldSecret(): Promise<FakeCore> {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await click(app, ".editor__tab", "Auth");
    await app.get(".auth select").setValue("bearer");
    await app.get('.auth input[type="text"]').setValue("eyJhbGciOi.xyz");
    await wait(750);
    setActivePinia(app.vm.$.appContext.config.globalProperties.$pinia);
    return core;
  }

  it("un brouillon en attente ailleurs n'empêche pas de dupliquer", async () => {
    const core = await heldSecret();
    const selection = useSelectionStore();
    selection.toggle(BOUTIQUE_TESTS, "sante.yml");

    await selection.apply({ kind: "duplicate" });

    expect(core.commands()).toContain("tree_batch");
  });

  it("un brouillon en attente de la requête choisie arrête l'action", async () => {
    const core = await heldSecret();
    const selection = useSelectionStore();
    selection.toggle(BOUTIQUE_TESTS, "Clients/lister.yml");

    await expect(selection.apply({ kind: "duplicate" })).rejects.toThrow(
      "« Lister » n'est pas enregistrée : choisissez où va le secret saisi en clair.",
    );
    expect(core.commands()).not.toContain("tree_batch");
  });
});

describe("éditeurs d'une fenêtre et changements faits dans une autre", () => {
  it("une requête supprimée ailleurs puis recréée au même chemin est relue", async () => {
    setActivePinia(createPinia());
    const core = installFakeCore(boutiqueSnapshot());
    core.workbench.requests.set(
      "nouvelle.yml",
      newRequest("Ancienne", { url: "{{baseUrl}}/ancienne" }),
    );
    const workbench = useWorkbenchStore();
    await workbench.subscribe();
    await workbench.openEditor(BOUTIQUE_TESTS, "nouvelle.yml");

    // Supprimée depuis une autre fenêtre (barre latérale détachée) : l'onglet est fermé par le cœur.
    core.workbench.requests.delete("nouvelle.yml");
    await core.emit(EVENTS.project, fromOther({ removed: ["nouvelle.yml"] }));
    await flushPromises();
    // Recréée au même chemin, puis ouverte ici.
    core.workbench.requests.set("nouvelle.yml", newRequest("Nouvelle", { url: "" }));
    await core.emit(EVENTS.project, fromOther({}));
    await flushPromises();
    const editor = await workbench.openEditor(BOUTIQUE_TESTS, "nouvelle.yml");

    expect({ name: editor.draft?.name, url: editor.draft?.url }).toEqual({
      name: "Nouvelle",
      url: "",
    });
  });

  it("un brouillon ne réécrit pas l'ancien fichier d'une requête renommée ailleurs", async () => {
    setActivePinia(createPinia());
    const core = installFakeCore(boutiqueSnapshot());
    core.workbench.requests.set("ancien.yml", newRequest("Ancien", { url: "{{baseUrl}}/a" }));
    const workbench = useWorkbenchStore();
    await workbench.subscribe();
    const editor = await workbench.openEditor(BOUTIQUE_TESTS, "ancien.yml");
    if (editor.draft) editor.draft.url = "{{baseUrl}}/a?modifiee=1";
    await flushPromises();

    // Renommée depuis la barre latérale détachée : le cœur fait suivre l'onglet, pas l'éditeur.
    const moved = core.workbench.requests.get("ancien.yml");
    core.workbench.requests.delete("ancien.yml");
    if (moved) core.workbench.requests.set("nouveau.yml", JSON.parse(JSON.stringify(moved)));
    await core.emit(
      EVENTS.project,
      fromOther({ renamed: [{ from: "ancien.yml", to: "nouveau.yml" }] }),
    );
    await wait(700);

    expect({
      written: saves(core),
      resurrected: core.workbench.requests.has("ancien.yml"),
    }).toEqual({ written: ["nouveau.yml"], resurrected: false });
  });

  it("une frappe faite pendant la relecture d'une requête n'est pas écrasée", async () => {
    setActivePinia(createPinia());
    const core = installFakeCore(boutiqueSnapshot());
    core.workbench.requests.set("a.yml", newRequest("A", { url: "{{baseUrl}}/a" }));
    const workbench = useWorkbenchStore();
    const key = requestKey(BOUTIQUE_TESTS, "a.yml");
    await workbench.openEditor(BOUTIQUE_TESTS, "a.yml");

    // La relecture part (requête sans brouillon) ; l'utilisateur tape entre l'appel de
    // `request_read` et sa réponse.
    const reads = (): number => core.calls.filter((c) => c.cmd === "request_read").length;
    const before = reads();
    const reading = workbench.onProjectChange(
      fromOther({ scope: "request", path: "a.yml", origin: "" }),
    );
    for (let i = 0; i < 50 && reads() === before; i += 1) await Promise.resolve();
    const draft = workbench.editors[key]?.draft;
    if (draft) draft.url = "{{baseUrl}}/a?tape=1";
    await reading;
    await flushPromises();

    expect({ reads: reads() - before, url: workbench.editors[key]?.draft?.url }).toEqual({
      reads: 1,
      url: "{{baseUrl}}/a?tape=1",
    });
  });

  it("les requêtes vues en aperçu puis remplacées sont oubliées", async () => {
    ({ app } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, '[role="treeitem"]', "Lister");
    await click(app, '[role="treeitem"]', "Créer");
    await flushPromises();
    const tabs = app.findAll('[role="tab"].zone__tab-button').length;
    setActivePinia(app.vm.$.appContext.config.globalProperties.$pinia);
    const workbench = useWorkbenchStore();

    expect({ tabs, editors: Object.keys(workbench.editors).length }).toEqual({
      tabs: 1,
      editors: 1,
    });
  });
});
