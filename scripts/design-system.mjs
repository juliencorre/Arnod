#!/usr/bin/env node
// Génère la partie calculée du design system d'Arnod (docs/design-system/) depuis le code :
// jetons (tokens.json et tokens.css) tirés de src/styles.css et de la grille de la barre latérale,
// styles des composants (components/bundle.css) recopiés des composants Vue, icônes des sections
// (assets/Icones/*.svg). Le reste du dossier (README, guides et aperçus des composants, couverture)
// est écrit à la main. Avec --check, ne réécrit rien et échoue si un fichier n'est plus à jour
// (`npm run check`, CI).
//
// Usage : node scripts/design-system.mjs [--check]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const OUT = "docs/design-system";
const CHECK = process.argv.includes("--check");

/** Note d'usage de chaque couleur : un jeton ajouté à styles.css sans note fait échouer le script. */
const COLOR_NOTES = {
  "color-bg": "Fond de la zone principale. Texte : color-text, color-muted.",
  "color-surface":
    "Cartes, champs, menus, fenêtres modales, éléments posés sur color-bg. Texte : color-text, color-muted.",
  "color-sidebar": "Barre latérale et barres d'onglets. Texte : color-text, color-muted.",
  "color-chip": "Badges et pastilles neutres (« par défaut »), fond de l'interrupteur éteint.",
  "color-accent-soft":
    "Ligne sélectionnée ou mise en évidence, élément actif d'un menu. Texte : color-text.",
  "color-border": "Bordure des champs, cartes, menus et fenêtres ; séparateurs.",
  "color-border-strong":
    "Cases à cocher, bord de l'interrupteur, séparateurs marqués, survol d'un bouton.",
  "color-text":
    "Texte principal, 4,5:1 sur color-bg, color-surface, color-sidebar, color-chip et color-accent-soft (vérifié par tests/theme.spec.ts).",
  "color-muted":
    "Texte secondaire (descriptions, compteurs, en-têtes de menu, icônes des sections), 4,5:1 sur les mêmes fonds que color-text.",
  "color-subtle":
    "Indications, textes désactivés, texte indicatif des champs : 3:1 au moins sur color-bg, color-surface et color-sidebar.",
  "color-link":
    "Liens et actions textuelles (« + Ajouter un projet »), méthodes POST, PUT et PATCH, mentions de menu : 4,5:1 sur color-bg, color-surface, color-sidebar.",
  "color-success":
    "Réussite : ✓ d'un statut, méthode GET, vérification réussie ; 4,5:1 sur color-bg, color-surface, color-sidebar et color-success-soft.",
  "color-success-soft": "Fond d'une vérification ou d'une ligne de run réussie.",
  "color-danger":
    "Échec, erreur, action destructrice, méthode DELETE ; 4,5:1 sur color-bg, color-surface, color-sidebar et color-danger-soft.",
  "color-danger-soft": "Fond d'une vérification ou d'une ligne de run en échec.",
  "color-accent":
    "Seule couleur d'action : fond du bouton principal, interrupteur allumé. Texte posé dessus : color-accent-text.",
  "color-accent-hover": "Bouton principal survolé.",
  "color-accent-text": "Texte sur color-accent et color-accent-hover, 4,5:1 dans les deux thèmes.",
  "color-focus":
    "Contour de focus au clavier : 2 px plein, décalé de 2 px ; 3:1 sur color-bg, color-surface et color-sidebar.",
  "color-toast-bg": "Fond des notifications (sombre en thème clair, relevé en thème sombre).",
  "color-toast-text": "Texte des notifications, 4,5:1 sur color-toast-bg.",
  "color-toast-action": "Action d'une notification (« Annuler »), 4,5:1 sur color-toast-bg.",
  backdrop: "Voile derrière une fenêtre modale (::backdrop de BaseDialog).",
};
const WORKSPACES = {
  gray: "Gris",
  orange: "Orange",
  blue: "Bleu",
  green: "Vert",
  purple: "Violet",
  red: "Rouge",
  yellow: "Jaune",
  teal: "Turquoise",
};
const FAMILIES = {
  calls: "Appels",
  data: "Données",
  messages: "Messages",
  jobs: "Traitements",
  interface: "Interface",
  code: "Code",
  files: "Fichiers",
};
for (const [key, label] of Object.entries(WORKSPACES)) {
  COLOR_NOTES[`color-ws-${key}`] =
    `Pastille du workspace « ${label} » (ColorDot, menus) : décorative, toujours à côté du nom du workspace.`;
}
for (const [key, label] of Object.entries(FAMILIES)) {
  COLOR_NOTES[`color-family-${key}-bg`] =
    `Famille « ${label} » : fond de la pastille de nature (NaturePill).`;
  COLOR_NOTES[`color-family-${key}-border`] =
    `Famille « ${label} » : bord de la pastille de nature.`;
  COLOR_NOTES[`color-family-${key}-text`] =
    `Famille « ${label} » : texte de la pastille, 4,5:1 sur son fond et sur color-surface.`;
}

const SHADOW_NOTES = {
  "shadow-popover": "Menus, sous-menus et notifications.",
  "shadow-dialog": "Fenêtres modales (BaseDialog).",
};

// Échelles retenues le 12 novembre 2026 (README, « Écarts à résorber ») : celles de la maquette,
// où le code s'aligne. La taille de base de l'application reste 16 px (racine) ; ces tailles sont
// celles que posent les composants.
const TYPE_GROUPS = [
  {
    name: "Interface",
    family: "sans",
    styles: [
      [
        "text-display",
        "24px",
        600,
        "Titre d'une vue entière : Tous les projets, accueil d'un projet, catalogue des étapes, run.",
      ],
      [
        "text-title",
        "18px",
        600,
        "Titre d'un onglet (contrat, catalogue) ou d'une fenêtre modale.",
      ],
      [
        "text-body-large",
        "15px",
        400,
        "Champ principal : adresse de la requête, entrée « Essayer une étape ».",
      ],
      [
        "text-body",
        "13px",
        400,
        "Texte courant de l'interface : arborescence, sections, menus, panneaux. Taille la plus employée.",
      ],
      ["text-small", "12px", 400, "Texte secondaire : pastilles, statuts, détails, légendes."],
      [
        "text-caption",
        "11px",
        600,
        "Plus petite taille de texte : compteurs, en-têtes de section.",
      ],
    ],
  },
  {
    name: "Données",
    family: "mono",
    styles: [
      [
        "mono-caption",
        "11px",
        700,
        "Méthodes HTTP de l'arborescence, compteurs, en-têtes de menu (capitales, espacement 0,05em).",
      ],
      [
        "mono-small",
        "12px",
        400,
        "Raccourcis clavier et détails d'un menu, pastilles de nature, chemins et valeurs.",
      ],
    ],
  },
];

const SPACING = [
  ["space-2", "2px", "Décalage du contour de focus ; marges fines."],
  ["space-4", "4px", "Petits écarts : bouton lien, marge au-dessus d'un en-tête de section."],
  ["space-6", "6px", "Écart icône-libellé d'un bouton, rembourrage des menus et de leurs lignes."],
  ["space-8", "8px", "Rembourrage vertical des boutons et des champs ; écart le plus employé."],
  [
    "space-10",
    "10px",
    "Écart icône-libellé des menus et des interrupteurs ; rembourrage horizontal d'une ligne de menu.",
  ],
  [
    "space-12",
    "12px",
    "Rembourrage horizontal des champs ; écart des boutons d'un pied de fenêtre modale.",
  ],
  [
    "space-16",
    "16px",
    "Rembourrage horizontal des boutons ; écart entre les blocs d'une fenêtre modale.",
  ],
  ["space-20", "20px", "Écart entre le message et l'action d'une notification."],
  ["space-24", "24px", "Rembourrage des fenêtres modales."],
  ["space-32", "32px", "Marges des vues entières (Tous les projets, accueil d'un workspace)."],
];
const TREE_NOTES = {
  "tree-pad": "Barre latérale : marge des lignes, à gauche et à droite.",
  "tree-indent": "Barre latérale : retrait d'un niveau de l'arborescence.",
  "tree-lead": "Barre latérale : colonne de tête (chevron, pastille ou icône), de largeur fixe.",
  "tree-gap": "Barre latérale : écart entre la colonne de tête et le libellé.",
};

const RADIUS = [
  ["radius-sm", "2px", "Petites marques : nuancier d'un menu, coins d'une pastille compacte."],
  [
    "radius-md",
    "6px",
    "Lignes de menu, actions de notification, lignes de l'arborescence, boutons et champs.",
  ],
  ["radius-lg", "10px", "Menus, notifications, cartes et fenêtres modales."],
  ["radius-pill", "999px", "Formes rondes : interrupteur, pastilles de nature et d'état."],
];

// Composants dont les styles sont recopiés dans components/bundle.css, dans cet ordre.
const COMPONENTS = {
  Button: "src/components/ui/BaseButton.vue",
  Toggle: "src/components/ui/ToggleSwitch.vue",
  SearchInput: "src/components/ui/SearchInput.vue",
  Menu: "src/components/ui/BaseMenu.vue",
  Dialog: "src/components/ui/BaseDialog.vue",
  Toast: "src/components/ui/ToastHost.vue",
  SectionHeading: "src/components/ui/SectionHeading.vue",
  SectionIcon: "src/components/ui/SectionIcon.vue",
  ColorDot: "src/components/ui/ColorDot.vue",
  StatusMark: "src/components/workbench/StatusMark.vue",
  NaturePill: "src/components/steps/NaturePill.vue",
};

const read = (file) => readFileSync(file, "utf8");
const errors = [];

/** Variables `--nom: valeur;` d'un bloc CSS, avec leur commentaire de fin de ligne. */
function variables(block) {
  const found = new Map();
  for (const m of block.matchAll(/^\s*--([a-z0-9-]+):\s*([^;]+);(?:\s*\/\*\s*(.*?)\s*\*\/)?/gm)) {
    found.set(m[1], { value: m[2].trim(), comment: m[3] ?? "" });
  }
  return found;
}

/** `rgb(29 27 24 / 14%)` → `rgba(29, 27, 24, 0.14)` : la forme que lit la page du design system. */
const rgba = (value) =>
  value.replace(
    /rgb\((\d+) (\d+) (\d+) \/ (\d+)%\)/g,
    (_, r, g, b, a) => `rgba(${r}, ${g}, ${b}, ${String(Number(a) / 100)})`,
  );

const css = read("src/styles.css");
const [lightPart, darkPart = ""] = css.split("@media (prefers-color-scheme: dark)");
const light = variables(lightPart);
const dark = variables(darkPart);
const sans = /font-family:\s*([^;]+);/.exec(lightPart)?.[1].replace(/\s+/g, " ").trim() ?? "";

const colors = [];
const shadows = [];
for (const [name, { value, comment }] of light) {
  const darkValue = dark.get(name)?.value;
  if (name.startsWith("color-") || name === "backdrop") {
    const usage = COLOR_NOTES[name] ?? "";
    if (usage === "")
      errors.push(
        `--${name} : note d'usage manquante dans COLOR_NOTES (scripts/design-system.mjs)${comment ? ` ; commentaire : « ${comment} »` : ""}`,
      );
    if (darkValue === undefined) errors.push(`--${name} : valeur du thème sombre manquante`);
    colors.push({ name, value: { light: rgba(value), dark: rgba(darkValue ?? value) }, usage });
  } else if (name.startsWith("shadow-")) {
    shadows.push({
      name,
      value: { light: rgba(value), dark: rgba(darkValue ?? value) },
      usage: SHADOW_NOTES[name] ?? "",
    });
  }
}
const mono = light.get("font-mono")?.value ?? "";

const sidebar = variables(read("src/components/layout/AppSidebar.vue"));
const tree = Object.entries(TREE_NOTES).map(([name, usage]) => {
  const value = sidebar.get(name)?.value ?? "";
  if (value === "") errors.push(`--${name} introuvable dans AppSidebar.vue`);
  const rem = /^([\d.]+)rem$/.exec(value);
  return { name, value: rem ? `${String(Number(rem[1]) * 16)}px` : value, usage };
});

const tokens = {
  name: "Arnod",
  version: 1,
  meta: {
    source: "github",
    repo: "juliencorre/Arnod",
    generator: "scripts/design-system.mjs",
    paths: {
      tokens: ["src/styles.css", "src/components/layout/AppSidebar.vue"],
      assets: ["src/components/ui/sectionIcons.ts", "src/components/ui/TrayIcon.vue"],
      docs: ["docs/design-system/README.md"],
    },
    components: COMPONENTS,
  },
  color: {
    themes: [
      { id: "light", name: "Clair" },
      { id: "dark", name: "Sombre" },
    ],
    tokens: colors,
  },
  type: {
    fonts: [],
    families: { sans, mono },
    groups: TYPE_GROUPS.map((group) => ({
      name: group.name,
      family: group.family,
      styles: group.styles.map(([name, fontSize, fontWeight, usage]) => ({
        name,
        fontSize,
        lineHeight: 1.5,
        fontWeight,
        usage,
      })),
    })),
  },
  spacing: {
    note: "Pas observés dans les composants, puis la grille des lignes de la barre latérale (AppSidebar.vue).",
    tokens: [...SPACING.map(([name, value, usage]) => ({ name, value, usage })), ...tree],
  },
  radius: {
    note: "Échelle retenue : 2, 6 et 10 px, plus les formes rondes.",
    tokens: RADIUS.map(([name, value, usage]) => ({ name, value, usage })),
  },
  shadow: { tokens: shadows },
};

// tokens.css, disposé comme le compile la page du design system (format.md).
const decl = (name, value) => `  --${name}: ${value};`;
const cssOut = [
  "/* Arnod — generated from tokens.json */",
  ':root, [data-theme="light"] {',
  ...colors.map((t) => decl(t.name, t.value.light)),
  ...shadows.map((t) => decl(t.name, t.value.light)),
  "}",
  '[data-theme="dark"] {',
  ...colors.map((t) => decl(t.name, t.value.dark)),
  ...shadows.map((t) => decl(t.name, t.value.dark)),
  "}",
  ":root {",
  ...tokens.spacing.tokens.map((t) => decl(t.name, t.value)),
  ...tokens.radius.tokens.map((t) => decl(t.name, t.value)),
  decl("font-sans", sans),
  decl("font-mono", mono),
  "}",
  ...tokens.type.groups.flatMap((group) =>
    group.styles.map(
      (s) =>
        `.${s.name} { font-family: var(--font-${group.family}); font-size: ${s.fontSize}; line-height: ${String(s.lineHeight)}; font-weight: ${String(s.fontWeight)}; }`,
    ),
  ),
  "",
].join("\n");

// components/bundle.css : styles des composants, tels qu'écrits dans le code.
const bundle = [
  "/* Styles des composants d'Arnod, recopiés du code par scripts/design-system.mjs (ne pas modifier). */",
  "",
  "/* src/styles.css */",
  /\.visually-hidden \{[^}]*\}/.exec(css)?.[0] ?? "",
  ...Object.values(COMPONENTS).map((file) => {
    const style = /<style scoped>([\s\S]*?)<\/style>/.exec(read(file))?.[1].trim() ?? "";
    if (style === "") errors.push(`${file} : pas de bloc <style scoped>`);
    return `\n/* ${file} */\n${style}`;
  }),
  "",
].join("\n");

// Icônes : traits de 1,4 px sur 16 px, encre color-muted du thème clair (une image ne prend pas
// la couleur du texte).
const ink = light.get("color-muted")?.value ?? "#6b665e";
const icons = new Map();
const iconSource = read("src/components/ui/sectionIcons.ts");
for (const m of iconSource.matchAll(/^ {2}([a-z]+): \[([\s\S]*?)\],?$/gm)) {
  const paths = [...m[2].matchAll(/"([^"]+)"/g)].map((p) => `<path d="${p[1]}"/>`).join("");
  icons.set(
    `section-${m[1]}.svg`,
    `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="${ink}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>\n`,
  );
}
const tray = [...read("src/components/ui/TrayIcon.vue").matchAll(/d="([^"]+)"/g)].map((m) => m[1]);
if (icons.size === 0 || tray.length !== 2)
  errors.push("icônes introuvables (sectionIcons.ts, TrayIcon.vue)");
icons.set(
  "non-classes.svg",
  `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="${ink}" stroke-width="1.1" stroke-linejoin="round"><path d="${tray[0] ?? ""}" fill="${light.get("color-surface")?.value ?? "#ffffff"}"/><path d="${tray[1] ?? ""}"/></svg>\n`,
);

if (errors.length > 0) {
  console.error(`Design system : ${String(errors.length)} problème(s)`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}

const outputs = new Map([
  ["tokens.json", `${JSON.stringify(tokens, null, 2)}\n`],
  ["tokens.css", cssOut],
  ["components/bundle.css", bundle],
  ...[...icons].map(([name, svg]) => [`assets/Icones/${name}`, svg]),
]);

const stale = [];
for (const [rel, content] of outputs) {
  const file = path.join(OUT, rel);
  const current = existsSync(file) ? read(file) : null;
  if (current === content) continue;
  if (CHECK) {
    stale.push(file);
  } else {
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, content);
    console.log(`écrit : ${file}`);
  }
}
if (stale.length > 0) {
  console.error("Design system pas à jour avec le code : lancer node scripts/design-system.mjs");
  for (const file of stale) console.error(`  - ${file}`);
  process.exit(1);
}
console.log(
  `Design system : ${String(colors.length)} couleurs × 2 thèmes, ${String(tokens.spacing.tokens.length)} espacements, ` +
    `${String(RADIUS.length)} rayons, ${String(Object.keys(COMPONENTS).length)} composants, ${String(icons.size)} icônes.`,
);
