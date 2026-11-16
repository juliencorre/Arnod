import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Sélection multiple de l'arborescence (écran 4.1-a quater). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

async function openShop(): Promise<{ app: AppWrapper; core: FakeCore }> {
  return openProject((core) => {
    const wb = core.workbench;
    wb.requests.clear();
    wb.folders.clear();
    wb.folders.add("archives");
    wb.requests.set("a-lister.yml", newRequest("A lister"));
    wb.requests.set("b-creer.yml", newRequest("B créer", { method: "POST" }));
    wb.requests.set("c-lire.yml", newRequest("C lire"));
  });
}

function item(wrapper: AppWrapper, name: string) {
  const found = wrapper.findAll('[role="treeitem"]').find((i) => i.text().includes(name));
  if (!found) throw new Error(`ligne introuvable : ${name}`);
  return found;
}

function picked(wrapper: AppWrapper): string[] {
  return wrapper.findAll(".row--picked .row__name").map((n) => n.text());
}

describe("sélection multiple (4.1-a quater)", () => {
  it("Ctrl+clic ajoute ou retire, Maj+clic prend la plage, Échap vide", async () => {
    ({ app } = await openShop());
    await item(app, "A lister").trigger("click", { ctrlKey: true });
    await item(app, "C lire").trigger("click", { ctrlKey: true });
    const two = { picked: picked(app), bar: app.get(".selection__count").text() };
    await item(app, "A lister").trigger("click", { ctrlKey: true });
    const one = picked(app);
    // L'ancre reste le dernier élément cliqué (« A lister »), comme dans un explorateur.
    await item(app, "C lire").trigger("click", { shiftKey: true });
    const range = picked(app);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await flushPromises();
    expect({ two, one, range, after: app.find(".selection").exists() }).toEqual({
      two: { picked: ["A lister", "C lire"], bar: "2 sélectionnés" },
      one: ["C lire"],
      range: ["A lister", "B créer", "C lire"],
      after: false,
    });
  });

  it("au clavier : Ctrl+Espace choisit la ligne ; la barre se parcourt à la tabulation", async () => {
    ({ app } = await openShop());
    await item(app, "B créer").trigger("keydown", { key: " ", ctrlKey: true });
    await item(app, "C lire").trigger("keydown", { key: " ", ctrlKey: true });
    expect({
      picked: picked(app),
      selected: item(app, "C lire").attributes("aria-selected"),
      actions: app.findAll(".selection button").map((b) => b.text()),
    }).toEqual({
      picked: ["B créer", "C lire"],
      selected: "true",
      actions: ["Déplacer vers…", "Dupliquer", "Exécuter", "Supprimer", "×"],
    });
  });

  it("« Dupliquer » duplique la sélection ; « Annuler » la remet", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await item(app, "A lister").trigger("click", { ctrlKey: true });
    await item(app, "B créer").trigger("click", { ctrlKey: true });
    await click(app, ".selection button", "Dupliquer");
    const after = [...core.workbench.requests.keys()].sort();
    const toast = app.get(".toast").text();
    await click(app, ".toast button", "Annuler");
    await flushPromises();
    expect({ after, toast, undone: [...core.workbench.requests.keys()].sort() }).toEqual({
      after: [
        "a-lister-copie.yml",
        "a-lister.yml",
        "b-creer-copie.yml",
        "b-creer.yml",
        "c-lire.yml",
      ],
      toast: expect.stringContaining("2 éléments dupliqués.") as unknown,
      undone: ["a-lister.yml", "b-creer.yml", "c-lire.yml"],
    });
  });

  it("« Déplacer vers… » range la sélection dans le dossier choisi", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await item(app, "A lister").trigger("click", { ctrlKey: true });
    await item(app, "C lire").trigger("click", { ctrlKey: true });
    await click(app, ".selection button", "Déplacer vers…");
    await app.get("dialog[open] select").setValue("archives");
    await click(app, "dialog[open] button", "Déplacer");
    expect(core.calls.find((c) => c.cmd === "tree_batch")?.args).toMatchObject({
      paths: ["a-lister.yml", "c-lire.yml"],
      action: { kind: "move", target: "archives" },
    });
    expect([...core.workbench.requests.keys()].sort()).toEqual([
      "archives/a-lister.yml",
      "archives/c-lire.yml",
      "b-creer.yml",
    ]);
  });

  it("« Exécuter » lance un run de la sélection ; « Supprimer » la met à la corbeille", async () => {
    let core: FakeCore;
    ({ app, core } = await openShop());
    await item(app, "A lister").trigger("click", { ctrlKey: true });
    await item(app, "B créer").trigger("click", { ctrlKey: true });
    await click(app, ".selection button", "Exécuter");
    const run = core.workbench.lastRun?.items.map((i) => i.path);
    await item(app, "C lire").trigger("click", { ctrlKey: true });
    await click(app, ".selection button", "Supprimer");
    expect({
      run,
      deleted: core.calls.find((c) => c.cmd === "tree_batch")?.args,
      left: [...core.workbench.requests.keys()],
    }).toEqual({
      run: ["a-lister.yml", "b-creer.yml"],
      deleted: expect.objectContaining({
        paths: ["c-lire.yml"],
        action: { kind: "delete", permanent: false },
      }) as unknown,
      left: ["a-lister.yml", "b-creer.yml"],
    });
  });
});
