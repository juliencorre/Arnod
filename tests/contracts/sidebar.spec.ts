/**
 * Requêtes rangées dans leur contrat, dans la barre latérale
 * (docs/plan-requetes-dans-contrats.md, RC1 à RC9 ; règles R1 à R6 testées dans le cœur).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { EVENTS } from "@/lib/ipc";
import type { HttpMethod, ProjectChange } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "../fakeContracts";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const PETSTORE = "contracts/petstore.yaml";

function linked(name: string, operation: string, method: HttpMethod = "GET") {
  return newRequest(name, { method, contract: { file: PETSTORE, operation } });
}

/**
 * « boutique-tests » avec le contrat Petstore : deux requêtes sur `GET /pets` (l'endpoint, puis un
 * essai), aucune sur `POST /pets` ni `GET /health`, une dont l'opération a disparu ; et les trois
 * requêtes sans contrat du projet de test.
 */
async function setup(): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject((core) => {
    core.contracts.files.set(PETSTORE, {
      name: "Petstore",
      version: "1.0.0",
      source: PETSTORE_SOURCE,
      operations: structuredClone(PETSTORE_OPERATIONS),
    });
    const wb = core.workbench;
    wb.folders.add("Petstore");
    wb.folders.add("Petstore/animaux");
    wb.requests.set("Petstore/animaux/a-lister.yml", linked("Lister les animaux", "GET /pets"));
    wb.requests.set("Petstore/animaux/b-page-2.yml", linked("Page 2", "GET /pets"));
    wb.requests.set("Petstore/ancienne.yml", linked("Ancienne", "DELETE /pets/{id}", "DELETE"));
  });
  app = opened.app;
  return opened;
}

function wrapper(): AppWrapper {
  if (!app) throw new Error("application non montée");
  return app;
}

/** Libellés de l'arborescence du référentiel de tests. */
function testItems(): string[] {
  return wrapper()
    .findAll('[role="treeitem"]')
    .map((i) => i.text());
}

function texts(selector: string): string[] {
  return wrapper()
    .findAll(selector)
    .map((e) => e.text());
}

async function press(selector: string): Promise<void> {
  await wrapper().get(selector).trigger("click");
  await flushPromises();
}

/** Déplie le contrat Petstore et sa ressource `/pets`. */
async function expandPets(): Promise<void> {
  await press('[aria-label="Déplier le contrat Petstore"]');
  await click(wrapper(), ".crow__resource", "/pets");
}

function lastOpened(core: FakeCore): unknown {
  return core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args;
}

function change(scope: ProjectChange["scope"]): ProjectChange {
  return { projectId: BOUTIQUE_TESTS, scope, path: null, origin: "fenetre-autre" };
}

describe("contrats et référentiel de tests (RC1, RC6)", () => {
  it("le référentiel ne garde que les requêtes sans contrat (R1, R5)", async () => {
    await setup();
    const heads = wrapper().findAll(".section-head");
    expect(heads.map((h) => h.get("h3").text())).toEqual([
      "Contrats de service",
      "Référentiel de tests",
    ]);
    // Le nombre est montré section repliée (lot D).
    expect(heads[1]?.find(".section-head__count").exists()).toBe(false);
    await heads[1]?.get(".section-head__toggle").trigger("click");
    await flushPromises();
    expect(wrapper().findAll(".section-head")[1]?.get(".section-head__count").text()).toBe("3");
    await wrapper().findAll(".section-head__toggle")[1]?.trigger("click");
    await flushPromises();
    expect(testItems().some((t) => t.includes("Clients"))).toBe(true);
    expect(testItems().some((t) => t.includes("Santé"))).toBe(true);
    // Le dossier « Petstore » n'avait que des requêtes liées : il n'y figure pas.
    expect(testItems().some((t) => t.includes("Petstore"))).toBe(false);
  });

  it("un contrat se déplie en ressources, opérations et requêtes ; replié par défaut (R2, R3, R4)", async () => {
    await setup();
    const chevron = wrapper().get('[aria-label="Déplier le contrat Petstore"]');
    expect(chevron.attributes("aria-expanded")).toBe("false");
    expect(wrapper().find(".crow").exists()).toBe(false);

    await chevron.trigger("click");
    await flushPromises();
    expect(
      wrapper().get('[aria-label="Replier le contrat Petstore"]').attributes("aria-expanded"),
    ).toBe("true");
    // Ressources dans l'ordre du contrat, avec leur nombre d'opérations ; puis « Hors contrat ».
    expect(texts(".crow__resource .crow__name")).toEqual(["/pets", "/health", "Hors contrat"]);
    expect(texts(".crow__resource .crow__count")).toEqual(["2", "1", "1"]);

    await click(wrapper(), ".crow__resource", "/pets");
    // La ligne de l'opération est son endpoint (lot D, Q4) : sans réponse attendue, pas de
    // nombre ; celle sans requête est estompée.
    const get = wrapper().get('[aria-label="Ouvrir GET /pets"]');
    expect(get.find(".crow__examples").exists()).toBe(false);
    const post = wrapper().get('[aria-label="Créer la requête de POST /pets"]');
    expect(post.classes()).toContain("crow__main--empty");
    expect(post.get(".crow__hint").text()).toBe("Créer");

    // Dépliée, l'opération liste les autres requêtes : l'endpoint est la ligne elle-même.
    await press('[aria-label="Déplier GET /pets"]');
    expect(texts(".linked__name")).toEqual(["Page 2"]);

    await click(wrapper(), ".crow__resource", "Hors contrat");
    expect(texts(".linked__method")).toEqual(["DELETE"]);
    expect(texts(".linked__name")).toContain("Ancienne");
  });
});

describe("ouvrir et créer depuis une opération (RC2 à RC4)", () => {
  it("un clic sur une opération ouvre son endpoint, mis en évidence (RC2)", async () => {
    const { core } = await setup();
    await expandPets();
    await press('[aria-label="Ouvrir GET /pets"]');
    expect(lastOpened(core)).toEqual({
      content: {
        kind: "request",
        projectId: BOUTIQUE_TESTS,
        path: "Petstore/animaux/a-lister.yml",
      },
    });
    // L'opération a deux requêtes : elle se déplie ; sa ligne, l'endpoint, est sélectionnée.
    expect(wrapper().get('[aria-label="Replier GET /pets"]').attributes("aria-expanded")).toBe(
      "true",
    );
    expect(wrapper().get(".crow--selected").text()).toContain("GET");

    // Repliée, l'opération porte la sélection ; replié, le contrat aussi.
    await press('[aria-label="Replier GET /pets"]');
    expect(wrapper().get(".crow--selected").text()).toContain("GET");
    await press('[aria-label="Replier le contrat Petstore"]');
    expect(wrapper().get(".contract-row--selected").text()).toContain("Petstore");
  });

  it("une opération sans requête crée la sienne, l'ouvre et la range sous le contrat (RC3)", async () => {
    const { core } = await setup();
    await expandPets();
    await press('[aria-label="Créer la requête de POST /pets"]');

    expect(core.calls.find((c) => c.cmd === "contract_generate")?.args).toEqual({
      project: BOUTIQUE_TESTS,
      path: PETSTORE,
      operations: ["POST /pets"],
      environment: "Local",
    });
    expect(lastOpened(core)).toEqual({
      content: {
        kind: "request",
        projectId: BOUTIQUE_TESTS,
        path: "Petstore/animaux/creer-un-animal.yml",
      },
    });
    expect(wrapper().find('[aria-label="Ouvrir POST /pets"]').exists()).toBe(true);
    expect(testItems().some((t) => t.includes("Petstore"))).toBe(false);
  });

  it("une requête ouverte depuis l'onglet du contrat est révélée dans la barre latérale (RC4)", async () => {
    await setup();
    await click(wrapper(), ".contract-row__main", "Petstore");
    expect(
      wrapper().get('[aria-label="Déplier le contrat Petstore"]').attributes("aria-expanded"),
    ).toBe("false");

    await press('[aria-label="Ouvrir la requête de GET /pets"]');
    expect(
      wrapper().get('[aria-label="Replier le contrat Petstore"]').attributes("aria-expanded"),
    ).toBe("true");
    expect(wrapper().get('[aria-label="Replier GET /pets"]').attributes("aria-expanded")).toBe(
      "true",
    );
    expect(wrapper().get(".crow--selected").text()).toContain("/pets");
  });
});

describe("agir sur une requête liée (RC5, R6)", () => {
  async function action(name: string, item: string): Promise<void> {
    const trigger = wrapper().get(`[aria-label="Actions pour ${name}"]`);
    await trigger.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const labels = texts('[role="menuitem"]');
    expect(labels).toEqual(["Renommer", "Dupliquer", "Mettre à la corbeille"]);
    // Flèche vers le bas jusqu'à l'entrée, puis Entrée.
    const index = labels.indexOf(item);
    for (let i = 0; i < index; i++) {
      await wrapper().findAll('[role="menuitem"]')[i]?.trigger("keydown", { key: "ArrowDown" });
      await flushPromises();
    }
    await wrapper().findAll('[role="menuitem"]')[index]?.trigger("keydown", { key: "Enter" });
    await flushPromises();
  }

  it("renomme, duplique et met à la corbeille, au clavier (RC5)", async () => {
    const { core } = await setup();
    await expandPets();
    await press('[aria-label="Déplier GET /pets"]');

    await action("Page 2", "Renommer");
    expect(wrapper().get("dialog[open] h2").text()).toBe("Renommer");
    await wrapper().get("dialog[open] input").setValue("Deuxième page");
    await wrapper().get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_rename")?.args).toMatchObject({
      path: "Petstore/animaux/b-page-2.yml",
      name: "Deuxième page",
    });
    expect(texts(".linked__name")).toEqual(["Deuxième page"]);

    // Dupliquer l'endpoint, depuis le ⋯ de la ligne de l'opération : un essai de plus, ouvert.
    await action("Lister les animaux", "Dupliquer");
    expect(core.calls.find((c) => c.cmd === "tree_duplicate")?.args).toMatchObject({
      path: "Petstore/animaux/a-lister.yml",
    });
    expect(texts(".linked__name")).toHaveLength(2);
    // La copie est ouverte (le faux cœur la range par chemin, avant l'original).
    const opened = lastOpened(core) as { content: { path: string } };
    expect(opened.content.path).not.toBe("Petstore/animaux/a-lister.yml");

    await action("Deuxième page", "Mettre à la corbeille");
    expect(core.calls.find((c) => c.cmd === "tree_delete")?.args).toMatchObject({
      path: "Petstore/animaux/b-page-2.yml",
      permanent: false,
    });
    expect(texts(".linked__name")).not.toContain("Deuxième page");
    expect(wrapper().get(".toast").text()).toContain("« Deuxième page » mis à la corbeille.");
  });

  it("un contrat mis à la corbeille rend ses requêtes au référentiel (R6)", async () => {
    await setup();
    await press('[aria-label="Actions pour le contrat Petstore"]');
    await click(wrapper(), '[role="menuitem"]', "Mettre à la corbeille");
    expect(wrapper().find(".contract-row").exists()).toBe(false);
    expect(testItems().some((t) => t.includes("Petstore"))).toBe(true);
  });
});

describe("entre fenêtres (RC9)", () => {
  it("une requête liée créée ailleurs paraît sous son contrat", async () => {
    const { core } = await setup();
    await press('[aria-label="Déplier le contrat Petstore"]');
    await click(wrapper(), ".crow__resource", "/health");
    expect(wrapper().find('[aria-label="Créer la requête de GET /health"]').exists()).toBe(true);

    core.workbench.requests.set("Petstore/sante.yml", linked("Santé du service", "GET /health"));
    await core.emit(EVENTS.project, change("tree"));
    await flushPromises();
    expect(wrapper().find('[aria-label="Ouvrir GET /health"]').exists()).toBe(true);
    expect(testItems().some((t) => t.includes("Petstore"))).toBe(false);
  });

  it("un contrat supprimé ailleurs rend ses requêtes au référentiel", async () => {
    const { core } = await setup();
    core.contracts.files.delete(PETSTORE);
    await core.emit(EVENTS.project, change("contracts"));
    await flushPromises();
    expect(wrapper().find(".contract-row").exists()).toBe(false);
    expect(testItems().some((t) => t.includes("Petstore"))).toBe(true);
  });
});

describe("statuts des requêtes liées (ST1, ST2)", () => {
  it("montre ✓ / ✕ sur les requêtes et « réussies / exécutées » sur la ressource", async () => {
    const { core } = await setup();
    await core.emit(EVENTS.statuses, {
      projectId: BOUTIQUE_TESTS,
      statuses: {
        "Petstore/animaux/a-lister.yml": { passed: true, at: 1 },
        "Petstore/animaux/b-page-2.yml": { passed: false, at: 1 },
      },
    });
    await flushPromises();
    await expandPets();
    const pets = wrapper()
      .findAll(".crow__resource")
      .find((r) => r.text().includes("/pets"));
    expect(pets?.get(".crow__count [aria-hidden='true']").text()).toBe("1/2");
    // La ligne de l'opération porte le statut de son endpoint (lot D, Q4).
    expect(wrapper().get('[aria-label="Ouvrir GET /pets"]').get(".crow__mark").text()).toContain(
      "✓",
    );
    await press('[aria-label="Déplier GET /pets"]');
    const marks = wrapper()
      .findAll(".linked")
      .map((l) => l.find(".linked__status").text());
    expect(marks).toEqual(["✕, en échec"]);
  });
});
