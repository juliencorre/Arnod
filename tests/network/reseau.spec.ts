/**
 * Lot H - Réseau et authentification (docs/plan-mvp-h-reseau-auth.md) : préférences réseau
 * (1.6-e), échec réseau expliqué (5.1-i), réglages OAuth 2 du projet (3.4-a), cookies (3.4-g),
 * clavier compris.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { Auth } from "@/types/ipc";

import { certificateFailureLotH, certificateLotH } from "../fakeNetwork";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function ctrlComma(): Promise<void> {
  window.dispatchEvent(new KeyboardEvent("keydown", { key: ",", ctrlKey: true }));
  await flushPromises();
}

describe("lot H - préférences réseau (1.6-e)", () => {
  it("Ctrl+, ouvre les préférences : proxy du système détecté, manuel enregistré", async () => {
    let core;
    ({ app, core } = await openProject());
    await ctrlComma();
    const dialog = app.get("dialog[open]");
    expect(dialog.text()).toContain("Préférences");
    expect(dialog.text()).toContain(
      "Détecté : proxy.entreprise.local:8080, par réglage de Windows.",
    );
    expect(dialog.text()).toContain("Les certificats des serveurs sont toujours vérifiés.");
    // Aucune case pour couper la vérification.
    expect(dialog.findAll('input[type="checkbox"]')).toHaveLength(0);

    const manual = dialog
      .findAll('input[type="radio"]')
      .find((r) => r.attributes("value") === "manual");
    await manual?.setValue(true);
    await manual?.trigger("change");
    await flushPromises();
    const address = app.get('input[placeholder="proxy.entreprise.local:8080"]');
    await address.setValue("proxy.corp:3128");
    await address.trigger("change");
    await flushPromises();
    expect(core.network.view.proxy).toEqual({
      mode: "manual",
      manual: { url: "proxy.corp:3128", noProxy: "", username: "" },
    });
    const saves = core.calls.filter((c) => c.cmd === "network_save");
    expect(saves.length).toBeGreaterThan(0);
  });

  it("une autorité ajoutée depuis un fichier est montrée avant d'être reconnue, puis retirée", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.network.picked = certificateLotH("Entreprise Root CA");
    }));
    await ctrlComma();
    await click(app, "dialog[open] button", "+ Ajouter une autorité…");
    const card = app.get('[aria-label="Autorité Entreprise Root CA"]');
    expect(card.text()).toContain("Empreinte SHA-256");
    expect(card.text()).toContain("4F:1A:9C:07");
    expect(core.network.view.authorities).toHaveLength(0);
    await click(app, "dialog[open] button", "Faire confiance");
    expect(core.network.view.authorities.map((a) => a.subject)).toEqual(["Entreprise Root CA"]);
    expect(app.get("dialog[open]").text()).toContain("Ajoutée sur ce poste");
    await click(app, '[aria-label="Retirer Entreprise Root CA"]');
    expect(core.network.view.authorities).toHaveLength(0);
  });

  it("Échap ferme les préférences", async () => {
    ({ app } = await openProject());
    await ctrlComma();
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();
    expect(app.find("dialog[open]").exists()).toBe(false);
  });
});

describe("lot H - échec réseau expliqué (5.1-i)", () => {
  it("certificat non reconnu : cause, empreinte, et « Faire confiance… » mène aux préférences", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.nextSend = certificateFailureLotH();
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    const panel = app.get(".net-failure");
    expect(panel.text()).toContain("Échec · certificat");
    expect(panel.text()).toContain("Le certificat de recette.boutique.local n'est pas reconnu");
    expect(panel.text()).toContain("rien n'a été écrit sur le serveur");
    expect(panel.text()).toContain("4F:1A:9C:07:…:B3:E2");
    expect(panel.text()).not.toContain("Ignorer");
    await click(app, ".net-failure button", "Faire confiance…");
    const dialog = app.get("dialog[open]");
    expect(dialog.text()).toContain("Faire confiance à « Boutique Interne CA » ?");
    await click(app, "dialog[open] button", "Faire confiance");
    expect(core.network.view.authorities.map((a) => a.subject)).toEqual(["Boutique Interne CA"]);
  });

  it("proxy : « Vérifier le proxy… » ouvre les préférences", async () => {
    ({ app } = await openProject((c) => {
      const error = certificateFailureLotH();
      if (error.failure) {
        error.failure.cause = "proxyAuth";
        error.failure.title = "Le proxy demande une identification (407)";
        error.failure.certificate = null;
        error.failure.proxy = "http://proxy.corp:3128";
      }
      c.workbench.nextSend = error;
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    expect(app.get(".net-failure").text()).toContain("http://proxy.corp:3128");
    expect(app.find(".net-failure").text()).not.toContain("Faire confiance");
    await click(app, ".net-failure button", "Vérifier le proxy…");
    expect(app.get("dialog[open]").text()).toContain("Proxy");
  });
});

describe("lot H - authentification du projet (3.4-a)", () => {
  it("⋯ → Réglages du projet : OAuth 2, secret au coffre, jeton obtenu", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.network.usage = {
        inheriting: 52,
        overrides: [
          { label: "dossier « v1-legacy »", path: "v1-legacy", type: "basic", other: "", count: 5 },
        ],
      };
    }));
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    await click(app, '[role="menuitem"]', "Réglages du projet…");
    const dialog = app.get("dialog[open]");
    expect(dialog.text()).toContain("52 requêtes l'héritent");
    expect(dialog.text()).toContain("dossier « v1-legacy » (Basic, 5 requêtes)");
    await dialog.get("select").setValue("oauth2");
    await flushPromises();
    await app
      .get('input[placeholder="{{authUrl}}/oauth/token"]')
      .setValue("{{authUrl}}/oauth/token");
    await app.get('input[type="password"]').setValue("s3cr3t");
    await click(app, "dialog[open] button", "Enregistrer");
    const saved = core.calls.find((c) => c.cmd === "project_auth_save");
    expect(saved?.args.clientSecret).toBe("s3cr3t");
    expect((saved?.args.auth as Auth).type).toBe("oauth2");
    expect(core.network.auth.get("p-boutique-tests")?.oauth2.clientSecret).toBe(
      "{{oauth2ClientSecret}}",
    );
    expect(app.get("dialog[open]").text()).toContain("il sera demandé au premier envoi");
    await click(app, "dialog[open] button", "Obtenir maintenant");
    expect(app.get("dialog[open]").text()).toContain("Obtenu automatiquement à");
    await click(app, "dialog[open] button", "Oublier le jeton");
    expect(core.network.token).toBeNull();
  });
});

describe("lot H - cookies gardés (3.4-g)", () => {
  it("les cookies du domaine sont listés sous les en-têtes, vidés et coupés", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.network.cookies.set("Local", {
        environment: "Local",
        host: "recette.boutique.dev",
        domains: [
          {
            domain: "recette.boutique.dev",
            sending: true,
            cookies: [
              {
                name: "session",
                value: "••••••••",
                path: "/",
                expiresAt: Date.now() + 28 * 60_000,
                httpOnly: true,
                secure: true,
                receivedFrom: "POST /login",
              },
            ],
          },
        ],
      });
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "En-têtes");
    const section = app.get('[aria-label="Cookies gardés de recette.boutique.dev"]');
    expect(section.text()).toContain("session");
    expect(section.text()).toContain("POST /login");
    expect(section.text()).toContain("dans 28 min");
    const box = section.get('input[type="checkbox"]');
    await box.setValue(false);
    await flushPromises();
    expect(core.network.cookies.get("Local")?.domains[0]?.sending).toBe(false);
    await click(app, '[aria-label="Vider les cookies de recette.boutique.dev"]');
    expect(app.find('[aria-label="Cookies gardés de recette.boutique.dev"]').exists()).toBe(false);
  });
});
