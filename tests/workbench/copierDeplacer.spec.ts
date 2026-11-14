import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useWorkbenchStore } from "@/stores/workbench";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Copier ou déplacer une requête (lot J, écrans 4.1-f, 4.1-g et 3.2-c). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** Le projet de démonstration, un test « v2 » de deux étapes et « Sans coupon » à la racine. */
async function openShop(): Promise<{ app: AppWrapper; core: FakeCore }> {
  return openProject((core) => {
    const wb = core.workbench;
    wb.folders.add("commandes");
    wb.folders.add("commandes/v2");
    wb.requests.set(
      "commandes/v2/creer.yml",
      newRequest("Créer la commande", { method: "POST", seq: 1 }),
    );
    wb.requests.set("commandes/v2/relire.yml", newRequest("Relire la commande", { seq: 2 }));
    wb.requests.set("sans-coupon.yml", newRequest("Sans coupon", { method: "POST" }));
  });
}

function row(wrapper: AppWrapper, name: string) {
  const found = wrapper
    .findAll('li[role="presentation"]')
    .find((li) => li.get('[role="treeitem"]').text().includes(name));
  if (!found) throw new Error(`ligne introuvable : ${name}`);
  return found;
}

async function openDialog(wrapper: AppWrapper, name: string): Promise<void> {
  await row(wrapper, name).get(".row__more").trigger("click");
  await flushPromises();
  await click(wrapper, '[role="menuitem"]', "Copier ou déplacer…");
}

function changes(wrapper: AppWrapper): string[] {
  return wrapper.findAll(".copy-move__item dt").map((d) => d.text());
}

describe("dialogue « Copier ou déplacer » (4.1-g)", () => {
  it("cherche une destination ; un test propose sa place et dit ce qui change", async () => {
    ({ app } = await openShop());
    await openDialog(app, "Sans coupon");
    const groups = app.findAll(".copy-move__group").map((g) => g.text());
    await app.get(".copy-move__search").setValue("v2");
    await flushPromises();
    const options = app.findAll('[role="option"]').map((o) => o.text());
    await app.get(".copy-move__place select").setValue("1");
    await flushPromises();
    expect({
      title: app.get("dialog[open] h2").text(),
      groups,
      options,
      changes: changes(app),
      detail: app.get(".copy-move__item dd").text(),
    }).toEqual({
      title: "Copier ou déplacer « Sans coupon »",
      groups: ["Requêtes libres", "Tests · étape", "Autres projets"],
      options: ["commandes / v2après l'étape 2"],
      changes: ["Devient l'étape 2 de « v2 »", "Déplacement"],
      detail: "L'actuelle étape 2 devient la 3.",
    });
  });

  it("« Déplacer » range l'étape à sa place, renumérote, et « Annuler » remet tout", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await openDialog(app, "Sans coupon");
    await app.get(".copy-move__search").setValue("v2");
    await flushPromises();
    await app.get(".copy-move__place select").setValue("1");
    await flushPromises();
    await click(app, "dialog[open] button", "Déplacer");
    const wb = core.workbench;
    const after = {
      moved: wb.requests.get("commandes/v2/sans-coupon.yml")?.seq,
      next: wb.requests.get("commandes/v2/relire.yml")?.seq,
      left: wb.requests.has("sans-coupon.yml"),
      toast: app.get(".toast").text(),
      open: app.find("dialog[open] .copy-move").exists(),
    };
    await click(app, ".toast button", "Annuler");
    expect({
      after,
      undone: {
        back: wb.requests.has("sans-coupon.yml"),
        gone: wb.requests.has("commandes/v2/sans-coupon.yml"),
        next: wb.requests.get("commandes/v2/relire.yml")?.seq,
      },
    }).toEqual({
      after: {
        moved: 2,
        next: 3,
        left: false,
        toast: expect.stringContaining("« Sans coupon » déplacée.") as unknown,
        open: false,
      },
      undone: { back: true, gone: false, next: 2 },
    });
  });

  it("au clavier : la place actuelle est sautée, ↓ choisit la suivante, « Copier » garde l'original", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await openDialog(app, "Sans coupon");
    const search = app.get(".copy-move__search");
    const first = app.get('[role="option"][aria-selected="true"]').text();
    await search.trigger("keydown", { key: "ArrowDown" });
    const second = app.get('[role="option"][aria-selected="true"]').text();
    await click(app, "dialog[open] button", "Copier");
    expect({
      first,
      second,
      copied: core.workbench.requests.get("Clients/sans-coupon.yml")?.seq,
      kept: core.workbench.requests.has("sans-coupon.yml"),
      activedescendant: search.attributes("aria-activedescendant") !== undefined,
    }).toEqual({
      first: "commandes",
      second: "Clientsaprès l'étape 2",
      copied: 3,
      kept: true,
      activedescendant: true,
    });
  });
});

describe("copier, couper, coller et glisser dans la barre latérale (4.1-f)", () => {
  it("Ctrl+C puis Ctrl+V sur un dossier copie ; Ctrl+Z annule", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await row(app, "Sans coupon").get('[role="treeitem"]').trigger("keydown", {
      code: "KeyC",
      ctrlKey: true,
    });
    await row(app, "commandes").get('[role="treeitem"]').trigger("keydown", {
      code: "KeyV",
      ctrlKey: true,
    });
    await flushPromises();
    const pasted = core.workbench.requests.has("commandes/sans-coupon.yml");
    await row(app, "commandes").get('[role="treeitem"]').trigger("keydown", {
      code: "KeyZ",
      ctrlKey: true,
    });
    await flushPromises();
    expect({
      pasted,
      kept: core.workbench.requests.has("sans-coupon.yml"),
      undone: core.workbench.requests.has("commandes/sans-coupon.yml"),
    }).toEqual({ pasted: true, kept: true, undone: false });
  });

  it("Ctrl+X puis Ctrl+V sur une étape la déplace juste après", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await row(app, "Sans coupon").get('[role="treeitem"]').trigger("keydown", {
      code: "KeyX",
      ctrlKey: true,
    });
    await row(app, "Créer la commande").get('[role="treeitem"]').trigger("keydown", {
      code: "KeyV",
      ctrlKey: true,
    });
    await flushPromises();
    const wb = core.workbench;
    expect([
      wb.requests.get("commandes/v2/creer.yml")?.seq,
      wb.requests.get("commandes/v2/sans-coupon.yml")?.seq,
      wb.requests.get("commandes/v2/relire.yml")?.seq,
      wb.requests.has("sans-coupon.yml"),
    ]).toEqual([1, 2, 3, false]);
  });

  it("Ctrl+glisser une requête sur un dossier la copie ; glisser la déplace", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    const data = new Map<string, string>();
    const dataTransfer = {
      setData: (type: string, value: string) => data.set(type, value),
      getData: (type: string) => data.get(type) ?? "",
      get types() {
        return [...data.keys()];
      },
      effectAllowed: "",
      dropEffect: "",
    };
    await row(app, "Sans coupon").get('[role="treeitem"]').trigger("dragstart", { dataTransfer });
    const target = row(app, "commandes").get(".row");
    await target.trigger("dragover", { dataTransfer, ctrlKey: true });
    await target.trigger("drop", { dataTransfer, ctrlKey: true });
    await flushPromises();
    expect({
      copied: core.workbench.requests.has("commandes/sans-coupon.yml"),
      kept: core.workbench.requests.has("sans-coupon.yml"),
    }).toEqual({ copied: true, kept: true });
  });
});

describe("« Requête au point ? » sous un endpoint (3.2-c)", () => {
  it("s'affiche pour une requête liée d'un projet, pas seulement dans « Non classés »", async () => {
    ({ app } = await openProject((core) => {
      core.contracts.files.set("contracts/commandes.yaml", {
        name: "API Commandes",
        version: "2.4.0",
        source: "openapi: 3.0.3\n",
        operations: [
          { key: "POST /orders", method: "POST", path: "/orders", summary: null, tag: null },
        ],
      });
      core.workbench.requests.set(
        "API Commandes/commande-simple.yml",
        newRequest("Commande simple", {
          method: "POST",
          contract: { file: "contracts/commandes.yaml", operation: "POST /orders" },
        }),
      );
    }));
    await useWorkbenchStore().openRequest(BOUTIQUE_TESTS, "API Commandes/commande-simple.yml");
    await flushPromises();
    expect(app.get(".trial-transfer").text()).toContain("Glissez-la dans un test");
  });
});
