/**
 * Formes et dispositions (docs/plan-alignement-maquette.md, lot B) : ce que la maquette dessine
 * et que Vitest ne calcule pas, lu dans les styles (tests/css.ts). Les comportements sont
 * vérifiés avec leurs composants.
 */
import { describe, expect, it } from "vitest";

import { cssValue } from "./css";

describe("formes et dispositions (lot B)", () => {
  it("AM16 (1.1-e, 2.2-f) : recherche et boutons à droite du titre de « Tous les projets »", () => {
    const file = "src/components/projects/AllProjects.vue";
    expect(cssValue(file, ".all__intro", "flex")).toBe("1 1 18rem");
    expect(cssValue(file, ".all__tools", "flex")).toBe("0 1 34rem");
    expect(cssValue(file, ".all__tools", "flex-wrap")).toBe("nowrap");
  });

  it("AM18 (1.2-e, 4.1-g) : aucun champ de recherche ne montre le bouton natif d'effacement", () => {
    const selector = 'input[type="search"]::-webkit-search-cancel-button';
    expect(cssValue("src/styles.css", selector, "display")).toBe("none");
    expect(cssValue("src/styles.css", selector, "-webkit-appearance")).toBe("none");
    // 4.1-g : la loupe est une icône posée sur le champ, plus un caractère du texte indicatif.
    expect(
      cssValue("src/components/workbench/CopyMoveDialog.vue", ".copy-move__loupe", "position"),
    ).toBe("absolute");
  });

  it("AM19 (1.3-a, Q1) : × visible sur l'onglet actif, sur un autre au survol ou au focus", () => {
    const file = "src/components/zones/ZoneView.vue";
    expect(cssValue(file, ".zone__close", "opacity")).toBe("0");
    for (const shown of [
      ".zone__tab--active .zone__close",
      ".zone__tab:hover .zone__close",
      ".zone__tab:focus-within .zone__close",
    ]) {
      expect(cssValue(file, shown, "opacity")).toBe("1");
    }
  });

  it("AM23 (2.2-b, 2.2-d) : libellés des champs en texte secondaire, taille small", () => {
    for (const [file, selector] of [
      ["src/components/workbench/TrialTransfer.vue", ".transfer__label"],
      ["src/components/contracts/FileTrialsDialog.vue", ".filing__label"],
    ] as const) {
      expect(cssValue(file, selector, "color")).toBe("var(--color-muted)");
      expect(cssValue(file, selector, "font-size")).toBe("var(--text-small)");
      expect(cssValue(file, selector, "font-weight")).toBeUndefined();
    }
  });

  it("AM25 (3.2-a) : une rangée au-dessus de l'adresse ; Nom, Description, Étiquettes sur une rangée", () => {
    const file = "src/components/workbench/RequestEditor.vue";
    expect(cssValue(file, ".editor__top", "flex-wrap")).toBe("nowrap");
    expect(cssValue(file, ".editor__crumbs", "flex-wrap")).toBe("nowrap");
    expect(cssValue(file, ".editor__contract", "text-overflow")).toBe("ellipsis");
    expect(cssValue(file, ".editor__saved", "flex")).toBe("none");
    expect(cssValue(file, ".editor__meta", "grid-template-columns")).toBe(
      "repeat(auto-fit, minmax(10rem, 1fr))",
    );
  });

  it("AM26 (3.4-a) : le libellé « Type » au-dessus de sa liste, comme les autres champs", () => {
    const file = "src/components/workbench/AuthEditor.vue";
    expect(cssValue(file, ".field", "flex-direction")).toBe("column");
    expect(cssValue(file, ".field", "grid-template-columns")).toBeUndefined();
  });

  it("AM27 (4.1-a quater) : « Supprimer » de la barre de sélection en rouge sur fond inversé", () => {
    expect(
      cssValue("src/components/workbench/SelectionBar.vue", ".selection__action--danger", "color"),
    ).toBe("var(--color-danger-inverse)");
  });

  it("AM17 (1.2-a bis) : une ligne d'ajout de menu en couleur de lien", () => {
    expect(cssValue("src/components/ui/BaseMenu.vue", ".menu__item--link", "color")).toBe(
      "var(--color-link)",
    );
  });
});
