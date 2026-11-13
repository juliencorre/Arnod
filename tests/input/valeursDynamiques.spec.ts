import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { dynamicQuery, filterDynamic, insertDynamic } from "@/lib/dynamicValues";

import { FAKE_DYNAMIC_VALUES } from "../fakeInput";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

/** Liste des valeurs dynamiques proposée à la frappe de `{{$` (écran 5.1-h). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

const ORDER = newRequest("Créer une commande", {
  method: "POST",
  url: "{{baseUrl}}/orders",
  body: {
    kind: "json",
    variants: [{ title: "", content: "", form: [] }],
    selected: 0,
    other: "",
  },
});

async function openOrder(): Promise<AppWrapper> {
  const opened = await openProject((core) => {
    core.workbench.requests.set("commandes.yml", structuredClone(ORDER));
  });
  await click(opened.app, '[role="treeitem"]', "Créer une commande");
  await flushPromises();
  return opened.app;
}

/** Tape `text` dans le champ (curseur à la fin) et laisse la liste se lire. */
async function typeIn(wrapper: AppWrapper, selector: string, text: string): Promise<void> {
  if (!wrapper.find(selector).exists()) await click(wrapper, ".editor__tab", "Corps");
  const field = wrapper.get<HTMLInputElement | HTMLTextAreaElement>(selector);
  field.element.value = text;
  field.element.setSelectionRange(text.length, text.length);
  await field.trigger("input");
  await flushPromises();
}

function options(wrapper: AppWrapper): string[] {
  return wrapper.findAll('[role="option"] .dynamic-values__name').map((o) => o.text());
}

describe("saisie d'une valeur dynamique (pur)", () => {
  it("repère la saisie {{$… qui finit au curseur", () => {
    expect([
      dynamicQuery('{"a": "{{$', 10),
      dynamicQuery("x {{$rand", 9),
      dynamicQuery("{{$date:+3", 10),
      dynamicQuery("{{baseUrl", 9),
      dynamicQuery("{{$guid}} ", 10),
      dynamicQuery("{{$a b", 6),
    ]).toEqual([
      { start: 7, query: "" },
      { start: 2, query: "rand" },
      { start: 0, query: "date:+3" },
      null,
      null,
      null,
    ]);
  });

  it("filtre par début du nom, puis par nom ou description qui contient le texte", () => {
    expect(filterDynamic(FAKE_DYNAMIC_VALUES, "rand").map((v) => v.name)).toEqual([
      "$randomUUID",
      "$randomInt",
      "$randomEmail",
      "$randomFirstName",
      "$randomLastName",
    ]);
  });

  it("insère {{$nom}} à la place de la saisie, sans doubler un }} déjà écrit", () => {
    expect([
      insertDynamic('{"a": "{{$ra"}', { start: 7, query: "ra" }, 12, "$randomInt"),
      insertDynamic("{{$ra}}/x", { start: 0, query: "ra" }, 5, "$randomUUID"),
      insertDynamic("{{$raXY}}", { start: 0, query: "ra" }, 5, "$randomInt"),
    ]).toEqual([
      { text: '{"a": "{{$randomInt}}"}', caret: 21 },
      { text: "{{$randomUUID}}/x", caret: 15 },
      { text: "{{$randomInt}}", caret: 14 },
    ]);
  });
});

describe("liste des valeurs dynamiques (5.1-h)", () => {
  it("s'ouvre à la frappe de {{$ dans le corps, avec description et exemple", async () => {
    app = await openOrder();
    await typeIn(app, ".body__content", '{"ref": "{{$');
    const first = app.get('[role="option"]');
    expect({
      title: app.get(".dynamic-values__title").text(),
      count: app.findAll('[role="option"]').length,
      first: [
        first.get(".dynamic-values__name").text(),
        first.get(".dynamic-values__desc").text(),
        first.get(".dynamic-values__example").text(),
      ],
      selected: first.attributes("aria-selected"),
      expanded: app.get(".body__content").attributes("aria-expanded"),
    }).toEqual({
      title: "Valeurs dynamiques",
      count: FAKE_DYNAMIC_VALUES.length,
      first: ["$isoTimestamp", "Date et heure, ISO 8601", "2026-11-12T15:02:11Z"],
      selected: "true",
      expanded: "true",
    });
  });

  it("se filtre à la frappe ; ↓ puis Entrée insère la valeur choisie", async () => {
    app = await openOrder();
    const url = ".editor__url";
    await typeIn(app, url, "{{baseUrl}}/orders/{{$rand");
    const field = app.get(url);
    await field.trigger("keydown", { key: "ArrowDown" });
    await field.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect({
      value: (field.element as HTMLInputElement).value,
      open: app.find(".dynamic-values").exists(),
    }).toEqual({ value: "{{baseUrl}}/orders/{{$randomInt}}", open: false });
  });

  it("Échap ferme la liste sans rien insérer ; une variable ordinaire ne l'ouvre pas", async () => {
    app = await openOrder();
    await typeIn(app, ".body__content", "{{$ra");
    const shown = options(app).length;
    await app.get(".body__content").trigger("keydown", { key: "Escape" });
    await typeIn(app, ".body__content", "{{baseU");
    expect({
      shown,
      open: app.find(".dynamic-values").exists(),
      value: app.get<HTMLTextAreaElement>(".body__content").element.value,
    }).toEqual({ shown: 5, open: false, value: "{{baseU" });
  });

  it("un clic sur une valeur l'insère", async () => {
    app = await openOrder();
    await typeIn(app, ".body__content", "{{$da");
    const option = app.findAll('[role="option"]').find((o) => o.text().includes("$date:+3d"));
    await option?.trigger("click");
    await flushPromises();
    expect(app.get<HTMLTextAreaElement>(".body__content").element.value).toBe("{{$date:+3d}}");
  });
});
