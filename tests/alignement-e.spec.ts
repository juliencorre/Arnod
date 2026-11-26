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
