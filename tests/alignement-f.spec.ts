/**
 * Alignement sur la maquette, lot F (docs/plan-alignement-maquette.md, T15 à T17) : import.
 * Chaque test est la preuve d'un écart `aligner: code` de docs/maquette-etat.yml.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { type FakeCore, installFakeCore } from "./fakeCore";
import { brunoReport, brunoSource } from "./fakeImports";
import { newRequest } from "./fakeWorkbench";
import { BOUTIQUE, BOUTIQUE_TESTS, boutiqueSnapshot } from "./fixtures";
import { type AppWrapper, mountApp } from "./mountApp";
import { openProject } from "./workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const BRUNO = "/home/julien/bruno/boutique-api";

async function clickButton(wrapper: AppWrapper, selector: string, text: string): Promise<void> {
  const found = wrapper.findAll(`${selector} button`).find((b) => b.text().trim() === text);
  if (!found) throw new Error(`bouton « ${text} » introuvable`);
  await found.trigger("click");
  await flushPromises();
}

function text(wrapper: AppWrapper, selector: string): string {
  return wrapper.get(selector).text().replace(/\s+/g, " ");
}

describe("« Importer… » d'« Ajouter un projet » par l'import unique (AM40, 1.1-c)", () => {
  it("reconnaît une collection Bruno et la range dans le workspace", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.imports.sources[BRUNO] = brunoSource(BRUNO);
    core.imports.picked = BRUNO;
    app = await mountApp();
    await app.get("main .add-card").trigger("click");
    await flushPromises();

    const choice = app
      .findAll("dialog[open] .choice")
      .find((c) => c.find(".choice__title").text() === "Importer…");
    expect(choice?.get(".choice__text").text().replace(/\s+/g, " ")).toBe(
      "Un projet Arnod, une collection Bruno, un fichier OpenAPI : Arnod reconnaît le format.",
    );
    await choice?.trigger("click");
    await flushPromises();

    // La fenêtre « Importer » (2.3-a) remplace l'ajout d'un seul dossier de projet Arnod.
    expect(app.get("dialog[open] h2").text()).toBe("Importer");
    expect(core.commands()).not.toContain("project_pick_folder");
    await clickButton(app, "dialog[open]", "Choisir un dossier…");
    expect(text(app, "dialog[open]")).toContain(
      "Nouveau projet « Boutique API », dans « Tous les projets » et « Boutique ».",
    );
    await clickButton(app, "dialog[open]", "Importer");

    expect(core.calls.find((c) => c.cmd === "import_run")?.args).toMatchObject({
      path: BRUNO,
      target: { kind: "newProject", location: null, workspaceId: BOUTIQUE },
    });
  });
});

describe("environnements et secrets à l'import (AM41, 2.3-b)", () => {
  /** Projet ouvert avec « Local » et « Recette fonctionnelle », puis « Importer » sur Bruno. */
  async function openImport(secrets: string[]): Promise<{ app: AppWrapper; core: FakeCore }> {
    const opened = await openProject((c) => {
      c.workbench.envs.push({
        name: "Recette fonctionnelle",
        color: "orange",
        protected: false,
        variables: [{ name: "baseUrl", value: "https://recette.boutique.dev" }],
        locals: ["token"],
      });
      c.imports.sources[BRUNO] = { ...brunoSource(BRUNO), secrets };
    });
    const store = (await import("@/stores/imports")).useImportStore();
    store.open(BOUTIQUE_TESTS, BOUTIQUE, opened.core.imports.sources[BRUNO] ?? null);
    await flushPromises();
    return opened;
  }

  it("fait correspondre chaque environnement et dit combien de variables il ajoute", async () => {
    const opened = await openImport([]);
    app = opened.app;
    const rows = app.findAll("dialog[open] .import__env");
    expect(rows.map((r) => r.get(".import__env-from").text())).toEqual(["Local", "Recette"]);
    const selects = app.findAll<HTMLSelectElement>("dialog[open] .import__env-select");
    // Même nom : l'environnement du projet ; sinon un nouvel environnement.
    expect(selects.map((s) => s.element.value)).toEqual(["Local", "Recette"]);
    expect(selects[1]?.findAll("option").map((o) => o.text())).toEqual([
      "Local",
      "Recette fonctionnelle",
      "Nouvel environnement « Recette »",
    ]);
    expect(selects[1]?.attributes("aria-label")).toBe("Environnement du projet qui reçoit Recette");
    expect(rows.map((r) => r.get(".import__hint").text())).toEqual([
      "0 variable ajoutée",
      "4 variables ajoutées",
    ]);

    // « Recette » vers « Recette fonctionnelle » : baseUrl et token y sont déjà.
    await selects[1]?.setValue("Recette fonctionnelle");
    expect(app.findAll("dialog[open] .import__env")[1]?.get(".import__hint").text()).toBe(
      "2 variables ajoutées",
    );
    await clickButton(app, "dialog[open]", "Importer");
    expect(opened.core.calls.find((c) => c.cmd === "import_run")?.args).toMatchObject({
      options: {
        environments: [
          { from: "Local", to: "Local" },
          { from: "Recette", to: "Recette fonctionnelle" },
        ],
        secretsLocal: true,
      },
    });
  });

  it("« N valeurs ressemblent à des secrets », case cochée par défaut", async () => {
    const opened = await openImport(["apiToken", "password"]);
    app = opened.app;
    const check = app.get<HTMLInputElement>("dialog[open] .import__check input[type=checkbox]");
    expect(check.element.checked).toBe(true);
    const label = app.get("dialog[open] .import__check");
    expect(label.get("span > span:first-child").text()).toBe("2 valeurs ressemblent à des secrets");
    expect(label.get(".import__hint").text()).toBe(
      "apiToken, password : elles iront en variables locales, au coffre du système, jamais partagées.",
    );

    await check.setValue(false);
    expect(text(app, "dialog[open] .import__check")).toContain(
      "elles resteront partagées, dans les fichiers du projet.",
    );
    await clickButton(app, "dialog[open]", "Importer");
    expect(opened.core.calls.find((c) => c.cmd === "import_run")?.args).toMatchObject({
      options: { secretsLocal: false },
    });
  });

  it("un nouveau projet garde les noms : pas de liste, seulement le nombre", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.imports.sources[BRUNO] = brunoSource(BRUNO);
    app = await mountApp();
    const store = (await import("@/stores/imports")).useImportStore();
    store.open(null, BOUTIQUE, brunoSource(BRUNO));
    await flushPromises();

    expect(app.find("dialog[open] .import__env-select").exists()).toBe(false);
    const rows = app.findAll("dialog[open] .import__env");
    expect(rows.map((r) => r.findAll("code, span").map((part) => part.text()))).toEqual([
      ["Local", "→", "nouvel environnement", "1 variable ajoutée"],
      ["Recette", "→", "nouvel environnement", "4 variables ajoutées"],
    ]);
  });
});

describe("scripts Bruno remplacés ou acceptés (AM42, 2.3-f)", () => {
  const CREATE = "Commandes/creer-une-commande.yml";

  /** Projet importé : la requête d'origine du script, le rapport ouvert dans son onglet. */
  async function openReport(): Promise<{ app: AppWrapper; core: FakeCore }> {
    const opened = await openProject((c) => {
      c.workbench.folders.add("Commandes");
      c.workbench.requests.set(CREATE, newRequest("Créer une commande", { method: "POST" }));
      c.imports.reports[BOUTIQUE_TESTS] = brunoReport(BOUTIQUE_TESTS, "boutique-tests");
    });
    const { useImportStore } = await import("@/stores/imports");
    await useImportStore().openReport(BOUTIQUE_TESTS);
    await flushPromises();
    return opened;
  }

  function actions(wrapper: AppWrapper): string[] {
    return wrapper.findAll(".report__actions button").map((b) => b.text());
  }

  it("« Accepter sans ce script » le marque repris, « Annuler » le remet", async () => {
    const opened = await openReport();
    app = opened.app;
    expect(actions(app)).toEqual(["Remplacer par une assertion…", "Accepter sans ce script"]);

    await clickButton(app, ".report__actions", "Accepter sans ce script");
    expect(opened.core.calls.find((c) => c.cmd === "import_script")?.args).toEqual({
      projectId: BOUTIQUE_TESTS,
      index: 0,
      choice: "accept",
    });
    expect(app.findAll(".report__item")[0]?.get(".report__status").text()).toBe("repris");
    expect(app.get(".report__after-box").text()).toContain("Repris sans ce script.");
    expect(app.find(".report__actions").exists()).toBe(false);
    const toast = app.get(".toast");
    expect(toast.text()).toContain("Script retiré du fichier : la requête s'exécute sans lui.");

    const undo = toast.findAll("button").find((b) => b.text() === "Annuler");
    await undo?.trigger("click");
    await flushPromises();
    expect(opened.core.commands()).toContain("import_script_undo");
    expect(app.findAll(".report__item")[0]?.get(".report__status").text()).toBe("non repris");
  });

  it("« Remplacer par une assertion… » ouvre la requête sur une assertion ajoutée", async () => {
    const opened = await openReport();
    app = opened.app;
    await clickButton(app, ".report__actions", "Remplacer par une assertion…");

    expect(opened.core.calls.find((c) => c.cmd === "import_script")?.args).toMatchObject({
      choice: "replace",
    });
    expect(opened.core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual(
      { content: { kind: "request", projectId: BOUTIQUE_TESTS, path: CREATE } },
    );
    const selected = app.get(".editor__tab[aria-selected='true']");
    expect(selected.text()).toContain("Assertions");
    // La requête avait déjà une assertion : la nouvelle s'ajoute après, sa cible prend le focus.
    expect(app.findAll(".assertions .assertion")).toHaveLength(2);
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Cible de l'assertion 2");
    expect(app.get(".toast").text()).toContain(
      "Script retiré du fichier : complétez l'assertion qui le remplace.",
    );
  });

  it("un script de dossier n'a pas de requête : seulement « Accepter sans ce script »", async () => {
    const opened = await openReport();
    app = opened.app;
    await app.findAll(".report__item")[1]?.trigger("click");
    await flushPromises();
    expect(actions(app)).toEqual(["Accepter sans ce script"]);
    // Une variable à renseigner n'est pas un script : aucune action.
    await app.findAll(".report__item")[3]?.trigger("click");
    await flushPromises();
    expect(app.find(".report__actions").exists()).toBe(false);
  });

  it("le menu ⋯ du projet rouvre le rapport d'import", async () => {
    const opened = await openProject((c) => {
      c.imports.reports[BOUTIQUE_TESTS] = brunoReport(BOUTIQUE_TESTS, "boutique-tests");
    });
    app = opened.app;
    const more = app
      .findAll("button")
      .find((b) => b.attributes("aria-label") === "Plus d'actions sur boutique-tests");
    await more?.trigger("click");
    await flushPromises();
    const entry = app
      .findAll("[role='menuitem']")
      .find((item) => item.text().includes("Rapport d'import"));
    expect(entry).toBeDefined();
    await entry?.trigger("click");
    await flushPromises();
    expect(opened.core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual(
      { content: { kind: "importReport", projectId: BOUTIQUE_TESTS } },
    );
  });
});
