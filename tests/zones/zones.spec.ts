import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TAB_DRAG_TYPE } from "@/lib/dnd";

import { newRequest } from "../fakeWorkbench";
import { zonesIn } from "../fakeWindows";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  vi.restoreAllMocks();
  clearMocks();
  document.body.innerHTML = "";
});

/** Noms des onglets de chaque zone, dans l'ordre de lecture. */
function zoneTabs(wrapper: AppWrapper): string[][] {
  return wrapper
    .findAll("section.zone")
    .map((zone) => zone.findAll(".zone__tab-button .zone__name").map((name) => name.text()));
}

async function openRequests(wrapper: AppWrapper, ...names: string[]): Promise<void> {
  for (const name of names) await click(wrapper, '[role="treeitem"]', name);
}

/** Fausse géométrie : happy-dom ne calcule aucune mise en page. */
function fakeGeometry(width = 1200, height = 800): void {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    width,
    height,
    right: width,
    bottom: height,
    toJSON: () => ({}),
  });
}

/** Transfert de glisser-déposer minimal, comme celui du navigateur. */
function transfer(): DataTransfer {
  const data = new Map<string, string>();
  return {
    get types() {
      return [...data.keys()];
    },
    setData: (type: string, value: string) => data.set(type, value),
    getData: (type: string) => data.get(type) ?? "",
    dropEffect: "none",
    effectAllowed: "all",
  } as unknown as DataTransfer;
}

describe("onglets de contenu (F1, Z3)", () => {
  it("ouvre chaque requête en onglet, une seule fois par fenêtre", async () => {
    let core;
    ({ app, core } = await openProject());
    expect(app.get("main h1").text()).toBe("boutique-tests");

    await openRequests(app, "Lister", "Santé");
    expect(zoneTabs(app)).toEqual([["Lister", "Santé"]]);
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection")).toHaveLength(2);

    await openRequests(app, "Lister");
    expect(zoneTabs(app)).toEqual([["Lister", "Santé"]]);
    expect(app.get('.zone__tab-button[aria-selected="true"]').text()).toContain("Lister");
    expect((app.get(".editor__url").element as HTMLInputElement).value).toBe("{{baseUrl}}/clients");
  });

  it("ferme un onglet par ×, par Suppr et par Ctrl+W, en enregistrant", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Créer", "Santé");

    await app.get('[aria-label="Fermer Créer"]').trigger("click");
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Lister", "Santé"]]);

    await app.get('.zone__tab-button[aria-selected="true"]').trigger("keydown", { key: "Delete" });
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Lister"]]);

    await app.get(".editor__url").setValue("{{baseUrl}}/clients?page=2");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "w", ctrlKey: true }));
    await flushPromises();
    expect(zoneTabs(app)).toEqual([[]]);
    expect(core.commands()).toContain("request_save");
    expect(core.workbench.requests.get("Clients/lister.yml")?.url).toBe(
      "{{baseUrl}}/clients?page=2",
    );
    expect(app.get("main h1").text()).toBe("boutique-tests");
  });

  it("parcourt les onglets au clavier", async () => {
    ({ app } = await openProject());
    await openRequests(app, "Lister", "Santé");
    const tabs = app.findAll(".zone__tab-button");
    expect(tabs.map((t) => t.attributes("tabindex"))).toEqual(["-1", "0"]);

    await tabs[1]?.trigger("keydown", { key: "ArrowRight" });
    await flushPromises();
    expect(document.activeElement).toBe(tabs[0]?.element);
  });
});

describe("zones (F2 à F5, F14, Z1)", () => {
  it("divise à droite, puis redimensionne au clavier et au double-clic", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");

    await app.get('[aria-label="Diviser à droite"]').trigger("click");
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Lister"], ["Santé"]]);
    expect(core.calls.find((c) => c.cmd === "layout_split")?.args).toMatchObject({
      side: "right",
    });

    const handle = app.get('[role="separator"][aria-label="Largeur des zones"]');
    expect(handle.attributes("aria-valuenow")).toBe("50");
    await handle.trigger("keydown", { key: "ArrowRight" });
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "layout_resize").at(-1)?.args.ratio).toBeCloseTo(
      0.55,
    );
    expect(handle.attributes("aria-valuenow")).toBe("55");

    await handle.trigger("dblclick");
    await flushPromises();
    expect(handle.attributes("aria-valuenow")).toBe("50");
  });

  it("Alt+Maj+flèches règle la séparation de la zone active (F4)", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");
    await app.get('[aria-label="Diviser à droite"]').trigger("click");
    await flushPromises();
    const handle = app.get('[role="separator"][aria-label="Largeur des zones"]');

    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowLeft", altKey: true, shiftKey: true }),
    );
    await flushPromises();
    expect(handle.attributes("aria-valuenow")).toBe("45");

    // Pas de séparation horizontale : rien ne change.
    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowUp", altKey: true, shiftKey: true }),
    );
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "layout_resize")).toHaveLength(1);

    // Sur la séparation focalisée, la touche n'est traitée qu'une fois.
    await handle.trigger("keydown", { key: "ArrowRight", altKey: true, shiftKey: true });
    await flushPromises();
    expect(handle.attributes("aria-valuenow")).toBe("50");
    expect(core.calls.filter((c) => c.cmd === "layout_resize")).toHaveLength(2);
  });

  it("divise en dessous par le menu de l'onglet, ouvert au clavier", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");

    await app.get('.zone__tab-button[aria-selected="true"]').trigger("keydown", {
      key: "ContextMenu",
    });
    await flushPromises();
    await click(app, '[role="menuitem"]', "Diviser en dessous");

    expect(zoneTabs(app)).toEqual([["Lister"], ["Santé"]]);
    expect(core.calls.find((c) => c.cmd === "layout_split")?.args).toMatchObject({
      side: "bottom",
    });
    expect(app.find('[role="separator"][aria-label="Hauteur des zones"]').exists()).toBe(true);
  });

  it("déplace un onglet vers une autre zone par le menu (F3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Créer", "Santé");
    await app.get('[aria-label="Diviser à droite"]').trigger("click");
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Lister", "Créer"], ["Santé"]]);

    const first = app.findAll("section.zone")[0];
    await first?.get('.zone__tab-button[aria-selected="true"]').trigger("keydown", {
      key: "ContextMenu",
    });
    await flushPromises();
    await click(app, '[role="menuitem"]', "Déplacer vers");
    await click(app, '[role="menuitem"][data-level="1"]', "Zone 2 · Santé");

    expect(zoneTabs(app)).toEqual([["Lister"], ["Santé", "Créer"]]);
    expect(core.calls.find((c) => c.cmd === "layout_move")?.args).toMatchObject({
      toWindow: "main",
      index: null,
    });
  });

  it("glisser un onglet sur le bord droit d'une zone l'ouvre à droite (F2)", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");
    fakeGeometry();

    const dataTransfer = transfer();
    const tab = app.findAll(".zone__tab-button")[0];
    await tab?.trigger("dragstart", { dataTransfer });
    expect(dataTransfer.types).toContain(TAB_DRAG_TYPE);

    const body = app.get(".zone__body");
    await body.trigger("dragover", { dataTransfer, clientX: 1150, clientY: 300 });
    expect(app.get(".zone__drop-label").text()).toBe("Déposer ici : ouvrir à droite");
    await body.trigger("drop", { dataTransfer, clientX: 1150, clientY: 300 });
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "layout_split")?.args).toMatchObject({
      side: "right",
    });
    expect(zoneTabs(app)).toEqual([["Santé"], ["Lister"]]);
    expect(app.find(".zone__drop").exists()).toBe(false);
  });

  it("au plus quatre zones : la division n'est plus proposée (Z1)", async () => {
    ({ app } = await openProject((core) => {
      core.workbench.requests.set("ping.yml", newRequest("Ping"));
    }));
    await openRequests(app, "Lister", "Créer", "Santé", "Ping");
    for (let i = 0; i < 3; i++) {
      await app.findAll('[aria-label="Diviser à droite"]')[0]?.trigger("click");
      await flushPromises();
    }
    expect(app.findAll("section.zone")).toHaveLength(4);
    for (const tool of app.findAll('[aria-label="Diviser à droite"]')) {
      expect(tool.attributes("disabled")).toBeDefined();
    }
  });

  it("agrandit une zone puis la rétablit (F14)", async () => {
    ({ app } = await openProject());
    await openRequests(app, "Lister", "Santé");
    expect(app.find('[aria-label="Agrandir la zone"]').exists()).toBe(false);
    await app.get('[aria-label="Diviser à droite"]').trigger("click");
    await flushPromises();

    await app.findAll('[aria-label="Agrandir la zone"]')[1]?.trigger("click");
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Santé"]]);

    await app.get('[aria-label="Rétablir la zone"]').trigger("click");
    await flushPromises();
    expect(zoneTabs(app)).toEqual([["Lister"], ["Santé"]]);
  });

  it("place le résultat dessous ou à droite (F6)", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister");
    const options = app.findAll(".placement__option");
    expect(options.map((o) => [o.text(), o.attributes("aria-pressed")])).toEqual([
      ["À droite · auto", "true"],
      ["Dessous", "false"],
    ]);

    await options[1]?.trigger("click");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "layout_set_placement")?.args).toMatchObject({
      placement: "below",
    });
    expect(app.findAll(".placement__option").map((o) => o.text())).toEqual(["À droite", "Dessous"]);
    expect(app.find(".request-tab--below").exists()).toBe(true);
  });
});

describe("fenêtres détachées (F8, F9)", () => {
  it("ouvre la zone dans une fenêtre de contenu", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");

    await app.get('[aria-label="Ouvrir la zone dans une fenêtre"]').trigger("click");
    await flushPromises();

    expect(core.commands()).toContain("window_detach_zone");
    const detached = core.windows.windows.find((w) => w.kind === "content");
    const tabs = zonesIn(
      detached?.layout?.root ?? {
        kind: "zone",
        id: "",
        tabs: [],
        activeTab: null,
        placement: "auto",
      },
    )
      .flatMap((z) => z.tabs)
      .map((t) => (t.content.kind === "request" ? t.content.path : "run"));
    expect(tabs).toEqual(["Clients/lister.yml", "sante.yml"]);
    expect(zoneTabs(app)).toEqual([[]]);
    expect(app.get("main h1").text()).toBe("boutique-tests");
  });

  it("un onglet glissé hors de la fenêtre devient une fenêtre à cet endroit", async () => {
    let core;
    ({ app, core } = await openProject());
    await openRequests(app, "Lister", "Santé");

    const dataTransfer = transfer();
    const tab = app.findAll(".zone__tab-button")[1];
    await tab?.trigger("dragstart", { dataTransfer });
    await tab?.trigger("dragend", {
      dataTransfer,
      clientX: -20,
      clientY: 40,
      screenX: 300,
      screenY: 200,
    });
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "window_detach_tab")?.args).toMatchObject({
      bounds: { x: 220, y: 180, width: 980, height: 720 },
    });
    expect(zoneTabs(app)).toEqual([["Lister"]]);
  });

  it("détache la barre latérale, qui quitte la fenêtre principale", async () => {
    let core;
    ({ app, core } = await openProject());
    expect(app.find("aside").exists()).toBe(true);

    await app.get('[aria-label="Détacher la barre latérale dans une fenêtre"]').trigger("click");
    await flushPromises();

    expect(core.commands()).toContain("window_detach_sidebar");
    expect(core.windows.windows.map((w) => w.kind)).toEqual(["main", "sidebar"]);
    expect(app.find("aside").exists()).toBe(false);
  });

  it("règle la largeur de la barre latérale au clavier (F5)", async () => {
    fakeGeometry(1000);
    let core;
    ({ app, core } = await openProject());
    const handle = app.get('[role="separator"][aria-label="Largeur de la barre latérale"]');
    expect(handle.attributes("aria-valuenow")).toBe("26");

    await handle.trigger("keydown", { key: "ArrowRight" });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "layout_set_sidebar_width")?.args).toEqual({
      width: 310,
    });
    expect(app.get(".shell__sidebar").attributes("style")).toContain("width: 310px");
  });
});
