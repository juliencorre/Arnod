#!/usr/bin/env node
/**
 * Lot L (`docs/plan-mvp-l-mises-a-jour.md`, MJ7) : écrit `latest.json`, le manifeste qu'Arnod lit
 * pour s'installer, d'après les paquets signés d'un dossier : installeur NSIS
 * (`Arnod_X.Y.Z_x64-setup.exe`) et AppImage (`Arnod_X.Y.Z_amd64.AppImage`), chacun avec son
 * `.sig` produit par `tauri build` (`bundle.createUpdaterArtifacts`, clé privée de la CI).
 *
 * Usage :
 *   node scripts/updater-manifest.mjs <dossier> [vX.Y.Z] [--partiel] [--sortie <fichier>]
 *
 * Le tag vaut par défaut `v` + la version de `package.json`. Sans `--partiel` (release), il faut
 * les deux plateformes et une clé publique dans `tauri.conf.json`. Chaque signature doit porter la
 * version du tag (`requireSignedVersion`). Le manifeste est écrit dans `<dossier>/latest.json`,
 * ou dans `--sortie`.
 */
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DOWNLOADS = "https://github.com/juliencorre/Arnod/releases/download";

/** Plateformes publiées : clés du manifeste (générique, puis propre à l'installeur). */
const PLATFORMS = [
  {
    name: "Windows (NSIS)",
    pattern: /_x64-setup\.exe$/,
    keys: ["windows-x86_64", "windows-x86_64-nsis"],
  },
  {
    name: "Linux (AppImage)",
    pattern: /_amd64\.AppImage$/,
    keys: ["linux-x86_64", "linux-x86_64-appimage"],
  },
];

function fail(message) {
  console.error(`updater-manifest : ${message}`);
  process.exit(1);
}

function options(argv) {
  const opts = { dir: null, tag: null, partial: false, out: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--partiel") opts.partial = true;
    else if (a === "--sortie") opts.out = argv[++i] ?? fail("--sortie attend un fichier");
    else if (opts.dir === null) opts.dir = a;
    else if (opts.tag === null) opts.tag = a;
    else fail(`argument inattendu « ${a} »`);
  }
  if (opts.dir === null) fail("dossier des paquets attendu");
  if (opts.tag === null) {
    const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
    opts.tag = `v${pkg.version}`;
  }
  if (!/^v\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(opts.tag)) fail(`tag « ${opts.tag} » invalide`);
  return opts;
}

/** Version signée, lue dans le commentaire de confiance de la signature minisign. */
function signedVersion(signature) {
  const text = Buffer.from(signature.trim(), "base64").toString("utf8");
  const trusted = text.split("\n").find((line) => line.startsWith("trusted comment: "));
  const field = trusted
    ?.slice("trusted comment: ".length)
    .split("\t")
    .find((part) => part.startsWith("version:"));
  return field?.slice("version:".length) ?? null;
}

const opts = options(process.argv.slice(2));
const version = opts.tag.slice(1);
const config = JSON.parse(readFileSync(join(ROOT, "src-tauri", "tauri.conf.json"), "utf8"));
const pubkey = config.plugins?.updater?.pubkey ?? "";
if (pubkey.trim() === "") {
  const message =
    "clé publique absente de tauri.conf.json (plugins.updater.pubkey) : Arnod ne pourrait pas vérifier ces paquets.";
  if (opts.partial) console.warn(`updater-manifest : avertissement, ${message}`);
  else fail(message);
}

const files = readdirSync(opts.dir);
const platforms = {};
for (const platform of PLATFORMS) {
  const packages = files.filter((f) => platform.pattern.test(f));
  if (packages.length === 0) {
    if (opts.partial) continue;
    fail(`aucun paquet ${platform.name} dans ${opts.dir}`);
  }
  if (packages.length > 1) fail(`plusieurs paquets ${platform.name} : ${packages.join(", ")}`);
  const name = packages[0];
  if (!name.includes(`_${version}_`)) fail(`${name} n'est pas un paquet de la version ${version}`);
  const sigPath = join(opts.dir, `${name}.sig`);
  if (!existsSync(sigPath)) fail(`${name} n'a pas de signature (${name}.sig)`);
  const signature = readFileSync(sigPath, "utf8").trim();
  const signed = signedVersion(signature);
  if (signed !== version) {
    fail(`${name} est signé pour la version ${signed ?? "(aucune)"} au lieu de ${version}`);
  }
  const entry = { url: `${DOWNLOADS}/${opts.tag}/${encodeURIComponent(name)}`, signature };
  for (const key of platform.keys) platforms[key] = entry;
}
if (Object.keys(platforms).length === 0) fail(`aucun paquet signé dans ${opts.dir}`);

const manifest = {
  version,
  notes: `Arnod ${version}`,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms,
};
const out = opts.out ?? join(opts.dir, "latest.json");
writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`${out} : Arnod ${version}, ${Object.keys(platforms).join(", ")}`);
