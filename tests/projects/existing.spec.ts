import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { installFakeCore } from "../fakeCore";
import { BOUTIQUE, boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

/** Clique sur un choix de la fenêtre « Ajouter un projet » (2c), par son titre. */
async function clickChoice(app: AppWrapper, title: string): Promise<void> {
  const found = app
    .findAll("dialog[open] .choice")
    .find((c) => c.find(".choice__title").text() === title);
  if (!found) throw new Error(`choix « ${title} » introuvable`);
  await found.trigger("click");
  await flushPromises();
}

/** Choisit un dossier dans la fenêtre « Importer » (import unique, AM40). */
async function chooseFolder(app: AppWrapper): Promise<void> {
  const button = app
    .findAll("dialog[open] button")
    .find((b) => b.text().trim() === "Choisir un dossier…");
  if (!button) throw new Error("« Choisir un dossier… » introuvable");
  await button.trigger("click");
  await flushPromises();
}

describe("importer un projet existant (P12, par l'import unique AM40)", () => {
  it("depuis « Ajouter un projet », l'inscrit dans « Tous les projets » et ce workspace", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    const path = "/home/julien/dev/catalogue-legacy";
    core.imports.sources[path] = {
      path,
      format: "openCollection",
      name: "catalogue-legacy",
      counts: {
        folders: 0,
        requests: 0,
        environments: 0,
        assertions: 0,
        extractions: 0,
        secrets: 0,
        dynamicValues: 0,
        scripts: 0,
      },
      files: 0,
      environments: [],
      secrets: [],
    };
    core.imports.picked = path;
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer…");
    await chooseFolder(wrapper);
    const open = wrapper.get("dialog[open]");
    expect(open.text()).toContain(
      "Ajouté à « Tous les projets » et à « Boutique », sans conversion.",
    );
    const importer = open.findAll("button").find((b) => b.text() === "Importer");
    await importer?.trigger("click");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toMatchObject({
      path,
      target: { kind: "newProject", location: null, workspaceId: BOUTIQUE },
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
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
    await chooseFolder(wrapper);

    expect(core.commands()).toContain("import_pick");
    expect(core.commands()).not.toContain("import_run");
    expect(wrapper.get("dialog[open]").text()).toContain("Choisir un dossier…");
    expect(wrapper.find(".toast").exists()).toBe(false);
  });

  it("explique un dossier qu'Arnod ne reconnaît pas", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.imports.picked = "/home/julien/vide";
    wrapper = await mountApp();
    await wrapper.get("main .add-card").trigger("click");
    await flushPromises();

    await clickChoice(wrapper, "Importer…");
    await chooseFolder(wrapper);

    expect(wrapper.get("dialog[open] [role=alert]").text()).toContain(
      "Arnod ne reconnaît pas /home/julien/vide",
    );
    expect(core.commands()).not.toContain("import_run");
  });
});
