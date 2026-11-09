import { clearMocks } from "@tauri-apps/api/mocks";
import { type DOMWrapper, flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { installFakeCore } from "../fakeCore";
import {
  AUTH_SSO,
  BOUTIQUE,
  BOUTIQUE_TESTS,
  MES_PROJETS,
  boutiqueSnapshot,
  project,
  workspace,
} from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

/** Élément dans lequel chercher des boutons (application ou partie de page). */
interface Searchable {
  findAll: (selector: string) => DOMWrapper<Element>[];
}

function button(within: Searchable, text: string) {
  const found = within.findAll("button").find((b) => b.text().trim() === text);
  if (!found) throw new Error(`bouton « ${text} » introuvable`);
  return found;
}

function cardNames(app: AppWrapper): string[] {
  return app.findAll("main article h2").map((h) => h.text());
}

/** Appuie sur une touche dans l'élément qui a le focus. */
async function press(key: string): Promise<void> {
  document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  await flushPromises();
}

/** Instantané avec un troisième workspace « Recette » vide et un projet hors de Boutique. */
function richSnapshot() {
  const snapshot = boutiqueSnapshot();
  snapshot.projects.push(
    project("p-notifications", "notifications", { workspaceIds: [MES_PROJETS] }),
  );
  snapshot.workspaces[0]?.projectIds.push("p-notifications");
  snapshot.workspaces.push(workspace("ws-recette", "Recette sprint 42", { color: "green" }));
  return snapshot;
}

describe("vue d'un workspace (2a, W3, W7)", () => {
  it("affiche ses projets en cartes et la carte d'ajout", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    expect(cardNames(wrapper)).toEqual(["boutique-tests", "auth-sso"]);
    expect(wrapper.get("main").text()).toContain("+ Ajouter un projet");
    const sidebarProjects = wrapper.findAll("aside .sidebar__project").map((b) => b.text());
    expect(sidebarProjects).toEqual(["Bboutique-tests", "Aauth-sso"]);
  });

  it("renomme le workspace sur place, avec l'erreur du cœur", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await button(wrapper.get("main"), "Renommer").trigger("click");
    const input = wrapper.get<HTMLInputElement>("main form input");
    expect(input.element.value).toBe("Boutique");

    await input.setValue("mes projets");
    await wrapper.get("main form").trigger("submit");
    await flushPromises();
    expect(wrapper.get("main form [role='alert']").text()).toBe(
      "Un workspace « Mes projets » existe déjà.",
    );

    await input.setValue("Boutique v2");
    await wrapper.get("main form").trigger("submit");
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "workspace_rename").at(-1)?.args).toEqual({
      workspaceId: BOUTIQUE,
      name: "Boutique v2",
    });
    expect(wrapper.get("main h1").text()).toBe("Boutique v2");
    expect(wrapper.find("main form").exists()).toBe(false);
  });

  it("Échap annule le renommage", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await button(wrapper.get("main"), "Renommer").trigger("click");

    await wrapper.get("main form").trigger("keydown", { key: "Escape" });

    expect(wrapper.find("main form").exists()).toBe(false);
    expect(core.commands()).not.toContain("workspace_rename");
  });

  it("« + Nouveau projet » ouvre la fenêtre avec ce workspace", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await button(wrapper.get("main"), "+ Nouveau projet").trigger("click");
    await flushPromises();

    const dialog = wrapper.get("dialog[open]");
    expect(dialog.get("h2").text()).toBe("Nouveau projet");
    expect(dialog.text()).toContain("Il sera rangé dans le workspace ouvert");
    expect(dialog.findAll(".storage__chip").map((c) => c.text().replace(/\s+/g, " "))).toEqual([
      "Tous les projets par défaut",
      "Boutique workspace ouvert ×",
    ]);
  });
});

describe("menu ⋯ d'un projet (P5, P6, P7)", () => {
  async function openMenu(app: AppWrapper, name: string): Promise<void> {
    const trigger = app.get(`button[aria-label="Actions du projet ${name}"]`);
    (trigger.element as HTMLElement).focus();
    await trigger.trigger("click");
    await flushPromises();
  }

  it("ouvre le projet", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await openMenu(wrapper, "auth-sso");
    await press("Enter");

    expect(core.calls.find((c) => c.cmd === "project_open")?.args).toEqual({ projectId: AUTH_SSO });
    expect(wrapper.get("main h1").text()).toBe("auth-sso");
  });

  it("ajoute à un autre workspace par le sous-menu", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await openMenu(wrapper, "boutique-tests");
    await press("ArrowDown");
    await press("ArrowRight");
    const items = wrapper
      .findAll('[data-level="1"]')
      .map((li) => [li.text(), li.attributes("aria-checked"), li.attributes("aria-disabled")]);
    expect(items).toEqual([
      ["✓Mes projets", "true", "true"],
      ["Recette sprint 42", "false", undefined],
    ]);
    await press("Enter");

    expect(core.calls.find((c) => c.cmd === "workspace_add_projects")?.args).toEqual({
      workspaceId: "ws-recette",
      projectIds: [BOUTIQUE_TESTS],
    });
    expect(wrapper.get(".toast").text()).toBe("« boutique-tests » ajouté à « Recette sprint 42 ».");
  });

  it("retire du workspace, puis « Annuler » le remet (R5, R6)", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await openMenu(wrapper, "boutique-tests");
    await press("End");
    expect(document.activeElement?.textContent).toContain("Retirer de « Boutique »");
    await press("Enter");

    expect(cardNames(wrapper)).toEqual(["auth-sso"]);
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "« boutique-tests » retiré de « Boutique ». Le projet reste dans « Tous les projets ».",
    );
    expect(core.snapshot().projects.map((p) => p.name)).toContain("boutique-tests");

    await button(toast, "Annuler").trigger("click");
    await flushPromises();
    expect(cardNames(wrapper)).toEqual(["boutique-tests", "auth-sso"]);
    expect(core.commands()).toContain("undo");
  });
});

describe("projet introuvable (P10)", () => {
  it("le signale sur la carte et dans la barre latérale, sans l'ouvrir", async () => {
    const snapshot = boutiqueSnapshot();
    const missing = snapshot.projects[1];
    if (missing) missing.status = "missing";
    installFakeCore(snapshot).missing.add(AUTH_SSO);
    wrapper = await mountApp();

    const card = wrapper.findAll("main article")[1];
    expect(card?.text()).toContain("Dossier introuvable : déplacé ou supprimé hors d'Arnod.");
    expect(card?.get("button").attributes("disabled")).toBeDefined();
    expect(wrapper.get("aside").text()).toContain("(dossier introuvable)");

    const sidebarButton = wrapper.findAll("aside .sidebar__project")[1];
    await sidebarButton?.trigger("click");
    await flushPromises();
    expect(wrapper.get(".toast").text()).toContain(
      "Le dossier du projet « auth-sso » est introuvable",
    );
    expect(wrapper.get("main h1").text()).toBe("Boutique");
  });
});

describe("ajouter un projet à un workspace (2c, W6)", () => {
  async function openAddProjects(app: AppWrapper) {
    await app.get("main .add-card").trigger("click");
    await flushPromises();
    return app.get("dialog[open]");
  }

  function choiceTitles(dialog: Searchable): string[] {
    return dialog.findAll(".choice__title").map((title) => title.text());
  }

  function choice(dialog: Searchable, title: string) {
    const found = dialog.findAll(".choice").find((c) => c.find(".choice__title").text() === title);
    if (!found) throw new Error(`choix « ${title} » introuvable`);
    return found;
  }

  /** Instantané où Boutique laisse de côté `count` projets sans workspace. */
  function longRegistrySnapshot(count: number) {
    const snapshot = boutiqueSnapshot();
    for (let i = 1; i <= count; i++) {
      snapshot.projects.push(project(`p-api-${String(i)}`, `api-${String(i)}`));
    }
    return snapshot;
  }

  it("propose d'abord de créer ou d'importer un projet", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    expect(dialog.get("h2").text()).toBe("Ajouter un projet à « Boutique »");
    expect(choiceTitles(dialog)).toEqual(["Créer un nouveau projet", "Importer…"]);
    const create = choice(dialog, "Créer un nouveau projet");
    expect(create.element.tagName).toBe("BUTTON");
    expect(create.attributes("aria-labelledby")).toBe(
      create.get(".choice__title").attributes("id"),
    );
    expect(create.attributes("aria-describedby")).toBe(
      create.get(".choice__text").attributes("id"),
    );
    // Tous les projets sont déjà là : ni liste, ni recherche, ni « Ajouter ».
    expect(dialog.find("section").exists()).toBe(false);
    expect(dialog.find('input[type="search"]').exists()).toBe(false);
    expect(dialog.findAll("footer button").map((b) => b.text())).toEqual(["Annuler"]);
  });

  it("« Créer un nouveau projet » est le premier élément focalisable", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    // `showModal()` donne le focus au premier élément focalisable : Entrée crée un projet.
    const first = dialog.element.querySelector("button, input, select, textarea, [tabindex]");
    expect(first).toBe(choice(dialog, "Créer un nouveau projet").element);
  });

  it("liste ensuite les seuls projets absents du workspace et ajoute la sélection", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    expect(dialog.get("section h3").text()).toBe("Ou reprendre un projet existant");
    const rows = dialog.findAll("li label");
    expect(rows.map((row) => row.get(".row__name").text())).toEqual(["notifications"]);
    expect(rows.map((row) => row.get(".row__note").text())).toEqual(["Mes projets"]);
    expect(dialog.find('input[type="search"]').exists()).toBe(false);
    expect(button(dialog, "Ajouter").attributes("disabled")).toBeDefined();

    await rows[0]?.get("input").setValue(true);
    await button(dialog, "Ajouter 1 projet").trigger("click");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "workspace_add_projects")?.args).toEqual({
      workspaceId: BOUTIQUE,
      projectIds: ["p-notifications"],
    });
    expect(cardNames(wrapper)).toEqual(["boutique-tests", "auth-sso", "notifications"]);
    expect(wrapper.get(".toast").text()).toBe("« notifications » ajouté à « Boutique ».");
  });

  it("n'affiche pas la recherche pour six projets proposés ou moins", async () => {
    installFakeCore(longRegistrySnapshot(6));
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    expect(dialog.findAll(".row__name")).toHaveLength(6);
    expect(dialog.find('input[type="search"]').exists()).toBe(false);
  });

  it("filtre par nom ou dossier quand la liste est longue", async () => {
    installFakeCore(longRegistrySnapshot(7));
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);
    expect(dialog.findAll(".row__note").map((n) => n.text())).toContain("Aucun workspace");

    await dialog.get('input[type="search"]').setValue("API-7");
    expect(dialog.findAll(".row__name").map((n) => n.text())).toEqual(["api-7"]);

    await dialog.get('input[type="search"]').setValue("introuvable");
    expect(dialog.text()).toContain("Aucun projet ne correspond.");
  });

  it("« Créer un nouveau projet » ouvre la fenêtre de création pour ce workspace", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    await choice(dialog, "Créer un nouveau projet").trigger("click");
    await flushPromises();

    const next = wrapper.get("dialog[open]");
    expect(next.get("h2").text()).toBe("Nouveau projet");
    expect(next.findAll(".storage__chip")[1]?.text()).toContain("Boutique");
  });
});
