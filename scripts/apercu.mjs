#!/usr/bin/env node
/**
 * Captures Playwright de l'aperçu (application sur le faux cœur, tests/apercu) et des écrans de la
 * maquette, pour les comparer côte à côte. Outil de développement : hors CI, Playwright est pris
 * dans l'installation globale de Node.
 *
 * Usage :
 *   node scripts/apercu.mjs <scénario>[,<scénario>…] [options]
 *   node scripts/apercu.mjs --liste
 *
 * Options :
 *   --maquette <repère>   écran de la maquette à capturer (ex. « 5.1-b bis ») ; avec un seul
 *                         scénario, une image côte à côte « comparaison-… » est produite
 *   --sombre              thème sombre (sinon clair) ; --deux-themes pour les deux
 *   --largeur <px>        largeur de la fenêtre (1280 par défaut) ; --hauteur <px> (775)
 *   --sortie <dossier>    dossier des images (.apercu/ par défaut, ignoré par Git)
 *
 * La maquette est lue dans .maquette/pages/ (scripts/maquette-sync.py, docs/plan-suivi-maquette.md).
 */
import { mkdirSync, readFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

function options(argv) {
  const opts = { scenarios: [], maquette: [], themes: ["light"], width: 1280, height: 775 };
  opts.sortie = join(ROOT, ".apercu");
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => {
      const v = argv[++i];
      if (v === undefined) throw new Error(`valeur attendue après ${a}`);
      return v;
    };
    if (a === "--maquette") opts.maquette.push(next());
    else if (a === "--sombre") opts.themes = ["dark"];
    else if (a === "--deux-themes") opts.themes = ["light", "dark"];
    else if (a === "--largeur") opts.width = Number(next());
    else if (a === "--hauteur") opts.height = Number(next());
    else if (a === "--sortie") opts.sortie = resolve(next());
    else if (a === "--liste") opts.liste = true;
    else opts.scenarios.push(...a.split(",").filter(Boolean));
  }
  return opts;
}

/** Fichier de la maquette d'une page, d'après compat.json (« 5.1 · Envoi et réponse »). */
function pageFile(code) {
  const pages = join(ROOT, ".maquette", "pages");
  const compat = JSON.parse(readFileSync(join(pages, "compat.json"), "utf8"));
  const page = compat.pages.find((p) => p.name.split(" · ")[0] === code);
  if (!page) throw new Error(`page de maquette inconnue : ${code}`);
  return join(pages, page.path);
}

const slug = (s) =>
  s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-zA-Z0-9.]+/g, "-")
    .replace(/(^-)|(-$)/g, "");

async function captureMaquette(browser, repere, theme, opts) {
  const code = repere.split("-")[0];
  const file = pageFile(code);
  if (!existsSync(file)) throw new Error(`page absente : ${file}`);
  const page = await browser.newPage({
    viewport: { width: 1600, height: 1000 },
    colorScheme: theme,
  });
  await page.goto(pathToFileURL(file).href);
  await page.waitForTimeout(1500);
  const card = await page.evaluateHandle((r) => {
    const opt = Array.from(document.querySelectorAll(".dv-opt")).find(
      (o) => o.querySelector(".dv-oid")?.textContent.trim() === r,
    );
    return opt?.querySelector(".dv-card") ?? null;
  }, repere);
  const el = card.asElement();
  if (!el) throw new Error(`écran introuvable dans la maquette : ${repere}`);
  const path = join(opts.sortie, `maquette-${slug(repere)}.png`);
  await el.screenshot({ path });
  await page.close();
  return path;
}

async function captureApercu(browser, base, scenario, theme, opts) {
  const page = await browser.newPage({
    viewport: { width: opts.width, height: opts.height },
    colorScheme: theme,
  });
  const erreurs = [];
  page.on("pageerror", (e) => erreurs.push(String(e)));
  await page.goto(`${base}?scenario=${encodeURIComponent(scenario)}`);
  await page.waitForFunction(() => document.documentElement.dataset.apercu !== undefined, null, {
    timeout: 20000,
  });
  const etat = await page.evaluate(() => document.documentElement.dataset);
  if (etat.apercu === "erreur") erreurs.push(etat.apercuMessage ?? "erreur");
  const nom = theme === "dark" ? "sombre" : "clair";
  const path = join(opts.sortie, `${slug(scenario)}-${nom}-${opts.width}.png`);
  await page.screenshot({ path });
  await page.close();
  return { path, erreurs };
}

async function cote(browser, gauche, droite, path) {
  const page = await browser.newPage({ viewport: { width: 2600, height: 900 } });
  const img = (p) => `data:image/png;base64,${readFileSync(p).toString("base64")}`;
  await page.setContent(
    `<body style="margin:0;display:flex;gap:24px;align-items:flex-start;padding:16px;background:#e9e5de;font:13px system-ui">` +
      `<figure style="margin:0"><figcaption>Application (aperçu)</figcaption><img src="${img(gauche)}"></figure>` +
      `<figure style="margin:0"><figcaption>Maquette</figcaption><img src="${img(droite)}"></figure></body>`,
  );
  const body = await page.$("body");
  await body.screenshot({ path });
  await page.close();
}

async function main() {
  const opts = options(process.argv.slice(2));
  if (opts.liste) {
    const src = readFileSync(join(ROOT, "tests", "apercu", "scenarios.ts"), "utf8");
    for (const m of src.matchAll(/^ {2}"?([\w-]+)"?: \{\n {4}description: "([^"]*)"/gm)) {
      console.log(`${m[1].padEnd(32)} ${m[2]}`);
    }
    return;
  }
  if (opts.scenarios.length === 0 && opts.maquette.length === 0) {
    throw new Error("usage : node scripts/apercu.mjs <scénario> [--maquette <repère>] [--sombre]");
  }
  mkdirSync(opts.sortie, { recursive: true });
  const { chromium } = require("playwright");
  const browser = await chromium.launch();
  let server;
  let echec = false;
  try {
    const produits = [];
    if (opts.scenarios.length > 0) {
      const { createServer } = await import("vite");
      server = await createServer({
        configFile: join(ROOT, "vite.apercu.config.ts"),
        logLevel: "warn",
      });
      await server.listen();
      const base = server.resolvedUrls.local[0];
      for (const theme of opts.themes) {
        for (const s of opts.scenarios) {
          const { path, erreurs } = await captureApercu(browser, base, s, theme, opts);
          produits.push(path);
          for (const e of erreurs) {
            echec = true;
            console.error(`✕ ${s} : ${e}`);
          }
        }
      }
    }
    const maquettes = [];
    for (const r of opts.maquette) {
      for (const theme of opts.themes.slice(0, 1)) {
        maquettes.push(await captureMaquette(browser, r, theme, opts));
      }
    }
    produits.push(...maquettes);
    if (opts.scenarios.length === 1 && maquettes.length === 1) {
      const app = produits[0];
      const path = join(opts.sortie, `comparaison-${slug(opts.scenarios[0])}.png`);
      await cote(browser, app, maquettes[0], path);
      produits.push(path);
    }
    for (const p of produits) console.log(p);
  } finally {
    await browser.close();
    await server?.close();
  }
  if (echec) process.exitCode = 1;
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
});
