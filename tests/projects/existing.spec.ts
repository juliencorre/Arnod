import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { installFakeCore } from "../fakeCore";
import { BOUTIQUE, boutiqueSnapshot, firstRunSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { chooseWorkspace } from "../switcher";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function clickButton(app: AppWrapper, selector: string, text: string): Promise<void> {
  const found = app.findAll(`${selector} button`).find((b) => b.text().trim() === text);
  if (!found) throw new Error(`bouton « ${text} » introuvable`);
  await found.trigger("click");
  await flushPromises();
}

/** Clique sur un choix de la fenêtre « Ajouter un projet » (2c), par son titre. */
async function clickChoice(app: AppWrapper, title: string): Promise<void> {
  const found = app
    .findAll("dialog[open] .choice")
    .find((c) => c.find(".choice__title").text() === title);
  if (!found) throw new Error(`choix « ${title} » introuvable`);
  await found.trigger("click");
  await flushPromises();
}

describe("importer un projet existant (P12)", () => {
  it("depuis « Ajouter un projet », l'inscrit dans « Tous les projets » et ce workspace", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.pickedFolder = "/home/julien/dev/catalogue-legacy";
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer…");

    expect(core.calls.find((c) => c.cmd === "project_add_existing")?.args).toEqual({
      path: "/home/julien/dev/catalogue-legacy",
      workspaceId: BOUTIQUE,
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(wrapper.findAll("main article h2").map((h) => h.text())).toContain("catalogue-legacy");
    expect(wrapper.get(".toast").text()).toBe(
      "« catalogue-legacy » ajouté à « Tous les projets » et à « Boutique ».",
    );
  });

  it("ne fait rien si le sélecteur est annulé", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer…");

    expect(core.commands()).toContain("project_pick_folder");
    expect(core.commands()).not.toContain("project_add_existing");
    expect(wrapper.find(".toast").exists()).toBe(false);
  });

  it("notifie un dossier qui n'est pas un projet", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.pickedFolder = "/home/julien/vide";
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer…");

    const toast = wrapper.get(".toast");
    expect(toast.classes()).toContain("toast--error");
    expect(toast.text()).toBe(
      "Le dossier /home/julien/vide ne contient pas de projet Arnod (arnod.yml absent).",
    );
  });

  it("depuis « Tous les projets », l'inscrit sans workspace", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.pickedFolder = "/home/julien/dev/partenaires-v2";
    wrapper = await mountApp();
    await chooseWorkspace(wrapper, "Tous les projets");

    await clickButton(wrapper, "main", "Importer…");

    expect(core.calls.find((c) => c.cmd === "project_add_existing")?.args).toEqual({
      path: "/home/julien/dev/partenaires-v2",
      workspaceId: null,
    });
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 3");
    expect(core.snapshot().workspaces.some((w) => w.projectIds.includes("p-partenaires-v2"))).toBe(
      false,
    );
    expect(wrapper.get(".toast").text()).toBe("« partenaires-v2 » ajouté à « Tous les projets ».");
  });

  it("depuis le premier lancement, « Importer… » inscrit le projet sans workspace", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.pickedFolder = "/home/julien/dev/boutique-tests";
    wrapper = await mountApp();

    await wrapper.get(".first-run__import").trigger("click");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "project_add_existing")?.args).toEqual({
      path: "/home/julien/dev/boutique-tests",
      workspaceId: null,
    });
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 1");
  });
});
