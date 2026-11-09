import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { okOutcome } from "../fakeWorkbench";
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
