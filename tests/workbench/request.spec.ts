import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { newRequest, okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

describe("éditer et envoyer une requête", () => {
  it("envoie la requête dans l'environnement actif et affiche la réponse", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await app.get(".editor__url").setValue("{{baseUrl}}/clients?x=1");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();

    const send = core.calls.find((c) => c.cmd === "request_send");
    expect(send?.args).toMatchObject({ environment: "Local", confirmed: false });
    expect((send?.args.request as { url: string }).url).toBe("{{baseUrl}}/clients?x=1");
    // La requête modifiée est enregistrée avant l'envoi.
    expect(core.commands().indexOf("request_save")).toBeLessThan(
      core.commands().indexOf("request_send"),
    );
    const response = app.get(".response");
    expect(response.text()).toContain("200 OK");
    expect(response.text()).toContain("Réussie");
    expect(response.get(".response__body").text()).toContain('"name": "Alice"');
  });

  it("affiche l'erreur de variable inconnue sans réponse", async () => {
    ({ app } = await openProject((core) => {
      core.workbench.nextSend = {
        kind: "unknownVariable",
        message: "Variable « baseUrl » absente de l'environnement Local.",
      };
    }));
    await click(app, '[role="treeitem"]', "Santé");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    expect(app.get(".response [role='alert']").text()).toContain("Variable « baseUrl »");
  });

  it("demande confirmation dans un environnement protégé, puis renvoie confirmé", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.nextSend = { kind: "protectedEnvironment", message: "Environnement protégé." };
    }));
    await click(app, '[role="treeitem"]', "Créer");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    expect(app.find("dialog[open]").exists()).toBe(true);

    core.workbench.nextSend = okOutcome();
    await click(app, "dialog[open] button", "Envoyer quand même");
    const sends = core.calls.filter((c) => c.cmd === "request_send");
    expect(sends.map((s) => s.args.confirmed)).toEqual([false, true]);
    expect(app.get(".response").text()).toContain("200 OK");
  });

  it("propose des vérifications d'après la réponse", async () => {
    ({ app } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    await click(app, ".response__tab", "Vérifications");
    await click(app, ".response button", "Proposer des vérifications");
    await click(app, ".editor__tab", "Vérifications");
    expect(app.findAll(".assertion")).toHaveLength(2);

    // Une vérification déjà présente n'est pas proposée deux fois.
    await click(app, ".response button", "Proposer des vérifications");
    expect(app.findAll(".assertion")).toHaveLength(2);
  });

  it("désactive une vérification et place un paramètre dans le chemin (S8)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".editor__tab", "Paramètres");
    await click(app, ".pairs__add");
    await app.get('[aria-label="Nom, ligne 1"]').setValue("id");
    await app.get('[aria-label="Valeur, ligne 1"]').setValue("42");
    const kind = app.get('[aria-label="Emplacement, ligne 1"]');
    expect(kind.findAll("option").map((o) => o.text())).toEqual(["Adresse", "Chemin"]);
    await kind.setValue("path");

    await click(app, ".editor__tab", "Vérifications");
    await app.get('[aria-label="Activer la vérification 1"]').setValue(false);
    expect(app.get(".assertion").classes()).toContain("assertion--off");

    await click(app, ".editor__line button", "Enregistrer");
    const saved = core.workbench.requests.get("sante.yml");
    expect(saved?.params).toEqual([{ name: "id", value: "42", enabled: true, kind: "path" }]);
    expect(saved?.assertions[0]?.enabled).toBe(false);
  });

  it("propose les opérateurs d'OpenCollection et masque la valeur inutile", async () => {
    ({ app } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".editor__tab", "Vérifications");
    const select = app.get('[aria-label="Opérateur de la vérification 1"]');
    expect(select.findAll("option")).toHaveLength(28);
    await select.setValue("isArray");
    expect(app.find('[aria-label="Valeur de la vérification 1"]').exists()).toBe(false);
    await select.setValue("between");
    expect(app.find('[aria-label="Valeur de la vérification 1"]').exists()).toBe(true);
  });

  it("signale ce que le fichier garde sans qu'Arnod le gère", async () => {
    ({ app } = await openProject((core) => {
      const request = newRequest("Jeton", { preserved: ["scripts", "documentation"] });
      request.auth = { ...request.auth, type: "other", other: "oauth2" };
      request.body = { ...request.body, kind: "other", other: "multipart-form" };
      core.workbench.requests.set("jeton.yml", request);
    }));
    await click(app, '[role="treeitem"]', "Jeton");
    expect(app.get(".editor__preserved").text()).toContain("scripts, documentation");
    await click(app, ".editor__tab", "Authentification");
    expect(app.get(".editor__panel").text()).toContain("« oauth2 » conservée");
    await click(app, ".editor__tab", "Corps");
    expect(app.get(".editor__panel").text()).toContain("« multipart-form » conservé");
    expect(app.find(".editor__body").exists()).toBe(false);
  });

  it("enregistre explicitement les modifications", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    const save = () => app?.findAll(".editor__line button").find((b) => b.text() === "Enregistrer");
    expect(save()?.attributes("disabled")).toBeDefined();
    await app.get(".editor__method").setValue("DELETE");
    expect(save()?.attributes("disabled")).toBeUndefined();
    await save()?.trigger("click");
    await flushPromises();
    expect(core.workbench.requests.get("sante.yml")?.method).toBe("DELETE");
  });
});
