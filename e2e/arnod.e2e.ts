/**
 * Parcours de bout en bout sur l'application réelle (binaire debug, cœur Rust,
 * vrai disque) : création et gestion des projets et des workspaces.
 *
 * Les étapes s'enchaînent et partagent un même bac à sable : elles suivent le
 * premier usage puis l'usage quotidien d'un utilisateur, avec les workspaces en
 * onglets et l'onglet épinglé « Tous les projets ». Le sélecteur de dossier
 * natif (« Changer », P3) ne se pilote pas par WebDriver : il est couvert par les
 * tests Rust et Vitest. Les dernières étapes divisent la fenêtre en zones et
 * ouvrent des fenêtres interconnectées (docs/plan-fenetres-zones.md).
 */
import type { ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
  type Browser,
  type Sandbox,
  cardNames,
  createSandbox,
  heading,
  launchApp,
  removeSandbox,
  readState,
  readWindows,
  type SavedNode,
  screenshot,
  startDriver,
  stopDriver,
  themeEnv,
  toastText,
  waitForHeading,
} from "./app";

let sandbox: Sandbox;
let driver: ChildProcess | undefined;
let browser: Browser;

beforeAll(async () => {
  sandbox = createSandbox();
  driver = await startDriver(sandbox, themeEnv());
  browser = await launchApp();
});

afterAll(async () => {
  try {
    // `browser` reste indéfini si le lancement a échoué.
    await (browser as Browser | undefined)?.deleteSession().catch(() => undefined);
  } finally {
    await stopDriver(driver);
    removeSandbox(sandbox);
  }
});

/** Redémarre Arnod sur le même bac à sable (nouvelle session WebDriver). */
async function restart(): Promise<void> {
  await browser.deleteSession();
  browser = await launchApp();
}

/** Libellés des onglets, « Tous les projets » compris (texte du DOM, défilement ignoré). */
async function tabNames(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[role="tablist"] [role="tab"]')).map((tab) =>
      tab.textContent.trim(),
    ),
  );
}

/** Onglet par son libellé (WebdriverIO : `balise=texte` sous un parent CSS). */
function tab(name: string) {
  return browser.$('[role="tablist"]').$(`button=${name}`);
}

async function openTabMenu(name: string): Promise<void> {
  // Clic droit émis dans la WebView : un clic droit par actions WebDriver dérègle
  // ensuite la frappe des majuscules sous WebKitWebDriver. Le vrai clic droit est
  // couvert par Vitest ; ici, le menu s'ouvre sur le même événement `contextmenu`.
  await browser.execute((label) => {
    const target = Array.from(document.querySelectorAll('[role="tablist"] [role="tab"]')).find(
      (t) => t.textContent.trim() === label,
    );
    if (!target) throw new Error(`onglet « ${label} » introuvable`);
    const box = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        button: 2,
        clientX: box.left + box.width / 2,
        clientY: box.top + box.height / 2,
      }),
    );
  }, name);
  await browser.$('.tabs__context [role="menu"]').waitForDisplayed();
}

async function chooseTabMenuItem(label: string): Promise<void> {
  await browser.$(".tabs__context").$(`[role="menuitem"]*=${label}`).click();
}

/** Crée un workspace par le « + » des onglets. */
async function createWorkspace(name: string): Promise<void> {
  await browser.$('button[aria-label="Nouveau workspace"]').click();
  const popover = browser.$('[role="dialog"][aria-labelledby]');
  await popover.waitForDisplayed();
  await popover.$("input").setValue(name);
  await browser.keys("Enter");
  await waitForHeading(browser, name);
}

async function openProjectMenu(name: string): Promise<void> {
  await browser.$(`button[aria-label="Actions du projet ${name}"]`).click();
  await browser.$('main [role="menu"]').waitForDisplayed();
}

async function dismissToast(): Promise<void> {
  await browser.execute(() => {
    document.querySelector(".toast")?.dispatchEvent(new MouseEvent("mouseleave"));
  });
}

/** Noms des onglets de chaque zone de la fenêtre courante. */
async function zoneTabs(): Promise<string[][]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll("section.zone")).map((zone) =>
      Array.from(zone.querySelectorAll(".zone__tab-button .zone__name")).map((name) =>
        name.textContent.trim(),
      ),
    ),
  );
}

async function waitForZoneTabs(expected: string[][]): Promise<void> {
  await browser.waitUntil(
    async () => JSON.stringify(await zoneTabs()) === JSON.stringify(expected),
    { timeout: 10_000, timeoutMsg: `onglets ${JSON.stringify(expected)} attendus` },
  );
}

/** Clic émis dans la WebView, pour un bouton qui n'apparaît qu'au survol. */
async function clickHidden(selector: string): Promise<void> {
  await browser.execute((css) => {
    const target = document.querySelector<HTMLElement>(css);
    if (!target) throw new Error(`élément « ${css} » introuvable`);
    target.click();
  }, selector);
}

/** Attend `count` fenêtres et renvoie leurs identifiants WebDriver. */
async function waitForWindows(count: number): Promise<string[]> {
  await browser.waitUntil(async () => (await browser.getWindowHandles()).length === count, {
    timeout: 15_000,
    timeoutMsg: `${count} fenêtre(s) attendue(s)`,
  });
  return browser.getWindowHandles();
}

function zonesOf(node: SavedNode): Extract<SavedNode, { kind: "zone" }>[] {
  return node.kind === "zone" ? [node] : [...zonesOf(node.first), ...zonesOf(node.second)];
}

function workspaceNamed(name: string) {
  return readState(sandbox).workspaces.find((w) => w.name === name);
}

describe("Arnod, de bout en bout", () => {
  it("premier lancement : seul l'onglet « Tous les projets », aucun workspace (1a, R3)", async () => {
    expect(await heading(browser)).toBe("Créez votre premier projet");
    expect(await tabNames()).toEqual(["Tous les projets"]);
    expect(await browser.$("aside").getText()).toBe("Aucun projet pour l'instant.");
    expect(await browser.$("main").getText()).toContain(
      "Un projet est toujours dans « Tous les projets ». Un workspace est facultatif.",
    );

    const state = readState(sandbox);
    expect(state.workspaces).toEqual([]);
    expect(state.projects).toEqual([]);
    expect(state.lastWorkspaceId).toBeNull();
    await screenshot(browser, "01-premier-lancement");
  });

  it("crée le premier projet sur disque, sans workspace, et l'ouvre (P1, P4, P5)", async () => {
    // Les fenêtres fermées restent dans le DOM, hors de `main` : leurs champs ne comptent pas.
    await browser.$("main form input").setValue("boutique-tests");
    const folder = browser.$('main [data-testid="project-folder"]');
    const expected = path.join(sandbox.home, "arnod", "boutique-tests");
    await browser.waitUntil(async () => (await folder.getText()) === expected, {
      timeoutMsg: `aperçu du dossier « ${expected} » attendu`,
    });

    await browser.$("main").$("button=Créer le projet").click();
    await waitForHeading(browser, "boutique-tests");

    expect(await toastText(browser)).toContain(
      "« boutique-tests » créé · visible dans « Tous les projets ».",
    );
    const manifest = readFileSync(path.join(expected, "arnod.yml"), "utf8");
    expect(manifest).toContain("name: boutique-tests");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).toEqual(["boutique-tests"]);
    expect(state.workspaces).toEqual([]);
    await screenshot(browser, "02-projet-cree");
  });

  it("refuse un nom déjà pris, message du cœur sous le champ (R9)", async () => {
    // Depuis un projet, l'onglet de départ ramène à sa liste.
    await tab("Tous les projets").click();
    await waitForHeading(browser, "Tous les projets · 1");
    await browser.$("main").$("button=Nouveau projet").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();

    await dialog.$("input").setValue("BOUTIQUE-TESTS");
    await dialog.$("button=Créer le projet").click();

    const error = dialog.$('[role="alert"]');
    await error.waitForDisplayed();
    expect(await error.getText()).toBe(
      "Un projet « boutique-tests » existe déjà dans « Tous les projets ».",
    );
    await dialog.$("button=Annuler").click();
    await dialog.waitForDisplayed({ reverse: true });
  });

  it("« + » crée un workspace : son onglet s'ajoute à la fin et s'ouvre (O3)", async () => {
    await createWorkspace("Boutique");

    expect(await tabNames()).toEqual(["Tous les projets", "Boutique"]);
    expect(await tab("Boutique").getAttribute("aria-selected")).toBe("true");
    expect(await browser.$("main").getText()).toContain("Aucun projet pour l'instant");
    const state = readState(sandbox);
    expect(state.workspaces.map((w) => w.name)).toEqual(["Boutique"]);
    expect(state.lastWorkspaceId).toBe(workspaceNamed("Boutique")?.id);
  });

  it("le menu contextuel d'un onglet s'affiche en entier, par-dessus la zone principale (O4)", async () => {
    await openTabMenu("Boutique");
    const layout = await browser.execute(() => {
      const menu = document.querySelector('.tabs__context [role="menu"]');
      if (!menu) return null;
      const box = menu.getBoundingClientRect();
      // Près de chaque coin, c'est bien le menu qui est affiché : il n'est pas rogné.
      const corners = [
        [box.left + 4, box.top + 4],
        [box.right - 4, box.top + 4],
        [box.left + 4, box.bottom - 4],
        [box.right - 4, box.bottom - 4],
      ] as const;
      return corners.every(([x, y]) => menu.contains(document.elementFromPoint(x, y)));
    });
    expect(layout).toBe(true);
    await screenshot(browser, "02b-menu-onglet");

    await browser.keys("Escape");
    await browser.$('.tabs__context [role="menu"]').waitForDisplayed({ reverse: true });
  });

  it("ajoute un projet existant au workspace (2c, W6)", async () => {
    await browser.$("main .add-card").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    expect(await dialog.$("h2").getText()).toBe("Ajouter un projet à « Boutique »");

    await dialog.$("label*=boutique-tests").click();
    await dialog.$("button=Ajouter 1 projet").click();

    await browser.waitUntil(async () => (await cardNames(browser)).length === 1);
    expect(await cardNames(browser)).toEqual(["boutique-tests"]);
    expect(await toastText(browser)).toBe("« boutique-tests » ajouté à « Boutique ».");
  });

  it("crée un projet depuis le workspace : rangé ici seulement (2a bis, R4)", async () => {
    await browser.$("main").$("button=+ Nouveau projet").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    expect(await dialog.getText()).toContain("workspace ouvert");

    await dialog.$("input").setValue("paiements-v3");
    await dialog.$("button=Créer le projet").click();
    await waitForHeading(browser, "paiements-v3");

    expect(existsSync(path.join(sandbox.home, "arnod", "paiements-v3", "arnod.yml"))).toBe(true);
    const state = readState(sandbox);
    const id = state.projects.find((p) => p.name === "paiements-v3")?.id;
    expect(
      state.workspaces.map((w) => [w.name, id !== undefined && w.projectIds.includes(id)]),
    ).toEqual([["Boutique", true]]);

    await tab("Boutique").click();
    await waitForHeading(browser, "Boutique");
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
    await screenshot(browser, "03-workspace-boutique");
  });

  it("retire un projet du workspace puis annule (P7, R5, R6)", async () => {
    await openProjectMenu("paiements-v3");
    await browser.$("main").$('[role="menuitem"]*=Retirer de « Boutique »').click();

    await browser.waitUntil(async () => (await cardNames(browser)).length === 1);
    expect(await toastText(browser)).toContain("Le projet reste dans « Tous les projets ».");
    expect(readState(sandbox).projects).toHaveLength(2);

    await browser.$(".toast").$("button=Annuler").click();
    await browser.waitUntil(async () => (await cardNames(browser)).length === 2);
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
    await dismissToast();
  });

  it("renomme un workspace dans son onglet (O4)", async () => {
    await openTabMenu("Boutique");
    await chooseTabMenuItem("Renommer");
    const input = browser.$(".tab--editing input");
    await input.waitForDisplayed();
    // Le nom est sélectionné à l'ouverture : la frappe le remplace. (Pas de
    // `setValue` : son effacement fait perdre le focus, ce qui valide l'édition.)
    expect(await input.isFocused()).toBe(true);
    await input.addValue("E-commerce");
    expect(await input.getValue()).toBe("E-commerce");
    await browser.keys("Enter");

    await waitForHeading(browser, "E-commerce");
    expect(await tabNames()).toEqual(["Tous les projets", "E-commerce"]);
    expect(workspaceNamed("E-commerce")?.projectIds).toHaveLength(2);
  });

  it("glisser une ligne de « Tous les projets » sur un onglet ajoute le projet (O7)", async () => {
    await createWorkspace("Recette");
    await tab("Tous les projets").click();
    await waitForHeading(browser, "Tous les projets · 2");

    // Le glisser natif ne se pilote pas par WebDriver sous WebKit : on émet les
    // événements HTML du glisser-déposer dans la vraie WebView.
    await browser.execute(() => {
      const row = Array.from(document.querySelectorAll("main tbody tr")).find(
        (tr) => tr.querySelector("th")?.textContent === "boutique-tests",
      );
      const target = Array.from(document.querySelectorAll('[role="tab"]')).find(
        (t) => t.textContent.trim() === "Recette",
      );
      if (!row || !target) throw new Error("ligne ou onglet introuvable");
      const dataTransfer = new DataTransfer();
      const fire = (element: Element, type: string) =>
        element.dispatchEvent(
          new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer }),
        );
      fire(row, "dragstart");
      fire(target, "dragenter");
      fire(target, "dragover");
      fire(target, "drop");
      fire(row, "dragend");
    });

    expect(await toastText(browser)).toBe("« boutique-tests » ajouté à « Recette ».");
    const boutique = readState(sandbox).projects.find((p) => p.name === "boutique-tests");
    expect(workspaceNamed("Recette")?.projectIds).toEqual([boutique?.id]);
    await dismissToast();
  });

  it("gère les workspaces : suppression sans confirmation, puis annulée (2d, O6, O11)", async () => {
    await openTabMenu("Recette");
    await chooseTabMenuItem("Gérer les workspaces…");
    await waitForHeading(browser, "Workspaces");
    expect(await browser.$("aside").isExisting()).toBe(false);
    expect(await browser.$("main").getText()).not.toContain("par défaut");
    await screenshot(browser, "04-gerer-workspaces");

    await browser.$("main").$("button=Supprimer ce workspace").click();
    expect(await toastText(browser)).toContain(
      "Workspace « Recette » supprimé · 1 projet conservé dans « Tous les projets ».",
    );
    expect(readState(sandbox).workspaces.map((w) => w.name)).toEqual(["E-commerce"]);
    expect(await tabNames()).toEqual(["Tous les projets", "E-commerce"]);

    await browser.$(".toast").$("button=Annuler").click();
    await browser.waitUntil(() => readState(sandbox).workspaces.length === 2);
    expect(await tabNames()).toEqual(["Tous les projets", "E-commerce", "Recette"]);
    await dismissToast();
    await browser.$("main").$("button*=Retour").click();
    await waitForHeading(browser, "Tous les projets · 2");
  });

  it("« Tous les projets » liste chaque projet une fois avec ses workspaces (2e, O1)", async () => {
    expect(await browser.$$("main tbody th").map((row) => row.getText())).toEqual([
      "boutique-tests",
      "paiements-v3",
    ]);
    expect(
      await browser.$$("main tbody tr:first-child .chip").map((chip) => chip.getText()),
    ).toEqual(["E-commerce", "Recette"]);
    expect(await browser.$("aside").isExisting()).toBe(false);
    await screenshot(browser, "05-tous-les-projets");
  });

  it("au redémarrage, rouvre le dernier onglet utilisé (R7)", async () => {
    await restart();
    await waitForHeading(browser, "Tous les projets · 2");
    expect(readState(sandbox).lastWorkspaceId).toBeNull();

    await tab("E-commerce").click();
    await waitForHeading(browser, "E-commerce");
    await restart();

    await waitForHeading(browser, "E-commerce");
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
  });

  it("signale un dossier supprimé hors d'Arnod sans planter (P10)", async () => {
    rmSync(path.join(sandbox.home, "arnod", "paiements-v3"), { recursive: true });
    await restart();

    const card = browser.$("main").$("article*=paiements-v3");
    expect(await card.getText()).toContain("Dossier introuvable");
    await browser.$("aside nav").$("button*=paiements-v3").click();
    expect(await toastText(browser)).toContain(
      "Le dossier du projet « paiements-v3 » est introuvable",
    );
    expect(await heading(browser)).toBe("E-commerce");
  });

  it("inscrit un dossier projet existant par son manifeste (P12)", async () => {
    const folder = path.join(sandbox.home, "dev", "catalogue-legacy");
    mkdirSync(folder, { recursive: true });
    writeFileSync(
      path.join(folder, "arnod.yml"),
      "version: 1\nid: 7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10\nname: catalogue-legacy\n",
    );
    const workspaceId = readState(sandbox).lastWorkspaceId;

    // Le sélecteur natif ne se pilote pas : on appelle la commande Rust réelle.
    await browser.execute(
      (target, workspace) =>
        (
          window as unknown as {
            __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
          }
        ).__TAURI_INTERNALS__.invoke("project_add_existing", {
          path: target,
          workspaceId: workspace,
        }),
      folder,
      workspaceId,
    );
    await restart();

    await waitForHeading(browser, "E-commerce");
    expect(await cardNames(browser)).toContain("catalogue-legacy");
    const added = readState(sandbox).projects.find((p) => p.name === "catalogue-legacy");
    expect(added?.id).toBe("7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10");
  });

  it("oublie un projet sans toucher à ses fichiers, puis annule (P11, R6)", async () => {
    await tab("Tous les projets").click();
    await waitForHeading(browser, "Tous les projets · 3");

    await browser.$('button[aria-label="Oublier catalogue-legacy"]').click();
    await waitForHeading(browser, "Tous les projets · 2");

    expect(await toastText(browser)).toContain("Ses fichiers restent sur le disque.");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).not.toContain("catalogue-legacy");
    expect(state.workspaces.flatMap((w) => w.projectIds)).not.toContain(
      "7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10",
    );
    expect(existsSync(path.join(sandbox.home, "dev", "catalogue-legacy", "arnod.yml"))).toBe(true);

    await browser.$(".toast").$("button=Annuler").click();
    await waitForHeading(browser, "Tous les projets · 3");
    expect(readState(sandbox).projects.map((p) => p.name)).toContain("catalogue-legacy");
  });
});

describe("Fenêtres et zones", () => {
  let mainWindow: string;

  it("ouvre les requêtes en onglets et divise la zone (F1, F2, F4)", async () => {
    await tab("E-commerce").click();
    await waitForHeading(browser, "E-commerce");
    await browser.$("aside nav").$("button*=boutique-tests").click();
    await waitForHeading(browser, "boutique-tests");

    await browser.$("main").$("button=+ Nouvelle requête").click();
    await waitForZoneTabs([["Nouvelle requête"]]);
    await browser.$("aside").$("button*=+ Requête").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    await dialog.$("input").setValue("lister");
    await browser.keys("Enter");
    await waitForZoneTabs([["Nouvelle requête", "lister"]]);

    await browser.$('button[aria-label="Diviser à droite"]').click();
    await waitForZoneTabs([["Nouvelle requête"], ["lister"]]);

    // Au clavier : la séparation prend le focus, la flèche droite élargit la première zone.
    await browser.execute(() => {
      document
        .querySelector<HTMLElement>('[role="separator"][aria-label="Largeur des zones"]')
        ?.focus();
    });
    await browser.keys("ArrowRight");
    const handle = browser.$('[role="separator"][aria-label="Largeur des zones"]');
    await browser.waitUntil(async () => (await handle.getAttribute("aria-valuenow")) === "55");
    await browser.waitUntil(() => {
      const root = readWindows(sandbox).windows[0]?.layout?.root;
      return root?.kind === "split" && Math.abs(root.ratio - 0.55) < 0.001;
    });
    await screenshot(browser, "06-zones");
  });

  it("ouvre une zone dans sa fenêtre, puis la rattache (F8, F12)", async () => {
    mainWindow = await browser.getWindowHandle();
    const tools = await browser.$$('button[aria-label="Ouvrir la zone dans une fenêtre"]');
    await tools[1]?.click();
    const handles = await waitForWindows(2);
    await waitForZoneTabs([["Nouvelle requête"]]);
    expect(readWindows(sandbox).windows.map((w) => w.kind)).toEqual(["main", "content"]);

    const detached = handles.find((h) => h !== mainWindow) ?? "";
    await browser.switchToWindow(detached);
    const crumbs = browser.$('nav[aria-label="Emplacement"]');
    await crumbs.waitForDisplayed({ timeout: 15_000 });
    await browser.waitUntil(async () => (await crumbs.getText()).includes("lister"));
    expect(await crumbs.getText()).toContain("boutique-tests");
    expect(await browser.$(".content-window__follow").getAttribute("aria-pressed")).toBe("true");
    await screenshot(browser, "07-fenetre-contenu");

    await browser.$(".content-window__bar").$("button=Rattacher").click();
    await waitForWindows(1);
    await browser.switchToWindow(mainWindow);
    await waitForZoneTabs([["Nouvelle requête", "lister"]]);
    expect(readWindows(sandbox).windows.map((w) => w.kind)).toEqual(["main"]);
  });

  it("détache la barre latérale, qui pilote la fenêtre principale (F9, Z5)", async () => {
    await clickHidden('button[aria-label="Détacher la barre latérale dans une fenêtre"]');
    const handles = await waitForWindows(2);
    await browser.waitUntil(async () => !(await browser.$("aside").isExisting()));

    await browser.switchToWindow(handles.find((h) => h !== mainWindow) ?? "");
    const pilot = browser.$(".sidebar-window__pilot");
    await pilot.waitForDisplayed({ timeout: 15_000 });
    expect(await pilot.getText()).toBe("Pilote 1 fenêtre de contenu");
    const expand = browser.$('button[aria-label="Déplier boutique-tests"]');
    if (await expand.isExisting()) await expand.click();
    await browser.$('[role="tree"]').$("button*=Nouvelle requête").click();
    await browser.waitUntil(() => {
      const layout = readWindows(sandbox).windows[0]?.layout;
      const zone = layout ? zonesOf(layout.root).find((z) => z.id === layout.activeZone) : null;
      const active = zone?.tabs.find((t) => t.id === zone.activeTab);
      return active?.content.path?.includes("nouvelle") ?? false;
    });
    await screenshot(browser, "08-barre-detachee");

    await browser.$(".sidebar-window__footer").$("button=Rattacher").click();
    await waitForWindows(1);
    await browser.switchToWindow(mainWindow);
    await browser.$("aside").waitForDisplayed();
    expect(readWindows(sandbox).windows[0]?.sidebarDetached).toBe(false);
  });

  it("au redémarrage, rouvre les onglets dans leurs zones (Z8)", async () => {
    await restart();
    await waitForZoneTabs([["Nouvelle requête", "lister"]]);
    expect(await browser.$("aside").isExisting()).toBe(true);
  });
});
