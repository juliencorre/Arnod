import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { componentPaths, cssRules, cssValue } from "../css";
import type { AppWrapper } from "../mountApp";
import { openProject } from "../workbench/helpers";

/**
 * Constats relevés par le détecteur d'Impeccable sur l'aperçu rendu (docs/audit-impeccable-2026-11.md,
 * DI1 à DI6, plan-qualite-mvp T36). Le détecteur sur les sources tourne dans `npm run check` ;
 * ces tests gardent ce qu'il ne voit que sur une page rendue.
 */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const BAR = "src/components/workbench/SelectionBar.vue";
const CONTRACTS = "src/components/contracts/ContractsSection.vue";

describe("constats d'Impeccable (DI1 à DI6)", () => {
  it("DI1 : la barre de la sélection multiple est inversée, « Supprimer » en danger inverse", () => {
    // Décision Q2 du plan d'alignement (AM27) : contraste vérifié par tests/theme.spec.ts.
    expect({
      background: cssValue(BAR, ".selection", "background"),
      color: cssValue(BAR, ".selection", "color"),
      danger: cssValue(BAR, ".selection__action--danger", "color"),
    }).toEqual({
      background: "var(--color-text)",
      color: "var(--color-bg)",
      danger: "var(--color-danger-inverse)",
    });
  });

  it("DI2 : une opération sans requête est estompée par la couleur, pas par l'opacité", () => {
    const selector = ".crow__main--empty .crow__method";
    expect({
      opacity: cssValue(CONTRACTS, selector, "opacity"),
      color: cssValue(CONTRACTS, selector, "color"),
    }).toEqual({ opacity: undefined, color: "var(--color-muted)" });
  });

  it("DI3 : le texte indicatif des champs est en color-subtle, opaque, dans tous les moteurs", () => {
    expect({
      color: cssValue("src/styles.css", "::placeholder", "color"),
      opacity: cssValue("src/styles.css", "::placeholder", "opacity"),
    }).toEqual({ color: "var(--color-subtle)", opacity: "1" });
  });

  it("DI4 : une sélection est un fond accent doux, jamais une barre sur le côté", () => {
    const stripes = componentPaths().flatMap((path) =>
      cssRules(path)
        .filter((rule) =>
          /^inset\s+-?[1-4]px\s+0\s+0/.test(rule.declarations.get("box-shadow") ?? ""),
        )
        .map((rule) => `${path} ${rule.selectors.join(", ")}`),
    );
    expect({
      stripes,
      shown: cssValue(CONTRACTS, ".crow--shown", "background"),
    }).toEqual({ stripes: [], shown: "var(--color-accent-soft)" });
  });

  it("DI5 : aucun niveau de titre sauté dans l'onglet d'une requête", async () => {
    ({ app } = await openProject());
    const row = app.findAll('[role="treeitem"]').find((r) => r.text().includes("Lister"));
    await row?.trigger("click");
    await flushPromises();
    const levels = app.findAll("h1, h2, h3, h4, h5, h6").map((h) => Number(h.element.tagName[1]));
    expect(levels.filter((level, i) => i > 0 && level > (levels[i - 1] ?? 0) + 1)).toEqual([]);
  });

  it("DI6 : l'explication d'un conflit garde des lignes lisibles", () => {
    expect(
      cssValue("src/components/workbench/ConflictPanel.vue", ".conflict__text", "max-width"),
    ).toBe("72ch");
  });
});
