import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { useDialogStore } from "@/stores/dialogs";

import { DEFAULT_LOCATION, installFakeCore } from "../fakeCore";
import {
  BOUTIQUE,
  BOUTIQUE_TESTS,
  MES_PROJETS,
  boutiqueSnapshot,
  firstRunSnapshot,
  workspace,
} from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { chooseWorkspace, currentWorkspace, switcherNames } from "../switcher";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function folderPreview(app: AppWrapper): string {
  return app.get('[data-testid="project-folder"]').text();
}

describe("premier lancement (1a, P1, P3, P4, O8)", () => {
  it("propose le dossier, crée le projet sans workspace, l'ouvre et l'annonce", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();
    expect(folderPreview(wrapper)).toBe(DEFAULT_LOCATION);
    expect(wrapper.text()).toContain(
      "Un projet est toujours dans « Tous les projets ». Un workspace est facultatif.",
    );
    expect(wrapper.findAll(".storage__chip").map((c) => c.text().replace(/\s+/g, " "))).toEqual([
      "Tous les projets par défaut",
    ]);

    await wrapper.get(".first-run__card form input").setValue("boutique-tests");
    await flushPromises();
    expect(folderPreview(wrapper)).toBe(`${DEFAULT_LOCATION}/boutique-tests`);

    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();

    expect(core.calls.find((call) => call.cmd === "project_create")?.args).toEqual({
      name: "boutique-tests",
      location: null,
      workspaceId: null,
    });
    expect(wrapper.get("main h1").text()).toBe("boutique-tests");
    expect(wrapper.get("main").text()).toContain("Aucune requête.");
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "✓ « boutique-tests » créé · visible dans « Tous les projets ».",
    );
    expect(toast.text()).toContain("Ranger dans un workspace…");
  });

  it("crée un workspace sans quitter le formulaire et y range le projet (1a bis)", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();

    const add = wrapper.get(".storage__add");
    (add.element as HTMLElement).focus();
    await add.trigger("click");
    await flushPromises();
    const items = wrapper.findAll('[role="menuitem"]').map((item) => item.text());
    expect(items).toEqual(["Aucun workspace pour l'instant", "Créer un workspace…"]);
    await wrapper
      .findAll('[role="menuitem"]')
      .find((item) => item.text() === "Créer un workspace…")
      ?.trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain(
      "Il apparaîtra dans le sélecteur, après « Tous les projets ».",
    );

    const input = wrapper.get(".storage__create input");
    expect(document.activeElement).toBe(input.element);
    await input.setValue("E-commerce");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "workspace_create")?.args).toEqual({
      name: "E-commerce",
    });
    expect(wrapper.findAll(".storage__chip").map((c) => c.text().replace(/\s+/g, " "))).toEqual([
      "Tous les projets par défaut",
      "E-commerce ×",
    ]);
    // Le workspace est déjà dans le sélecteur, après « Tous les projets ».
    expect(await switcherNames(wrapper)).toEqual(["Tous les projets", "E-commerce"]);

    await wrapper.get(".first-run__card form input").setValue("boutique-tests");
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();
    const created = core.calls.find((c) => c.cmd === "project_create")?.args;
    expect(created?.workspaceId).toBe("ws-e-commerce");
    expect(wrapper.get(".toast").text()).toContain(
      "✓ « boutique-tests » créé dans « E-commerce » · visible dans « Tous les projets ».",
    );
    expect(currentWorkspace(wrapper)).toBe("E-commerce");
  });

  it("« Changer » choisit l'emplacement parent du dossier", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.pickedFolder = "/home/julien/dev";
    wrapper = await mountApp();
    await wrapper.get(".first-run__card form input").setValue("auth-sso");

    await wrapper.get('button[aria-label="Changer l\'emplacement du dossier"]').trigger("click");
    await flushPromises();

    expect(folderPreview(wrapper)).toBe("/home/julien/dev/auth-sso");
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();
    expect(core.calls.find((call) => call.cmd === "project_create")?.args).toMatchObject({
      location: "/home/julien/dev",
    });
  });

  it("garde l'emplacement si le sélecteur est annulé", async () => {
    installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();

    await wrapper.get('button[aria-label="Changer l\'emplacement du dossier"]').trigger("click");
    await flushPromises();

    expect(folderPreview(wrapper)).toBe(DEFAULT_LOCATION);
  });

  it("affiche l'erreur du cœur sous le champ concerné", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();
    const input = wrapper.get(".first-run__card form input");

    await input.setValue("api/v2");
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();
    expect(input.attributes("aria-invalid")).toBe("true");
    const nameError = wrapper.get(`#${input.attributes("aria-describedby") ?? ""}`);
    expect(nameError.text()).toBe("Le caractère « / » n'est pas autorisé dans un nom de projet.");

    core.failNext("project_create", {
      kind: "folderNotEmpty",
      message: "Le dossier /home/julien/arnod/boutique existe déjà et n'est pas vide.",
    });
    await input.setValue("boutique");
    expect(input.attributes("aria-invalid")).toBe("false");
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();
    expect(wrapper.get('[role="group"] + [role="alert"]').text()).toBe(
      "Le dossier /home/julien/arnod/boutique existe déjà et n'est pas vide.",
    );
    expect(wrapper.get("main h1").text()).toBe("Bienvenue dans Arnod");
  });

  it("n'envoie rien tant que le nom est vide", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();

    expect(
      wrapper.get('.first-run__card form button[type="submit"]').attributes("disabled"),
    ).toBeDefined();
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();

    expect(core.commands()).not.toContain("project_create");
  });
});

describe("nouveau projet depuis un workspace (2a bis, R4)", () => {
  it("préremplit le workspace ouvert et crée dans celui-ci seulement", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    useDialogStore().openNewProject(BOUTIQUE);
    await flushPromises();

    const dialog = wrapper.get<HTMLDialogElement>("dialog[open]");
    expect(dialog.get("h2").text()).toBe("Nouveau projet");
    expect(dialog.findAll(".storage__chip").map((c) => c.text().replace(/\s+/g, " "))).toEqual([
      "Tous les projets par défaut",
      "Boutique workspace ouvert ×",
    ]);

    await dialog.get("input").setValue("paiements-v3");
    await dialog.get("form").trigger("submit");
    await flushPromises();

    expect(core.calls.find((call) => call.cmd === "project_create")?.args).toEqual({
      name: "paiements-v3",
      location: null,
      workspaceId: BOUTIQUE,
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(wrapper.get("main h1").text()).toBe("paiements-v3");
    expect(wrapper.get(".toast").text()).toContain(
      "✓ « paiements-v3 » créé dans « Boutique » · visible dans « Tous les projets ».",
    );
    expect(core.snapshot().workspaces.find((w) => w.id === MES_PROJETS)?.projectIds).not.toContain(
      "p-paiements-v3",
    );
  });

  it("× retire le workspace : le projet n'est rangé nulle part", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    useDialogStore().openNewProject(BOUTIQUE);
    await flushPromises();
    const dialog = wrapper.get("dialog[open]");

    await dialog.get('button[aria-label="Ne pas ranger dans « Boutique »"]').trigger("click");
    expect(dialog.find(".storage__add").exists()).toBe(true);
    await dialog.get("input").setValue("paiements-v3");
    await dialog.get("form").trigger("submit");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "project_create")?.args.workspaceId).toBeNull();
  });

  it("« Annuler » ferme la fenêtre sans rien créer", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    useDialogStore().openNewProject(BOUTIQUE);
    await flushPromises();

    const cancel = wrapper
      .findAll("dialog[open] button")
      .find((button) => button.text() === "Annuler");
    await cancel?.trigger("click");
    await flushPromises();

    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(core.commands()).not.toContain("project_create");
  });
});

describe("ajouter un projet à d'autres workspaces (P4, P6)", () => {
  it("depuis l'annonce de création, coche les workspaces voulus", async () => {
    const snapshot = boutiqueSnapshot();
    snapshot.workspaces.push(workspace("ws-recette", "Recette", { color: "green" }));
    const core = installFakeCore(snapshot);
    wrapper = await mountApp();
    useDialogStore().openAddToWorkspaces(BOUTIQUE_TESTS);
    await flushPromises();

    const dialog = wrapper.get("dialog[open]");
    expect(dialog.get("h2").text()).toBe("Ajouter « boutique-tests » à des workspaces");
    const boxes = dialog.findAll<HTMLInputElement>('input[type="checkbox"]');
    expect(boxes.map((box) => [box.element.checked, box.element.disabled])).toEqual([
      [true, true],
      [true, true],
      [false, false],
    ]);
    const add = dialog.findAll("button").find((button) => button.text() === "Ajouter");
    expect(add?.attributes("disabled")).toBeDefined();

    await boxes[2]?.setValue(true);
    await add?.trigger("click");
    await flushPromises();

    expect(core.calls.find((call) => call.cmd === "workspace_add_projects")?.args).toEqual({
      workspaceId: "ws-recette",
      projectIds: [BOUTIQUE_TESTS],
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(wrapper.get(".toast").text()).toBe("« boutique-tests » ajouté à « Recette ».");
  });
});

describe("vue projet (1b, P5)", () => {
  it("revient à « Tous les projets » par le sélecteur", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();
    await wrapper.get(".first-run__card form input").setValue("boutique-tests");
    await wrapper.get(".first-run__card form").trigger("submit");
    await flushPromises();
    expect(wrapper.get("main h1").text()).toBe("boutique-tests");

    await chooseWorkspace(wrapper, "Tous les projets");

    expect(core.calls.filter((c) => c.cmd === "workspace_open").at(-1)?.args).toEqual({
      workspaceId: null,
    });
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 1");
  });
});
