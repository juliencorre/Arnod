#!/usr/bin/env node
// Mesure la taille d'un paquet produit par `tauri build`, l'affiche, l'ajoute
// au résumé GitHub Actions si disponible et échoue au-delà d'une limite.
//
// Usage : node scripts/bundle-size.mjs <nsis|appimage> [--max-mb <Mo>]
// 1 Mo = 1 000 000 octets.
import { appendFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";

const BUNDLES = {
  nsis: {
    label: "Installeur Windows (NSIS)",
    dir: "src-tauri/target/release/bundle/nsis",
    suffix: "-setup.exe",
  },
  appimage: {
    label: "AppImage Linux",
    dir: "src-tauri/target/release/bundle/appimage",
    suffix: ".AppImage",
  },
};

const { positionals, values } = parseArgs({
  allowPositionals: true,
  options: { "max-mb": { type: "string" } },
});

const kind = positionals[0];
const bundle = BUNDLES[kind];
if (!bundle) {
  console.error(
    `Usage : node scripts/bundle-size.mjs <${Object.keys(BUNDLES).join("|")}> [--max-mb <Mo>]`,
  );
  process.exit(2);
}

const files = existsSync(bundle.dir)
  ? readdirSync(bundle.dir).filter((name) => name.endsWith(bundle.suffix))
  : [];
if (files.length === 0) {
  console.error(
    `Aucun fichier *${bundle.suffix} dans ${bundle.dir}. Lancer « npm run tauri build » d'abord.`,
  );
  process.exit(1);
}

const maxMb = values["max-mb"] === undefined ? undefined : Number(values["max-mb"]);
const rows = files.map((name) => {
  const bytes = statSync(join(bundle.dir, name)).size;
  const mb = bytes / 1_000_000;
  return { name, bytes, mb, ok: maxMb === undefined || mb < maxMb };
});

const format = (mb) => mb.toLocaleString("fr-FR", { maximumFractionDigits: 2 });
const lines = [
  `### ${bundle.label}`,
  "",
  "| Fichier | Taille | Octets |" + (maxMb === undefined ? "" : " Cible |"),
  "|---|---|---|" + (maxMb === undefined ? "" : "---|"),
  ...rows.map(
    (r) =>
      `| \`${r.name}\` | ${format(r.mb)} Mo | ${r.bytes} |` +
      (maxMb === undefined ? "" : ` ${r.ok ? "✅" : "❌"} < ${maxMb} Mo |`),
  ),
  "",
];
const report = lines.join("\n");

console.log(report);
if (process.env.GITHUB_STEP_SUMMARY) {
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, report + "\n");
}

if (rows.some((r) => !r.ok)) {
  console.error(`Taille supérieure à la cible de ${maxMb} Mo.`);
  process.exit(1);
}
