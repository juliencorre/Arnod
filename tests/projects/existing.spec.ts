import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { installFakeCore } from "../fakeCore";
import { BOUTIQUE, MES_PROJETS, boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

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
  it("depuis « Ajouter un projet », l'inscrit dans ce workspace et le défaut", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.pickedFolder = "/home/julien/dev/catalogue-legacy";
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer un projet existant…");

    expect(core.calls.find((c) => c.cmd === "project_add_existing")?.args).toEqual({
      path: "/home/julien/dev/catalogue-legacy",
      workspaceId: BOUTIQUE,
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(wrapper.findAll("main article h2").map((h) => h.text())).toContain("catalogue-legacy");
    expect(wrapper.get(".toast").text()).toBe(
      "« catalogue-legacy » ajouté au référentiel et à Mes projets et Boutique.",
    );
  });

  it("ne fait rien si le sélecteur est annulé", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer un projet existant…");

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

    await clickChoice(wrapper, "Importer un projet existant…");

    const toast = wrapper.get(".toast");
    expect(toast.classes()).toContain("toast--error");
    expect(toast.text()).toBe(
      "Le dossier /home/julien/vide ne contient pas de projet Arnod (arnod.yml absent).",
    );
  });

  it("depuis le référentiel, l'inscrit dans le dernier workspace ouvert", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.pickedFolder = "/home/julien/dev/partenaires-v2";
    wrapper = await mountApp();
    await wrapper.get('aside button[aria-haspopup="dialog"]').trigger("click");
    await flushPromises();
    await clickButton(wrapper, "aside", "Référentiel des projets2 projets");

    await clickButton(wrapper, "main", "Importer un projet existant…");

    expect(core.calls.find((c) => c.cmd === "project_add_existing")?.args).toEqual({
      path: "/home/julien/dev/partenaires-v2",
      workspaceId: BOUTIQUE,
    });
    expect(wrapper.get("main h1").text()).toBe("3 projets");
    expect(core.snapshot().workspaces.find((w) => w.id === MES_PROJETS)?.projectIds).toContain(
      "p-partenaires-v2",
    );
  });
});
