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

function button(within: AppWrapper | DOMWrapper<Element>, text: string) {
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
    const sidebarProjects = wrapper.findAll("aside nav li").map((li) => li.text());
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
    expect(dialog.get<HTMLSelectElement>("select").element.value).toBe(BOUTIQUE);
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
    expect(wrapper.get(".toast").text()).toBe("« boutique-tests » ajouté à Recette sprint 42.");
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
      "« boutique-tests » retiré de « Boutique ». Le projet reste dans le référentiel.",
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

    const sidebarButton = wrapper.findAll("aside nav li button")[1];
    await sidebarButton?.trigger("click");
    await flushPromises();
    expect(wrapper.get(".toast").text()).toContain(
      "Le dossier du projet « auth-sso » est introuvable",
    );
    expect(wrapper.get("main h1").text()).toBe("Boutique");
  });
});

describe("sélecteur de workspaces (2b, W4, W5)", () => {
  async function openSwitcher(app: AppWrapper): Promise<void> {
    const trigger = app.get('aside button[aria-haspopup="dialog"]');
    (trigger.element as HTMLElement).focus();
    await trigger.trigger("click");
    await flushPromises();
  }

  it("liste, filtre et change de workspace", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);

    const popover = wrapper.get('[role="dialog"][aria-label="Changer de workspace"]');
    expect(document.activeElement).toBe(popover.get('input[type="search"]').element);
    const names = () => popover.findAll(".switcher__item-name").map((n) => n.text());
    expect(names()).toEqual(["Mes projets", "Boutique", "Recette sprint 42"]);
    expect(popover.get('[aria-current="true"]').text()).toContain("Boutique");

    await popover.get('input[type="search"]').setValue("rec");
    expect(names()).toEqual(["Recette sprint 42"]);

    await press("ArrowDown");
    expect(document.activeElement?.textContent).toContain("Recette sprint 42");
    (document.activeElement as HTMLElement).click();
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "workspace_open")?.args).toEqual({
      workspaceId: "ws-recette",
    });
    expect(wrapper.get("main h1").text()).toBe("Recette sprint 42");
    expect(wrapper.find('[aria-label="Changer de workspace"]').exists()).toBe(false);
  });

  it("Échap ferme et rend le focus au sélecteur", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);

    await press("Escape");

    expect(wrapper.find('[aria-label="Changer de workspace"]').exists()).toBe(false);
    expect(document.activeElement).toBe(
      wrapper.get('aside button[aria-haspopup="dialog"]').element,
    );
  });

  it("crée un workspace vide et l'ouvre", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);

    await button(wrapper.get("aside"), "+ Nouveau workspace").trigger("click");
    await flushPromises();
    const dialog = wrapper.get("dialog[open]");
    expect(dialog.get("h2").text()).toBe("Nouveau workspace");
    expect(dialog.text()).toContain("Il démarre vide : ajoutez-y des projets du référentiel.");

    await dialog.get("input").setValue("Boutique");
    await dialog.get("form").trigger("submit");
    await flushPromises();
    expect(dialog.get("[role='alert']").text()).toBe("Un workspace « Boutique » existe déjà.");

    await dialog.get("input").setValue("Migration paiements");
    await dialog.get("form").trigger("submit");
    await flushPromises();

    expect(
      core.calls.find((c) => c.cmd === "workspace_create" && c.args.name === "Migration paiements"),
    ).toBeDefined();
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(wrapper.get("main h1").text()).toBe("Migration paiements");
    expect(wrapper.get("main").text()).toContain("Aucun projet pour l'instant");
    expect(wrapper.get(".toast").text()).toBe("Workspace « Migration paiements » créé.");
  });
});

describe("ajouter des projets du référentiel (2c, W6)", () => {
  async function openAddProjects(app: AppWrapper) {
    await app.get("main .add-card").trigger("click");
    await flushPromises();
    return app.get("dialog[open]");
  }

  it("grise les projets présents et ajoute la sélection", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    expect(dialog.get("h2").text()).toBe("Ajouter des projets à « Boutique »");
    const rows = dialog.findAll("li label");
    expect(rows.map((row) => row.find(".row__note").text())).toEqual([
      "déjà dans ce workspace",
      "déjà dans ce workspace",
      "Mes projets",
    ]);
    expect(rows.map((row) => row.get<HTMLInputElement>("input").element.disabled)).toEqual([
      true,
      true,
      false,
    ]);
    expect(button(dialog, "Ajouter").attributes("disabled")).toBeDefined();

    await rows[2]?.get("input").setValue(true);
    await button(dialog, "Ajouter 1 projet").trigger("click");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "workspace_add_projects")?.args).toEqual({
      workspaceId: BOUTIQUE,
      projectIds: ["p-notifications"],
    });
    expect(cardNames(wrapper)).toEqual(["boutique-tests", "auth-sso", "notifications"]);
    expect(wrapper.get(".toast").text()).toBe("« notifications » ajouté à « Boutique ».");
  });

  it("filtre par nom ou dossier", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);

    await dialog.get('input[type="search"]').setValue("NOTIF");
    expect(dialog.findAll(".row__name").map((n) => n.text())).toEqual(["notifications"]);

    await dialog.get('input[type="search"]').setValue("introuvable");
    expect(dialog.text()).toContain("Aucun projet ne correspond.");
  });

  it("« + Créer un nouveau projet » ouvre la fenêtre de création pour ce workspace", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const dialog = await openAddProjects(wrapper);
    expect(dialog.text()).toContain("ajouté ici et à « Mes projets » (par défaut)");

    await button(dialog, "+ Créer un nouveau projet").trigger("click");
    await flushPromises();

    const next = wrapper.get("dialog[open]");
    expect(next.get("h2").text()).toBe("Nouveau projet");
    expect(next.get<HTMLSelectElement>("select").element.value).toBe(BOUTIQUE);
  });
});
