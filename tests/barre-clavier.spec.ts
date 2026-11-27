/**
 * Lot H du plan de qualité, T25 (docs/plan-qualite-mvp.md) : clavier de l'arbre (AC1), noms
 * accessibles des lignes d'un contrat (AC9), infobulle d'un nom coupé (DE5). Chaque test est la
 * preuve d'un constat de l'audit (docs/audit-2026-11-mvp.md).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import { defineComponent, h, ref, withDirectives } from "vue";

import { EVENTS } from "@/lib/ipc";

import { cssValue } from "./css";
import { vVariables } from "@/lib/variables";

import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "./fakeContracts";
import { newRequest } from "./fakeWorkbench";
import { BOUTIQUE_TESTS } from "./fixtures";
import type { AppWrapper } from "./mountApp";
import { click, openProject } from "./workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const PETSTORE = "contracts/petstore.yaml";
const TREE = '[role="tree"][aria-label="Référentiel de tests de boutique-tests"]';

function items(wrapper: AppWrapper) {
  return wrapper.findAll(`${TREE} [role="treeitem"]`);
}

function item(wrapper: AppWrapper, path: string) {
  return wrapper.get(`${TREE} [role="treeitem"][data-path="${path}"]`);
}

/** Touche `key` sur l'élément qui a le focus, comme le ferait le clavier. */
async function press(key: string, init: KeyboardEventInit = {}): Promise<void> {
  const target = document.activeElement ?? document.body;
  target.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }),
  );
  await flushPromises();
}

function focusedPath(): string | undefined {
  return (document.activeElement as HTMLElement | null)?.dataset.path;
}

describe("clavier de l'arbre (AC1)", () => {
  it("un seul arrêt de tabulation : une ligne à 0, les autres et les ⋯ à -1", async () => {
    ({ app } = await openProject());
    const stops = items(app).filter((i) => i.attributes("tabindex") === "0");
    expect(stops.map((i) => i.attributes("data-path"))).toEqual(["Clients"]);
    const more = app.findAll(`${TREE} .row__more`);
    expect(more.length).toBeGreaterThan(0);
    expect(more.every((m) => m.attributes("tabindex") === "-1")).toBe(true);
  });

  it("haut, bas, Début et Fin passent d'une ligne visible à l'autre ; l'arrêt suit le focus", async () => {
    ({ app } = await openProject());
    (item(app, "Clients").element as HTMLElement).focus();
    const order = items(app).map((i) => i.attributes("data-path"));
    await press("ArrowDown");
    expect(focusedPath()).toBe(order[1]);
    await press("End");
    expect(focusedPath()).toBe("sante.yml");
    await press("ArrowUp");
    expect(focusedPath()).toBe(order.at(-2));
    await press("Home");
    expect(focusedPath()).toBe("Clients");
    await press("End");
    expect(item(app, "sante.yml").attributes("tabindex")).toBe("0");
    expect(item(app, "Clients").attributes("tabindex")).toBe("-1");
  });

  it("gauche replie ou remonte au parent, droite déplie ou va au premier enfant", async () => {
    let core;
    ({ app, core } = await openProject());
    (item(app, "Clients/lister.yml").element as HTMLElement).focus();
    await press("ArrowLeft");
    expect(focusedPath()).toBe("Clients");
    await press("ArrowLeft");
    expect(core.workbench.collapsed).toContain("Clients");
    expect(app.find(`${TREE} [data-path="Clients/lister.yml"]`).exists()).toBe(false);
    await press("ArrowRight");
    expect(core.workbench.collapsed).not.toContain("Clients");
    await press("ArrowRight");
    expect(focusedPath()?.startsWith("Clients/")).toBe(true);
  });

  it("F2 renomme, Suppr met à la corbeille, Maj+F10 ouvre le menu de la ligne", async () => {
    let core;
    ({ app, core } = await openProject());
    (item(app, "sante.yml").element as HTMLElement).focus();
    await press("F2");
    expect(app.get("dialog[open]").text()).toContain("Renommer");
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();

    (item(app, "sante.yml").element as HTMLElement).focus();
    await press("F10", { shiftKey: true });
    expect(app.find('[role="menu"][aria-label="Actions pour Santé"]').exists()).toBe(true);
    await press("Escape");

    (item(app, "sante.yml").element as HTMLElement).focus();
    await press("Delete");
    expect(core.calls.find((c) => c.cmd === "tree_batch")?.args).toMatchObject({
      paths: ["sante.yml"],
      action: { kind: "delete", permanent: false },
    });
    expect(app.get(".toast").text()).toContain("« Santé » mis à la corbeille.");
  });
});

describe("noms des lignes d'un contrat (AC9)", () => {
  it("le nom accessible commence par le texte visible, puis dit ses marques", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.contracts.files.set(PETSTORE, {
        name: "Petstore",
        version: "1.0.0",
        source: PETSTORE_SOURCE,
        operations: structuredClone(PETSTORE_OPERATIONS),
      });
      c.workbench.folders.add("Petstore");
      c.workbench.folders.add("Petstore/animaux");
      c.workbench.requests.set(
        "Petstore/animaux/a-lister.yml",
        newRequest("Lister les animaux", { contract: { file: PETSTORE, operation: "GET /pets" } }),
      );
    }));
    await core.emit(EVENTS.statuses, {
      projectId: BOUTIQUE_TESTS,
      statuses: { "Petstore/animaux/a-lister.yml": { passed: true, at: 1 } },
    });
    await flushPromises();
    await click(app, '[aria-label="Déplier le contrat Petstore"]');
    await click(app, ".crow__resource", "/pets");
    const rows = app.findAll(".crow__main:not(.crow__resource)");
    expect(rows.length).toBeGreaterThan(1);
    for (const row of rows) {
      const visible = `${row.get(".crow__method").text()} ${row.get(".crow__name").text()}`;
      expect(row.attributes("aria-label")?.startsWith(visible), visible).toBe(true);
    }
    expect(rows.map((r) => r.attributes("aria-label"))).toEqual([
      "GET /pets, réussie",
      "POST /pets, créer la requête",
    ]);
  });
});

describe("infobulle d'un nom coupé (DE5)", () => {
  it("au survol, un nom qui dépasse reçoit son texte en infobulle ; un nom entier n'en a pas", async () => {
    ({ app } = await openProject());
    const name = item(app, "sante.yml").get(".row__name");
    const element = name.element as HTMLElement;
    Object.defineProperty(element, "clientWidth", { configurable: true, value: 40 });
    Object.defineProperty(element, "scrollWidth", { configurable: true, value: 120 });
    await name.trigger("mouseenter");
    expect(element.title).toBe("Santé");
    Object.defineProperty(element, "scrollWidth", { configurable: true, value: 40 });
    await name.trigger("mouseenter");
    expect(element.hasAttribute("title")).toBe(false);
  });

  it("⋯ n'apparaît qu'au survol ou au focus de la ligne, et rend sa place au nom sinon", () => {
    const file = "src/components/workbench/TreeRow.vue";
    expect(cssValue(file, ".row__more", "opacity")).toBe("0");
    expect(cssValue(file, ".row__more", "width")).toBe("0");
    expect(cssValue(file, ".row:hover .row__more", "opacity")).toBe("1");
    expect(cssValue(file, ".row:focus-within .row__more", "width")).toBe("auto");
  });
});

describe("calque des variables (PI6)", () => {
  it("un nouveau rendu sans changement ne relit ni le style ni la mise en page des champs", async () => {
    const known = new Set(["baseUrl"]);
    const tick = ref(0);
    const Fields = defineComponent({
      setup() {
        return () =>
          h("div", { "data-tick": tick.value }, [
            ...Array.from({ length: 50 }, (_, index) =>
              withDirectives(h("input", { value: `{{inconnue${String(index)}}}/x` }), [
                [vVariables, known],
              ]),
            ),
          ]);
      },
    });
    const wrapper = mount(Fields, { attachTo: document.body });
    expect(wrapper.findAll("mark")).toHaveLength(50);
    const style = vi.spyOn(window, "getComputedStyle");
    tick.value++;
    await flushPromises();
    expect(style).not.toHaveBeenCalled();
    // Le champ modifié, lui, est redessiné.
    const first = wrapper.get("input");
    await first.setValue("{{baseUrl}}/x");
    expect(wrapper.findAll("mark")).toHaveLength(49);
    style.mockRestore();
    wrapper.unmount();
  });
});
