/**
 * Alignement sur la maquette, lot D (docs/plan-alignement-maquette.md, T10 et T11) :
 * environnements et authentification. Chaque test est la preuve d'un écart `aligner: code`.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { emptyOAuth2 } from "@/lib/auth";
import { AUTOSAVE_MS } from "@/stores/workbench";

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

const RECETTE = {
  name: "Recette",
  color: "blue",
  protected: false,
  variables: [
    { name: "baseUrl", value: "https://recette.boutique.dev" },
    { name: "devise", value: "EUR" },
  ],
  locals: ["token"],
};

async function openEnvironments(): Promise<void> {
  if (!app) throw new Error("application absente");
  await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
  await click(app, '[role="menuitem"]', "Gérer les environnements…");
}

describe("page des environnements (AM33, 3.3-a)", () => {
  it("liste numérotée avec l'adresse, « Couleur ▾ », protection et « Supprimer l'environnement »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.envs.push(structuredClone(RECETTE));
    }));
    await openEnvironments();
    const items = app.findAll(".envs .envs__item");
    expect(items.map((i) => i.get(".envs__number").text())).toEqual(["1", "2"]);
    expect(items[1]?.get(".envs__item-host").text()).toBe("recette.boutique.dev");
    await items[1]?.trigger("click");
    await flushPromises();
    expect(app.get(".envs .envs__check").text()).toContain(
      "Demander une confirmation avant POST, PUT, PATCH et DELETE",
    );
    expect(app.findAll(".envs button").map((b) => b.text())).toContain("Supprimer l'environnement");
    await click(app, ".envs .envs__color");
    await click(app, '[role="menuitemradio"]', "Vert");
    await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
    await flushPromises();
    expect(core.workbench.envs.find((e) => e.name === "Recette")?.color).toBe("green");
  });
});

describe("fenêtre « Nouvel environnement » (AM34, 3.3-b)", () => {
  it("nom, URL de base, variables reprises, place dans la chaîne et protection", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.envs.push(structuredClone(RECETTE));
    }));
    await openEnvironments();
    await click(app, ".envs button", "+ Nouvel environnement");
    const form = app.get("#new-environment");
    const [name, url] = form.findAll("input");
    await name?.setValue("Recette client");
    await url?.setValue("https://client.boutique.dev");
    const [copy, after] = form.findAll("select");
    await copy?.setValue("Recette");
    expect(form.text()).toContain("3 variables, secrets non copiés");
    await after?.setValue("Local");
    expect(form.text()).toContain("avant Recette");
    await form.get("input[type='checkbox']").setValue(true);
    await form.trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "environment_create")?.args.environment).toEqual({
      name: "Recette client",
      baseUrl: "https://client.boutique.dev",
      copyFrom: "Recette",
      after: "Local",
      protected: true,
    });
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local", "Recette client", "Recette"]);
    // L'environnement créé est ouvert dans la page des environnements.
    expect(app.get(".envs .envs__item[aria-current='true'] .envs__item-name").text()).toBe(
      "Recette client",
    );
  });
});

describe("authentification héritée (AM35, 3.4-b)", () => {
  it("« Type » et « Hériter », carte OAuth 2 du projet avec jeton, « Ouvrir » ; jeton dans Envoyé", async () => {
    ({ app } = await openProject((c) => {
      c.network.token = {
        obtainedAt: Date.now() - 8 * 60_000,
        expiresAt: Date.now() + 52 * 60_000,
      };
      const request = newRequest("Payer");
      request.auth = { ...request.auth, type: "inherit" };
      request.inherited.auth = {
        source: "projet « boutique-tests »",
        auth: {
          ...request.auth,
          type: "oauth2",
          oauth2: {
            ...emptyOAuth2(),
            accessTokenUrl: "{{authUrl}}/oauth/token",
            clientId: "{{oauthClientId}}",
            clientSecret: "{{oauth2ClientSecret}}",
          },
        },
      };
      c.workbench.requests.set("payer.yml", request);
      c.workbench.nextSend = okOutcome({
        sent: {
          environment: "Local",
          method: "GET",
          url: "https://api.test/payer",
          headers: [["Authorization", "Bearer ••••••••"]],
          body: null,
          drawn: [],
          used: [],
          extracted: [],
          oauth2: {
            tokenUrl: "https://auth.test/oauth/token",
            status: { obtainedAt: Date.now(), expiresAt: Date.now() + 3_600_000 },
          },
        },
      });
    }));
    await click(app, '[role="treeitem"]', "Payer");
    await click(app, ".editor__tab", "Auth");
    await flushPromises();
    expect(app.get(".auth .field label").text()).toBe("Type");
    expect(app.get(".auth select option[value='inherit']").text()).toBe("Hériter");
    const card = app.get(".inherited");
    expect(card.get(".inherited__kind").text()).toBe("OAuth 2 · identifiants client");
    const facts = card.findAll(".inherited__facts dd").map((d) => d.text());
    expect(facts[0]).toMatch(/^obtenu à \d\d:\d\d · expire dans 52 min$/);
    expect(facts.slice(1)).toEqual([
      "Authorization: Bearer ••••••••",
      "authUrl, oauthClientId, oauth2ClientSecret",
    ]);
    await click(app, ".inherited__open", "Ouvrir");
    expect(app.find("dialog[open]").text()).toContain("Réglages du projet");
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();

    await click(app, "button", "Envoyer");
    await flushPromises();
    await click(app, '[role="tab"]', "Envoyé");
    expect(app.findAll(".sent__title").map((t) => t.text())).toContain("Jeton OAuth 2");
    expect(app.get(".sent__token").text()).toContain("https://auth.test/oauth/token");
  });
});
