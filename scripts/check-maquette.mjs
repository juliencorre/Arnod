#!/usr/bin/env node
// Vérifie docs/maquette-etat.yml, l'état de chaque écran de la maquette par rapport au code :
// structure, états connus, plans et captures qui existent, traçabilité des écrans livrés, écarts
// renseignés, et repères d'écran cités dans la documentation qui existent. Lancé par
// `npm run check` et par la CI. Affiche le décompte par état.
//
// Usage : node scripts/check-maquette.mjs
import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { parse } from "yaml";

const FILE = "docs/maquette-etat.yml";
const STATES = ["livré", "en partie", "à faire", "variante", "remplacé", "à décider", "référence"];
const VERSIONS = ["Cadre", "MVP", "V1", "V2", "V3", "V4", "V5", "V6", "V7", "V8", "V9"];
const ALIGN = ["maquette", "code", "à décider"];
const PAGE_STATES = ["livré", "en partie", "à faire", "référence"];
const CODE = /^[0-7]\.\d+$/;
const LETTER = /^(0|[a-z]\d?|e — survol)( (bis|ter|quater|quinquies|sexies))?$/;
// Repère cité dans un texte : « 3.2-a2 », « 1.2-d bis ».
const CITED = /\b([0-7]\.\d+)-([a-z]\d?(?: (?:bis|ter|quater|quinquies|sexies))?)\b/g;

const errors = [];
const fail = (where, message) => errors.push(`${where} : ${message}`);

const doc = parse(readFileSync(FILE, "utf8"));
if (doc?.version !== 1 || typeof doc.pages !== "object") {
  console.error(`${FILE} : « version: 1 » et « pages » sont attendus.`);
  process.exit(1);
}

/** Noms des captures prises par les tests e2e (`screenshot(browser, "…")`). */
const e2e = readFileSync("e2e/arnod.e2e.ts", "utf8");
const captures = new Set([...e2e.matchAll(/screenshot\(browser, "([^"]+)"\)/g)].map((m) => m[1]));

const list = (value) => (value === undefined ? [] : Array.isArray(value) ? value : [value]);
const counts = Object.fromEntries(STATES.map((s) => [s, 0]));
const known = new Set();
let gaps = 0;

for (const [code, page] of Object.entries(doc.pages)) {
  const where = `page ${code}`;
  if (!CODE.test(code)) fail(where, "code de page attendu sous la forme « 3.2 »");
  if (typeof page?.nom !== "string" || page.nom === "") fail(where, "« nom » manquant");
  for (const v of list(page?.version)) {
    if (!VERSIONS.includes(v)) fail(where, `version inconnue « ${v} »`);
  }
  if (page?.etat !== undefined && !PAGE_STATES.includes(page.etat)) {
    fail(where, `état de page inconnu « ${page.etat} »`);
  }
  for (const plan of list(page?.plans)) {
    const file = `docs/${String(plan).split(" ")[0]}.md`;
    if (!existsSync(file)) fail(where, `plan introuvable : ${file}`);
  }
  for (const linked of list(page?.liees)) {
    if (!(String(linked) in doc.pages)) fail(where, `page liée inconnue « ${linked} »`);
  }
  const screens = page?.ecrans ?? {};
  for (const [letter, screen] of Object.entries(screens)) {
    const id = `${code}-${letter}`;
    known.add(id);
    if (!LETTER.test(letter)) fail(id, "lettre d'écran inattendue");
    const state = screen?.etat;
    if (!STATES.includes(state)) {
      fail(id, `état inconnu « ${state} » (${STATES.join(", ")})`);
      continue;
    }
    counts[state] += 1;
    if (screen.version !== undefined && !VERSIONS.includes(screen.version)) {
      fail(id, `version inconnue « ${screen.version} »`);
    }
    if (
      (state === "livré" || state === "en partie") &&
      list(screen.pr).length === 0 &&
      list(page.plans).length === 0
    ) {
      fail(id, "un écran livré ou en partie cite sa pull request (pr) ou le plan de sa page");
    }
    for (const pr of list(screen.pr)) {
      if (!Number.isInteger(pr) || pr <= 0) fail(id, `numéro de pull request invalide « ${pr} »`);
    }
    for (const capture of list(screen.captures)) {
      if (!captures.has(capture)) fail(id, `capture inconnue des tests e2e : « ${capture} »`);
    }
    if (screen.ecart !== undefined) {
      gaps += 1;
      if (typeof screen.ecart?.texte !== "string" || screen.ecart.texte === "") {
        fail(id, "un écart dit ce qui diffère (texte)");
      }
      if (!ALIGN.includes(screen.ecart?.aligner)) {
        fail(id, `un écart dit quoi aligner : ${ALIGN.join(", ")}`);
      }
    }
  }
}

for (const [i, gap] of (doc.transverses ?? []).entries()) {
  const where = `écart transverse ${String(i + 1)}`;
  gaps += 1;
  if (typeof gap?.texte !== "string" || gap.texte === "") fail(where, "texte manquant");
  if (!ALIGN.includes(gap?.aligner)) fail(where, `quoi aligner : ${ALIGN.join(", ")}`);
  if (gap?.page !== undefined && !(String(gap.page) in doc.pages)) {
    fail(where, `page inconnue « ${String(gap.page)} »`);
  }
}

/** Fichiers Markdown du dépôt où un repère peut être cité. */
function markdown(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === "opencollection" ? [] : markdown(full);
    return entry.name.endsWith(".md") ? [full] : [];
  });
}

for (const file of ["README.md", "CLAUDE.md", ...markdown("docs")]) {
  const text = readFileSync(file, "utf8");
  for (const match of text.matchAll(CITED)) {
    const id = `${match[1]}-${match[2]}`;
    if (match[1] in doc.pages && !known.has(id)) fail(file, `repère inconnu « ${id} »`);
  }
}

if (errors.length > 0) {
  console.error(`${FILE} : ${String(errors.length)} problème(s)`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
const total = Object.values(counts).reduce((a, b) => a + b, 0);
const summary = STATES.filter((s) => counts[s] > 0)
  .map((s) => `${String(counts[s])} ${s}`)
  .join(" · ");
console.log(
  `Maquette : ${String(Object.keys(doc.pages).length)} pages, ${String(total)} écrans (${summary}), ` +
    `${String(gaps)} écart(s) avec le code.`,
);
