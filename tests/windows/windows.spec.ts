import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { EVENTS } from "@/lib/ipc";
import type { ProjectChange } from "@/types/ipc";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { newWindow, tab, zonesIn } from "../fakeWindows";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { click, openProject } from "../workbench/helpers";
import { switcherNames } from "../switcher";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const lister = { kind: "request", projectId: BOUTIQUE_TESTS, path: "Clients/lister.yml" } as const;

/** Faux cœur avec un petit projet ; l'interface sert la fenêtre `kind`. */
function setup(kind: "content" | "sidebar"): FakeCore {
  const core = installFakeCore(boutiqueSnapshot());
  core.workbench.folders.add("Clients");
  core.workbench.requests.set(
    "Clients/lister.yml",
    newRequest("Lister", { url: "{{baseUrl}}/clients" }),
  );
  core.workbench.requests.set("sante.yml", newRequest("Santé"));
  const window = kind === "content" ? newWindow("content", [tab(lister)]) : newWindow("sidebar");
  core.windows.windows.push(window);
  if (kind === "sidebar") {
    const main = core.windows.windows[0];
    if (main) main.sidebarDetached = true;
  }
  core.windows.label = window.label;
  return core;
}

/** Contenus ouverts dans la fenêtre principale du faux cœur. */
function mainContents(core: FakeCore): string[] {
  const root = core.windows.windows[0]?.layout?.root;
  if (!root) return [];
  return zonesIn(root)
    .flatMap((z) => z.tabs)
    .map((t) => (t.content.kind === "request" ? t.content.path : "run"));
}

function change(overrides: Partial<ProjectChange>): ProjectChange {
  return {
    projectId: BOUTIQUE_TESTS,
    scope: "tree",
    path: null,
    origin: "fenetre-autre",
    ...overrides,
  };
}

describe("fenêtre de contenu (F8, F10, F12)", () => {
  it("affiche l'emplacement de l'onglet actif et son éditeur", async () => {
    const core = setup("content");
    app = await mountApp();

    expect(app.get('nav[aria-label="Emplacement"]').text()).toBe("boutique-tests›Clients›Lister");
    expect((app.get(".editor__url").element as HTMLInputElement).value).toBe("{{baseUrl}}/clients");
    expect(app.find("aside").exists()).toBe(false);
    expect(core.calls.filter((c) => c.cmd === "window_set_title").at(-1)?.args).toEqual({
      subject: "Lister",
    });
  });

  it("épingle la fenêtre, puis suit de nouveau la sélection", async () => {
    const core = setup("content");
    app = await mountApp();
    const toggle = app.get(".content-window__follow");
    expect([toggle.text(), toggle.attributes("aria-pressed")]).toEqual([
      "⇆ Suit la sélection",
      "true",
    ]);

    await toggle.trigger("click");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "layout_set_follow")?.args).toEqual({ follow: false });
    expect([toggle.text(), toggle.attributes("aria-pressed")]).toEqual(["Épinglée", "false"]);

    await toggle.trigger("click");
    await flushPromises();
    expect(toggle.text()).toBe("⇆ Suit la sélection");
  });

  it("« Rattacher » rend ses onglets à la fenêtre principale", async () => {
    const core = setup("content");
    app = await mountApp();

    await click(app, ".content-window__bar button", "Rattacher");

    expect(core.commands()).toContain("window_reattach");
    expect(core.windows.windows.map((w) => w.kind)).toEqual(["main"]);
    expect(mainContents(core)).toEqual(["Clients/lister.yml"]);
  });

  it("à la fermeture, enregistre la requête modifiée puis se rattache (Z6)", async () => {
    const core = setup("content");
    app = await mountApp();
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?actifs=1");

    await core.emit(EVENTS.close, null);
    await flushPromises();

    const commands = core.commands();
    expect(commands.indexOf("request_save")).toBeGreaterThan(-1);
    expect(commands.indexOf("request_save")).toBeLessThan(commands.indexOf("window_reattach"));
    expect(core.workbench.requests.get("Clients/lister.yml")?.url).toBe(
      "{{baseUrl}}/clients?actifs=1",
    );
    expect(mainContents(core)).toEqual(["Clients/lister.yml"]);
  });
});

describe("barre latérale détachée (F9, Z5)", () => {
  it("pilote les fenêtres qui suivent la sélection", async () => {
    const core = setup("sidebar");
    app = await mountApp();

    expect(await switcherNames(app)).toContain("Tous les projets");
    expect(app.get(".sidebar-window__pilot").text()).toBe("Pilote 1 fenêtre de contenu");

    core.windows.windows.push(newWindow("content", [tab(lister)]));
    await core.emitWindows();
    await flushPromises();
    expect(app.get(".sidebar-window__pilot").text()).toBe("Pilote 2 fenêtres de contenu");
  });

  it("une requête choisie s'ouvre dans la fenêtre qui suit la sélection", async () => {
    const core = setup("sidebar");
    app = await mountApp();

    // Sans vue projet ici, choisir le projet le déplie.
    await click(app, "aside .project__main", "boutique-tests");
    expect(app.find('[aria-label="Replier boutique-tests"]').exists()).toBe(true);
    await click(app, '[role="treeitem"]', "Santé");

    expect(core.calls.find((c) => c.cmd === "layout_open_selection")?.args).toEqual({
      content: { kind: "request", projectId: BOUTIQUE_TESTS, path: "sante.yml" },
    });
    expect(mainContents(core)).toEqual(["sante.yml"]);
  });

  it("« Rattacher » remet la barre dans la fenêtre principale", async () => {
    const core = setup("sidebar");
    app = await mountApp();

    await click(app, ".sidebar-window__footer button", "Rattacher");

    expect(core.commands()).toContain("window_reattach");
    expect(core.windows.windows[0]?.sidebarDetached).toBe(false);
  });
});

describe("redémarrage (Z8)", () => {
  it("rouvre la zone de travail sur l'onglet actif de la fenêtre principale", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.workbench.folders.add("Clients");
    core.workbench.requests.set(
      "Clients/lister.yml",
      newRequest("Lister", { url: "{{baseUrl}}/clients" }),
    );
    const main = core.windows.windows[0]?.layout;
    const zone = main ? zonesIn(main.root)[0] : undefined;
    const restored = tab(lister);
    zone?.tabs.push(restored);
    if (zone) zone.activeTab = restored.id;

    app = await mountApp();

    expect(app.get("main h1").text()).toBe("boutique-tests");
    expect(app.get("main h1").classes()).toContain("visually-hidden");
    expect((app.get(".editor__url").element as HTMLInputElement).value).toBe("{{baseUrl}}/clients");
    expect(app.get('[aria-label="Replier boutique-tests"]').attributes("aria-expanded")).toBe(
      "true",
    );
  });
});

describe("fenêtres interconnectées (F11)", () => {
  it("suit l'état des fenêtres diffusé par le cœur", async () => {
    let core;
    ({ app, core } = await openProject());
    const main = core.windows.windows[0]?.layout;
    const zone = main ? zonesIn(main.root)[0] : undefined;
    zone?.tabs.push(tab({ kind: "request", projectId: BOUTIQUE_TESTS, path: "sante.yml" }));

    await core.emitWindows();
    await flushPromises();

    expect(app.findAll(".zone__tab-button .zone__name").map((n) => n.text())).toEqual(["Santé"]);
  });

  it("recharge l'arborescence modifiée dans une autre fenêtre", async () => {
    let core;
    ({ app, core } = await openProject());
    core.workbench.requests.set("ping.yml", newRequest("Ping"));

    await core.emit(EVENTS.project, change({ origin: "main" }));
    await flushPromises();
    expect(app.findAll('[role="treeitem"]').map((i) => i.text())).not.toContain("GETPing");

    await core.emit(EVENTS.project, change({}));
    await flushPromises();
    expect(app.findAll('[role="treeitem"]').map((i) => i.text())).toContain("GETPing");
  });

  it("recharge une requête enregistrée ailleurs, ou le signale si elle est modifiée ici", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    const stored = core.workbench.requests.get("Clients/lister.yml");
    if (stored) stored.url = "{{baseUrl}}/v2/clients";

    await core.emit(EVENTS.project, change({ scope: "request", path: "Clients/lister.yml" }));
    await flushPromises();
    const url = app.get<HTMLInputElement>(".editor__url");
    expect(url.element.value).toBe("{{baseUrl}}/v2/clients");

    await url.setValue("{{baseUrl}}/ici");
    if (stored) stored.url = "{{baseUrl}}/v3/clients";
    await core.emit(EVENTS.project, change({ scope: "request", path: "Clients/lister.yml" }));
    await flushPromises();
    expect(url.element.value).toBe("{{baseUrl}}/ici");
    expect(app.get(".editor__elsewhere").text()).toContain("Modifiée dans une autre fenêtre.");

    await click(app, ".editor__elsewhere button", "Recharger");
    expect(url.element.value).toBe("{{baseUrl}}/v3/clients");
    expect(app.find(".editor__elsewhere").exists()).toBe(false);
  });

  it("suit le catalogue modifié dans une autre fenêtre", async () => {
    let core;
    ({ app, core } = await openProject());
    const snapshot = core.snapshot();
    const renamed = snapshot.projects.find((p) => p.id === BOUTIQUE_TESTS);
    if (renamed) renamed.name = "boutique-recette";

    await core.emit(EVENTS.catalog, snapshot);
    await flushPromises();

    expect(app.findAll("aside .project__main").map((b) => b.text())).toContain("Bboutique-recette");
  });
});
