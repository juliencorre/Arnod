/**
 * Alignement sur la maquette, lot C (docs/plan-alignement-maquette.md, T5 à T9) : réponse, envoyé
 * et run. Chaque test est la preuve d'un écart `aligner: code` de docs/maquette-etat.yml.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { drawnParts } from "@/lib/sent";
import type { Sent } from "@/types/ipc";

import { cssValue } from "./css";
import { newRequest, okOutcome } from "./fakeWorkbench";
import type { AppWrapper } from "./mountApp";
import { click, openProject } from "./workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const SENT: Sent = {
  environment: "Recette",
  method: "POST",
  url: "https://recette.test/orders",
  headers: [
    ["Content-Type", "application/json"],
    ["Authorization", "Bearer ••••••••"],
  ],
  body: '{"ref":"7c1e2b04","email":"lea@example.com"}',
  drawn: [
    { name: "$randomUUID", value: "7c1e2b04", source: "request.body.ref" },
    { name: "$randomEmail", value: "lea@example.com", source: "request.body.email" },
  ],
  used: [
    { name: "baseUrl", value: "https://recette.test", source: "environment" },
    { name: "token", value: "••••••••", source: "local" },
    { name: "client.id", value: "cus_7", source: "extracted" },
  ],
  extracted: [],
};

async function sent(): Promise<AppWrapper> {
  const opened = await openProject((c) => {
    c.workbench.nextSend = okOutcome({ sent: structuredClone(SENT) });
  });
  await click(opened.app, '[role="treeitem"]', "Lister");
  await click(opened.app, "button", "Envoyer");
  await flushPromises();
  return opened.app;
}

describe("ligne d'état de la réponse (AM28, 5.1-b)", () => {
  it("statut en pastille, durée, taille, et l'environnement de l'envoi à droite", async () => {
    app = await sent();
    const status = app.get(".response__status");
    expect(status.classes()).toContain("response__status--ok");
    expect(app.get(".response__env").text()).toBe("Recette");
    const file = "src/components/workbench/ResponsePanel.vue";
    expect(cssValue(file, ".response__status", "border-radius")).toBe("var(--radius-pill)");
    expect(cssValue(file, ".response__status--ok", "background")).toBe("var(--color-success-soft)");
    expect(cssValue(file, ".response__status--ko", "background")).toBe("var(--color-danger-soft)");
    expect(cssValue(file, ".response__env", "margin-left")).toBe("auto");
  });
});

describe("onglet Envoyé (AM29, 5.1-h)", () => {
  it("URL résolue, en-têtes, corps, puis variables avec leur source, et la note", async () => {
    app = await sent();
    await click(app, '[role="tab"]', "Envoyé");
    const titles = app.findAll(".sent__title").map((t) => t.text());
    expect(titles.map((t) => t.split(" · ")[0])).toEqual([
      "URL résolue",
      "En-têtes",
      "Corps envoyé",
      "Valeurs tirées à cet envoi",
      "Variables utilisées",
    ]);
    expect(
      app.findAll('[aria-label="Variables utilisées"] .sent__origin').map((o) => o.text()),
    ).toEqual(["Recette", "locale", "extraite"]);
    expect(app.findAll(".sent__body .sent__drawn").map((m) => m.text())).toEqual([
      "7c1e2b04",
      "lea@example.com",
    ]);
    expect(app.get(".sent").text()).toContain("Exactement ce qui est parti sur le réseau");
  });

  it("le découpage retrouve chaque valeur tirée, la plus longue d'abord", () => {
    expect(
      drawnParts("a-12-123-b", [
        { name: "$n", value: "12", source: null },
        { name: "$m", value: "123", source: null },
      ]),
    ).toEqual([
      { text: "a-", drawn: null },
      { text: "12", drawn: "$n" },
      { text: "-", drawn: null },
      { text: "123", drawn: "$m" },
      { text: "-b", drawn: null },
    ]);
  });
});

describe("écran du run (AM30, 5.2-e et 5.2-f)", () => {
  it("fil d'Ariane, date de fin, « Relancer les échecs » plein et « Ouvrir le test »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.runStatus.set("Clients/creer.yml", "failed");
    }));
    const row = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Clients"));
    await row?.get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Exécuter le test");
    const run = app.get(".run");
    expect(run.get(".run__title").text()).toBe(
      "Référentiel de tests / Clients / Dernière exécution",
    );
    expect(run.get(".run__meta").text()).toMatch(/ · (aujourd'hui \d\d:\d\d|\d+ \S+ \d\d:\d\d)$/);
    const retry = run.findAll("button").find((b) => b.text() === "Relancer les échecs");
    expect(retry?.classes()).toContain("button--primary");
    await click(app, ".run__open-test", "Ouvrir le test");
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toMatchObject({
      content: { kind: "test", path: "Clients" },
    });
  });
});

describe("confirmation d'un environnement protégé (AM31, 3.3-c)", () => {
  it("« Envoyer vers <env> ? », la requête et l'hôte, bouton plein ; 🔒 sur la pastille", async () => {
    ({ app } = await openProject((c) => {
      const env = c.workbench.envs[0];
      if (env) {
        env.name = "Pré-production";
        env.protected = true;
        env.variables = [{ name: "baseUrl", value: "https://preprod.boutique.dev" }];
      }
      c.workbench.active = "Pré-production";
      c.workbench.requests.set(
        "Clients/creer.yml",
        newRequest("Créer", { method: "POST", url: "{{baseUrl}}/orders" }),
      );
      c.workbench.nextSend = { kind: "protectedEnvironment", message: "Environnement protégé." };
    }));
    expect(
      app
        .get('[aria-label="Environnement de boutique-tests : Pré-production (protégé)"]')
        .find("svg")
        .exists(),
    ).toBe(true);
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    const dialog = app.get("dialog[open]");
    expect(dialog.get(".dialog__title").text()).toBe("Envoyer vers Pré-production ?");
    expect(dialog.get(".confirm__text").text().replace(/\s+/g, " ")).toBe(
      "Cet environnement est protégé. POST /orders peut modifier des données sur preprod.boutique.dev.",
    );
    const send = dialog.findAll("button").find((b) => b.text() === "Envoyer vers Pré-production");
    expect(send?.classes()).toContain("button--primary");
  });
});

describe("extraire depuis une valeur du corps (AM32, 4.2-b bis)", () => {
  it("un clic sur une valeur propose « Extraire dans une variable », puis la marque « → nom »", async () => {
    ({ app } = await openProject((c) => {
      const body = '{"id":"cus_7","items":[{"sku":"A1"}]}';
      const outcome = okOutcome();
      outcome.response = {
        ...outcome.response,
        headers: [["content-type", "application/json"]],
        body,
        sizeBytes: body.length,
      };
      c.workbench.nextSend = outcome;
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    const value = app.get('.code [data-path="$.id"]');
    await value.trigger("click");
    await flushPromises();
    const item = app.get('.body__pick [role="menuitem"]');
    expect(document.activeElement).toBe(item.element);
    await item.trigger("click");
    await flushPromises();
    expect(app.get(".toast").text()).toContain("Variable « id » extraite de body.id");
    expect(app.find(".body__pick").exists()).toBe(false);
    expect(app.get(".tok__extracted").text()).toBe("→ id");
    // Une clé ne se propose pas : seules les valeurs portent leur chemin.
    expect(app.findAll(".code .tok--key[data-path]")).toHaveLength(0);
  });
});
