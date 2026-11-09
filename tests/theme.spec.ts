import { describe, expect, it } from "vitest";

import css from "@/styles.css?raw";

/** Jetons `--color-*` d'un bloc de déclarations CSS. */
function colorTokens(block: string): Map<string, string> {
  const tokens = new Map<string, string>();
  for (const match of block.matchAll(/(?<name>--color-[a-z-]+):\s*(?<value>[^;]+);/g)) {
    const { name, value } = match.groups ?? {};
    if (name !== undefined && value !== undefined) tokens.set(name, value.trim());
  }
  return tokens;
}

/** Contenu du premier bloc `:root { ... }` qui suit `from` dans la feuille de style. */
function rootBlock(from: number): string {
  const open = css.indexOf(":root {", from);
  const close = css.indexOf("}", open);
  expect(open).toBeGreaterThanOrEqual(0);
  return css.slice(open, close);
}

const light = colorTokens(rootBlock(0));
const dark = colorTokens(rootBlock(css.indexOf("@media (prefers-color-scheme: dark)")));
const themes = [
  ["clair", light],
  ["sombre", dark],
] as const;

/** Luminance relative WCAG 2 d'une couleur `#rrggbb`. */
function luminance(hex: string): number {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (match === null) throw new Error(`Couleur hexadécimale attendue : ${hex}`);
  const [r, g, b] = match.slice(1).map((channel) => {
    const value = parseInt(channel, 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
}

function contrast(foreground: string, background: string): number {
  const [lighter, darker] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return ((lighter ?? 0) + 0.05) / ((darker ?? 0) + 0.05);
}

/** Paires premier plan / fond et contraste minimal exigé. */
const PAIRS: { foreground: string[]; background: string[]; minimum: number }[] = [
  {
    foreground: ["--color-text", "--color-muted"],
    background: [
      "--color-bg",
      "--color-surface",
      "--color-sidebar",
      "--color-chip",
      "--color-accent-soft",
    ],
    minimum: 4.5,
  },
  {
    foreground: ["--color-link", "--color-success", "--color-danger"],
    background: ["--color-bg", "--color-surface", "--color-sidebar"],
    minimum: 4.5,
  },
  {
    foreground: ["--color-success", "--color-text"],
    background: ["--color-success-soft"],
    minimum: 4.5,
  },
  {
    foreground: ["--color-danger", "--color-text"],
    background: ["--color-danger-soft"],
    minimum: 4.5,
  },
  {
    foreground: ["--color-accent-text"],
    background: ["--color-accent", "--color-accent-hover"],
    minimum: 4.5,
  },
  {
    foreground: ["--color-toast-text", "--color-toast-action"],
    background: ["--color-toast-bg"],
    minimum: 4.5,
  },
  {
    foreground: ["--color-subtle", "--color-focus"],
    background: ["--color-bg", "--color-surface", "--color-sidebar"],
    minimum: 3,
  },
];

describe("thème (styles.css)", () => {
  it("définit chaque jeton de couleur dans les deux thèmes", () => {
    expect(light.size).toBeGreaterThan(0);
    expect([...dark.keys()].sort()).toEqual([...light.keys()].sort());
  });

  it.each(themes)("thème %s : couleurs en hexadécimal #rrggbb", (_name, tokens) => {
    for (const [name, value] of tokens) {
      expect(value, name).toMatch(/^#[0-9a-f]{6}$/i);
    }
  });

  it.each(themes)("thème %s : contrastes WCAG respectés", (_name, tokens) => {
    for (const { foreground, background, minimum } of PAIRS) {
      for (const fg of foreground) {
        for (const bg of background) {
          const fgValue = tokens.get(fg);
          const bgValue = tokens.get(bg);
          expect(fgValue, fg).toBeDefined();
          expect(bgValue, bg).toBeDefined();
          const ratio = contrast(fgValue ?? "", bgValue ?? "");
          expect(ratio, `${fg} sur ${bg} : ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(minimum);
        }
      }
    }
  });
});
