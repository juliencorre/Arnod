import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

describe("arborescence du projet", () => {
  it("affiche les dossiers puis les requêtes avec leur méthode", async () => {
    ({ app } = await openProject());
    const items = app.findAll('[role="treeitem"]').map((i) => i.text());
    expect(items).toEqual(["▾Clients2", "POSTCréer", "GETLister", "GETSanté"]);
  });

  it("replie un dossier et mémorise l'état", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Clients");
    expect(app.findAll('[role="treeitem"]')).toHaveLength(2);
    expect(core.calls.filter((c) => c.cmd === "tree_set_collapsed").at(-1)?.args).toMatchObject({
      collapsed: ["Clients"],
    });
  });

  it("ouvre une requête dans l'éditeur", async () => {
    ({ app } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    expect((app.get(".editor__url").element as HTMLInputElement).value).toBe("{{baseUrl}}/clients");
    expect(app.get('[role="treeitem"][aria-selected="true"]').text()).toContain("Lister");
  });

  it("crée une requête depuis la fenêtre de nom", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, ".tree__head button", "+ Requête");
    const input = app.get("dialog[open] input");
    await input.setValue("Supprimer");
    await app.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_request")?.args).toMatchObject({
      folder: "",
      name: "Supprimer",
    });
    expect(app.findAll('[role="treeitem"]').map((i) => i.text())).toContain("GETSupprimer");
  });

  it("affiche l'erreur du cœur quand le nom est déjà pris", async () => {
    ({ app } = await openProject());
    await click(app, ".tree__head button", "+ Dossier");
    await app.get("dialog[open] input").setValue("Clients");
    await app.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(app.get("dialog[open] [role='alert']").text()).toContain("existe déjà");
  });

  it("met un élément à la corbeille avec une notification", async () => {
    let core;
    ({ app, core } = await openProject());
    const row = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Santé"));
    await row?.get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Mettre à la corbeille");
    expect(core.calls.find((c) => c.cmd === "tree_delete")?.args).toMatchObject({
      path: "sante.yml",
      permanent: false,
    });
    expect(app.get(".toast").text()).toContain("« Santé » mis à la corbeille.");
  });

  it("le menu d'une ligne s'ouvre au clavier", async () => {
    ({ app } = await openProject());
    const more = app.get(".row__more");
    await more.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(more.attributes("aria-expanded")).toBe("true");
    expect(app.findAll('[role="menuitem"]').length).toBeGreaterThan(0);
  });
});
