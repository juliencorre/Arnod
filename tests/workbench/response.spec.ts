/**
 * Corps de la réponse (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis) : format, aperçu, copier,
 * enregistrer, effacer, filtre JSONPath et vérification tirée d'un résultat, clavier compris.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FILTER_DEBOUNCE_MS } from "@/stores/response";
import { AUTOSAVE_MS } from "@/stores/workbench";

import type { FakeCore } from "../fakeCore";
import { okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

const ORDER =
  '{"id":"ord_8F2kQ","items":[{"sku":"TSH-BLU-M","qty":2},{"sku":"MUG-NOIR","qty":1}],"total":47.82}';

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function respond(core: FakeCore, body: string, type: string, truncated = false): void {
  core.workbench.nextSend = okOutcome({
    response: {
      status: 201,
      statusText: "Created",
      headers: [["content-type", type]],
      body,
      truncated,
      sizeBytes: body.length,
      waitMs: 30,
      receiveMs: 2,
      totalMs: 32,
    },
  });
}

/** Ouvre « Créer » et l'envoie ; la réponse est `body` de type `type`. */
async function sent(
  body = ORDER,
  type = "application/json",
  truncated = false,
): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject((core) => {
    respond(core, body, type, truncated);
  });
  await click(opened.app, '[role="treeitem"]', "Créer");
  await opened.app.get(".editor__line").trigger("submit");
  await flushPromises();
  return opened;
}

async function filter(expression: string): Promise<void> {
  if (!app) throw new Error("application non montée");
  await app.get(".filter__input").setValue(expression);
  await new Promise((resolve) => setTimeout(resolve, FILTER_DEBOUNCE_MS + 30));
  await flushPromises();
}

describe("corps de la réponse (5.1-b bis)", () => {
  it("met en forme le JSON détecté, numéroté, et change de format (R1, R2)", async () => {
    let core;
    ({ app, core } = await sent());
    const select = app.get<HTMLSelectElement>('select[aria-label="Format du corps"]');
    expect(select.element.value).toBe("json");
    expect(app.findAll(".code__line").length).toBeGreaterThan(3);
    expect(app.get(".code").text()).toContain('"sku": "TSH-BLU-M"');

    await select.setValue("text");
    await flushPromises();
    const render = core.calls.filter((c) => c.cmd === "response_render").at(-1);
    expect(render?.args.format).toBe("text");
    expect(app.findAll(".code__line")).toHaveLength(1);
    // L'aperçu ne rend qu'un corps HTML.
    expect(app.get(".body__bar [role='switch']").attributes("disabled")).toBeDefined();
  });

  it("montre un HTML en aperçu, sans script (R4)", async () => {
    ({ app } = await sent(
      "<html><head><script>alert(1)</script></head><body><h1>Panne</h1><p>Réessayez.</p></body></html>",
      "text/html",
    ));
    expect(app.get<HTMLSelectElement>('select[aria-label="Format du corps"]').element.value).toBe(
      "html",
    );
    await app.get(".body__bar [role='switch']").trigger("click");
    await flushPromises();
    const preview = app.get(".preview");
    expect(preview.get("h1").text()).toBe("Panne");
    expect(preview.find("script").exists()).toBe(false);
    expect(preview.text()).not.toContain("alert");
  });

  it("filtre en JSONPath et surligne les valeurs ; Entrée passe au suivant (R5)", async () => {
    let core;
    ({ app, core } = await sent());
    await filter("$.items[*].sku");
    expect(core.calls.find((c) => c.cmd === "response_filter")?.args).toMatchObject({
      expression: "$.items[*].sku",
    });
    expect(app.get(".filter__count").text()).toBe("2 résultats");
    const marks = app.findAll(".tok--match");
    expect(marks.map((m) => m.text())).toEqual(['"TSH-BLU-M"', '"MUG-NOIR"']);
    expect(app.get(".tok--current").text()).toBe('"TSH-BLU-M"');
    expect(app.get(".filter__target").text()).toBe("sur body.items[0].sku");

    await app.get(".filter__input").trigger("keydown", { key: "Enter" });
    expect(app.get(".tok--current").text()).toBe('"MUG-NOIR"');
    expect(app.get(".filter__position").text()).toBe("2/2");
    await app.get(".filter__input").trigger("keydown", { key: "Enter", shiftKey: true });
    expect(app.get(".filter__position").text()).toBe("1/2");

    // Échap vide le filtre.
    await app.get(".filter__input").trigger("keydown", { key: "Escape" });
    await new Promise((resolve) => setTimeout(resolve, FILTER_DEBOUNCE_MS + 30));
    await flushPromises();
    expect(app.findAll(".tok--match")).toHaveLength(0);
  });

  it("explique une expression invalide en français (R7)", async () => {
    ({ app } = await sent());
    await filter("items[0]");
    expect(app.get(".filter [role='alert']").text()).toContain("commence par $");
    expect(app.get(".filter__input").attributes("aria-invalid")).toBe("true");
    await filter("$.items[0");
    expect(app.get(".filter [role='alert']").text()).toContain("Crochet « [ » non fermé");
  });

  it("un résultat devient une vérification de la requête (R6)", async () => {
    let core;
    ({ app, core } = await sent());
    await filter("$.items[*].sku");
    await app.get(".filter__input").trigger("keydown", { key: "Enter" });
    expect(app.get(".filter__hint").text()).toContain('body.items[1].sku égal à "MUG-NOIR"');
    await click(app, ".filter__actions button", "+ Vérification");
    expect(app.get(".toast").text()).toContain("Vérification ajoutée");

    await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
    await flushPromises();
    const saved = core.workbench.requests.get("Clients/creer.yml");
    expect(saved?.assertions.at(-1)).toMatchObject({
      target: "body.items[1].sku",
      operator: "equals",
      value: "MUG-NOIR",
    });

    // Une deuxième fois : déjà présente, rien n'est ajouté.
    const count = saved?.assertions.length;
    await click(app, ".filter__actions button", "+ Vérification");
    await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
    await flushPromises();
    expect(core.workbench.requests.get("Clients/creer.yml")?.assertions.length).toBe(count);
  });

  it("Ctrl+F ouvre le filtre depuis le corps", async () => {
    ({ app } = await sent());
    await app.get(".code").trigger("keydown", { key: "f", ctrlKey: true });
    expect(document.activeElement).toBe(app.get(".filter__input").element);
  });

  it("copie le corps et signale un corps tronqué (R10)", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    ({ app } = await sent(ORDER, "application/json", true));
    expect(app.get(".body").text()).toContain("Le corps dépasse 5 Mo");
    await click(app, 'button[aria-label="Copier le corps"]');
    expect(writeText).toHaveBeenCalledWith(ORDER);
    expect(document.body.textContent).toContain("Corps copié (tronqué à 5 Mo).");
  });

  it("enregistre le corps dans un fichier choisi par le cœur (R8)", async () => {
    let core;
    ({ app, core } = await sent());
    await click(app, 'button[aria-label="Enregistrer le corps dans un fichier…"]');
    expect(core.calls.find((c) => c.cmd === "response_save")?.args).toEqual({
      body: ORDER,
      name: "Créer",
      format: "json",
      truncated: false,
    });
    expect(document.body.textContent).toContain("Corps enregistré dans");

    // Annulé dans le sélecteur : rien n'est signalé.
    core.response.savePath = null;
    await click(app, 'button[aria-label="Enregistrer le corps dans un fichier…"]');
    expect(core.calls.filter((c) => c.cmd === "response_save")).toHaveLength(2);
  });

  it("efface la réponse affichée (R9)", async () => {
    let core;
    ({ app, core } = await sent());
    await click(app, 'button[aria-label="Effacer la réponse"]');
    expect(core.calls.find((c) => c.cmd === "response_clear")?.args).toMatchObject({
      path: "Clients/creer.yml",
    });
    expect(app.get(".response").text()).toContain("Envoyez la requête pour voir la réponse.");
    expect(app.find(".body").exists()).toBe(false);
  });

  it("les actions du corps se jouent au clavier", async () => {
    ({ app } = await sent());
    const tools = app.findAll(".body__tool");
    expect(tools.map((t) => t.attributes("aria-label"))).toEqual([
      "Copier le corps",
      "Enregistrer le corps dans un fichier…",
      "Effacer la réponse",
    ]);
    for (const tool of tools) expect(tool.element.tagName).toBe("BUTTON");
    expect(app.get(".code").attributes("tabindex")).toBe("0");
  });
});
