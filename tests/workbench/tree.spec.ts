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
    // « Clients » est un test : ses étapes sont numérotées (lot E).
    expect(items).toEqual(["▾Clients2", "POST1Créer", "GET2Lister", "GETSanté"]);
  });

  it("liste un fichier de requête illisible sans méthode (MA1)", async () => {
    // Le cœur envoie `method: null` pour un fichier illisible ; la ligne ne doit pas casser le rendu.
    ({ app } = await openProject((core) => {
      core.workbench.unreadable.add("casse.yml");
    }));
    const row = app.findAll('[role="treeitem"]').find((i) => i.text().includes("casse"));
    expect(row?.find(".row__method").text()).toBe("");
    expect(row?.find(".row__invalid").exists()).toBe(true);
  });

  it("replie un dossier et mémorise l'état", async () => {
    let core;
    ({ app, core } = await openProject());
    // Un test se replie par son chevron (lot E) ; un clic sur son nom ouvre son onglet.
    await click(app, '[role="treeitem"] .row__chevron');
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
    // « + » de la ligne du projet, puis « Requête libre » (B3, 1.2-a ter).
    await click(app, '[aria-label="Ajouter au projet boutique-tests"]');
    await click(app, '[role="menuitem"]', "Requête libre");
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
    await click(app, '[aria-label="Ajouter au projet boutique-tests"]');
    await click(app, '[role="menuitem"]', "Nouveau dossier");
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
    expect(core.calls.find((c) => c.cmd === "tree_batch")?.args).toMatchObject({
      paths: ["sante.yml"],
      action: { kind: "delete", permanent: false },
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
