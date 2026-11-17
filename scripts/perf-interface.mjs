#!/usr/bin/env node
/**
 * Mesures de performance de l'interface (audit de novembre 2026, plan-qualite-mvp T7) :
 * l'application réelle sur le faux cœur gonflé (tests/perf), en build de production, dans
 * Chromium piloté par Playwright. Lancées chaque semaine et à la demande par la CI
 * (.github/workflows/qualite.yml).
 *
 * Usage :
 *   npx vite build --config vite.perf.config.ts      # une fois, et après chaque changement
 *   node scripts/perf-interface.mjs [scénario…] [--repetitions 5] [--ralenti 4] [--sortie f.json]
 *
 * Budgets de R7 (projet de 2 000 requêtes, processeur ralenti 4×) : comparés à la fin
 * (`BUDGETS`). Un dépassement est signalé ; avec ARNOD_BANC_STRICT=1, il fait échouer le banc
 * (activé quand les lots F et G auront tenu ces budgets).
 *
 * Scénarios : barre, editeur, inconnues, rendus, reponse, filtre, html, run, onglets.
 * Chaque mesure est répétée (`--repetitions`, 5) sur une page neuve ; la médiane est gardée. Une
 * passe de plus est faite avec le processeur ralenti (`--ralenti`, 4 ; 0 pour s'en passer).
 *
 * Temps donnés, en millisecondes : de l'événement (clic, frappe, événement du cœur) à l'image
 * suivante peinte, temps du faux cœur retiré. Les fonctions passées à la page viennent de ce
 * fichier (outil local : rien d'extérieur n'y est évalué).
 */
import { createReadStream, existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const ROOT = fileURLToPath(new URL("..", import.meta.url));
const DIST = process.env.PERF_DIST ?? join(ROOT, "dist-perf");

function playwright() {
  const pistes = ["playwright", process.env.PLAYWRIGHT_DIR].filter(Boolean);
  for (const piste of pistes) {
    try {
      return require(piste);
    } catch {
      // Piste suivante.
    }
  }
  throw new Error("Playwright introuvable : installez-le ou donnez PLAYWRIGHT_DIR.");
}

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".map": "application/json",
};

/** Fichiers de `dossier` par adresse (`/assets/x.js`) ; `/` sert `index.html`. */
function fichiersServis(dossier) {
  const servis = new Map();
  const parcourir = (relatif) => {
    for (const entree of readdirSync(join(dossier, relatif), { withFileTypes: true })) {
      const enfant = relatif === "" ? entree.name : `${relatif}/${entree.name}`;
      if (entree.isDirectory()) parcourir(enfant);
      else if (entree.isFile()) servis.set(`/${enfant}`, join(dossier, enfant));
    }
  };
  parcourir("");
  const index = servis.get("/index.html");
  if (index) servis.set("/", index);
  return servis;
}

/**
 * Sert la page construite sur 127.0.0.1. Seuls les fichiers listés au démarrage sont servis :
 * l'adresse demandée ne sert qu'à choisir dans cette liste, jamais à construire un chemin.
 */
function servir(dossier) {
  const servis = fichiersServis(dossier);
  return new Promise((ok) => {
    const server = createServer((req, res) => {
      const fichier = servis.get((req.url ?? "/").split("?")[0]);
      if (fichier === undefined) {
        res.writeHead(404).end();
        return;
      }
      res.writeHead(200, { "content-type": TYPES[extname(fichier)] ?? "application/octet-stream" });
      createReadStream(fichier).pipe(res);
    });
    server.listen(0, "127.0.0.1", () =>
      ok({ server, base: `http://127.0.0.1:${server.address().port}` }),
    );
  });
}

// --- Outils ---------------------------------------------------------------------------

const mediane = (valeurs) => {
  const tri = valeurs
    .filter((v) => typeof v === "number" && Number.isFinite(v))
    .sort((a, b) => a - b);
  if (tri.length === 0) return null;
  const milieu = Math.floor(tri.length / 2);
  return tri.length % 2 === 1 ? tri[milieu] : (tri[milieu - 1] + tri[milieu]) / 2;
};
const centile = (valeurs, p) => {
  const tri = [...valeurs].sort((a, b) => a - b);
  return tri[Math.min(tri.length - 1, Math.floor((p / 100) * tri.length))] ?? null;
};
const moyenne = (valeurs) => valeurs.reduce((a, b) => a + b, 0) / Math.max(1, valeurs.length);
const arrondi = (v) => (typeof v === "number" ? Math.round(v * 10) / 10 : (v ?? null));

async function ouvrir(browser, base, params, ralenti = 1) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(600000);
  const erreurs = [];
  page.on("pageerror", (e) => erreurs.push(String(e)));
  const cdp = await context.newCDPSession(page);
  await cdp.send("Performance.enable");
  if (ralenti > 1) await cdp.send("Emulation.setCPUThrottlingRate", { rate: ralenti });
  await page.goto(`${base}/index.html?${new URLSearchParams(params)}`);
  await page.waitForFunction(() => document.documentElement.dataset.perf === "pret");
  return { page, cdp, context, erreurs };
}

async function memoire(cdp) {
  await cdp.send("HeapProfiler.collectGarbage");
  const { metrics } = await cdp.send("Performance.getMetrics");
  const m = Object.fromEntries(metrics.map((x) => [x.name, x.value]));
  return { tasMo: m.JSHeapUsedSize / 1048576, noeuds: m.Nodes };
}

/** Arme le suivi des entrées et vide les relevés ; renvoie le temps du faux cœur déjà compté. */
function armer(page, types) {
  return page.evaluate((liste) => {
    for (const type of liste) window.__perf.suivre(type);
    window.__perf.releve();
    return window.__perf.compteur.faux;
  }, types);
}

/** Relevé depuis `armer` : entrées, tâches longues et images longues. */
async function relever(page) {
  await page.evaluate(() => window.__perf.pause(60));
  return page.evaluate(() => window.__perf.releve());
}

const temps = (entree) => entree.image - entree.faux;

/**
 * Clic réel, mesuré jusqu'à l'image qui montre la condition. `fini` est une fabrique : Playwright
 * la sérialise comme toute fonction passée à `evaluate` et la fonction qu'elle rend (la condition,
 * appelée avec `argument`) reste dans la page. Aucun texte n'est évalué.
 */
async function clic(page, selecteur, fini, argument) {
  const faux = await armer(page, ["click"]);
  await page.click(selecteur);
  const test = await page.evaluateHandle(fini);
  const fin = await page.evaluate(
    ([condition, arg]) => window.__perf.attendre(() => condition(arg), 600000),
    [test, argument],
  );
  await test.dispose();
  const releve = await relever(page);
  const entree = releve.entrees.find((e) => e.type === "click");
  const etat = await page.evaluate(() => ({
    faux: window.__perf.compteur.faux,
    parse: window.__perf.compteur.parse,
  }));
  return {
    ms: fin - entree.debut - (etat.faux - faux),
    premiereImage: temps(entree),
    calcul: entree.calcul,
    longues: releve.longues.map((l) => l.duree),
  };
}

/** Frappe réelle caractère par caractère ; une entrée par frappe. */
async function frapper(page, texte, delai = 80) {
  await armer(page, ["input"]);
  for (const caractere of texte) {
    await page.keyboard.type(caractere);
    await page.waitForTimeout(delai);
  }
  const releve = await relever(page);
  const frappes = releve.entrees.filter((e) => e.type === "input").map(temps);
  return {
    frappes,
    mediane: mediane(frappes),
    max: Math.max(...frappes),
    premiere: frappes[0],
    longues: releve.longues.map((l) => l.duree),
  };
}

/**
 * Action faite dans la page (événement du cœur, store), mesurée par `__perf.chrono`. `action` et
 * `fini` sont des fabriques, comme pour `clic`.
 */
async function chrono(page, action, fini, argument) {
  const agir = await page.evaluateHandle(action);
  const test = fini ? await page.evaluateHandle(fini) : null;
  const r = await page.evaluate(
    ([a, f, arg]) => window.__perf.chrono(() => a(arg), f ? () => f(arg) : undefined),
    [agir, test, argument],
  );
  await agir.dispose();
  await test?.dispose();
  return { ms: r.image - r.faux, calcul: r.calcul - r.faux, parse: r.parse };
}

const LIGNES = () => document.querySelectorAll('[role="tree"] [role="treeitem"]').length;
const AU_MOINS = () => (n) =>
  document.querySelectorAll('[role="tree"] [role="treeitem"]').length >= n;
const AU_PLUS = () => (n) =>
  document.querySelectorAll('[role="tree"] [role="treeitem"]').length <= n;

/** Ouvre le projet depuis l'accueil du workspace : premier rendu de la barre latérale. */
async function ouvrirProjet(page) {
  const lignes = await page.evaluate(() => window.__perf.projet.lignes);
  return clic(page, 'main article button:has-text("Ouvrir")', AU_MOINS, lignes);
}

/** Ouvre une requête par le store (`GROSSE` ou `REPONSE`) et attend son éditeur. */
function ouvrirRequete(page, laquelle) {
  return chrono(
    page,
    () => (nom) => {
      const p = window.__perf;
      return p.workbench.openRequest(p.PROJET, p[nom]);
    },
    () => () => document.querySelector(".editor__url") !== null,
    laquelle,
  );
}

/**
 * Enregistrement automatique qui suit une frappe : de l'appel de `request_save` à l'image qui
 * montre l'arborescence et les contrats relus.
 */
function sauvegarde(page) {
  return page.evaluate(async () => {
    const p = window.__perf;
    const P = p.PROJET;
    const arbre = p.workbench.trees[P];
    const liste = p.contracts.lists[P];
    const n = p.compteur.journal.length;
    const faux = p.compteur.faux;
    p.releve();
    const fin = await p.attendre(
      () => p.workbench.trees[P] !== arbre && p.contracts.lists[P] !== liste,
      15000,
    );
    await p.pause(120);
    const save = p.compteur.journal.slice(n).find((j) => j.cmd === "request_save");
    if (!save) return null;
    const rel = p.releve();
    const pendant = (l) => l.debut >= save.debut - 1 && l.debut <= fin;
    return {
      ms: fin - save.debut - (p.compteur.faux - faux),
      longue: Math.max(0, ...rel.longues.filter(pendant).map((l) => l.duree)),
    };
  });
}

/** Images par seconde pendant un défilement à la molette au-dessus de `selecteur`. */
async function defiler(page, selecteur) {
  await page.hover(selecteur);
  await page.evaluate(() => window.__perf.debutCadence());
  for (let i = 0; i < 40; i++) {
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(16);
  }
  return page.evaluate(() => window.__perf.finCadence());
}

// --- 1. Barre latérale ------------------------------------------------------------------

async function barre({ page, cdp }) {
  const r = {};
  r["premier rendu"] = (await ouvrirProjet(page)).ms;
  r["nœuds DOM"] = await page.evaluate(() => window.__perf.noeuds());
  r["lignes de l'arbre"] = await page.evaluate(LIGNES);
  r["tas JS (Mo)"] = (await memoire(cdp)).tasMo;
  const total = r["lignes de l'arbre"];

  const dossier = `[role="treeitem"][data-path="Catalogue/Produits"]`;
  r["replier un dossier de 200"] = (await clic(page, dossier, AU_PLUS, total - 200)).ms;
  r["déplier un dossier de 200"] = (await clic(page, dossier, AU_MOINS, total)).ms;

  const petit = '[role="treeitem"][data-path="Parcours/Parcours 01"] .row__chevron';
  r["replier un test de 8 étapes"] = (await clic(page, petit, AU_PLUS, total - 8)).ms;
  await page.click(petit);
  await page.evaluate(() => window.__perf.image());

  await page.evaluate(async () => {
    const p = window.__perf;
    p.workbench.trees[p.PROJET].collapsed = [...p.projet.dossiers];
    await p.image();
  });
  r["tout déplier"] = (
    await chrono(
      page,
      () => () => {
        const p = window.__perf;
        p.workbench.trees[p.PROJET].collapsed = [];
      },
      AU_MOINS,
      total,
    )
  ).ms;

  r["replier le projet"] = (await clic(page, ".project__chevron", AU_PLUS, 0)).ms;
  r["déplier le projet"] = (await clic(page, ".project__chevron", AU_MOINS, total)).ms;

  await page.click('input[aria-label^="Filtrer le référentiel"]');
  const filtre = await frapper(page, "commandes");
  r["filtre · 1re frappe"] = filtre.premiere;
  r["filtre · frappe médiane"] = filtre.mediane;
  r["filtre · pire frappe"] = filtre.max;
  r["filtre · lignes restantes"] = await page.evaluate(LIGNES);
  await armer(page, ["keydown"]);
  await page.keyboard.press("Escape");
  const efface = await relever(page);
  r["filtre · effacer (Échap)"] = Math.max(...efface.entrees.map(temps));

  const statuts = [];
  for (let tour = 0; tour < 3; tour++) {
    await page.evaluate((t) => {
      window.__statuts = window.__perf.statutsChanges(500, t);
    }, tour);
    statuts.push(
      await chrono(page, () => () => {
        const p = window.__perf;
        return p.emettre("arnod://statuts", { projectId: p.PROJET, statuses: window.__statuts });
      }),
    );
  }
  r["arnod://statuts · 500 changés"] = mediane(statuts.map((s) => s.ms));

  const relus = [];
  for (let tour = 0; tour < 3; tour++) {
    await page.evaluate(() => {
      const p = window.__perf;
      window.__arbre = p.workbench.trees[p.PROJET];
      window.__liste = p.contracts.lists[p.PROJET];
    });
    relus.push(
      await chrono(
        page,
        () => () => {
          const p = window.__perf;
          return p.emettre("arnod://projet", {
            projectId: p.PROJET,
            scope: "tree",
            path: null,
            origin: "autre",
          });
        },
        () => () => {
          const p = window.__perf;
          return (
            p.workbench.trees[p.PROJET] !== window.__arbre &&
            p.contracts.lists[p.PROJET] !== window.__liste
          );
        },
      ),
    );
  }
  r["arnod://projet · arbre relu"] = mediane(relus.map((s) => s.ms));
  r["arnod://projet · dont JSON.parse"] = mediane(relus.map((s) => s.parse));

  // Gros contrat déplié : ses ressources et ses opérations.
  r["déplier le gros contrat"] = (
    await clic(
      page,
      ".contract-row__chevron",
      () => () => document.querySelectorAll(".crow").length > 0,
    )
  ).ms;
  return r;
}

// --- 2. Éditeur ---------------------------------------------------------------------------

async function editeur({ page }) {
  const r = {};
  await ouvrirProjet(page);
  r["ouvrir la requête"] = (await ouvrirRequete(page, "GROSSE")).ms;
  await page.waitForTimeout(1200);

  await page.click(".editor__url");
  await page.keyboard.press("End");
  const url = await frapper(page, "/abc?x=12&y=3");
  r["adresse · 1re frappe"] = url.premiere;
  r["adresse · frappe médiane"] = url.mediane;
  r["adresse · pire frappe"] = url.max;
  const apresUrl = await sauvegarde(page);
  r["enregistrement auto · arbre relu"] = apresUrl?.ms ?? null;
  r["enregistrement auto · tâche la plus longue"] = apresUrl?.longue ?? null;

  r["onglet Corps · ouvrir"] = (
    await clic(page, "#tab-body", () => () => document.querySelector(".body__content") !== null)
  ).ms;
  await page.click(".body__content");
  await page.keyboard.press("Control+Home");
  const corps = await frapper(page, "abcdefghijkl");
  r["corps 200 ko · 1re frappe"] = corps.premiere;
  r["corps 200 ko · frappe médiane"] = corps.mediane;
  r["corps 200 ko · pire frappe"] = corps.max;
  const apresCorps = await sauvegarde(page);
  r["corps · enregistrement auto"] = apresCorps?.ms ?? null;
  const cadence = await defiler(page, ".body__content");
  r["corps · défilement (images/s)"] = cadence.ips;
  r["corps · défilement, pire image"] = cadence.pire;
  r["sérialisation du brouillon (isDirty)"] = await page.evaluate(() => {
    const p = window.__perf;
    const brouillon = p.workbench.editors[`${p.PROJET}::${p.GROSSE}`].draft;
    const durees = [];
    for (let i = 0; i < 15; i++) {
      const debut = performance.now();
      JSON.stringify(brouillon);
      durees.push(performance.now() - debut);
    }
    return durees.sort((a, b) => a - b)[7];
  });

  r["onglet En-têtes · ouvrir (100 lignes)"] = (
    await clic(
      page,
      "#tab-headers",
      () => () => document.querySelectorAll(".pairs__row").length >= 100,
    )
  ).ms;
  await page.click('input[aria-label="Valeur, ligne 50"]');
  const entete = await frapper(page, "abcdefghijkl");
  r["en-tête · 1re frappe"] = entete.premiere;
  r["en-tête · frappe médiane"] = entete.mediane;
  r["en-tête · pire frappe"] = entete.max;
  r["nœuds DOM"] = await page.evaluate(() => window.__perf.noeuds());
  return r;
}

/** Composants re-rendus par frappe (`?compte`), une passe. */
async function rendus({ page }) {
  const r = {};
  await ouvrirProjet(page);
  await ouvrirRequete(page, "GROSSE");
  await page.waitForTimeout(1200);
  const compter = async (nom, preparer) => {
    await preparer();
    await page.waitForTimeout(1500);
    await page.evaluate(() => window.__perf.razRendus());
    for (const caractere of "abcde") {
      await page.keyboard.type(caractere);
      await page.waitForTimeout(30);
    }
    const comptes = await page.evaluate(() => ({ ...window.__perf.rendus }));
    const total = Object.values(comptes).reduce((a, b) => a + b, 0);
    r[`composants re-rendus par frappe · ${nom}`] = total / 5;
    r[`détail · ${nom}`] = Object.entries(comptes)
      .filter(([, n]) => n > 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([k, n]) => `${k}×${n / 5}`)
      .join(", ");
  };
  await compter("adresse", async () => {
    await page.click(".editor__url");
    await page.keyboard.press("End");
  });
  await compter("corps", async () => {
    await page.click("#tab-body");
    await page.click(".body__content");
    await page.keyboard.press("Control+Home");
  });
  await compter("en-tête", async () => {
    await page.click("#tab-headers");
    await page.click('input[aria-label="Valeur, ligne 50"]');
  });
  await compter("filtre de l'arbre", async () => {
    await page.click('input[aria-label^="Filtrer le référentiel"]');
  });
  r["composants montés"] = await page.evaluate(() =>
    Object.values(window.__perf.montes).reduce((a, b) => a + b, 0),
  );
  r["détail · montés"] = await page.evaluate(() =>
    Object.entries(window.__perf.montes)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([k, n]) => `${k}×${n}`)
      .join(", "),
  );
  return r;
}

// --- 3. Réponse ---------------------------------------------------------------------------

async function envoyer(page, cdp, r) {
  await ouvrirProjet(page);
  await ouvrirRequete(page, "REPONSE");
  await page.waitForTimeout(800);
  const avant = await memoire(cdp);
  const noeudsAvant = await page.evaluate(() => window.__perf.noeuds());
  const parse = await page.evaluate(() => window.__perf.compteur.parse);
  const envoi = await clic(
    page,
    '.editor__line button[type="submit"]',
    () => () => document.querySelector(".code__line") !== null,
  );
  r["afficher le corps"] = envoi.ms;
  r["dont lecture JSON de la mise en forme"] =
    (await page.evaluate(() => window.__perf.compteur.parse)) - parse;
  r["mise en forme reçue (Mo)"] =
    (await page.evaluate(() => window.__perf.compteur.octets.response_render ?? 0)) / 1e6;
  r["lignes"] = await page.evaluate(() => document.querySelectorAll(".code__line").length);
  r["nœuds DOM du corps"] = (await page.evaluate(() => window.__perf.noeuds())) - noeudsAvant;
  r["tas JS ajouté (Mo)"] = (await memoire(cdp)).tasMo - avant.tasMo;
}

async function reponse({ page, cdp }) {
  const r = {};
  await envoyer(page, cdp, r);
  const cadence = await defiler(page, ".body__view.code");
  r["défilement (images/s)"] = cadence.ips;
  r["défilement, pire image"] = cadence.pire;
  return r;
}

async function filtre({ page, cdp }) {
  const r = {};
  await envoyer(page, cdp, r);
  await page.click('input[aria-label="Filtre JSONPath"]');
  await page.evaluate(() => {
    window.__n = window.__perf.compteur.journal.length;
  });
  await page.fill('input[aria-label="Filtre JSONPath"]', "$.items[*].sku");
  const trouve = await page.evaluate(async () => {
    const p = window.__perf;
    const fin = await p.attendre(() => document.querySelectorAll(".tok--match").length >= 5000);
    const appel = p.compteur.journal.slice(window.__n).find((j) => j.cmd === "response_filter");
    return { ms: fin - appel.fin, surlignes: document.querySelectorAll(".tok--match").length };
  });
  r["filtre · surligner 5 000 correspondances"] = trouve.ms;
  r["filtre · valeurs surlignées"] = trouve.surlignes;
  const suivants = [];
  for (let i = 0; i < 4; i++) {
    suivants.push(
      (await clic(page, 'button[aria-label="Résultat suivant"]', () => () => true)).premiereImage,
    );
  }
  r["filtre · correspondance suivante"] = mediane(suivants);
  return r;
}

async function html({ page, cdp }) {
  const r = {};
  await envoyer(page, cdp, r);
  const noeuds = await page.evaluate(() => window.__perf.noeuds());
  r["aperçu HTML"] = (
    await clic(page, '.body__bar [role="switch"]', () => () => {
      const apercu = document.querySelector(".preview");
      return apercu !== null && apercu.childElementCount > 0;
    })
  ).ms;
  r["aperçu · éléments"] = await page.evaluate(
    () => document.querySelector(".preview")?.querySelectorAll("*").length ?? 0,
  );
  r["aperçu · nœuds DOM en plus"] = (await page.evaluate(() => window.__perf.noeuds())) - noeuds;
  return r;
}

// --- 4. Run ----------------------------------------------------------------------------------

async function run({ page }, taille) {
  const r = {};
  await ouvrirProjet(page);
  const lignes = await page.evaluate(() => window.__perf.preparerRun(true));
  r["lignes du run"] = lignes;
  r["ouvrir le run terminé (Échecs)"] = (
    await chrono(
      page,
      () => () => window.__perf.ouvrirRun(),
      () => () => document.querySelector(".run__item") !== null,
    )
  ).ms;
  const ITEMS_AU_MOINS = () => (n) => document.querySelectorAll(".run__item").length >= n;
  const ITEMS_AU_PLUS = () => (n) => document.querySelectorAll(".run__item").length <= n;
  r["filtre Tous"] = (await clic(page, '.run__tab:has-text("Tous")', ITEMS_AU_MOINS, lignes)).ms;
  r["nœuds DOM (Tous)"] = await page.evaluate(() => window.__perf.noeuds());
  r["filtre Lents"] = (
    await clic(page, '.run__tab:has-text("Lents")', ITEMS_AU_PLUS, lignes / 4)
  ).ms;
  r["filtre Échecs"] = (
    await clic(page, '.run__tab:has-text("Échecs")', ITEMS_AU_PLUS, lignes / 8)
  ).ms;

  // Flot d'événements : un sur `pas` est émis et mesuré, tous comptent dans le total estimé.
  const pas = taille >= 2000 ? 20 : 5;
  for (const [nom, statuts] of [
    ["run seul", false],
    ["run et statuts", true],
  ]) {
    await page.evaluate(async () => {
      const p = window.__perf;
      p.preparerRun(false);
      await p.ouvrirRun();
      await p.attendre(() => document.querySelector(".run__item") !== null);
    });
    const flot = await page.evaluate(([s, n]) => window.__perf.flot(0, s, n), [statuts, pas]);
    r[`flot (${nom}) · par événement, médiane`] = mediane(flot.traitement);
    r[`flot (${nom}) · par événement, 95e centile`] = centile(flot.traitement, 95);
    r[`flot (${nom}) · pire événement`] = Math.max(...flot.traitement);
    if (statuts) r["flot · arnod://statuts, par événement"] = flot.statuts / flot.evenements;
    const parLigne = moyenne(flot.traitement) + (statuts ? flot.statuts / flot.evenements : 0);
    r[`flot (${nom}) · interface figée sur tout le run (s)`] = (parLigne * lignes) / 1000;
    r["flot · copie JSON de l'état, par événement"] = flot.copies / flot.evenements;
  }

  // Au rythme d'un vrai run (12 ms entre deux lignes) : images par seconde.
  await page.evaluate(async () => {
    const p = window.__perf;
    p.preparerRun(false);
    await p.ouvrirRun();
    await p.attendre(() => document.querySelector(".run__item") !== null);
    p.debutCadence();
  });
  const rythme = await page.evaluate(() => window.__perf.flot(12, true, 1, 120));
  const cadence = await page.evaluate(() => window.__perf.finCadence());
  r["flot à 80 lignes/s · images par seconde"] = cadence.ips;
  r["flot à 80 lignes/s · pire image"] = cadence.pire;
  r["flot à 80 lignes/s · lignes traitées par seconde"] = (rythme.evenements / rythme.total) * 1000;
  return r;
}

// --- 5. Onglets et zones ------------------------------------------------------------------

async function onglets({ page }) {
  const r = {};
  // Les zones s'affichent dans la vue du projet : 4 éditeurs et la barre latérale.
  const ouverture = await clic(
    page,
    'main article button:has-text("Ouvrir")',
    () => () => document.querySelectorAll(".editor__url").length >= 4,
  );
  r["ouvrir le projet · 4 éditeurs et l'arbre"] = ouverture.ms;
  await page.waitForTimeout(1500);
  r["nœuds DOM"] = await page.evaluate(() => window.__perf.noeuds());
  const actif = () => (i) => {
    const zone = document.querySelectorAll("section.zone")[0];
    const onglet = zone.querySelectorAll('[role="tab"]')[i];
    return (
      onglet.getAttribute("aria-selected") === "true" &&
      zone.querySelector(".editor__url") !== null &&
      zone.querySelector(".request-tab__loading") === null
    );
  };
  const changer = (i) => clic(page, `section.zone >> nth=0 >> [role="tab"] >> nth=${i}`, actif, i);
  const froid = [(await changer(1)).ms, (await changer(2)).ms];
  const chaud = [(await changer(0)).ms, (await changer(1)).ms, (await changer(2)).ms];
  r["changer d'onglet · jamais ouvert"] = mediane(froid);
  r["changer d'onglet · déjà ouvert"] = mediane(chaud);

  const fenetres = [];
  for (let i = 0; i < 3; i++) {
    fenetres.push(
      await chrono(page, () => () => {
        const p = window.__perf;
        return p.emettre("arnod://fenetres", p.fenetres());
      }),
    );
  }
  r["arnod://fenetres"] = mediane(fenetres.map((f) => f.ms));

  const poignee = await page.locator(".handle--row").first().boundingBox();
  const x = poignee.x + poignee.width / 2;
  const y = poignee.y + poignee.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await armer(page, ["pointermove"]);
  await page.evaluate(() => window.__perf.debutCadence());
  for (let i = 1; i <= 90; i++) {
    await page.mouse.move(x + Math.round(120 * Math.sin(i / 14)), y);
    await page.waitForTimeout(11);
  }
  const cadence = await page.evaluate(() => window.__perf.finCadence());
  await page.mouse.up();
  const gestes = await relever(page);
  r["séparation · images par seconde"] = cadence.ips;
  r["séparation · pire image"] = cadence.pire;
  r["séparation · images de plus de 33 ms"] = cadence.plus33;
  r["séparation · déplacement médian"] = mediane(gestes.entrees.map(temps));
  return r;
}

// --- Passes -------------------------------------------------------------------------------

const SCENARIOS = {
  barre: { tailles: [500, 2000], params: (t) => ({ scenario: "barre", taille: t }), jouer: barre },
  editeur: {
    tailles: [500, 2000],
    params: (t) => ({ scenario: "editeur", taille: t }),
    jouer: editeur,
  },
  inconnues: {
    tailles: [2000],
    params: (t) => ({ scenario: "editeur", taille: t, inconnues: 1 }),
    jouer: editeur,
  },
  rendus: {
    tailles: [2000],
    params: (t) => ({ scenario: "editeur", taille: t, compte: 1 }),
    jouer: rendus,
    repetitions: 1,
    sansRalenti: true,
  },
  reponse: {
    tailles: [100, 1000, 5000],
    params: (t) => ({ scenario: "reponse", taille: t }),
    jouer: reponse,
    repetitions: (t) => (t >= 5000 ? 2 : undefined),
    sansRalenti: (t, passes) =>
      t >= 5000 && mediane(passes.map((p) => p["afficher le corps"])) > 20000,
  },
  filtre: {
    tailles: [5000],
    params: (t) => ({ scenario: "reponse", taille: 1, articles: t }),
    jouer: filtre,
    repetitions: 3,
  },
  html: {
    tailles: [2048],
    params: (t) => ({ scenario: "html", taille: t }),
    jouer: html,
    repetitions: 3,
  },
  run: {
    tailles: [500, 2000],
    params: (t) => ({ scenario: "run", taille: t }),
    jouer: run,
    repetitions: 3,
  },
  onglets: {
    tailles: [500, 2000],
    params: (t) => ({ scenario: "onglets", taille: t }),
    jouer: onglets,
    repetitions: 3,
  },
};

const option = (valeur, ...args) => (typeof valeur === "function" ? valeur(...args) : valeur);

/**
 * Budgets de R7 et des critères d'acceptation (plan-qualite-mvp § 9), en millisecondes, sur la
 * passe ralentie (sinon la médiane) : frappe sous 50 ms, ouverture sous 100 ms, aucun événement
 * de run au-delà de 16 ms, réponse de 5 Mo affichée sous 100 ms.
 */
const BUDGETS = [
  { cle: "editeur · 2000", mesure: /^adresse · frappe médiane$/, max: 50 },
  { cle: "editeur · 2000", mesure: /^en-tête · frappe médiane$/, max: 50 },
  { cle: "editeur · 2000", mesure: /^ouvrir la requête$/, max: 100 },
  { cle: "barre · 2000", mesure: /^déplier le projet$/, max: 100 },
  { cle: "run · 2000", mesure: /^flot \(.*\) · pire événement$/, max: 16 },
  { cle: "reponse · 5000", mesure: /^afficher le corps$/, max: 100 },
];

/** Compare les résultats aux budgets ; renvoie le nombre de dépassements. */
function verifierBudgets(resultats) {
  let depasses = 0;
  console.log("\n## Budgets R7\n");
  for (const budget of BUDGETS) {
    const mesures = resultats[budget.cle];
    if (!mesures) continue;
    for (const [nom, m] of Object.entries(mesures)) {
      if (!budget.mesure.test(nom) || m.mediane === undefined) continue;
      const valeur = m.ralenti ?? m.mediane;
      const tenu = valeur !== null && valeur <= budget.max;
      if (!tenu) depasses++;
      console.log(
        `${tenu ? "tenu   " : "DÉPASSÉ"} ${budget.cle} · ${nom} : ${String(valeur)} ms (au plus ${String(budget.max)} ms)`,
      );
    }
  }
  return depasses;
}

async function main() {
  const args = process.argv.slice(2);
  const opts = { repetitions: 5, ralenti: 4, sortie: null, noms: [] };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--repetitions") opts.repetitions = Number(args[++i]);
    else if (args[i] === "--ralenti") opts.ralenti = Number(args[++i]);
    else if (args[i] === "--sortie") opts.sortie = resolve(args[++i]);
    else opts.noms.push(...args[i].split(","));
  }
  if (opts.noms.length === 0) opts.noms = Object.keys(SCENARIOS);
  if (!existsSync(join(DIST, "index.html"))) {
    throw new Error(`build absent : npx vite build --config vite.perf.config.ts (${DIST})`);
  }
  const { chromium } = playwright();
  const { server, base } = await servir(DIST);
  const browser = await chromium.launch();
  const resultats = {};
  const ecrire = () => {
    if (!opts.sortie) return;
    mkdirSync(dirname(opts.sortie), { recursive: true });
    writeFileSync(opts.sortie, JSON.stringify(resultats, null, 2));
  };
  try {
    for (const nom of opts.noms) {
      const scenario = SCENARIOS[nom];
      if (!scenario) throw new Error(`scénario inconnu : ${nom}`);
      for (const taille of scenario.tailles) {
        const cle = `${nom} · ${taille}`;
        const repetitions = Math.min(
          opts.repetitions,
          option(scenario.repetitions, taille) ?? opts.repetitions,
        );
        const passes = [];
        for (let i = 0; i < repetitions; i++) {
          const session = await ouvrir(browser, base, scenario.params(taille));
          try {
            passes.push(await scenario.jouer(session, taille));
          } catch (erreur) {
            console.error(`✕ ${cle} passe ${i + 1} : ${String(erreur).slice(0, 400)}`);
          } finally {
            if (session.erreurs.length > 0)
              console.error(`✕ ${cle} : ${session.erreurs.join(" | ").slice(0, 400)}`);
            await session.context.close();
          }
          process.stderr.write(`· ${cle} passe ${i + 1}/${repetitions}\n`);
        }
        let lente = null;
        if (
          opts.ralenti > 1 &&
          passes.length > 0 &&
          !option(scenario.sansRalenti, taille, passes)
        ) {
          const session = await ouvrir(browser, base, scenario.params(taille), opts.ralenti);
          try {
            lente = await scenario.jouer(session, taille);
          } catch (erreur) {
            console.error(`✕ ${cle} passe ralentie : ${String(erreur).slice(0, 400)}`);
          } finally {
            await session.context.close();
          }
          process.stderr.write(`· ${cle} passe ralentie ×${opts.ralenti}\n`);
        }
        const mesures = {};
        for (const nomMesure of Object.keys(passes[0] ?? {})) {
          const valeurs = passes.map((p) => p[nomMesure]);
          if (typeof valeurs[0] === "string") {
            mesures[nomMesure] = { texte: valeurs[0] };
            continue;
          }
          const nombres = valeurs.filter((v) => typeof v === "number");
          mesures[nomMesure] = {
            mediane: arrondi(mediane(valeurs)),
            min: arrondi(Math.min(...nombres)),
            max: arrondi(Math.max(...nombres)),
            ralenti: lente ? arrondi(lente[nomMesure]) : null,
          };
        }
        resultats[cle] = mesures;
        ecrire();
        console.log(`\n## ${cle} (${passes.length} passes)`);
        for (const [nomMesure, m] of Object.entries(mesures)) {
          console.log(
            m.texte !== undefined
              ? `${nomMesure.padEnd(52)} ${m.texte}`
              : `${nomMesure.padEnd(52)} ${String(m.mediane).padStart(9)}  [${m.min} – ${m.max}]  ×${opts.ralenti} : ${m.ralenti}`,
          );
        }
      }
    }
  } finally {
    await browser.close();
    server.close();
  }
  const depasses = verifierBudgets(resultats);
  const strict =
    (process.env.ARNOD_BANC_STRICT ?? "") !== "" && process.env.ARNOD_BANC_STRICT !== "0";
  if (depasses > 0 && strict) {
    throw new Error(`${String(depasses)} budget(s) dépassé(s)`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
