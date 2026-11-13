import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { type FakeCore, installFakeCore } from "../fakeCore";
import { brunoReport, brunoSource } from "../fakeImports";
import { BOUTIQUE, BOUTIQUE_TESTS, boutiqueSnapshot, firstRunSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { chooseWorkspace } from "../switcher";
import { openProject } from "../workbench/helpers";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const BRUNO = "/home/julien/bruno/boutique-api";

async function clickButton(app: AppWrapper, selector: string, text: string): Promise<void> {
  const found = app.findAll(`${selector} button`).find((b) => b.text().trim() === text);
  if (!found) throw new Error(`bouton « ${text} » introuvable`);
  await found.trigger("click");
  await flushPromises();
}

function dialog(app: AppWrapper) {
  return app.get("dialog[open]");
}

function withBruno(core: FakeCore): void {
  core.imports.sources[BRUNO] = brunoSource(BRUNO);
  core.imports.picked = BRUNO;
}

describe("importer (2.3-a)", () => {
  it("au premier lancement, une collection Bruno crée le projet et ouvre son rapport", async () => {
    const core = installFakeCore(firstRunSnapshot());
    withBruno(core);
    wrapper = await mountApp();

    await clickButton(wrapper, ".first-run__import", "Choisir un dossier…");

    expect(core.calls.find((c) => c.cmd === "import_pick")?.args).toEqual({ file: false });
    const open = dialog(wrapper);
    expect(open.text()).toContain("boutique-api");
    expect(open.text()).toContain(
      "✓ Collection Bruno · 6 requêtes dans 2 dossiers · 2 environnements",
    );
    expect(open.get(".import__folder code").text()).toBe("/home/julien/arnod/Boutique API");
    expect(open.text()).toContain("3 variables secrètes");
    expect(open.text()).toContain("3 scripts gardés dans les fichiers, pas exécutés.");
    expect(core.commands()).not.toContain("import_run");

    await clickButton(wrapper, "dialog[open]", "Importer");

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toEqual({
      path: BRUNO,
      target: { kind: "newProject", location: null, workspaceId: null },
    });
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(core.calls.find((c) => c.cmd === "layout_open_selection")?.args).toEqual({
      content: { kind: "importReport", projectId: "p-Boutique API" },
    });
    const report = wrapper.get(".report");
    expect(report.get("h1").text()).toContain("Boutique API importée");
    expect(report.get("h1").text()).toContain("collection Bruno · 12 fichiers .bru");
    expect(report.findAll(".report__stat").map((s) => s.text())).toEqual([
      "2dossiers",
      "6requêtes",
      "2environnements",
      "10vérifications sans code",
      "2extractions",
      "3secrets en variables locales",
    ]);
    expect(wrapper.get(".toast").text()).toContain("« Boutique API » créé");
  });

  it("« Changer » choisit l'emplacement du projet créé", async () => {
    const core = installFakeCore(firstRunSnapshot());
    withBruno(core);
    wrapper = await mountApp();
    await clickButton(wrapper, ".first-run__import", "Choisir un dossier…");
    core.pickedFolder = "/home/julien/projets";

    await clickButton(wrapper, ".import__folder", "Changer");
    expect(dialog(wrapper).get(".import__folder code").text()).toBe(
      "/home/julien/projets/Boutique API",
    );
    await clickButton(wrapper, "dialog[open]", "Importer");

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toMatchObject({
      target: { kind: "newProject", location: "/home/julien/projets" },
    });
  });

  it("un projet Arnod est ajouté tel quel, sans rapport", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    const path = "/home/julien/dev/partenaires-v2";
    core.imports.sources[path] = {
      path,
      format: "openCollection",
      name: "partenaires-v2",
      counts: { ...brunoSource().counts, requests: 0 },
      files: 0,
    };
    core.imports.picked = path;
    wrapper = await mountApp();
    await chooseWorkspace(wrapper, "Tous les projets");

    await clickButton(wrapper, "main", "Importer…");
    await clickButton(wrapper, "dialog[open]", "Choisir un dossier…");
    expect(dialog(wrapper).text()).toContain("Ajouté à « Tous les projets », sans conversion.");
    expect(dialog(wrapper).find(".import__folder").exists()).toBe(false);
    await clickButton(wrapper, "dialog[open]", "Importer");

    expect(core.snapshot().projects.some((p) => p.name === "partenaires-v2")).toBe(true);
    expect(core.commands()).not.toContain("layout_open_selection");
    expect(wrapper.get(".toast").text()).toBe("« partenaires-v2 » ajouté à « Tous les projets ».");
  });

  it("un format inconnu est expliqué dans la fenêtre", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.imports.picked = "/home/julien/notes.txt";
    wrapper = await mountApp();
    await chooseWorkspace(wrapper, "Tous les projets");
    await clickButton(wrapper, "main", "Importer…");

    await clickButton(wrapper, "dialog[open]", "Choisir un fichier…");

    expect(core.calls.find((c) => c.cmd === "import_pick")?.args).toEqual({ file: true });
    expect(dialog(wrapper).get("[role=alert]").text()).toContain(
      "Arnod ne reconnaît pas /home/julien/notes.txt",
    );
    const importer = dialog(wrapper)
      .findAll("button")
      .find((b) => b.text() === "Importer");
    expect(importer?.attributes("disabled")).toBeDefined();
  });

  it("rien n'est fait si le sélecteur est annulé, Échap ferme la fenêtre", async () => {
    const core = installFakeCore(firstRunSnapshot());
    wrapper = await mountApp();
    await clickButton(wrapper, ".first-run__import", "Choisir un dossier…");
    expect(wrapper.find("dialog[open]").exists()).toBe(false);

    await clickButton(wrapper, ".first-run__compact, .first-run__import", "ou un fichier…");
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    core.imports.picked = null;

    const store = (await import("@/stores/imports")).useImportStore();
    store.open(null, null, brunoSource(BRUNO));
    await flushPromises();
    await wrapper.get("dialog[open]").trigger("cancel");
    await flushPromises();
    expect(wrapper.find("dialog[open]").exists()).toBe(false);
    expect(core.commands()).not.toContain("import_run");
  });
});

describe("reprendre l'existant (2.3-b)", () => {
  it("dans un projet vide, importe la collection dans un nouveau dossier du projet", async () => {
    const { app, core } = await openProject((c) => {
      c.workbench.requests.clear();
      c.workbench.folders.clear();
      withBruno(c);
    });
    wrapper = app;
    const start = app.get(".start");
    expect(start.text()).toContain("Reprendre l'existant");

    await clickButton(app, ".start", "Importer…");
    await clickButton(app, "dialog[open]", "Choisir un dossier…");
    const radios = dialog(app).findAll<HTMLInputElement>("input[type=radio]");
    expect(radios.map((r) => r.element.checked)).toEqual([true, false]);
    expect(dialog(app).text()).toContain(
      "Un nouveau dossier « Boutique API » dans Référentiel de tests.",
    );
    await clickButton(app, "dialog[open]", "Importer");

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toEqual({
      path: BRUNO,
      target: { kind: "project", projectId: BOUTIQUE_TESTS },
    });
    expect(core.calls.filter((c) => c.cmd === "tree_load").length).toBeGreaterThan(1);
    expect(app.get(".report h1").text()).toContain("dans boutique-tests, dossier « Boutique API »");
  });

  it("« Nouveau projet » reste possible depuis un projet, dans son workspace", async () => {
    const { app, core } = await openProject((c) => {
      c.workbench.requests.clear();
      c.workbench.folders.clear();
      withBruno(c);
    });
    wrapper = app;
    await clickButton(app, ".start", "Importer…");
    await clickButton(app, "dialog[open]", "Choisir un dossier…");
    await dialog(app).findAll("input[type=radio]")[1]?.setValue(true);
    await clickButton(app, "dialog[open]", "Importer");

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toEqual({
      path: BRUNO,
      target: { kind: "newProject", location: null, workspaceId: BOUTIQUE },
    });
  });
});

describe("rapport d'import (2.3-f)", () => {
  async function openReport(): Promise<{ app: AppWrapper; core: FakeCore }> {
    const result = await openProject((c) => {
      c.imports.reports[BOUTIQUE_TESTS] = brunoReport(BOUTIQUE_TESTS, "boutique-tests");
    });
    const { useImportStore } = await import("@/stores/imports");
    await useImportStore().openReport(BOUTIQUE_TESTS);
    await flushPromises();
    return result;
  }

  it("relit le rapport gardé, liste ce qui est à reprendre et montre avant / après", async () => {
    const { app, core } = await openReport();
    wrapper = app;
    expect(core.calls.find((c) => c.cmd === "import_report")?.args).toEqual({
      projectId: BOUTIQUE_TESTS,
    });
    const rows = app.findAll(".report__item");
    expect(rows).toHaveLength(5);
    expect(rows[0]?.text()).toContain("TESTS");
    expect(rows[0]?.text()).toContain("Créer une commande");
    expect(rows[0]?.text()).toContain("non repris");
    expect(app.get(".report__code").text()).toContain('tests {\n  test("total cohérent"');
    expect(app.get(".report__after-box").text()).toContain("Gardé dans le fichier, pas exécuté.");
    expect(app.findAll(".report__example")).toHaveLength(4);
    expect(app.get(".zone__name").text()).toBe("Rapport d'import · Boutique API");

    await clickButton(app, ".report__main", "… 1 autre");
    expect(app.findAll(".report__item")).toHaveLength(6);
  });

  it("se parcourt au clavier et ouvre la requête d'origine", async () => {
    const { app, core } = await openReport();
    wrapper = app;
    const first = app.get(".report__item");
    expect(first.attributes("aria-pressed")).toBe("true");
    expect(first.attributes("tabindex")).toBe("0");

    await first.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const rows = app.findAll(".report__item");
    expect(rows[1]?.attributes("aria-pressed")).toBe("true");
    expect(document.activeElement).toBe(rows[1]?.element);
    expect(app.get(".report__detail h2").text()).toBe("réglages du dossier · Commandes");
    expect(app.find(".report__open").exists()).toBe(false);

    await rows[1]?.trigger("keydown", { key: "End" });
    await flushPromises();
    expect(app.get(".report__detail").text()).toContain("bloc auth:oauth2");
    await rows[4]?.trigger("keydown", { key: "Home" });
    await flushPromises();

    await clickButton(app, ".report__detail", "Ouvrir la requête");
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual({
      content: {
        kind: "request",
        projectId: BOUTIQUE_TESTS,
        path: "Commandes/creer-une-commande.yml",
      },
    });
  });

  it("sans rapport sur ce poste, le dit", async () => {
    const { app } = await openProject();
    wrapper = app;
    const { useImportStore } = await import("@/stores/imports");
    await useImportStore().openReport(BOUTIQUE_TESTS);
    await flushPromises();
    expect(app.get(".report__empty").text()).toBe(
      "Aucun rapport d'import pour ce projet sur ce poste.",
    );
  });
});
