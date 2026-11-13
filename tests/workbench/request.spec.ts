import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";

import { newRequest, okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import type { RequestDoc } from "@/types/ipc";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

/** Laisse passer l'enregistrement automatique (EP1). */
async function waitAutosave(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
  await flushPromises();
}

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
    expect(response.get(".code").text()).toContain('"name": "Alice"');
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

  it("paramètres de requête décrits, paramètres de chemin tirés de l'adresse (EP2)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".editor__tab", "Paramètres");
    expect(app.get(".editor__panel").text()).toContain("Ils viennent du chemin");
    await click(app, ".pairs__add", "+ Paramètre");
    await app.get('[aria-label="Nom, ligne 1"]').setValue("notify");
    await app.get('[aria-label="Valeur, ligne 1"]').setValue("true");
    await app.get('[aria-label="Description, ligne 1"]').setValue("Envoie l'e-mail");

    await app.get(".editor__url").setValue("{{baseUrl}}/orders/:id");
    expect(app.get(".path-param__name").text()).toContain("id");
    await app.get('[aria-label="Valeur de id"]').setValue("ord_1042");

    await click(app, ".editor__tab", "Vérifications");
    await app.get('[aria-label="Activer la vérification 1"]').setValue(false);
    expect(app.get(".assertion").classes()).toContain("assertion--off");

    await waitAutosave();
    const saved = core.workbench.requests.get("sante.yml");
    expect(saved?.params).toEqual([
      { name: "id", value: "ord_1042", enabled: true, kind: "path", description: "" },
      {
        name: "notify",
        value: "true",
        enabled: true,
        kind: "query",
        description: "Envoie l'e-mail",
      },
    ]);
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
      const request = newRequest("Jeton", { preserved: ["scripts", "variables de requête"] });
      request.auth = { ...request.auth, type: "other", other: "oauth2" };
      request.body = { ...request.body, kind: "other", other: "multipart-form" };
      core.workbench.requests.set("jeton.yml", request);
    }));
    await click(app, '[role="treeitem"]', "Jeton");
    expect(app.get(".editor__preserved").text()).toContain("scripts, variables de requête");
    await click(app, ".editor__tab", "Auth");
    expect(app.get(".editor__panel").text()).toContain("« oauth2 » conservée");
    await click(app, ".editor__tab", "Corps");
    expect(app.get(".editor__panel").text()).toContain("« multipart-form » conservé");
    expect(app.find(".body__content").exists()).toBe(false);
  });

  it("enregistre seul, 600 ms après la dernière modification (EP1)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    expect(app.get(".editor__saved").text()).toBe("✓ Enregistré");
    await app.get(".editor__method").setValue("DELETE");
    await app.get(".meta__input").setValue("Santé du service");
    expect(app.get(".editor__saved").text()).toBe("Modifications en attente…");
    const saves = () =>
      core.calls.filter(
        (c) =>
          c.cmd === "request_save" && (c.args.request as RequestDoc).name === "Santé du service",
      );
    expect(saves()).toHaveLength(0);
    await waitAutosave();
    const saved = core.workbench.requests.get("sante.yml");
    expect(saved?.method).toBe("DELETE");
    expect(saved?.name).toBe("Santé du service");
    expect(app.get(".editor__saved").text()).toBe("✓ Enregistré");
    expect(saves()).toHaveLength(1);
  });

  it("nom, description et étiquettes au clavier (EP1)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    const [name, description] = app.findAll(".meta__input");
    await description?.setValue("Vérifie que le service répond.");
    await click(app, ".tag__add");
    const input = app.get(".tag__input");
    expect(document.activeElement).toBe(input.element);
    await input.setValue("lecture");
    await input.trigger("keydown", { key: "Enter" });
    await click(app, ".tag__add");
    await app.get(".tag__input").setValue("lent");
    await app.get(".tag__input").trigger("keydown", { key: "Escape" });
    expect(app.findAll(".tag").map((t) => t.text().replace("×", "").trim())).toEqual(["lecture"]);
    await click(app, '[aria-label="Retirer l\'étiquette lecture"]');
    await click(app, ".tag__add");
    await app.get(".tag__input").setValue("critique");
    await app.get(".tag__input").trigger("keydown", { key: "Enter" });
    expect(name?.element).toBeDefined();
    await waitAutosave();
    const saved = core.workbench.requests.get("sante.yml");
    expect(saved?.description).toBe("Vérifie que le service répond.");
    expect(saved?.tags).toEqual(["critique"]);
  });

  it("corps à variantes : la sélectionnée part à l'envoi (EP3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Corps");
    expect(app.get(".editor__panel").text()).toContain("Cette requête part sans corps.");
    await app.get(".body__type").setValue("json");
    await app.get(".body__content").setValue('{ "qty": 2 }');
    await click(app, ".body__action", "+ Variante");
    const chips = app.findAll(".body__variant");
    expect(chips.map((c) => c.text())).toEqual(["Variante 1", "Variante 2"]);
    expect(chips[1]?.attributes("aria-pressed")).toBe("true");
    await app.get(".body__name input").setValue("Vide");
    await app.get(".body__content").setValue("{}");
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    const sent = core.calls.find((c) => c.cmd === "request_send")?.args as {
      request: { body: { selected: number; variants: { title: string; content: string }[] } };
      path: string;
    };
    expect(sent.path).toBe("Clients/creer.yml");
    expect(sent.request.body.selected).toBe(1);
    expect(sent.request.body.variants.map((v) => [v.title, v.content])).toEqual([
      ["Variante 1", '{ "qty": 2 }'],
      ["Vide", "{}"],
    ]);
  });

  it("réglages hérités ou propres (EP7)", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".editor__tab", "Réglages");
    const timeout = app.get('[aria-labelledby="setting-timeoutMs"]');
    const choices = timeout.findAll("button");
    expect(choices.map((b) => b.text())).toEqual(["Hérité · 30 s", "5 s", "10 s", "60 s", "Aucun"]);
    expect(choices[0]?.attributes("aria-pressed")).toBe("true");
    await choices[1]?.trigger("click");
    const follow = app.get('[aria-labelledby="setting-followRedirects"]').findAll("button");
    await follow[2]?.trigger("click");
    await waitAutosave();
    expect(core.workbench.requests.get("sante.yml")?.settings).toEqual({
      timeoutMs: 5000,
      followRedirects: false,
      maxRedirects: null,
      encodeUrl: null,
    });
  });

  it("auth héritée : la note dit ce qui part (EP5)", async () => {
    ({ app } = await openProject((core) => {
      const request = newRequest("Payer");
      request.auth = { ...request.auth, type: "inherit" };
      request.inherited.auth = {
        source: "dossier « Paiements »",
        auth: { ...request.auth, type: "bearer", token: "{{token}}" },
      };
      core.workbench.requests.set("payer.yml", request);
    }));
    await click(app, '[role="treeitem"]', "Payer");
    await click(app, ".editor__tab", "Auth");
    expect(app.get(".note").text()).toBe(
      "Reprend l'authentification du dossier « Paiements » : Bearer, jeton {{token}}.",
    );
  });
});

describe("réponses attendues (EP8, EP9)", () => {
  it("ajoute, modifie, duplique et supprime avec « Annuler »", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".side__tab", "Réponses attendues");
    expect(app.get(".expected").text()).toContain("Aucune réponse attendue.");
    await click(app, ".action", "+ Réponse");
    await app.get(".grid input[type=number]").setValue("201");
    await app.findAll(".grid input")[1]?.setValue("Created");
    await app.findAll(".grid input")[2]?.setValue("Commande créée");
    await click(app, ".action", "← Réponses attendues");
    expect(app.get(".expected__row").text()).toContain("Commande créée");
    expect(app.get(".side__badge").text()).toBe("1");

    await click(app, '[aria-label="Dupliquer la réponse 201 Commande créée"]');
    expect(app.findAll(".expected__name").map((n) => n.text())).toEqual([
      "Commande créée",
      "Commande créée (copie)",
    ]);
    await click(app, '[aria-label="Supprimer la réponse 201 Commande créée (copie)"]');
    expect(app.findAll(".expected__row")).toHaveLength(1);
    expect(document.body.textContent).toContain("Réponse 201 Commande créée (copie) supprimée.");
    await click(app, ".toast button", "Annuler");
    expect(app.findAll(".expected__row")).toHaveLength(2);

    await waitAutosave();
    const saved = core.workbench.requests.get("sante.yml");
    expect(saved?.examples.map((e) => [e.status, e.statusText, e.name])).toEqual([
      [201, "Created", "Commande créée"],
      [201, "Created", "Commande créée (copie)"],
    ]);
  });

  it("reprend la dernière réponse reçue", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    await click(app, ".side__tab", "Réponses attendues");
    const fromLast = () => app?.findAll(".action").find((b) => b.text().startsWith("Depuis"));
    expect(fromLast()?.attributes("disabled")).toBeDefined();
    await app.get(".editor__line").trigger("submit");
    await flushPromises();
    await click(app, ".side__tab", "Réponses attendues");
    expect(fromLast()?.text()).toMatch(/^Depuis la dernière réponse reçue · 200 · \d\d:\d\d$/);
    await fromLast()?.trigger("click");
    await flushPromises();
    expect((app.get(".grid input[type=number]").element as HTMLInputElement).value).toBe("200");
    expect(core.commands()).toContain("request_example_from_response");
  });
});
