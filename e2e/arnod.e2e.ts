/**
 * Parcours de bout en bout sur l'application réelle (binaire debug, cœur Rust,
 * vrai disque) : création et gestion des projets et des workspaces.
 *
 * Les étapes s'enchaînent et partagent un même bac à sable : elles suivent le
 * premier usage puis l'usage quotidien d'un utilisateur, avec les workspaces en
 * onglets et l'onglet épinglé « Tous les projets ». Le sélecteur de dossier
 * natif (« Changer », P3) ne se pilote pas par WebDriver : il est couvert par les
 * tests Rust et Vitest. Les dernières étapes divisent la fenêtre en zones et
 * ouvrent des fenêtres interconnectées (docs/plan-fenetres-zones.md), puis
 * ajoutent un contrat de service et exécutent ses requêtes contre un serveur
 * HTTP local (docs/plan-contrats-services.md). Les dernières essaient une étape
 * REST depuis « Tous les projets » : l'essai est gardé dans « Non classés », sous
 * le contrat déduit de son serveur (docs/plan-accueil-non-classes.md). Les toutes dernières suivent
 * le premier usage jusqu'au run d'un dossier : vérifications en échec, résultat par dossier et
 * statuts de la barre latérale gardés au redémarrage (docs/plan-fin-premier-usage.md).
 */
import type { ChildProcess } from "node:child_process";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
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

/** Workspace affiché par le sélecteur (docs/plan-selecteur-workspace.md, S1). */
async function currentWorkspace(): Promise<string> {
  // Texte du DOM : `getText` de WebKitWebDriver rend vide un libellé tronqué par ellipse.
  return browser.execute(
    () => document.querySelector(".switcher__label")?.textContent.trim() ?? "",
  );
}

async function openSwitcher(): Promise<void> {
  const menu = browser.$('.switcher__popover [role="menu"]');
  if (await menu.isExisting()) return;
  await browser.$(".switcher__trigger").click();
  await menu.waitForDisplayed();
}

async function closeSwitcher(): Promise<void> {
  if (!(await browser.$('.switcher__popover [role="menu"]').isExisting())) return;
  await browser.keys("Escape");
  await browser.$('.switcher__popover [role="menu"]').waitForExist({ reverse: true });
}

/** « Tous les projets » puis les workspaces, lus dans le menu du sélecteur. */
async function workspaceNames(): Promise<string[]> {
  await openSwitcher();
  const names = await browser.execute(() =>
    Array.from(
      document.querySelectorAll('.switcher__popover [role="menuitemradio"] .switcher__name'),
    ).map((name) => name.textContent.trim()),
  );
  await closeSwitcher();
  return names;
}

/** Ligne d'un workspace (ou de « Tous les projets ») dans le menu ouvert. */
async function clickSwitcherEntry(label: string, role = "menuitemradio"): Promise<void> {
  await browser.execute(
    (text, kind) => {
      const entry = Array.from(
        document.querySelectorAll<HTMLElement>(`.switcher__popover [role="${kind}"]`),
      ).find((item) => item.querySelector(".switcher__name")?.textContent.trim() === text);
      if (!entry) throw new Error(`« ${text} » absent du sélecteur`);
      entry.click();
    },
    label,
    role,
  );
}

/** Ouvre un workspace, ou « Tous les projets », par le sélecteur. */
async function chooseWorkspace(name: string): Promise<void> {
  await openSwitcher();
  await clickSwitcherEntry(name);
  await browser.$('.switcher__popover [role="menu"]').waitForExist({ reverse: true });
}

/** Actions d'un workspace : menu du sélecteur, puis clic droit sur sa ligne (S4). */
async function openWorkspaceActions(name: string): Promise<void> {
  await openSwitcher();
  // Clic droit émis dans la WebView : un clic droit par actions WebDriver dérègle
  // ensuite la frappe des majuscules sous WebKitWebDriver. Le vrai clic droit est
  // couvert par Vitest ; ici, le menu s'ouvre sur le même événement `contextmenu`.
  await browser.execute((label) => {
    const target = Array.from(
      document.querySelectorAll('.switcher__popover [role="menuitemradio"]'),
    ).find((t) => t.querySelector(".switcher__name")?.textContent.trim() === label);
    if (!target) throw new Error(`workspace « ${label} » introuvable`);
    const box = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        button: 2,
        clientX: box.right - 24,
        clientY: box.top + box.height / 2,
      }),
    );
  }, name);
  await browser.$('.switcher__context [role="menu"]').waitForDisplayed();
}

async function chooseWorkspaceAction(label: string): Promise<void> {
  await browser.$(".switcher__context").$(`[role="menuitem"]*=${label}`).click();
}

/** Crée un workspace par « + Nouveau workspace » du sélecteur (S3). */
async function createWorkspace(name: string): Promise<void> {
  await openSwitcher();
  await clickSwitcherEntry("Nouveau workspace", "menuitem");
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

/** Clique, depuis la page, le premier élément `selector` dont le texte contient `text`. */
async function clickText(selector: string, text: string): Promise<void> {
  await browser.execute(
    (css, label) => {
      const target = Array.from(document.querySelectorAll<HTMLElement>(css)).find((element) =>
        element.textContent.includes(label),
      );
      if (!target) throw new Error(`« ${label} » introuvable dans ${css}`);
      target.click();
    },
    selector,
    text,
  );
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
  it("premier lancement : seul l'onglet « Tous les projets », aucun workspace (1a, R3, A1)", async () => {
    expect(await heading(browser)).toBe("Bienvenue dans Arnod");
    expect(await currentWorkspace()).toBe("Tous les projets");
    expect(await workspaceNames()).toEqual(["Tous les projets"]);
    expect(await browser.$("aside").getText()).toBe("Aucun projet pour l'instant.");
    const main = await browser.$("main").getText();
    expect(main).toContain("Essayer une étape");
    expect(main).toContain("Créer un projet");
    expect(main).toContain(
      "Un projet = un dossier sur votre poste, prêt à être partagé avec l'équipe.",
    );

    const state = readState(sandbox);
    expect(state.workspaces).toEqual([]);
    expect(state.projects).toEqual([]);
    expect(state.lastWorkspaceId).toBeNull();
    expect(state.inboxProjectId).toBeUndefined();
    await screenshot(browser, "01-premier-lancement");
  });

  it("choisir REST ouvre son formulaire, créer et importer se réduisent à une ligne (A3, A4)", async () => {
    await browser.$('main button[title^="REST"]').click();
    const form = browser.$('main [role="group"][aria-label="Formulaire de la nature REST"]');
    await form.waitForDisplayed();
    expect(await browser.$("main .first-run__compact").isDisplayed()).toBe(true);
    expect(await browser.$("main .first-run__cards").isExisting()).toBe(false);
    expect(await browser.$("main .try__form").$("button=Envoyer").isDisplayed()).toBe(true);
    // POST montre le type et le corps (champs conditionnels de la définition).
    await form.$("button=POST").click();
    await form.$("textarea").waitForDisplayed();
    await screenshot(browser, "01b-accueil-rest");

    // Échap ferme le formulaire : les cartes reviennent.
    await browser.$("main .try__input").click();
    await browser.keys("Escape");
    await browser.$("main .first-run__cards").waitForDisplayed();
    expect(await form.isExisting()).toBe(false);
  });

  it("crée le premier projet sur disque, sans workspace, et l'ouvre (P1, P4, P5)", async () => {
    // Les fenêtres fermées restent dans le DOM, hors de `main` : leurs champs ne comptent pas.
    await browser.$("main .first-run__card form input").setValue("boutique-tests");
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
    // Le projet est une collection OpenCollection 1.0.0.
    const manifest = readFileSync(path.join(expected, "opencollection.yml"), "utf8");
    expect(manifest).toMatch(/^opencollection: "?1\.0\.0"?\n/);
    expect(manifest).toContain("name: boutique-tests");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).toEqual(["boutique-tests"]);
    expect(state.workspaces).toEqual([]);
    await screenshot(browser, "02-projet-cree");
  });

  it("refuse un nom déjà pris, message du cœur sous le champ (R9)", async () => {
    // Depuis un projet, l'onglet de départ ramène à sa liste.
    await chooseWorkspace("Tous les projets");
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

  it("« + Nouveau workspace » du sélecteur crée un workspace et l'ouvre (S3)", async () => {
    await createWorkspace("Boutique");

    expect(await workspaceNames()).toEqual(["Tous les projets", "Boutique"]);
    expect(await currentWorkspace()).toBe("Boutique");
    expect(await browser.$("main").getText()).toContain("Aucun projet pour l'instant");
    const state = readState(sandbox);
    expect(state.workspaces.map((w) => w.name)).toEqual(["Boutique"]);
    expect(state.lastWorkspaceId).toBe(workspaceNamed("Boutique")?.id);
  });

  it("le sélecteur et les actions d'un workspace s'affichent en entier, par-dessus la zone principale (S2, S4)", async () => {
    await openWorkspaceActions("Boutique");
    const layout = await browser.execute(() => {
      const menu = document.querySelector('.switcher__context [role="menu"]');
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
    const popover = await browser.execute(() => {
      const menu = document.querySelector(".switcher__popover");
      if (!menu) return null;
      const box = menu.getBoundingClientRect();
      // Le bas du menu du sélecteur passe par-dessus la zone principale.
      return menu.contains(document.elementFromPoint(box.left + 4, box.bottom - 4));
    });
    expect(popover).toBe(true);
    await screenshot(browser, "02b-selecteur-workspace");

    await browser.keys("Escape");
    await browser.$('.switcher__context [role="menu"]').waitForExist({ reverse: true });
    await closeSwitcher();
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

    expect(existsSync(path.join(sandbox.home, "arnod", "paiements-v3", "opencollection.yml"))).toBe(
      true,
    );
    const state = readState(sandbox);
    const id = state.projects.find((p) => p.name === "paiements-v3")?.id;
    expect(
      state.workspaces.map((w) => [w.name, id !== undefined && w.projectIds.includes(id)]),
    ).toEqual([["Boutique", true]]);

    await chooseWorkspace("Boutique");
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

  it("renomme un workspace depuis le sélecteur (S4)", async () => {
    await openWorkspaceActions("Boutique");
    await chooseWorkspaceAction("Renommer");
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    expect(await dialog.$("h2").getText()).toBe("Renommer le workspace");
    const input = dialog.$("input");
    expect(await input.getValue()).toBe("Boutique");
    await input.setValue("E-commerce");
    await browser.keys("Enter");

    await waitForHeading(browser, "E-commerce");
    expect(await currentWorkspace()).toBe("E-commerce");
    expect(await workspaceNames()).toEqual(["Tous les projets", "E-commerce"]);
    expect(workspaceNamed("E-commerce")?.projectIds).toHaveLength(2);
  });

  it("glisser une ligne de « Tous les projets » sur le sélecteur ajoute le projet (S6)", async () => {
    await createWorkspace("Recette");
    await chooseWorkspace("Tous les projets");
    await waitForHeading(browser, "Tous les projets · 2");

    // Le glisser natif ne se pilote pas par WebDriver sous WebKit : on émet les
    // événements HTML du glisser-déposer dans la vraie WebView. Le sélecteur
    // s'ouvre au survol, puis le projet se dépose sur un workspace.
    await browser.execute(() => {
      const row = Array.from(document.querySelectorAll("main tbody tr")).find(
        (tr) => tr.querySelector(".all__project")?.textContent.trim() === "boutique-tests",
      );
      const trigger = document.querySelector(".switcher__trigger");
      if (!row || !trigger) throw new Error("ligne ou sélecteur introuvable");
      const dataTransfer = new DataTransfer();
      (window as unknown as { __drag: { row: Element; dataTransfer: DataTransfer } }).__drag = {
        row,
        dataTransfer,
      };
      for (const [element, type] of [
        [row, "dragstart"],
        [trigger, "dragenter"],
      ] as const) {
        element.dispatchEvent(
          new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer }),
        );
      }
    });
    await browser.$('.switcher__popover [role="menu"]').waitForDisplayed();
    await browser.execute(() => {
      const { row, dataTransfer } = (
        window as unknown as { __drag: { row: Element; dataTransfer: DataTransfer } }
      ).__drag;
      const target = Array.from(
        document.querySelectorAll('.switcher__popover [role="menuitemradio"]'),
      ).find((t) => t.querySelector(".switcher__name")?.textContent.trim() === "Recette");
      if (!target) throw new Error("workspace « Recette » absent du sélecteur");
      const fire = (element: Element, type: string) =>
        element.dispatchEvent(
          new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer }),
        );
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
    await openWorkspaceActions("Recette");
    await chooseWorkspaceAction("Gérer les workspaces…");
    await waitForHeading(browser, "Workspaces");
    expect(await browser.$("aside").isExisting()).toBe(false);
    expect(await browser.$("main").getText()).not.toContain("par défaut");
    await screenshot(browser, "04-gerer-workspaces");

    await browser.$("main").$("button=Supprimer ce workspace").click();
    expect(await toastText(browser)).toContain(
      "Workspace « Recette » supprimé · 1 projet conservé dans « Tous les projets ».",
    );
    expect(readState(sandbox).workspaces.map((w) => w.name)).toEqual(["E-commerce"]);
    expect(await workspaceNames()).toEqual(["Tous les projets", "E-commerce"]);

    await browser.$(".toast").$("button=Annuler").click();
    await browser.waitUntil(() => readState(sandbox).workspaces.length === 2);
    expect(await workspaceNames()).toEqual(["Tous les projets", "E-commerce", "Recette"]);
    await dismissToast();
    await browser.$("main").$("button*=Retour").click();
    await waitForHeading(browser, "Tous les projets · 2");
  });

  it("« Tous les projets » liste chaque projet une fois avec ses workspaces (2e, O1)", async () => {
    expect(await browser.$$("main tbody .all__project").map((name) => name.getText())).toEqual([
      "boutique-tests",
      "paiements-v3",
    ]);
    expect(
      await browser.$$("main tbody tr:first-child .chip").map((chip) => chip.getText()),
    ).toEqual(["E-commerce", "Recette"]);
    // La barre latérale reste : on en sort en ouvrant un projet ou par le sélecteur (2e, 2f).
    // Noms lus par textContent, comme ceux du sélecteur : sous WebKitWebDriver, getText
    // (innerText) renvoie une chaîne vide pour un nom en text-overflow: ellipsis.
    expect(
      await browser.execute(() =>
        Array.from(document.querySelectorAll("aside .project__name")).map((name) =>
          name.textContent.trim(),
        ),
      ),
    ).toEqual(["boutique-tests", "paiements-v3"]);
    expect(await browser.$("aside .sidebar__hint").getText()).toBe(
      "Ouvrez un projet ici, ou choisissez un workspace dans le sélecteur.",
    );
    await screenshot(browser, "05-tous-les-projets");
  });

  it("au redémarrage, rouvre le dernier onglet utilisé (R7)", async () => {
    await restart();
    await waitForHeading(browser, "Tous les projets · 2");
    expect(readState(sandbox).lastWorkspaceId).toBeNull();

    await chooseWorkspace("E-commerce");
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

  it("inscrit un projet du format précédent et le convertit (P12, R6)", async () => {
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
    // Converti une fois en collection OpenCollection, identifiant gardé.
    expect(existsSync(path.join(folder, "arnod.yml"))).toBe(false);
    const root = readFileSync(path.join(folder, "opencollection.yml"), "utf8");
    expect(root).toContain("name: catalogue-legacy");
    expect(root).toContain("id: 7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10");
  });

  it("oublie un projet sans toucher à ses fichiers, puis annule (P11, R6)", async () => {
    await chooseWorkspace("Tous les projets");
    await waitForHeading(browser, "Tous les projets · 3");

    await browser.$('button[aria-label="Oublier catalogue-legacy"]').click();
    await waitForHeading(browser, "Tous les projets · 2");

    expect(await toastText(browser)).toContain("Ses fichiers restent sur le disque.");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).not.toContain("catalogue-legacy");
    expect(state.workspaces.flatMap((w) => w.projectIds)).not.toContain(
      "7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10",
    );
    expect(
      existsSync(path.join(sandbox.home, "dev", "catalogue-legacy", "opencollection.yml")),
    ).toBe(true);

    await browser.$(".toast").$("button=Annuler").click();
    await waitForHeading(browser, "Tous les projets · 3");
    expect(readState(sandbox).projects.map((p) => p.name)).toContain("catalogue-legacy");
  });
});

describe("Fenêtres et zones", () => {
  let mainWindow: string;

  it("ouvre les requêtes en onglets et divise la zone (F1, F2, F4)", async () => {
    await chooseWorkspace("E-commerce");
    await waitForHeading(browser, "E-commerce");
    await browser.$("aside nav").$("button*=boutique-tests").click();
    await waitForHeading(browser, "boutique-tests");

    await browser.$("main").$("button=+ Requête").click();
    await waitForZoneTabs([["Nouvelle requête"]]);
    // « + » de la ligne du projet, puis « Nouvelle requête » (barre latérale du projet, B3).
    await browser.$('aside button[aria-label="Ajouter au projet boutique-tests"]').click();
    await browser.$('[role="menuitem"]*=Nouvelle requête').click();
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
    expect(readWindows(sandbox).windows[0]?.layout?.root).toMatchObject({
      kind: "split",
      axis: "row",
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

  it("le menu de l'environnement n'est pas rogné par la barre latérale, sa gestion tient dans la fenêtre", async () => {
    await browser.$('aside button[aria-label^="Environnement de boutique-tests"]').click();
    await browser.$('aside [role="menu"]').waitForDisplayed();
    const whole = await browser.execute(() => {
      const menu = document.querySelector("aside .menu__popover");
      if (!menu) return null;
      const box = menu.getBoundingClientRect();
      // Près de chaque coin, c'est bien le menu qui est affiché, même hors de la barre latérale.
      const corners = [
        [box.left + 4, box.top + 4],
        [box.right - 4, box.top + 4],
        [box.left + 4, box.bottom - 4],
        [box.right - 4, box.bottom - 4],
      ] as const;
      return corners.every(([x, y]) => menu.contains(document.elementFromPoint(x, y)));
    });
    expect(whole).toBe(true);
    await screenshot(browser, "08b-menu-environnement");

    await browser.$('[role="menuitem"]*=Gérer les environnements').click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    // Aucun champ ne dépasse : le contenu suit la largeur de la fenêtre, sans défilement horizontal.
    const fits = await browser.execute(() => {
      const body = document.querySelector("dialog[open] .dialog__body");
      return body ? body.scrollWidth <= body.clientWidth : null;
    });
    expect(fits).toBe(true);
    await screenshot(browser, "08c-environnements");
    await browser.keys("Escape");
    await browser.$("dialog[open]").waitForExist({ reverse: true });
  });
});

/** Contrat OpenAPI du serveur de test, servi à `url`. */
function animalerie(url: string): string {
  return `openapi: 3.0.3
info:
  title: Animalerie
  version: 2.1.0
servers:
  - url: ${url}
paths:
  /pets:
    get:
      tags: [animaux]
      summary: Lister les animaux
      responses:
        "200":
          description: Liste
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Pet" }
    post:
      tags: [animaux]
      summary: Créer un animal
      requestBody:
        content:
          application/json:
            schema: { $ref: "#/components/schemas/Pet" }
      responses:
        "201":
          description: Créé
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Pet" }
  /pets/{petId}:
    get:
      tags: [animaux]
      summary: Trouver un animal
      parameters:
        - { name: petId, in: path, required: true, schema: { type: integer }, example: 1 }
      responses:
        "200":
          description: Un animal
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Pet" }
components:
  schemas:
    Pet:
      type: object
      required: [id, name]
      properties:
        id: { type: integer }
        name: { type: string }
`;
}

describe("Contrats de services", () => {
  let server: Server;
  let url = "";
  /** Le serveur répond un identifiant texte : écart au contrat. */
  let broken = false;

  beforeAll(async () => {
    server = createServer((request, response) => {
      const json = (status: number, body: unknown): void => {
        response.writeHead(status, { "Content-Type": "application/json" });
        response.end(JSON.stringify(body));
      };
      const pet = { id: broken ? "un" : 1, name: "Rex" };
      if (request.method === "GET" && request.url === "/pets") json(200, [pet]);
      else if (request.method === "GET" && request.url === "/pets/1") json(200, pet);
      else if (request.method === "POST" && request.url === "/pets") json(201, pet);
      else json(404, { message: "absent" });
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    url = `http://127.0.0.1:${String((server.address() as AddressInfo).port)}`;
  });

  afterAll(async () => {
    await new Promise<void>((resolve) => {
      server.close(() => {
        resolve();
      });
    });
  });

  const project = () => path.join(sandbox.home, "arnod", "boutique-tests");

  it("ajoute un contrat collé, présenté dans son onglet (C2, C5)", async () => {
    await browser.$('button[aria-label="Ajouter un contrat à boutique-tests"]').click();
    await clickText('[role="menuitem"]', "Coller un contrat");
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    // Texte collé d'un bloc, comme un Ctrl+V.
    await browser.execute((text) => {
      const area = document.querySelector<HTMLTextAreaElement>("dialog[open] textarea");
      if (!area) throw new Error("zone de texte introuvable");
      area.value = text;
      area.dispatchEvent(new Event("input", { bubbles: true }));
    }, animalerie(url));
    await dialog.$("button=Ajouter").click();

    const name = browser.$(".contract__name");
    await name.waitForDisplayed({ timeout: 10_000 });
    expect(await name.getText()).toBe("Animalerie");
    expect(await browser.$(".contract__coverage").getText()).toBe("3 opérations · 0 avec requête");
    expect(await browser.$(".contract-row__version").getText()).toBe("2.1.0");
    expect(readFileSync(path.join(project(), "contracts", "animalerie.yaml"), "utf8")).toBe(
      animalerie(url),
    );
  });

  it("crée les requêtes du contrat et renseigne baseUrl (C6, R8)", async () => {
    await browser.$(".contract__actions").$("button*=Créer les requêtes (3)").click();
    await browser.waitUntil(
      async () =>
        (await browser.$(".contract__coverage").getText()) === "3 opérations · 3 avec requête",
      { timeout: 10_000 },
    );
    const request = readFileSync(
      path.join(project(), "Animalerie", "animaux", "trouver-un-animal.yml"),
      "utf8",
    );
    // Requête OpenCollection, liée par une étiquette ; la conformité n'est pas écrite (R8).
    expect(request).toContain("type: http");
    // Endpoint : paramètre de chemin dans l'adresse, réponses attendues du contrat (R8).
    expect(request).toContain("{{baseUrl}}/pets/:petId");
    expect(request).toContain("type: path");
    expect(request).toContain("name: Un animal");
    expect(request).toContain("contract:contracts/animalerie.yaml#GET /pets/{petId}");
    expect(request).toContain("operator: eq");
    expect(request).not.toContain("matchesContract");
    expect(readFileSync(path.join(project(), "environments", "local.yml"), "utf8")).toContain(url);

    // Rangées sous le contrat, qui se déplie (RC7) ; aucune dans le référentiel de tests (RC6).
    const operation = browser.$('aside [aria-label="Ouvrir GET /pets/{petId}"]');
    await operation.waitForExist({
      timeout: 10_000,
      timeoutMsg: "opération absente du contrat déplié",
    });
    // Texte du DOM : une ligne hors de la partie visible de la barre latérale compte aussi.
    const sidebar = () =>
      browser.execute(() => ({
        resources: Array.from(document.querySelectorAll("aside .crow__resource .crow__name")).map(
          (n) => n.textContent.trim(),
        ),
        tests: Array.from(document.querySelectorAll('aside [role="treeitem"]')).map((i) =>
          i.textContent.trim(),
        ),
      }));
    expect((await sidebar()).resources).toEqual(["/pets"]);
    expect((await sidebar()).tests.some((text) => text.includes("Animalerie"))).toBe(false);
    // Un clic sur l'opération ouvre son endpoint (RC2).
    await operation.click();
    await browser.waitUntil(async () => (await zoneTabs()).flat().includes("Trouver un animal"), {
      timeout: 10_000,
      timeoutMsg: "endpoint de l'opération non ouvert",
    });
    await clickText(".zone__tab-button", "Animalerie");
    await screenshot(browser, "09-contrat");
  });

  it("exécute le contrat : réponses conformes (C10)", async () => {
    await browser.$(".contract__actions").$("button*=Exécuter (3)").click();
    const progress = browser.$(".run__progress");
    await progress.waitForDisplayed({ timeout: 10_000 });
    await browser.waitUntil(async () => (await progress.getText()).includes("tout est réussi"), {
      timeout: 15_000,
      timeoutMsg: "run du contrat non réussi",
    });
    expect(await browser.$(".run__title").getText()).toContain("Run du contrat « Animalerie »");
    expect(await browser.$$(".run__item--passed").length).toBe(3);
  });

  it("un écart au contrat fait échouer la conformité, avec le chemin (C8)", async () => {
    broken = true;
    await clickText(".zone__tab-button", "Animalerie");
    await browser.$(".contract__actions").$("button*=Exécuter (3)").click();
    const progress = browser.$(".run__progress");
    await browser.waitUntil(async () => (await progress.getText()).includes("3 en échec"), {
      timeout: 15_000,
      timeoutMsg: "écart au contrat non détecté",
    });
    const errors = await browser.$$(".run__error").map((e) => e.getText());
    expect(errors.some((text) => text.includes("body[0].id : attendu entier"))).toBe(true);
    expect(errors.some((text) => text.includes("body.id : attendu entier"))).toBe(true);
    await screenshot(browser, "10-run-contrat");
    broken = false;
  });

  it("une requête liée est vérifiée contre le contrat à l'envoi (R8, S3)", async () => {
    await clickText(".zone__tab-button", "Animalerie");
    await browser.$('[aria-label="Ouvrir la requête de GET /pets/{petId}"]').click();
    // Éditeur de la requête liée (d'autres zones peuvent en afficher d'autres).
    const inEditor = (action: "verifications" | "send" | "state") =>
      browser.execute((step) => {
        const editor = Array.from(document.querySelectorAll<HTMLElement>(".editor")).find((e) =>
          e.querySelector(".editor__contract")?.textContent.includes("GET /pets/{petId}"),
        );
        if (!editor) throw new Error("éditeur de la requête liée introuvable");
        const buttons = Array.from(editor.querySelectorAll<HTMLButtonElement>("button"));
        if (step === "verifications")
          buttons.find((b) => b.textContent.includes("Vérifications"))?.click();
        if (step === "send") buttons.find((b) => b.textContent.trim() === "Envoyer")?.click();
        const contract = editor.querySelector(".assertion--contract");
        return {
          contract: contract?.className ?? null,
          operators: editor.querySelectorAll('[aria-label="Opérateur de la vérification 1"] option')
            .length,
        };
      }, action);
    await browser.waitUntil(async () => (await inEditor("verifications")).contract !== null, {
      timeout: 10_000,
      timeoutMsg: "conformité au contrat absente des vérifications",
    });
    expect((await inEditor("state")).operators).toBe(28);
    await inEditor("send");
    await browser.waitUntil(
      async () => (await inEditor("state")).contract?.includes("assertion--ok") === true,
      { timeout: 15_000, timeoutMsg: "conformité non vérifiée à l'envoi" },
    );
    await screenshot(browser, "11-requete-liee");
  });

  it("configure l'endpoint : réglages et réponses attendues (EP7 à EP9)", async () => {
    // Onglet de la requête liée, dans sa zone (d'autres zones peuvent en afficher d'autres).
    const inTab = (step: "expected" | "fromLast" | "back" | "settings" | "ten" | "state") =>
      browser.execute((action) => {
        const tab = Array.from(document.querySelectorAll<HTMLElement>(".request-tab")).find((t) =>
          t.querySelector(".editor__contract")?.textContent.includes("GET /pets/{petId}"),
        );
        if (!tab) throw new Error("onglet de la requête liée introuvable");
        const buttons = Array.from(tab.querySelectorAll<HTMLButtonElement>("button"));
        const find = (text: string) => buttons.find((b) => b.textContent.trim().startsWith(text));
        if (action === "expected") find("Réponses attendues")?.click();
        if (action === "fromLast") find("Depuis la dernière réponse reçue")?.click();
        if (action === "back") find("← Réponses attendues")?.click();
        if (action === "settings") find("Réglages")?.click();
        if (action === "ten") {
          tab
            .querySelector<HTMLButtonElement>(
              '[aria-labelledby="setting-timeoutMs"] button:nth-child(3)',
            )
            ?.click();
        }
        return {
          rows: Array.from(tab.querySelectorAll(".expected__name")).map((n) => n.textContent),
          saved: tab.querySelector(".editor__saved")?.textContent.trim() ?? "",
          fromLast: find("Depuis la dernière réponse reçue")?.disabled ?? true,
        };
      }, step);
    await browser.waitUntil(async () => (await inTab("expected")).rows.length === 1, {
      timeout: 10_000,
      timeoutMsg: "réponse attendue du contrat absente",
    });
    expect((await inTab("state")).rows).toEqual(["Un animal"]);
    expect((await inTab("state")).fromLast).toBe(false);
    await inTab("fromLast");
    await browser.waitUntil(async () => (await inTab("back")).rows.length === 2, {
      timeout: 10_000,
      timeoutMsg: "réponse reçue non reprise",
    });
    await inTab("settings");
    await inTab("ten");
    await browser.waitUntil(async () => (await inTab("state")).saved === "✓ Enregistré", {
      timeout: 10_000,
      timeoutMsg: "endpoint non enregistré",
    });
    const file = readFileSync(
      path.join(project(), "Animalerie", "animaux", "trouver-un-animal.yml"),
      "utf8",
    );
    expect(file).toContain("timeout: 10000");
    expect(file).toContain("examples:");
    expect(file).toMatch(/name: "?Reçue le \d\d\/\d\d à \d\d:\d\d/);
    expect(file).toContain("status: 200");
    await screenshot(browser, "12-endpoint");
  });

  it("trois fenêtres : barre latérale, requête et résultat qui suit la requête active (D1 à D6)", async () => {
    const mainWindow = await browser.getWindowHandle();
    // Dans la requête liée : éditeur, envoi, ⧉ du résultat.
    const linked = (action: "detach" | "send" | "state") =>
      browser.execute((step) => {
        const tab = Array.from(document.querySelectorAll<HTMLElement>(".request-tab")).find((t) =>
          t.querySelector(".editor__contract")?.textContent.includes("GET /pets/{petId}"),
        );
        if (!tab) throw new Error("onglet de la requête liée introuvable");
        if (step === "detach") {
          tab
            .querySelector<HTMLButtonElement>(
              'button[aria-label="Ouvrir le résultat dans une fenêtre"]',
            )
            ?.click();
        }
        if (step === "send") {
          Array.from(tab.querySelectorAll<HTMLButtonElement>("button"))
            .find((b) => b.textContent.trim() === "Envoyer")
            ?.click();
        }
        return { response: tab.querySelector(".request-tab__response") !== null };
      }, action);

    await linked("detach");
    let handles = await waitForWindows(2);
    const result = handles.find((h) => h !== mainWindow) ?? "";
    expect(readWindows(sandbox).windows.map((w) => [w.kind, w.follow])).toEqual([
      ["main", true],
      ["content", false],
    ]);
    await browser.waitUntil(async () => !(await linked("state")).response, {
      timeoutMsg: "le résultat reste dans l'onglet de la requête",
    });

    await clickHidden('button[aria-label="Détacher la barre latérale dans une fenêtre"]');
    handles = await waitForWindows(3);
    const sidebar = handles.find((h) => h !== mainWindow && h !== result) ?? "";

    // Envoi depuis la fenêtre principale : la réponse arrive dans la fenêtre de résultat.
    await browser.switchToWindow(mainWindow);
    await linked("send");
    await browser.switchToWindow(result);
    const name = browser.$(".result-tab__name");
    await name.waitForDisplayed({ timeout: 15_000 });
    expect((await name.getText()).toLowerCase()).toContain("animal");
    expect(await browser.$(".content-window__follow").getText()).toBe("⇆ Suit la requête active");
    await browser.waitUntil(async () => (await browser.$(".response").getText()).includes("200"), {
      timeout: 15_000,
      timeoutMsg: "réponse absente de la fenêtre de résultat",
    });
    await screenshot(browser, "14-fenetre-resultat");
    await browser.switchToWindow(sidebar);
    await browser.$(".sidebar-window__pilot").waitForDisplayed({ timeout: 15_000 });
    await screenshot(browser, "15-fenetre-barre");
    await browser.switchToWindow(mainWindow);
    await screenshot(browser, "13-fenetre-requete");

    // Rattacher : le résultat revient dans l'onglet de la requête (R4).
    await browser.switchToWindow(result);
    await browser.$(".content-window__bar").$("button=Rattacher").click();
    await waitForWindows(2);
    await browser.switchToWindow(sidebar);
    await browser.$(".sidebar-window__footer").$("button=Rattacher").click();
    await waitForWindows(1);
    await browser.switchToWindow(mainWindow);
    await browser.waitUntil(async () => (await linked("state")).response, {
      timeoutMsg: "le résultat n'est pas revenu dans l'onglet",
    });
    expect(readWindows(sandbox).windows.map((w) => w.kind)).toEqual(["main"]);
  });
});

describe("Accueil, catalogue des étapes et Non classés", () => {
  let server: Server;
  let url = "";
  let host = "";

  beforeAll(async () => {
    server = createServer((request, response) => {
      const json = (status: number, body: unknown): void => {
        response.writeHead(status, { "Content-Type": "application/json" });
        response.end(JSON.stringify(body));
      };
      if (request.method === "GET" && request.url === "/sante") {
        json(200, { etat: "ok", version: "1.4.2" });
      } else if (request.method === "POST" && request.url === "/commandes") {
        json(201, { id: 42, statut: "créée" });
      } else json(404, { message: "absent" });
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const port = String((server.address() as AddressInfo).port);
    url = `http://127.0.0.1:${port}`;
    host = `127.0.0.1:${port}`;
  });

  afterAll(async () => {
    await new Promise<void>((resolve) => {
      server.close(() => {
        resolve();
      });
    });
  });

  const inbox = () => path.join(sandbox.home, "arnod", "Non classés");
  const contractFile = () => path.join(inbox(), "contracts", `${host.replace(/[.:]/g, "-")}.yaml`);

  async function showAllProjects(): Promise<void> {
    // Les zones ont aussi leurs listes d'onglets : on vise celle des workspaces.
    await chooseWorkspace("Tous les projets");
    await browser.waitUntil(async () => (await heading(browser)).startsWith("Tous les projets"), {
      timeoutMsg: "« Tous les projets » attendu",
    });
  }

  /** Colle `text` d'un bloc dans l'entrée « Essayer une étape », comme un Ctrl+V. */
  async function paste(text: string): Promise<void> {
    await browser.execute((value) => {
      const input = document.querySelector<HTMLInputElement>("main .try__input");
      if (!input) throw new Error("entrée « Essayer une étape » introuvable");
      input.value = value;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, text);
  }

  /** Envoie l'essai reconnu et attend son onglet. */
  async function sendTrial(name: string): Promise<void> {
    const submit = browser.$("main .try__form").$("button=Envoyer");
    await submit.waitForDisplayed({ timeoutMsg: "nature REST non reconnue" });
    await submit.click();
    await browser.waitUntil(async () => (await zoneTabs()).flat().includes(name), {
      timeout: 15_000,
      timeoutMsg: `onglet « ${name} » attendu`,
    });
    await browser.waitUntil(async () => (await toastText(browser)).includes("Essai gardé"), {
      timeoutMsg: "notification de l'essai attendue",
    });
  }

  it("le catalogue des étapes liste REST, intégrée à Arnod (K1, K2)", async () => {
    await showAllProjects();
    await browser.$("main .try").$("button*=Parcourir le catalogue").click();
    await waitForHeading(browser, "Catalogue des étapes");
    const table = await browser.$("main .catalog__table").getText();
    expect(table).toContain("REST");
    expect(table).toContain("Intégrée");
    expect(await browser.$("main").getText()).toContain("arnod-steps");
    await screenshot(browser, "16-catalogue-etapes");

    await browser.$("main").$("button=← Retour").click();
    await browser.waitUntil(async () => (await heading(browser)).startsWith("Tous les projets"), {
      timeoutMsg: "retour à « Tous les projets » attendu",
    });
  });

  it("une adresse collée crée « Non classés », son contrat déduit et « Essai 1 » (A2, A5, N1)", async () => {
    await paste(`${url}/sante`);
    await sendTrial("Essai 1");

    // « Non classés » : un vrai projet, premier de la liste, hors des workspaces.
    const state = readState(sandbox);
    const project = state.projects.find((p) => p.name === "Non classés");
    expect(project).toBeDefined();
    expect(state.inboxProjectId).toBe(project?.id);
    expect(state.workspaces.some((w) => w.projectIds.includes(project?.id ?? ""))).toBe(false);
    expect(existsSync(path.join(inbox(), "opencollection.yml"))).toBe(true);

    // Contrat déduit du serveur et requête liée à son opération.
    const contract = readFileSync(contractFile(), "utf8");
    expect(contract).toContain("x-arnod-deduced: true");
    expect(contract).toContain("/sante:");
    const request = readFileSync(path.join(inbox(), host.replace(":", " "), "essai-1.yml"), "utf8");
    expect(request).toContain("name: Essai 1");
    expect(request).toContain("contract:contracts/");
    expect(request).toContain("GET /sante");

    // Dans la barre latérale, l'essai est sous le contrat déduit, à son opération (RC8).
    const operation = browser.$('aside [aria-label="Ouvrir GET /sante"]');
    await operation.waitForExist({
      timeout: 10_000,
      timeoutMsg: "essai absent du contrat déduit",
    });
    const tests = await browser.execute(() =>
      Array.from(document.querySelectorAll('aside [role="treeitem"]')).map((i) => i.textContent),
    );
    expect(tests.some((text) => text.includes("Essai 1"))).toBe(false);

    // La réponse est déjà dans l'onglet, vérifications proposées comprises.
    const status = await browser.execute(() => {
      const tab = Array.from(document.querySelectorAll<HTMLElement>(".request-tab")).find((t) =>
        t.querySelector(".editor__contract")?.textContent.includes("GET /sante"),
      );
      return tab?.querySelector(".response__status")?.textContent.trim() ?? "";
    });
    expect(status).toContain("200");
    await screenshot(browser, "17-essai-non-classes");
  });

  it("une méthode en tête est reprise ; l'essai suivant enrichit le même contrat (A2, N2, N3)", async () => {
    await showAllProjects();
    await paste(`POST ${url}/commandes`);
    const form = browser.$('main [role="group"][aria-label="Formulaire de la nature REST"]');
    await form.waitForDisplayed();
    // Options cochées : méthode, puis type du corps (visible pour un POST).
    const checked = () => form.$$('[role="radio"][aria-checked="true"]').map((r) => r.getText());
    await browser.waitUntil(async () => (await checked())[0] === "POST", {
      timeoutMsg: "méthode POST non reprise de la saisie",
    });
    expect(await checked()).toEqual(["POST", "JSON"]);
    expect(await browser.$("main .try__input").getValue()).toBe(`${url}/commandes`);
    await form.$("textarea").setValue('{"article":"T-shirt"}');
    await sendTrial("Essai 2");

    const contract = readFileSync(contractFile(), "utf8");
    expect(contract).toContain("/sante:");
    expect(contract).toContain("/commandes:");
    expect(contract).toMatch(/"?201"?:/);
    expect(existsSync(path.join(inbox(), host.replace(":", " "), "essai-2.yml"))).toBe(true);
    // « Non classés » reste premier dans « Tous les projets ».
    await showAllProjects();
    const rows = await browser.$$("main tr.all__row").map((row) => row.getText());
    expect(rows[0]).toContain("Non classés");
    expect(rows[0]).toContain("Hors workspaces");
    await screenshot(browser, "18-tous-les-projets-non-classes");
  });
});

describe("Premier usage : vérifications, run et statuts", () => {
  let server: Server;
  let url = "";

  beforeAll(async () => {
    server = createServer((request, response) => {
      const json = (status: number, body: unknown): void => {
        response.writeHead(status, { "Content-Type": "application/json" });
        response.end(JSON.stringify(body));
      };
      // Pas d'en-tête Location sur la création : la vérification ajoutée échoue (1e).
      if (request.method === "GET" && request.url === "/commandes") {
        json(200, [{ id: 1, amount: "129.90" }]);
      } else if (request.method === "POST" && request.url === "/commandes") {
        json(201, { id: 2, status: "pending" });
      } else json(404, { message: "absent" });
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    url = `http://127.0.0.1:${String((server.address() as AddressInfo).port)}`;
  });

  afterAll(async () => {
    await new Promise<void>((resolve) => {
      server.close(() => {
        resolve();
      });
    });
  });

  const project = () => path.join(sandbox.home, "arnod", "boutique-tests");

  /** Ligne de l'arborescence (référentiel de tests) dont le nom est `name`. */
  async function treeRow(name: string): Promise<{ status: string; count: string } | null> {
    return browser.execute((label) => {
      const item = Array.from(document.querySelectorAll('aside [role="treeitem"]')).find(
        (row) => row.querySelector(".row__name")?.textContent.trim() === label,
      );
      if (!item) return null;
      return {
        status: item.querySelector(".row__status")?.textContent.trim() ?? "",
        count: item.querySelector(".row__count")?.getAttribute("title") ?? "",
      };
    }, name);
  }

  /** Ouvre le menu « ⋯ » d'une ligne de l'arborescence et choisit `action`. */
  async function rowAction(name: string, action: string): Promise<void> {
    await browser.execute((label) => {
      const row = Array.from(document.querySelectorAll('aside li[role="presentation"]')).find(
        (li) => li.querySelector(".row__name")?.textContent.trim() === label,
      );
      const more = row?.querySelector<HTMLElement>(".row__more");
      if (!more) throw new Error(`ligne « ${label} » introuvable`);
      more.click();
    }, name);
    await clickText('aside [role="menuitem"]', action);
  }

  /** Saisit `name` dans la fenêtre de nom ouverte et valide. */
  async function nameIt(name: string): Promise<void> {
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    await dialog.$("input").setValue(name);
    await browser.keys("Enter");
    await dialog.waitForExist({ reverse: true, timeout: 10_000 });
  }

  type Step =
    | { kind: "url"; value: string }
    | { kind: "method"; value: string }
    | { kind: "click"; value: string }
    | { kind: "addCheck"; target: string; operator: string }
    | { kind: "editorChecks" }
    | { kind: "state" };

  /**
   * Agit dans l'onglet de la requête `name` (d'autres zones en montrent d'autres) et renvoie son
   * état : statut de la réponse, verdict, encadrés d'échec, vérifications de l'éditeur.
   */
  function inRequest(name: string, step: Step) {
    return browser.execute(
      (label, action) => {
        const tab = Array.from(document.querySelectorAll<HTMLElement>(".request-tab")).find(
          (t) => t.querySelector<HTMLInputElement>(".meta__input")?.value === label,
        );
        if (!tab) throw new Error(`onglet « ${label} » introuvable`);
        const input = (el: HTMLInputElement | HTMLSelectElement | null, value: string): void => {
          if (!el) throw new Error("champ introuvable");
          el.value = value;
          el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? "change" : "input"));
        };
        const button = (text: string): HTMLButtonElement | undefined =>
          Array.from(tab.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
            b.textContent.trim().startsWith(text),
          );
        if (action.kind === "url") input(tab.querySelector(".editor__url"), action.value);
        if (action.kind === "method") input(tab.querySelector(".editor__method"), action.value);
        if (action.kind === "click") {
          const found = button(action.value);
          if (!found) throw new Error(`bouton « ${action.value} » introuvable`);
          found.click();
        }
        // Onglet « Vérifications » de l'éditeur (celui de la réponse a son propre onglet).
        const editorChecks = (): void => {
          Array.from(tab.querySelectorAll<HTMLButtonElement>('.editor [role="tab"]'))
            .find((b) => b.textContent.includes("Vérifications"))
            ?.click();
        };
        if (action.kind === "editorChecks") editorChecks();
        if (action.kind === "addCheck") {
          editorChecks();
          const rows = tab.querySelectorAll(".assertions .assertion").length;
          button("+ Ajouter une vérification")?.click();
          const n = String(rows + 1);
          setTimeout(() => {
            input(tab.querySelector(`[aria-label="Cible de la vérification ${n}"]`), action.target);
            input(
              tab.querySelector(`[aria-label="Opérateur de la vérification ${n}"]`),
              action.operator,
            );
          }, 50);
        }
        return {
          status: tab.querySelector(".response__status")?.textContent.trim() ?? "",
          verdict: tab.querySelector(".response__verdict")?.textContent.trim() ?? "",
          failures: Array.from(tab.querySelectorAll(".failure__title")).map((f) =>
            f.textContent.trim(),
          ),
          checks: Array.from(
            tab.querySelectorAll<HTMLInputElement>('.assertion input[aria-label^="Cible"]'),
          ).map((i) => i.value),
          saved: tab.querySelector(".editor__saved")?.textContent.trim() ?? "",
        };
      },
      name,
      step,
    );
  }

  async function createRequest(name: string): Promise<void> {
    await rowAction("Commandes", "Nouvelle requête");
    await nameIt(name);
    await browser.waitUntil(async () => (await zoneTabs()).flat().includes(name), {
      timeout: 10_000,
      timeoutMsg: `onglet « ${name} » attendu`,
    });
  }

  async function sendAndWait(name: string, status: string): Promise<void> {
    await inRequest(name, { kind: "click", value: "Envoyer" });
    await browser.waitUntil(
      async () => (await inRequest(name, { kind: "state" })).status.startsWith(status),
      { timeout: 15_000, timeoutMsg: `réponse ${status} attendue pour « ${name} »` },
    );
  }

  it("crée un dossier et une requête, l'envoie et accepte les vérifications proposées (B1, X1, V1)", async () => {
    // La notification de l'essai précédent ne doit pas masquer les captures.
    await dismissToast();
    await browser.$(".toast").waitForExist({ reverse: true, timeout: 15_000 });
    await chooseWorkspace("E-commerce");
    // Les onglets ouverts restent dans la zone principale : le projet se déplie dans la barre.
    const expand = browser.$('aside button[aria-label="Déplier boutique-tests"]');
    if (await expand.isExisting()) await expand.click();
    // « + » n'apparaît qu'au survol ou sur le projet ouvert.
    await clickHidden('aside button[aria-label="Ajouter au projet boutique-tests"]');
    await browser.$('[role="menuitem"]*=Nouveau dossier').click();
    await nameIt("Commandes");
    await browser.waitUntil(async () => (await treeRow("Commandes")) !== null, {
      timeoutMsg: "dossier « Commandes » attendu",
    });
    expect(existsSync(path.join(project(), "Commandes"))).toBe(true);

    await createRequest("Lister les commandes");
    await inRequest("Lister les commandes", { kind: "url", value: `${url}/commandes` });
    await sendAndWait("Lister les commandes", "200");
    await inRequest("Lister les commandes", { kind: "click", value: "Vérifications (" });
    await inRequest("Lister les commandes", { kind: "click", value: "Proposer des vérifications" });
    await inRequest("Lister les commandes", { kind: "editorChecks" });
    await browser.waitUntil(
      async () =>
        (await inRequest("Lister les commandes", { kind: "state" })).checks.includes("status"),
      { timeoutMsg: "vérification de statut proposée attendue" },
    );
    expect((await treeRow("Lister les commandes"))?.status).toBe("✓, réussie");
  });

  it("une vérification en échec se détaille, s'ignore pour ce run et reste dans la requête (VT1, VT2)", async () => {
    await createRequest("Créer une commande");
    await inRequest("Créer une commande", { kind: "method", value: "POST" });
    await inRequest("Créer une commande", { kind: "url", value: `${url}/commandes` });
    await inRequest("Créer une commande", {
      kind: "addCheck",
      target: "headers.Location",
      operator: "exists",
    });
    await browser.waitUntil(
      async () =>
        (await inRequest("Créer une commande", { kind: "state" })).checks.includes(
          "headers.Location",
        ),
      { timeoutMsg: "vérification ajoutée attendue" },
    );
    await sendAndWait("Créer une commande", "201");
    const failed = await inRequest("Créer une commande", { kind: "state" });
    expect(failed.failures).toEqual(["✕ headers.Location"]);
    expect(failed.verdict).toBe("Échec · 1 vérification(s)");
    expect((await treeRow("Créer une commande"))?.status).toBe("✕, en échec");
    await screenshot(browser, "19-verification-en-echec");

    await inRequest("Créer une commande", { kind: "click", value: "Ignorer pour ce run" });
    await browser.waitUntil(
      async () => (await inRequest("Créer une commande", { kind: "state" })).verdict === "Réussie",
      { timeoutMsg: "verdict recalculé attendu" },
    );
    expect((await treeRow("Créer une commande"))?.status).toBe("✓, réussie");
    // La vérification reste dans le fichier : elle sera de nouveau évaluée (R7).
    const file = readFileSync(path.join(project(), "Commandes", "creer-une-commande.yml"), "utf8");
    // OpenCollection écrit le nom de l'en-tête en minuscules.
    expect(file).toContain('res.headers["location"]');
  });

  it("exécute le dossier : résumé, échecs détaillés, résultat par dossier (RU1 à RU4)", async () => {
    await rowAction("Commandes", "Exécuter le dossier");
    const progress = browser.$(".run__progress");
    await browser.waitUntil(async () => (await progress.getText()).includes("1 en échec"), {
      timeout: 15_000,
      timeoutMsg: "run du dossier attendu",
    });
    expect(await progress.getText()).toBe("1 réussi, 1 en échec");
    expect(await browser.$(".run__meta").getText()).toMatch(
      /^run #\d+ · Local · 8 en même temps · \d/,
    );
    expect(await browser.$('.run__tab[aria-selected="true"]').getText()).toContain("Échecs");
    const failures = await browser.$$(".run__item").map((item) => item.getText());
    expect(failures).toHaveLength(1);
    expect(failures[0]).toContain("Créer une commande");
    expect(await browser.$(".run__failure .run__error").getText()).toBe(
      "headers.location : en-tête absent de la réponse",
    );
    // Texte du DOM : `getText` de WebKitWebDriver rend vide un libellé tronqué par ellipse.
    const group = await browser.execute(
      () => document.querySelector(".run__group-name")?.textContent.trim() ?? "",
    );
    expect(group).toBe("Commandes");
    expect(await browser.$(".run__group-count").getText()).toBe("1/2");
    // La barre latérale suit le run (R4, R6).
    expect((await treeRow("Commandes"))?.count).toBe("1 réussie sur 2 exécutées (2 requêtes)");
    expect((await treeRow("Créer une commande"))?.status).toBe("✕, en échec");
    await screenshot(browser, "20-run-dossier");

    await browser.$(".run__link=Ignorer pour ce run").click();
    await browser.waitUntil(async () => (await progress.getText()).includes("tout est réussi"), {
      timeoutMsg: "échec ignoré attendu",
    });
    expect((await treeRow("Commandes"))?.count).toBe("2 réussies sur 2 exécutées (2 requêtes)");
  });

  it("au redémarrage, les statuts et le dernier run sont relus (ST4)", async () => {
    await restart();
    const expand = browser.$('aside button[aria-label="Déplier boutique-tests"]');
    if (await expand.isExisting()) await expand.click();
    await browser.waitUntil(async () => (await treeRow("Créer une commande"))?.status !== "", {
      timeout: 15_000,
      timeoutMsg: "statut relu attendu",
    });
    expect((await treeRow("Créer une commande"))?.status).toBe("✓, réussie");
    expect((await treeRow("Lister les commandes"))?.status).toBe("✓, réussie");
    expect((await treeRow("Commandes"))?.count).toBe("2 réussies sur 2 exécutées (2 requêtes)");
  });
});

// --- Lot D - Barre latérale (docs/plan-mvp-d-barre-laterale.md) -----------------------------

describe("Lot D - Barre latérale", () => {
  /** Valeur saisie dans le champ, événement `input` compris (le filtre réagit à la frappe). */
  async function typeFilter(value: string): Promise<void> {
    await browser.execute((text) => {
      const input = document.querySelector<HTMLInputElement>("aside .tree .tree-filter input");
      if (!input) throw new Error("filtre du référentiel introuvable");
      input.focus();
      input.value = text;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, value);
  }

  function filterState() {
    return browser.execute(() => ({
      count: document.querySelector("aside .tree .tree-filter__count")?.textContent.trim() ?? "",
      names: Array.from(document.querySelectorAll('aside .tree [role="treeitem"] .row__name')).map(
        (n) => n.textContent.trim(),
      ),
      none: document.querySelector("aside .tree__none")?.textContent.trim() ?? "",
    }));
  }

  it("filtre le référentiel : dossiers dépliés, compte affiché, Échap efface (1.2-e)", async () => {
    await chooseWorkspace("E-commerce");
    const expand = browser.$('aside button[aria-label="Déplier boutique-tests"]');
    if (await expand.isExisting()) await expand.click();
    await browser.$("aside .tree .tree-filter input").waitForExist({ timeout: 10_000 });

    await typeFilter("créer");
    await browser.waitUntil(async () => (await filterState()).count.startsWith("1 sur"), {
      timeoutMsg: "compte du filtre attendu",
    });
    expect((await filterState()).names).toEqual(["Commandes", "Créer une commande"]);
    await screenshot(browser, "21-filtre-referentiel");

    await typeFilter("introuvable");
    await browser.waitUntil(async () => (await filterState()).none !== "", {
      timeoutMsg: "« aucun résultat » attendu",
    });
    expect((await filterState()).none).toBe("Aucune requête ne correspond.");

    await browser.keys("Escape");
    await browser.waitUntil(async () => (await filterState()).count === "", {
      timeoutMsg: "filtre effacé attendu",
    });
  });

  it("le menu « + » du projet propose ses créations avec leurs raccourcis (1.2-a ter)", async () => {
    await clickHidden('aside button[aria-label="Ajouter au projet boutique-tests"]');
    const items = await browser.$$('aside [role="menuitem"]').map((i) => i.getText());
    expect(items.some((t) => t.includes("Nouvelle requête") && t.includes("Ctrl+N"))).toBe(true);
    expect(items.some((t) => t.includes("Nouveau dossier") && t.includes("Ctrl+Maj+N"))).toBe(true);
    expect(items.some((t) => t.includes("Nouvel environnement"))).toBe(true);
    await screenshot(browser, "22-menu-ajouter");
    await browser.keys("Escape");
  });

  it("l'endpoint liste ses réponses attendues ; un clic ouvre la réponse (3.2-a3)", async () => {
    const contract = browser.$('aside button[aria-label="Déplier le contrat Animalerie"]');
    if (await contract.isExisting()) await contract.click();
    await clickText("aside .crow__resource", "/pets");
    const operation = browser.$('aside button[aria-label="Déplier GET /pets/{petId}"]');
    await operation.waitForExist({ timeout: 10_000 });
    await operation.click();
    const rows = () =>
      browser.execute(() =>
        Array.from(document.querySelectorAll("aside .crow__example .crow__name")).map((n) =>
          n.textContent.trim(),
        ),
      );
    await browser.waitUntil(async () => (await rows()).includes("Un animal"), {
      timeoutMsg: "réponses attendues sous l'endpoint attendues",
    });
    expect(await rows()).toContain("Un animal");
    await clickText("aside .crow__example", "Un animal");
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Array.from(document.querySelectorAll<HTMLInputElement>(".expected input")).some(
            (i) => i.value === "Un animal",
          ),
        ),
      { timeout: 10_000, timeoutMsg: "réponse attendue ouverte attendue" },
    );
    await screenshot(browser, "23-reponses-attendues-arborescence");
  });

  it("les cartes de projet disent leur nombre de contrats (1.1-a)", async () => {
    await chooseWorkspace("E-commerce");
    await waitForHeading(browser, "E-commerce");
    // Une carte par projet du workspace : chacune dit son nombre de contrats.
    await browser.waitUntil(
      async () => (await browser.$$("main article .card__stats")).length > 0,
      {
        timeout: 10_000,
        timeoutMsg: "cartes de projet attendues",
      },
    );
    const stats = await browser.$$("main article .card__stats").map((s) => s.getText());
    expect(stats.length).toBeGreaterThan(0);
    for (const text of stats) expect(text).toMatch(/contrats? de service/);
    await screenshot(browser, "24-cartes-contrats");
  });
});
