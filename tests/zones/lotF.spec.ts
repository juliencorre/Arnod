/**
 * Lot F - Onglets et clavier (docs/plan-mvp-f-onglets-clavier.md) : aperçu et ◐ (1.3-a), onglets
 * en trop (1.3-b), Ctrl+Tab (1.3-c), Ctrl+J et Ctrl+\ (1.4-j). Le comportement du cœur est
 * testé dans `layout/tests.rs` ; ici, l'interface et le clavier sur le faux cœur.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { byUse, fitTabs, isRunning } from "@/lib/tabs";
import { nextPlacement } from "@/stores/windows";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { RunView } from "@/types/ipc";

import type { AppWrapper } from "../mountApp";
import { openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  vi.restoreAllMocks();
  clearMocks();
  document.body.innerHTML = "";
});

function row(wrapper: AppWrapper, name: string) {
  const found = wrapper.findAll('[role="treeitem"]').find((e) => e.text().includes(name));
  if (!found) throw new Error(`ligne introuvable : ${name}`);
  return found;
}

async function clickRow(wrapper: AppWrapper, name: string, dbl = false): Promise<void> {
  await row(wrapper, name).trigger(dbl ? "dblclick" : "click");
  await flushPromises();
}

function tabs(wrapper: AppWrapper): { name: string; preview: boolean }[] {
  return wrapper.findAll(".zone__tab:not(.zone__tab--hidden)").map((t) => ({
    name: t.get(".zone__name").text(),
    preview: t.classes().includes("zone__tab--preview"),
  }));
}

function key(code: string, options: KeyboardEventInit = {}, type = "keydown"): void {
  const keyName = code === "Tab" ? "Tab" : code.replace(/^Key/, "").toLowerCase();
  window.dispatchEvent(new KeyboardEvent(type, { code, key: keyName, ...options }));
}

describe("fonctions d'affichage des onglets (lib/tabs)", () => {
  it("garde les onglets qui tiennent, la place de « +N » et l'onglet actif", () => {
    expect(fitTabs([100, 100, 100], 400, 50, 0)).toEqual([0, 1, 2]);
    expect(fitTabs([100, 100, 100, 100], 300, 50, 0)).toEqual([0, 1]);
    // L'onglet actif caché prend la place du dernier visible.
    expect(fitTabs([100, 100, 100, 100], 300, 50, 3)).toEqual([0, 3]);
  });

  it("range les onglets dans l'ordre d'utilisation", () => {
    const list = [{ id: "a" }, { id: "b" }, { id: "c" }];
    expect(byUse(list, ["c", "a"]).map((t) => t.id)).toEqual(["c", "a", "b"]);
  });

  it("◐ pour un envoi, une étape ou un run en cours", () => {
    const run = {
      finished: false,
      items: [{ path: "t/1.yml", test: "t", status: "running" }],
    } as unknown as RunView;
    const activity = { sending: (_: string, path: string) => path === "a.yml", run: () => run };
    expect(isRunning({ kind: "request", projectId: "p", path: "a.yml" }, activity)).toBe(true);
    expect(isRunning({ kind: "request", projectId: "p", path: "t/1.yml" }, activity)).toBe(true);
    expect(isRunning({ kind: "test", projectId: "p", path: "t" }, activity)).toBe(true);
    expect(isRunning({ kind: "run", projectId: "p" }, activity)).toBe(true);
    expect(isRunning({ kind: "result" }, activity)).toBe(false);
    const done = { ...activity, run: () => ({ ...run, finished: true }) };
    expect(isRunning({ kind: "run", projectId: "p" }, done)).toBe(false);
  });

  it("Ctrl+J passe de l'autre côté, ou revient à l'automatique", () => {
    expect(nextPlacement("auto", { shown: "right", auto: "right" })).toBe("below");
    expect(nextPlacement("below", { shown: "below", auto: "right" })).toBe("auto");
    expect(nextPlacement("right", { shown: "right", auto: "below" })).toBe("auto");
    expect(nextPlacement("right", undefined)).toBe("below");
  });
});

describe("onglet d'aperçu (1.3-a)", () => {
  it("un simple clic ouvre en aperçu, en italique, remplacé par le suivant", async () => {
    ({ app } = await openProject());
    await clickRow(app, "Lister");
    expect(tabs(app)).toEqual([{ name: "Lister", preview: true }]);
    expect(app.get(".zone__tab-button").text()).toContain("(aperçu)");

    await clickRow(app, "Santé");
    expect(tabs(app)).toEqual([{ name: "Santé", preview: true }]);
  });

  it("double-clic dans la barre latérale ou sur l'onglet : il reste", async () => {
    let core;
    ({ app, core } = await openProject());
    await clickRow(app, "Lister");
    await app.get(".zone__tab-button").trigger("dblclick");
    await flushPromises();
    expect(core.commands()).toContain("layout_pin");

    await clickRow(app, "Santé", true);
    await clickRow(app, "Créer");
    expect(tabs(app)).toEqual([
      { name: "Lister", preview: false },
      { name: "Santé", preview: false },
      { name: "Créer", preview: true },
    ]);
  });

  it("une modification rend l'aperçu permanent", async () => {
    let core;
    ({ app, core } = await openProject());
    await clickRow(app, "Lister");
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?page=2");
    await flushPromises();
    expect(core.commands()).toContain("layout_pin");
    expect(tabs(app)).toEqual([{ name: "Lister", preview: false }]);
  });

  it("◐ dans l'onglet pendant un envoi", async () => {
    ({ app } = await openProject());
    await clickRow(app, "Lister", true);
    expect(app.find(".zone__running").exists()).toBe(false);
    const workbench = useWorkbenchStore();
    const editor = workbench.editors[requestKey("p-boutique-tests", "Clients/lister.yml")];
    expect(editor).toBeDefined();
    if (editor) editor.sending = true;
    await flushPromises();
    expect(app.get(".zone__running").attributes("aria-label")).toBe("en cours");
  });
});

describe("trop d'onglets (1.3-b)", () => {
  it("« +N » liste les onglets cachés, au clavier, et ferme les autres", async () => {
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(150);
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(400);
    let core;
    ({ app, core } = await openProject());
    for (const name of ["Lister", "Créer", "Santé"]) await clickRow(app, name, true);
    await flushPromises();

    const more = app.get(".zone__more");
    expect(more.text()).toContain("+2");
    expect(tabs(app).map((t) => t.name)).toEqual(["Santé"]);

    await more.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const items = app.findAll('[role="menuitem"]').map((i) => i.text());
    expect(items[0]).toContain("Lister");
    expect(items).toContain("Fermer les autres onglets");
    expect(items).toContain("Fermer tous les onglets");

    await app.get('[role="menu"]').trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "layout_activate").length).toBeGreaterThan(0);

    await app.get(".zone__more").trigger("click");
    await flushPromises();
    const closeOthers = app
      .findAll('[role="menuitem"]')
      .find((i) => i.text() === "Fermer les autres onglets");
    await closeOthers?.trigger("click");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "layout_close_tabs")?.args.tabs).toHaveLength(2);
    expect(app.findAll(".zone__tab")).toHaveLength(1);

    // Tout fermé puis rouvert : le nouvel onglet est visible, pas rangé sous « +N ».
    await app.get(".zone__tab-button").trigger("keydown", { key: "F10", shiftKey: true });
    await flushPromises();
    await app
      .findAll('[role="menuitem"]')
      .find((i) => i.text() === "Fermer tout")
      ?.trigger("click");
    await flushPromises();
    await clickRow(app, "Lister");
    expect(tabs(app).map((t) => t.name)).toEqual(["Lister"]);
  });

  it("« Fermer tout » depuis le menu de l'onglet", async () => {
    ({ app } = await openProject());
    for (const name of ["Lister", "Santé"]) await clickRow(app, name, true);
    await app.get('.zone__tab-button[aria-selected="true"]').trigger("keydown", {
      key: "F10",
      shiftKey: true,
    });
    await flushPromises();
    const all = app.findAll('[role="menuitem"]').find((i) => i.text() === "Fermer tout");
    await all?.trigger("click");
    await flushPromises();
    expect(app.findAll(".zone__tab")).toHaveLength(0);
  });
});

describe("Ctrl+Tab (1.3-c)", () => {
  it("liste les onglets dans l'ordre d'utilisation ; relâcher Ctrl ouvre le choix", async () => {
    let core;
    ({ app, core } = await openProject());
    for (const name of ["Lister", "Créer", "Santé"]) await clickRow(app, name, true);

    key("Tab", { ctrlKey: true });
    await flushPromises();
    const list = () => app?.findAll(".zone__switcher-name").map((n) => n.text()) ?? [];
    expect(list()).toEqual(["Santé", "Créer", "Lister"]);
    expect(app.get('[role="option"][aria-selected="true"]').text()).toContain("Créer");

    key("Tab", { ctrlKey: true });
    await flushPromises();
    expect(app.get('[role="option"][aria-selected="true"]').text()).toContain("Lister");
    key("Tab", { ctrlKey: true, shiftKey: true });
    await flushPromises();
    expect(app.get('[role="option"][aria-selected="true"]').text()).toContain("Créer");

    window.dispatchEvent(new KeyboardEvent("keyup", { key: "Control", code: "ControlLeft" }));
    await flushPromises();
    expect(app.find(".zone__switcher").exists()).toBe(false);
    expect(core.calls.at(-1)?.cmd).toBe("layout_activate");
    expect(app.get('.zone__tab-button[aria-selected="true"]').text()).toContain("Créer");

    // Créer est maintenant le plus récent : Ctrl+Tab revient à Santé.
    key("Tab", { ctrlKey: true });
    await flushPromises();
    expect(list()[0]).toBe("Créer");
    key("Escape", { key: "Escape" });
    await flushPromises();
    expect(app.find(".zone__switcher").exists()).toBe(false);
  });
});

describe("au clavier (1.4-j)", () => {
  it("Ctrl+J met le résultat dessous puis à côté, par le code de la touche", async () => {
    let core;
    ({ app, core } = await openProject());
    await clickRow(app, "Lister", true);
    // AZERTY : la touche J donne toujours le code KeyJ.
    key("KeyJ", { ctrlKey: true });
    await flushPromises();
    expect(app.find(".request-tab--below").exists()).toBe(true);
    key("KeyJ", { ctrlKey: true });
    await flushPromises();
    expect(app.find(".request-tab--below").exists()).toBe(false);
    expect(
      core.calls.filter((c) => c.cmd === "layout_set_placement").map((c) => c.args.placement),
    ).toEqual(["below", "auto"]);
  });

  it("Ctrl+\\ masque la barre latérale, gardé par la fenêtre ; le bouton la rouvre", async () => {
    let core;
    ({ app, core } = await openProject());
    expect(app.find(".shell__sidebar").exists()).toBe(true);
    key("Backslash", { ctrlKey: true, key: "\\" });
    await flushPromises();
    expect(app.find(".shell__sidebar").exists()).toBe(false);
    expect(core.windows.windows[0]?.sidebarHidden).toBe(true);

    await app.get('[aria-label="Afficher la barre latérale"]').trigger("click");
    await flushPromises();
    expect(app.find(".shell__sidebar").exists()).toBe(true);
    expect(core.windows.windows[0]?.sidebarHidden).toBe(false);
  });

  it("les entrées du menu Fenêtre agissent comme le clavier", async () => {
    let core;
    ({ app, core } = await openProject());
    await clickRow(app, "Lister", true);
    await core.emit("arnod://raccourci", "placement");
    await flushPromises();
    expect(app.find(".request-tab--below").exists()).toBe(true);
    await core.emit("arnod://raccourci", "sidebar");
    await flushPromises();
    expect(app.find(".shell__sidebar").exists()).toBe(false);
  });
});
