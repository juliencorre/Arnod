/**
 * Version 2, lot A (`docs/plan-v2.md`) : recherche Ctrl+K (1.5-a, 1.5-b). La palette s'ouvre
 * par le raccourci du poste, cherche 150 ms après la dernière frappe, surligne la saisie, se
 * parcourt au clavier, ouvre le résultat choisi (Entrée, Ctrl+Entrée dans une autre zone) et se
 * ferme par Échap en rendant le focus.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { SEARCH_DELAY_MS } from "@/stores/search";

import type { FakeCore } from "../fakeCore";
import type { AppWrapper } from "../mountApp";
import { openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function press(code: string, init: KeyboardEventInit = {}, target: EventTarget = window): void {
  const key = code.startsWith("Key") ? code.slice(3).toLowerCase() : code;
  target.dispatchEvent(new KeyboardEvent("keydown", { code, key, bubbles: true, ...init }));
}

async function openPalette(wrapper: AppWrapper): Promise<HTMLInputElement> {
  press("KeyK", { ctrlKey: true });
  await flushPromises();
  return wrapper.get<HTMLInputElement>(".palette__input").element;
}

async function type(wrapper: AppWrapper, text: string): Promise<void> {
  await wrapper.get(".palette__input").setValue(text);
  await new Promise((resolve) => setTimeout(resolve, SEARCH_DELAY_MS + 20));
  await flushPromises();
}

function shown(wrapper: AppWrapper): [string, string[]][] {
  return wrapper
    .findAll(".palette__group")
    .map((g) => [
      g.get(".palette__group-title").text(),
      g.findAll(".palette__label").map((l) => l.text()),
    ]);
}

describe("recherche Ctrl+K (1.5)", () => {
  it("Ctrl+K ouvre la palette sur le workspace, le champ prend le focus", async () => {
    ({ app } = await openProject());
    expect(app.find(".palette").exists()).toBe(false);
    const input = await openPalette(app);
    expect(document.activeElement).toBe(input);
    expect(input.getAttribute("role")).toBe("combobox");
    expect(app.get(".palette__scope").text()).toMatch(/projets?$/);
    expect(app.get(".palette__empty").text()).toContain("POST /orders");
  });

  it("cherche par nom, sans casse, et surligne la saisie", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openPalette(app);
    await type(app, "LIST");
    expect(core.calls.filter((c) => c.cmd === "search_query").at(-1)?.args).toMatchObject({
      text: "LIST",
      refresh: true,
    });
    expect(shown(app)).toEqual([["Tests", ["Lister"]]]);
    expect(app.get(".palette__mark").text()).toBe("List");
    expect(app.get(".palette__detail").text()).toBe("boutique-tests · Clients");
    // La seconde recherche reprend les projets lus.
    await type(app, "cl");
    expect(core.calls.filter((c) => c.cmd === "search_query").at(-1)?.args).toMatchObject({
      refresh: false,
    });
    expect(shown(app)).toEqual([["Dossiers", ["Clients"]]]);
  });

  it("↑ ↓ choisissent, Entrée ouvre le résultat dans la zone active", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    const input = await openPalette(app);
    await type(app, "e");
    const options = () => app?.findAll('[role="option"]') ?? [];
    expect(options().map((o) => o.attributes("aria-selected"))[0]).toBe("true");
    press("ArrowDown", { key: "ArrowDown" }, input);
    await flushPromises();
    const chosen = options().findIndex((o) => o.attributes("aria-selected") === "true");
    expect(chosen).toBe(1);
    expect(input.getAttribute("aria-activedescendant")).toBe(options()[1]?.attributes("id"));
    const label = options()[1]?.get(".palette__label").text();
    press("Enter", { key: "Enter" }, input);
    await flushPromises();
    expect(app.find(".palette").exists()).toBe(false);
    const opened = core.calls.filter((c) => c.cmd === "layout_open").at(-1)?.args.content as {
      kind: string;
      path: string;
    };
    expect(opened.kind).toBe("request");
    expect(core.workbench.requests.get(opened.path)?.name).toBe(label);
  });

  it("Ctrl+Entrée ouvre dans une autre zone, créée à droite s'il n'y en a qu'une", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    const input = await openPalette(app);
    await type(app, "santé");
    press("Enter", { key: "Enter", ctrlKey: true }, input);
    await flushPromises();
    expect(core.commands()).toContain("layout_split");
    expect(core.calls.find((c) => c.cmd === "layout_split")?.args.side).toBe("right");
  });

  it("une adresse trouve les requêtes qui l'appellent", async () => {
    ({ app } = await openProject());
    await openPalette(app);
    await type(app, "GET /clients");
    expect(shown(app)).toEqual([["Tests qui l'appellent", ["Lister"]]]);
  });

  it("aucun résultat le dit ; Échap ferme et rend le focus", async () => {
    ({ app } = await openProject());
    const before = document.activeElement;
    const input = await openPalette(app);
    await type(app, "introuvable");
    expect(app.get(".palette__empty").text()).toBe("Aucun résultat pour « introuvable ».");
    press("Escape", { key: "Escape" }, input);
    await flushPromises();
    expect(app.find(".palette").exists()).toBe(false);
    expect(document.activeElement).toBe(before);
  });

  it("le raccourci se change dans les préférences (1.6-f bis)", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    core.interface.overrides.set("search", "Alt+K");
    await core.emit("arnod://interface", (await import("../fakeInterface")).viewOf(core.interface));
    await flushPromises();
    press("KeyK", { ctrlKey: true });
    await flushPromises();
    expect(app.find(".palette").exists()).toBe(false);
    press("KeyK", { altKey: true });
    await flushPromises();
    expect(app.find(".palette").exists()).toBe(true);
  });
});
