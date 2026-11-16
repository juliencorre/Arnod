import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { INBOX, installFakeCore } from "../fakeCore";
import { boutiqueSnapshot, firstRunSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

/** Accueil toutes natures, catalogue des étapes et Non classés (docs/plan-accueil-non-classes.md). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const URL_PRODUCTS = "https://api.boutique.dev/products";

function tryInput(wrapper: AppWrapper) {
  return wrapper.get<HTMLInputElement>(".try__input");
}

async function type(wrapper: AppWrapper, text: string): Promise<void> {
  await tryInput(wrapper).setValue(text);
  await flushPromises();
}

function natureButton(wrapper: AppWrapper, name: string) {
  const found = wrapper.findAll(".try__chip").find((chip) => chip.text() === name);
  if (!found) throw new Error(`pastille ${name} absente`);
  return found;
}

describe("accueil (A1 à A5)", () => {
  it("A1 : bienvenue, entrée unique, natures du catalogue, créer et importer", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();

    expect(app.get("main h1").text()).toBe("Bienvenue dans Arnod");
    expect(app.get(".try__title").text()).toBe("Essayer une étape");
    expect(app.get(".try__nature").text()).toBe("Auto");
    expect(tryInput(app).attributes("placeholder")).toBe(
      "Collez ce que vous voulez essayer, par exemple https://…",
    );
    expect(app.findAll(".try__chip").map((chip) => chip.text())).toEqual(["REST"]);
    expect(app.get("main").text()).toContain("Parcourir le catalogue…");
    expect(app.get("main").text()).toContain(
      "Sans projet : l'essai est gardé dans « Non classés ».",
    );
    expect(app.find(".first-run__card").exists()).toBe(true);
    expect(app.find(".first-run__import").exists()).toBe(true);
    expect(app.find(".nature-form").exists()).toBe(false);
  });

  it("A2 : une adresse collée est reconnue REST ; le formulaire prend la place", async () => {
    const core = installFakeCore(firstRunSnapshot());
    app = await mountApp();

    await type(app, URL_PRODUCTS);

    expect(core.calls.find((c) => c.cmd === "steps_detect")?.args).toEqual({ input: URL_PRODUCTS });
    expect(app.get(".try__nature").text()).toBe("REST");
    expect(app.get('.try [aria-live="polite"]').text()).toBe("Reconnu : REST (Une requête HTTP).");
    const method = app.get('[role="radiogroup"]');
    expect(method.get('[aria-checked="true"]').text()).toBe("GET");
    // Un GET n'a que ses en-têtes en plusieurs lignes, sans corps.
    expect(app.findAll("textarea")).toHaveLength(1);
    expect(app.find(".first-run__card").exists()).toBe(false);
    expect(app.get(".first-run__compact").text()).toContain("Pas encore de projet ?");
    expect(app.get('.try__row button[type="submit"]').text()).toBe("Envoyer");
  });

  it("A2 : la méthode en tête passe dans le formulaire", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();

    await type(app, `post ${URL_PRODUCTS}`);

    expect(tryInput(app).element.value).toBe(URL_PRODUCTS);
    expect(app.get('[role="radio"][aria-checked="true"]').text()).toBe("POST");
    expect(app.findAll(".nature-form__label").map((l) => l.text())).toEqual([
      "Méthode",
      "En-têtes",
      "Type du corps",
      "Corps",
    ]);
  });

  it("A3 : choisir la pastille ouvre le formulaire ; flèches dans la méthode", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    const chip = natureButton(app, "REST");

    (chip.element as HTMLButtonElement).focus();
    await chip.trigger("click");
    await flushPromises();

    expect(chip.attributes("aria-pressed")).toBe("true");
    expect(document.activeElement).toBe(tryInput(app).element);
    expect(tryInput(app).attributes("placeholder")).toBe("https://…");
    expect(app.get('.try [aria-live="polite"]').text()).toBe("Nature choisie : REST.");
    const checked = app.get('[role="radio"][aria-checked="true"]');
    expect(checked.attributes("tabindex")).toBe("0");
    await checked.trigger("keydown", { key: "ArrowRight" });
    await flushPromises();
    expect(app.get('[role="radio"][aria-checked="true"]').text()).toBe("POST");
    expect(app.find("textarea").exists()).toBe(true);
    await app.get('[role="radio"][aria-checked="true"]').trigger("keydown", { key: "ArrowLeft" });
    await flushPromises();
    expect(app.get('[role="radio"][aria-checked="true"]').text()).toBe("GET");
  });

  it("A4 : Échap vide l'entrée et rend les cartes", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await type(app, URL_PRODUCTS);

    await tryInput(app).trigger("keydown", { key: "Escape" });
    await flushPromises();

    expect(tryInput(app).element.value).toBe("");
    expect(app.find(".nature-form").exists()).toBe(false);
    expect(app.find(".first-run__card").exists()).toBe(true);
  });

  it("A5 : « Envoyer » garde l'essai dans « Non classés » et l'ouvre", async () => {
    const core = installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await type(app, URL_PRODUCTS);

    await app.get(".try__form").trigger("submit");
    await flushPromises();

    const sent = core.calls.find((c) => c.cmd === "trial_send")?.args;
    expect(sent).toMatchObject({ nature: "rest", subject: URL_PRODUCTS, values: {} });
    expect(typeof sent?.execId).toBe("string");
    expect(core.calls.find((c) => c.cmd === "layout_open_selection")?.args).toEqual({
      content: { kind: "request", projectId: INBOX, path: "api.boutique.dev/essai-1.yml" },
    });
    expect(app.get(".toast").text()).toContain(
      "✓ Essai gardé dans « Non classés », sous le contrat de son serveur.",
    );
    expect(app.get("aside").text()).toContain("Non classés");
    // Liée à l'opération du contrat déduit, sans version à montrer.
    expect(app.get(".editor__contract").text()).toBe("api.boutique.dev · GET /products");
    expect(app.find(".editor__version").exists()).toBe(false);
  });

  it("RC8 : l'essai paraît sous le contrat déduit, à son opération, hors du référentiel", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await type(app, URL_PRODUCTS);
    await app.get(".try__form").trigger("submit");
    await flushPromises();

    // La requête active est révélée : contrat et ressource dépliés, opération mise en évidence.
    const aside = app.get("aside");
    expect(
      aside.get('[aria-label="Replier le contrat api.boutique.dev"]').attributes("aria-expanded"),
    ).toBe("true");
    expect(aside.findAll(".crow__resource .crow__name").map((n) => n.text())).toEqual([
      "/products",
    ]);
    expect(aside.get(".crow--selected").attributes("class")).toContain("crow--operation");
    expect(aside.find('[aria-label="Ouvrir GET /products"]').exists()).toBe(true);
    expect(aside.findAll('[role="treeitem"]')).toHaveLength(0);
  });

  it("A5 : un envoi en échec garde l'accueil et dit pourquoi", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.failNext("trial_send", {
      kind: "network",
      message: "Impossible de joindre api.boutique.dev. Vérifiez l'adresse et la connexion.",
    });
    app = await mountApp();
    await type(app, URL_PRODUCTS);

    await app.get(".try__form").trigger("submit");
    await flushPromises();

    const alert = app.get(".try__error");
    expect(alert.attributes("role")).toBe("alert");
    expect(alert.text()).toBe(
      "Impossible de joindre api.boutique.dev. Vérifiez l'adresse et la connexion.",
    );
    expect(tryInput(app).attributes("aria-invalid")).toBe("true");
    expect(app.get("main h1").text()).toBe("Bienvenue dans Arnod");
    expect(core.snapshot().projects).toEqual([]);
  });

  it("A5 : une saisie non reconnue n'envoie rien", async () => {
    const core = installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await type(app, "api.boutique.dev/products");

    await app.get(".try__form").trigger("submit");
    await flushPromises();

    expect(app.get(".try__error").text()).toBe(
      "Arnod ne reconnaît pas cette saisie : collez une adresse http(s)://, ou choisissez une nature.",
    );
    expect(core.commands()).not.toContain("trial_send");
  });
});

describe("catalogue des étapes (K1, K2)", () => {
  it("liste les natures, montre la fiche et revient", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();

    const browse = app.findAll("button").find((b) => b.text() === "Parcourir le catalogue…");
    await browse?.trigger("click");
    await flushPromises();

    expect(app.get("main h1").text()).toBe("Catalogue des étapes");
    expect(app.get(".catalog__source").text()).toBe(
      "arnod-steps · 2026-11-10 · définitions intégrées",
    );
    const cells = app.findAll("tbody tr:first-child > *").map((cell) => cell.text());
    expect(cells).toEqual([
      "REST Une requête HTTP",
      "Appels",
      "intégré",
      "Arnod",
      "1.0.0",
      "✓ Intégrée",
    ]);
    const detail = app.get(".detail");
    expect(detail.get("h2").text()).toBe("REST");
    expect(detail.text()).toContain("Méthode");
    expect(detail.text()).toContain("GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS");
    expect(detail.text()).toContain(
      "Une adresse http:// ou https://, éventuellement précédée de la méthode (POST https://…).",
    );
    const name = app.get(".catalog__name");
    expect(name.attributes("aria-pressed")).toBe("true");

    const back = app.findAll("button").find((b) => b.text() === "← Retour");
    await back?.trigger("click");
    await flushPromises();
    expect(app.get("main h1").text()).toBe("Bienvenue dans Arnod");
  });
});

describe("Non classés dans « Tous les projets » (N2, N3)", () => {
  it("en tête, avec son icône, hors workspaces ; l'entrée d'essai reste là", async () => {
    const snapshot = boutiqueSnapshot();
    snapshot.lastWorkspaceId = null;
    snapshot.inboxProjectId = INBOX;
    snapshot.projects.unshift({
      id: INBOX,
      name: "Non classés",
      path: "/home/julien/arnod/Non classés",
      status: "ok",
      workspaceIds: [],
    });
    installFakeCore(snapshot);
    app = await mountApp();

    const first = app.get("main tbody tr");
    expect(first.get(".all__project").text()).toBe("Non classés");
    expect(first.find("th svg").exists()).toBe(true);
    expect(first.attributes("draggable")).toBe("false");
    expect(first.text()).toContain("Hors workspaces");
    expect(first.text()).not.toContain("Ajouter à…");
    expect(first.text()).toContain("Oublier");
    expect(app.get(".all__try").text()).toContain("L'essai est gardé dans « Non classés ».");
  });
});
