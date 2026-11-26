/**
 * Alignement sur la maquette, lot E (docs/plan-alignement-maquette.md, T12 à T14) : dossiers.
 * Chaque test est la preuve d'un écart `aligner: code` de docs/maquette-etat.yml.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { FakeCore } from "./fakeCore";
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

/** « commandes » : un dossier qui range le test « v2 » (deux étapes) et une requête libre. */
async function openShop(): Promise<{ app: AppWrapper; core: FakeCore }> {
  return openProject((c) => {
    const wb = c.workbench;
    wb.folders.add("commandes");
    wb.folders.add("commandes/v2");
    wb.requests.set("commandes/v2/creer.yml", newRequest("Créer", { method: "POST", seq: 1 }));
    wb.requests.set("commandes/v2/relire.yml", newRequest("Relire", { seq: 2 }));
    wb.requests.set("commandes/annuler.yml", newRequest("Annuler", { method: "DELETE" }));
  });
}

/** Entrées d'un menu : libellé et raccourci, séparés d'une espace. */
function entries(menu: ReturnType<AppWrapper["get"]>): string[] {
  const top = menu
    .findAll('[role="menuitem"]')
    .filter((item) => item.element.closest(".menu__list--sub") === null);
  return top.map((item) => {
    const label = item.get(".menu__label").text();
    const shortcut = item.find(".menu__shortcut");
    return shortcut.exists() ? `${label} ${shortcut.text()}` : label;
  });
}

function folderRow(wrapper: AppWrapper, path: string) {
  const item = wrapper.get(`[role="treeitem"][data-path="${path}"]`);
  const row = item.element.closest(".row");
  if (!row) throw new Error(`ligne ${path} introuvable`);
  return wrapper.findAll(".row").find((r) => r.element === row) ?? wrapper.get(".row");
}

describe("menu d'un dossier et outils au survol (AM36, AM39 ; 4.1-a, 5.2-d)", () => {
  it("en-tête, entrées et raccourcis de la maquette", async () => {
    ({ app } = await openShop());
    await folderRow(app, "commandes").get(".row__more").trigger("click");
    await flushPromises();
    const menu = app.get('[role="menu"][aria-label="Actions pour commandes"]');
    expect(menu.element.parentElement?.querySelector(".menu__heading")?.textContent).toBe(
      "Dans le dossier « commandes »",
    );
    expect(entries(menu)).toEqual([
      "Nouveau test Ctrl+N",
      "Nouveau dossier Ctrl+Maj+N",
      "Dupliquer un test…",
      "Renommer F2",
      "Déplacer vers…",
      "Exécuter les tests du dossier",
      "Supprimer le dossier",
    ]);
  });

  it("« Dupliquer un test… » copie le test choisi ; « Déplacer vers… » ouvre le choix du dossier", async () => {
    let core;
    ({ app, core } = await openShop());
    await folderRow(app, "commandes").get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Dupliquer un test…");
    await click(app, '.menu__list--sub [role="menuitem"]', "v2");
    expect(core.calls.find((c) => c.cmd === "tree_batch")?.args).toMatchObject({
      paths: ["commandes/v2"],
      action: { kind: "duplicate" },
    });
    await folderRow(app, "commandes").get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Déplacer vers…");
    expect(app.find("dialog[open]").text()).toContain("Déplacer");
  });

  it("Ctrl+N et Ctrl+Maj+N sur un dossier créent dedans", async () => {
    let core;
    ({ app, core } = await openShop());
    const item = app.get('[role="treeitem"][data-path="commandes"]');
    (item.element as HTMLElement).focus();
    await item.trigger("keydown", { key: "n", code: "KeyN", ctrlKey: true });
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_test")?.args).toMatchObject({
      parent: "commandes",
    });
    await app
      .get('[role="treeitem"][data-path="commandes"]')
      .trigger("keydown", { key: "N", code: "KeyN", ctrlKey: true, shiftKey: true });
    await flushPromises();
    expect(app.get("dialog[open]").text()).toContain("Nouveau dossier");
  });

  it("▶ au survol lance le run du dossier ; « + » propose test, requête et dossier", async () => {
    let core;
    ({ app, core } = await openShop());
    const row = folderRow(app, "commandes");
    await row.get('[aria-label="Exécuter les tests de commandes"]').trigger("click");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "run_start")?.args).toMatchObject({
      folder: "commandes",
    });
    await folderRow(app, "commandes").get('[aria-label="Ajouter dans commandes"]').trigger("click");
    await flushPromises();
    expect(entries(app.get('[role="menu"][aria-label="Ajouter dans commandes"]'))).toEqual([
      "Nouveau test Ctrl+N",
      "Requête libre",
      "Nouveau dossier Ctrl+Maj+N",
    ]);
    // Le test lui-même garde son menu : pas de ▶ ni de « + » sur sa ligne.
    expect(folderRow(app, "commandes/v2").find(".row__tool").exists()).toBe(false);
  });
});

describe("vue d'un dossier (AM37, 4.1-a bis)", () => {
  async function openView(): Promise<FakeCore> {
    const opened = await openShop();
    app = opened.app;
    await click(app, '[role="treeitem"][data-path="commandes"]');
    return opened.core;
  }

  it("un clic sur un dossier ouvre sa vue en aperçu ; le chevron plie", async () => {
    const core = await openView();
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual({
      content: { kind: "folder", projectId: "p-boutique-tests", path: "commandes" },
      preview: true,
    });
    if (!app) throw new Error("application absente");
    expect(app.get(".folder__crumbs").text().replace(/\s+/g, " ")).toBe(
      "Référentiel de tests / commandes",
    );
    expect(app.get(".folder__summary").text()).toBe(
      "dossier · 1 test et 1 requête libre · 1 test au total",
    );
    const rows = app.findAll(".folder__row").map((row) => row.findAll("td").map((td) => td.text()));
    expect(rows).toEqual([
      ["1", "test", "v2", "2 étapes", "—"],
      ["2", "DELETE", "Annuler", "—", "—"],
    ]);
    await folderRow(app, "commandes").get(".row__chevron").trigger("click");
    await flushPromises();
    expect(core.workbench.collapsed).toContain("commandes");
  });

  it("« + Sous-dossier » et « + Test » nomment sur place, « ▶ Exécuter le dossier » lance le run", async () => {
    const core = await openView();
    if (!app) throw new Error("application absente");
    await click(app, ".folder button", "+ Sous-dossier");
    const field = app.get(".folder__create input");
    expect(document.activeElement).toBe(field.element);
    await field.setValue("erreurs");
    await app.get(".folder__create").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_folder")?.args).toMatchObject({
      parent: "commandes",
      name: "erreurs",
    });
    await click(app, ".folder button", "▶ Exécuter le dossier");
    expect(core.calls.find((c) => c.cmd === "run_start")?.args).toMatchObject({
      folder: "commandes",
    });
    await click(app, ".zone__tab-button", "commandes");
    await click(app, ".folder button", "+ Test");
    await app.get(".folder__create input").setValue("Relancer");
    await app.get(".folder__create").trigger("submit");
    await flushPromises();
    // Le test est créé avec son nom ; son étape 1 s'ouvre, comme depuis la barre latérale.
    expect(core.calls.find((c) => c.cmd === "tree_create_test")?.args).toMatchObject({
      parent: "commandes",
      name: "Relancer",
    });
    expect(core.workbench.folders.has("commandes/Relancer")).toBe(true);
  });
});
