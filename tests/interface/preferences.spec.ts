/**
 * Version 1 (`docs/plan-v1.md`) : Préférences → Apparence (1.6-f : thème forcé, zoom, police
 * des éditeurs) et Préférences → Raccourcis (1.6-f bis : un clic change la combinaison, un
 * conflit est signalé avant d'être remplacé, « Rétablir » revient aux raccourcis par défaut) ;
 * un raccourci changé agit à la place de l'ancien.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { comboOf, formatCombo } from "@/lib/shortcuts";

import type { FakeCore } from "../fakeCore";
import { viewOf } from "../fakeInterface";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
  delete document.documentElement.dataset.theme;
  document.documentElement.removeAttribute("style");
});

function press(code: string, options: KeyboardEventInit = {}, target: EventTarget = window): void {
  const key = code.startsWith("Key") ? code.slice(3).toLowerCase() : code;
  target.dispatchEvent(new KeyboardEvent("keydown", { code, key, bubbles: true, ...options }));
}

async function openSection(wrapper: AppWrapper, label: string): Promise<void> {
  press("Comma", { key: ",", ctrlKey: true });
  await flushPromises();
  await click(wrapper, ".preferences__section", label);
}

function row(wrapper: AppWrapper, label: string) {
  const item = wrapper
    .findAll(".shortcuts__item")
    .find((li) => li.get(".shortcuts__label").text() === label);
  if (!item) throw new Error(`action introuvable : ${label}`);
  return item;
}

/** Clic sur la combinaison de l'action, puis la touche pressée sur le bouton. */
async function capture(
  wrapper: AppWrapper,
  label: string,
  code: string,
  options: KeyboardEventInit,
): Promise<void> {
  const button = row(wrapper, label).get("button.shortcuts__keys");
  await button.trigger("click");
  expect(button.text()).toBe("Appuyez sur une combinaison…");
  press(code, options, button.element);
  await flushPromises();
}

describe("lecture des touches (lib/shortcuts)", () => {
  it("forme canonique d'après la touche physique, affichage en français", () => {
    const event = (init: KeyboardEventInit) => new KeyboardEvent("keydown", init);
    expect(comboOf(event({ code: "KeyK", key: "k", ctrlKey: true, shiftKey: true }))).toBe(
      "Ctrl+Shift+K",
    );
    // AZERTY : Ctrl+1 donne « & », la touche reste 1.
    expect(comboOf(event({ code: "Digit1", key: "&", ctrlKey: true }))).toBe("Ctrl+1");
    expect(comboOf(event({ code: "NumpadEnter", key: "Enter", metaKey: true }))).toBe("Ctrl+Enter");
    expect(comboOf(event({ code: "Backslash", key: "\\", ctrlKey: true }))).toBe("Ctrl+Backslash");
    expect(comboOf(event({ code: "KeyM", key: ",", ctrlKey: true }))).toBe("Ctrl+M");
    expect(comboOf(event({ code: "Comma", key: ",", ctrlKey: true }))).toBe("Ctrl+,");
    expect(comboOf(event({ code: "ControlLeft", key: "Control", ctrlKey: true }))).toBeNull();
    expect(comboOf(event({ code: "Escape", key: "Escape" }))).toBeNull();
    expect(
      ["Ctrl+Shift+N", "Ctrl+Enter", "Ctrl+Backslash", "Alt+ArrowUp", "Ctrl+1…9"].map(formatCombo),
    ).toEqual(["Ctrl+Maj+N", "Ctrl+Entrée", "Ctrl+\\", "Alt+↑", "Ctrl+1…9"]);
  });
});

describe("Préférences → Apparence (1.6-f)", () => {
  it("le thème se force puis revient au système ; enregistré par le cœur", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openSection(app, "Apparence");
    const themes = app.findAll(".appearance__theme");
    expect(
      themes.map((t) =>
        t
          .findAll("span")
          .map((part) => part.text())
          .join(" "),
      ),
    ).toEqual(["Comme le système clair en ce moment", "Clair", "Sombre"]);
    await themes[2]?.get("input").trigger("change");
    await flushPromises();
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(core.interface.appearance.theme).toBe("dark");
    expect(app.get(".appearance__theme--on").text()).toBe("Sombre");

    await themes[0]?.get("input").trigger("change");
    await flushPromises();
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(core.interface.appearance.theme).toBe("system");
  });

  it("zoom, police et taille des éditeurs ; la police s'applique aux éditeurs", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openSection(app, "Apparence");
    const selects = app.findAll(".appearance select");
    expect(selects).toHaveLength(3);
    await selects[0]?.setValue("125");
    await selects[1]?.setValue("Consolas");
    await selects[2]?.setValue("15");
    await flushPromises();
    expect(core.interface.appearance).toEqual({
      theme: "system",
      zoom: 125,
      editorFont: "Consolas",
      editorFontSize: 15,
    });
    const style = document.documentElement.style;
    expect(style.getPropertyValue("--editor-font")).toBe('"Consolas", var(--font-mono)');
    expect(style.getPropertyValue("--editor-font-size")).toBe("0.9375rem");

    await selects[1]?.setValue("");
    await flushPromises();
    expect(core.interface.appearance.editorFont).toBeNull();
    expect(style.getPropertyValue("--editor-font")).toBe("");
  });

  it("un changement fait dans une autre fenêtre s'applique ici", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    core.interface.appearance.theme = "light";
    await core.emit("arnod://interface", viewOf(core.interface));
    await flushPromises();
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});

describe("Préférences → Raccourcis (1.6-f bis)", () => {
  it("liste les actions par usage, avec leurs combinaisons ; les fixes ne se changent pas", async () => {
    ({ app } = await openProject());
    await openSection(app, "Raccourcis");
    expect(app.findAll(".shortcuts__group-title").map((h) => h.text())).toEqual([
      "Requête et réponse",
      "Fenêtre et onglets",
      "Projet et workspace",
    ]);
    expect(row(app, "Envoyer").get(".shortcuts__keys").text()).toBe("Ctrl+Entrée");
    expect(row(app, "Afficher ou masquer la barre latérale").text()).toContain("Ctrl+\\");
    const fixed = row(app, "Workspace 1 à 9, Tous les projets");
    expect(fixed.find("button").exists()).toBe(false);
    expect(fixed.text()).toContain("Ctrl+1…9");
    expect(fixed.text()).toContain("fixe");
    // « Rétablir » n'a rien à rétablir.
    const reset = app.findAll(".shortcuts button").find((b) => b.text().includes("Rétablir"));
    expect(reset?.attributes("disabled")).toBeDefined();
  });

  it("la recherche filtre par action ou par touche", async () => {
    ({ app } = await openProject());
    await openSection(app, "Raccourcis");
    const search = app.get(".shortcuts input");
    await search.setValue("onglet");
    expect(app.findAll(".shortcuts__label").map((l) => l.text())).toEqual([
      "Onglet suivant, précédent",
      "Fermer l'onglet",
    ]);
    await search.setValue("maj+n");
    expect(app.findAll(".shortcuts__label").map((l) => l.text())).toEqual(["Nouveau dossier"]);
    await search.setValue("introuvable");
    expect(app.get(".shortcuts__empty").text()).toBe("Aucune action ne correspond.");
  });

  it("un conflit est signalé avant d'être remplacé ; « Annuler » ne change rien", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openSection(app, "Raccourcis");
    await capture(app, "Copier en cURL", "KeyJ", { ctrlKey: true });
    const alert = row(app, "Copier en cURL").get(".shortcuts__conflict");
    expect(alert.text()).toContain("Ctrl+J est déjà pris par « Résultat dessous ou à côté ».");
    expect(core.interface.overrides.size).toBe(0);
    await click(app, ".shortcuts__conflict button", "Annuler");
    expect(app.find(".shortcuts__conflict").exists()).toBe(false);
    expect(row(app, "Copier en cURL").get(".shortcuts__keys").text()).toBe("Ctrl+Maj+C");

    await capture(app, "Copier en cURL", "KeyJ", { ctrlKey: true });
    await click(app, ".shortcuts__conflict button", "Remplacer");
    expect(row(app, "Copier en cURL").get(".shortcuts__keys").text()).toBe("Ctrl+J");
    expect(row(app, "Résultat dessous ou à côté").get(".shortcuts__keys").text()).toBe("Aucun");

    await click(app, ".shortcuts button", "Rétablir les raccourcis par défaut");
    expect(row(app, "Copier en cURL").get(".shortcuts__keys").text()).toBe("Ctrl+Maj+C");
    expect(row(app, "Résultat dessous ou à côté").get(".shortcuts__keys").text()).toBe("Ctrl+J");
  });

  it("Échap annule l'attente ; une combinaison réservée est refusée et expliquée", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openSection(app, "Raccourcis");
    const button = row(app, "Envoyer").get("button.shortcuts__keys");
    await button.trigger("click");
    press("Escape", { key: "Escape" }, button.element);
    await flushPromises();
    expect(button.text()).toBe("Ctrl+Entrée");
    // Échap n'a pas fermé les préférences.
    expect(app.find(".preferences").exists()).toBe(true);
    expect(document.activeElement).toBe(button.element);

    await capture(app, "Envoyer", "KeyC", { ctrlKey: true });
    expect(row(app, "Envoyer").get(".shortcuts__refusal").text()).toBe("Ctrl+C est réservé.");
    expect(core.interface.overrides.size).toBe(0);
  });

  it("un raccourci changé agit à la place de l'ancien", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await openSection(app, "Raccourcis");
    await capture(app, "Afficher ou masquer la barre latérale", "KeyB", { altKey: true });
    expect(core.interface.overrides.get("toggleSidebar")).toBe("Alt+B");
    await app.get(".dialog__close, [aria-label='Fermer']").trigger("click");
    await flushPromises();

    press("Backslash", { key: "\\", ctrlKey: true });
    await flushPromises();
    expect(app.find(".shell__sidebar").exists()).toBe(true);
    press("KeyB", { altKey: true });
    await flushPromises();
    expect(app.find(".shell__sidebar").exists()).toBe(false);
  });

  it("Ctrl+Entrée envoie la requête depuis l'éditeur", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Santé");
    press("Enter", { key: "Enter", ctrlKey: true }, app.get(".editor__url").element);
    await flushPromises();
    expect(core.commands().filter((c) => c === "request_send")).toHaveLength(1);
  });
});
