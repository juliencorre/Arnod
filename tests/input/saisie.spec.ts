import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { variableSegments } from "@/lib/variables";
import type { Pair, RequestDoc } from "@/types/ipc";

import { installFakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { firstRunSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

/** Saisie, texte et cURL (docs/plan-mvp-b-saisie-curl.md, 5.1-b ter, 5.1-b quater, 2.1-a sexies). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** Laisse passer la relecture des variables après une frappe (250 ms). */
async function waitInput(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 300));
  await flushPromises();
}

const ORDER = newRequest("Créer une commande", {
  method: "POST",
  url: "{{baseUrl}}/{{apiVersion}}/orders",
  headers: [
    {
      name: "Content-Type",
      value: "application/json",
      enabled: true,
      kind: "query",
      description: "",
    },
    { name: "X-Client", value: "arnod-tests", enabled: true, kind: "query", description: "" },
  ],
  body: {
    kind: "json",
    variants: [{ title: "", content: '{"ref":"{{$randomUUID}}"}', form: [] }],
    selected: 0,
    other: "",
  },
});

async function openOrder(): Promise<{ app: AppWrapper; core: ReturnType<typeof installFakeCore> }> {
  const opened = await openProject((core) => {
    core.workbench.requests.set("commandes.yml", structuredClone(ORDER));
    core.workbench.envs.push({
      name: "Recette",
      color: "orange",
      variables: [{ name: "apiVersion", value: "v2" }],
      locals: [],
    });
  });
  await click(opened.app, '[role="treeitem"]', "Créer une commande");
  await waitInput();
  return opened;
}

describe("variables inconnues soulignées (5.1-b ter)", () => {
  it("découpe un texte autour de ses variables ; les valeurs dynamiques ne sont jamais inconnues", () => {
    const known = new Set(["baseUrl"]);
    expect(variableSegments("{{baseUrl}}/{{ x }}/{{$guid}}", known)).toEqual([
      { text: "{{baseUrl}}", unknown: false },
      { text: "/", unknown: false },
      { text: "{{ x }}", unknown: true },
      { text: "/", unknown: false },
      { text: "{{$guid}}", unknown: false },
    ]);
    expect(variableSegments("sans variable {{", known)).toEqual([
      { text: "sans variable {{", unknown: false },
    ]);
  });

  it("souligne et signale la variable inconnue de l'environnement actif, avec où elle est définie", async () => {
    let core;
    ({ app, core } = await openOrder());

    const input = core.calls.find((c) => c.cmd === "request_input");
    expect(input?.args).toMatchObject({ environment: "Local", path: "commandes.yml" });
    const url = app.get(".editor__url");
    expect(url.attributes("data-unknown-variables")).toBe("{{apiVersion}}");
    expect(app.get(".variable-marks mark").text()).toBe("{{apiVersion}}");
    const banner = app.get(".unknown");
    expect(banner.text()).toContain(
      "apiVersion est inconnue dans Local ; définie dans Recette (v2).",
    );
    expect(url.attributes("aria-describedby")).toBe(banner.attributes("id"));
    // Le corps n'a qu'une valeur dynamique : rien n'y est souligné.
    await click(app, '[role="tab"]', "Corps");
    expect(app.get(".body__content").attributes("data-unknown-variables")).toBeUndefined();
  });

  it("« Ajouter » met la variable dans l'environnement actif, avec la valeur connue ailleurs", async () => {
    let core;
    ({ app, core } = await openOrder());
    await click(app, ".unknown button", "Ajouter");
    await waitInput();

    const save = core.calls.find((c) => c.cmd === "environment_save");
    const env = save?.args.environment as { name: string; variables: Pair[] };
    expect(env.name).toBe("Local");
    expect(env.variables).toContainEqual({ name: "apiVersion", value: "v2" });
    expect(app.find(".unknown").exists()).toBe(false);
    expect(app.get(".editor__url").attributes("data-unknown-variables")).toBeUndefined();
  });

  it("« Environnements… » ouvre la gestion des environnements", async () => {
    ({ app } = await openOrder());
    await click(app, ".unknown button", "Environnements…");
    expect(app.find("dialog").exists()).toBe(true);
  });

  it("un nombre ou un point par onglet rempli", async () => {
    ({ app } = await openOrder());
    const tab = (label: string) => {
      const found = app?.findAll('[role="tab"]').find((t) => t.text().startsWith(label));
      if (!found) throw new Error(`onglet ${label} absent`);
      return found;
    };
    expect(tab("En-têtes").get(".editor__badge").text()).toBe("2");
    expect(tab("Corps").find(".editor__dot").exists()).toBe(true);
    expect(tab("Corps").text()).toContain("rempli");
    expect(tab("Auth").find(".editor__dot").exists()).toBe(false);
    expect(tab("Docs").find(".editor__dot").exists()).toBe(false);
  });

  it("liste, grisés, les en-têtes ajoutés par Arnod à l'envoi", async () => {
    ({ app } = await openOrder());
    await click(app, '[role="tab"]', "En-têtes");
    const added = app.get('[aria-label="En-têtes ajoutés par Arnod à l\'envoi"]');
    const names = added.findAll(".sent__name").map((n) => n.text());
    // Content-Type est saisi dans la requête : il remplace celui d'Arnod.
    expect(names).toEqual(["User-Agent", "Accept", "Content-Length", "Host"]);
    expect(app.get(".stack").text()).toContain("Ajoutés à l'envoi · 4");
  });
});

describe("la requête en texte (5.1-b quater)", () => {
  it("« Éditer en texte » : la liste devient du texte, dix lignes collées d'un coup", async () => {
    let core;
    ({ app, core } = await openOrder());
    await click(app, '[role="tab"]', "En-têtes");
    await click(app, ".pairs__mode", "Éditer en texte");

    const area = app.get<HTMLTextAreaElement>(".pairs__textarea");
    expect(area.element.value).toBe("Content-Type: application/json\nX-Client: arnod-tests");
    expect(document.activeElement).toBe(area.element);
    const lines = Array.from({ length: 10 }, (_, i) => `X-H${String(i)}: ${String(i)}`);
    await area.setValue(lines.join("\n"));
    await flushPromises();
    await click(app, ".pairs__mode", "Revenir au tableau");

    expect(app.findAll(".pairs__row")).toHaveLength(10);
    expect(core.calls.some((c) => c.cmd === "pairs_from_text")).toBe(true);
    const tab = app.findAll('[role="tab"]').find((t) => t.text().startsWith("En-têtes"));
    expect(tab?.get(".editor__badge").text()).toBe("10");
  });

  it("une ligne illisible est signalée et garde le texte", async () => {
    ({ app } = await openOrder());
    await click(app, '[role="tab"]', "En-têtes");
    await click(app, ".pairs__mode", "Éditer en texte");
    const area = app.get<HTMLTextAreaElement>(".pairs__textarea");
    await area.setValue("Accept: */*\n# X-Debug: true\nCache-Control no-cache");
    await flushPromises();

    expect(app.get(".pairs__errors").text()).toBe(
      "Ligne 3 : « : » manquant entre le nom et la valeur.",
    );
    expect(area.attributes("aria-invalid")).toBe("true");
    expect(app.get(".pairs__gutter-line--error").text()).toBe("3");
    await click(app, ".pairs__mode", "Revenir au tableau");
    // Toujours en texte : rien n'est perdu.
    expect(app.find(".pairs__textarea").exists()).toBe(true);
    await area.setValue("Accept: */*\n# X-Debug: true\nCache-Control: no-cache");
    await flushPromises();
    await click(app, ".pairs__mode", "Revenir au tableau");
    const rows = app.findAll(".pairs__row");
    expect(rows).toHaveLength(3);
    expect(rows[1]?.classes()).toContain("pairs__row--off");
  });

  it("les paramètres de requête s'éditent en texte sans toucher aux paramètres de chemin", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "lire.yml",
        newRequest("Lire", {
          url: "{{baseUrl}}/orders/:id",
          params: [
            { name: "id", value: "42", enabled: true, kind: "path", description: "" },
            { name: "page", value: "1", enabled: true, kind: "query", description: "Page" },
          ],
        }),
      );
    }));
    await click(app, '[role="treeitem"]', "Lire");
    await click(app, ".pairs__mode", "Éditer en texte");
    const area = app.get<HTMLTextAreaElement>(".pairs__textarea");
    expect(area.element.value).toBe("page=1");
    await area.setValue("page=2\ntri=nom");
    await flushPromises();
    await click(app, ".pairs__mode", "Revenir au tableau");
    await new Promise((resolve) => setTimeout(resolve, 700));
    await flushPromises();

    const saved = core.calls.filter((c) => c.cmd === "request_save").at(-1);
    const params = (saved?.args.request as RequestDoc).params;
    expect(params.map((p) => [p.kind, p.name, p.value, p.description])).toEqual([
      ["path", "id", "42", ""],
      ["query", "page", "2", "Page"],
      ["query", "tri", "nom", ""],
    ]);
  });

  it("</> et Ctrl+Maj+C copient la requête en cURL, jeton remplacé par $TOKEN", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    let core;
    ({ app, core } = await openOrder());

    const button = app.get(".editor__curl-button");
    expect(button.attributes("aria-label")).toBe("Copier la requête en cURL");
    await button.trigger("click");
    await flushPromises();
    const box = app.get('.curl[role="dialog"]');
    expect(box.text()).toContain("variables de Local");
    expect(box.get(".curl__command").text()).toContain("curl -X POST");
    expect(writeText).not.toHaveBeenCalled();
    await box.trigger("keydown", { key: "Escape" });
    await flushPromises();
    expect(app.find(".curl").exists()).toBe(false);
    // Échap rend le focus à « </> ».
    expect(document.activeElement).toBe(button.element);

    await app
      .get(".editor__url")
      .trigger("keydown", { key: "C", code: "KeyC", ctrlKey: true, shiftKey: true });
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "request_curl")).toHaveLength(2);
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("curl -X POST"));
    expect(app.find(".curl").exists()).toBe(true);
  });

  it("le jeton apparaît en variable du shell dans la commande", async () => {
    ({ app } = await openProject((core) => {
      core.workbench.requests.set(
        "moi.yml",
        newRequest("Moi", {
          url: "https://api.test/moi",
          auth: {
            type: "bearer",
            username: "",
            password: "",
            token: "{{token}}",
            name: "",
            value: "",
            location: "header",
            other: "",
          },
        }),
      );
    }));
    await click(app, '[role="treeitem"]', "Moi");
    await click(app, ".editor__curl-button");
    expect(app.get(".curl__variable").text()).toBe("$TOKEN");
    expect(app.get(".curl__note").text()).toBe(
      "Le jeton est remplacé par $TOKEN : aucun secret ne quitte Arnod.",
    );
  });
});

describe("une commande cURL collée (2.1-a sexies)", () => {
  const CURL =
    "curl -X POST https://api.boutique.dev/orders -H 'Content-Type: application/json' -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.x' -d '{\"customerId\":\"cus_31\"}' --compressed";

  it("est reconnue REST : méthode, adresse, en-têtes et corps repris, jeton retiré, option signalée", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await app.get(".try__input").setValue(CURL);
    await flushPromises();

    expect(app.get<HTMLInputElement>(".try__input").element.value).toBe(
      "https://api.boutique.dev/orders",
    );
    expect(app.get(".try__note").text()).toBe("Reconnu : une commande cURL.");
    expect(app.get('[role="radio"][aria-checked="true"]').text()).toBe("POST");
    const areas = app.findAll<HTMLTextAreaElement>("textarea").map((t) => t.element.value);
    expect(areas).toContain("Content-Type: application/json\nAuthorization: Bearer {{token}}");
    expect(areas).toContain('{"customerId":"cus_31"}');
    expect(areas.join()).not.toContain("eyJ");
    const note = app.get(".try__import").text();
    expect(note).toContain("Repris de la commande : méthode, adresse, 2 en-têtes et le corps.");
    expect(note).toContain("variable locale {{token}}");
    expect(note).toContain("--compressed est ignorée");
  });

  it("l'envoi passe le jeton au cœur, à part des valeurs du formulaire", async () => {
    const core = installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await app.get(".try__input").setValue(CURL);
    await flushPromises();
    await app.get(".try__form").trigger("submit");
    await flushPromises();

    const send = core.calls.find((c) => c.cmd === "trial_send");
    expect(send?.args.secrets).toEqual({ token: "eyJhbGciOiJIUzI1NiJ9.x" });
    expect(JSON.stringify(send?.args.values)).not.toContain("eyJ");
  });

  it("Échap oublie la commande et son jeton", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await app.get(".try__input").setValue(CURL);
    await flushPromises();
    await app.get(".try__input").trigger("keydown", { key: "Escape" });
    await flushPromises();
    expect(app.find(".try__import").exists()).toBe(false);
    expect(app.find(".nature-form").exists()).toBe(false);
  });
});
