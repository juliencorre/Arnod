/**
 * Lisibilité (plan-qualite-mvp, lot I, T26) : couleurs d'état et de texte, bord des contrôles et
 * focus, lus dans les styles (DE1, DE2, DE4, AC5, AC6).
 */
import { describe, expect, it } from "vitest";

import { componentPaths, cssRules, cssValue } from "./css";

describe("couleurs d'état et de texte (DE1, DE2, AC6)", () => {
  it("DE1 : un message neutre de la réponse n'est pas écrit en rouge d'erreur", () => {
    const file = "src/components/workbench/ResponsePanel.vue";
    expect(cssValue(file, ".response__info", "color")).toBe("var(--color-muted)");
    expect(cssValue(file, ".response__info", "background")).toBeUndefined();
    expect(cssValue(file, ".response__error-title", "color")).toBe("var(--color-danger)");
  });

  it("DE2 : les trois textes illisibles en sombre prennent la couleur du texte ou du lien", () => {
    expect(
      cssValue("src/components/preferences/UpdatesSection.vue", ".updates__link", "color"),
    ).toBe("var(--color-link)");
    expect(
      cssValue("src/components/workbench/CopyMoveDialog.vue", ".copy-move__note--accent", "color"),
    ).toBe("var(--color-text)");
    expect(cssValue("src/components/workbench/SecretNotice.vue", ".secret__note", "color")).toBe(
      "var(--color-text)",
    );
  });

  it("DE2 : --color-accent-text n'est écrit que sur un fond accent", () => {
    const outside: string[] = [];
    for (const file of componentPaths()) {
      for (const rule of cssRules(file)) {
        const color = rule.declarations.get("color");
        if (color !== "var(--color-accent-text)") continue;
        const background =
          rule.declarations.get("background") ?? rule.declarations.get("background-color") ?? "";
        if (!background.includes("--color-accent"))
          outside.push(`${file} ${rule.selectors.join(", ")}`);
      }
    }
    expect(outside).toEqual([]);
  });

  it("AC6 : l'aide du filtre et les en-têtes ajoutés se lisent (pas --color-subtle)", () => {
    expect(cssValue("src/components/workbench/TreePanel.vue", ".tree__help", "color")).toBe(
      "var(--color-muted)",
    );
    expect(
      cssValue("src/components/workbench/RequestEditor.vue", ".sent--added .sent__name", "color"),
    ).toBe("var(--color-muted)");
  });
});

describe("vue partagée à 1280 px (DE3)", () => {
  it("l'éditeur a 58 % de la largeur et chaque rangée d'onglets tient sur une ligne", () => {
    const tab = "src/components/zones/RequestTab.vue";
    expect(cssValue(tab, ".request-tab__editor", "flex")).toBe("1 1 58%");
    expect(cssValue(tab, ".request-tab__response", "flex")).toBe("1 1 42%");
    const editor = "src/components/workbench/RequestEditor.vue";
    expect(cssValue(editor, ".editor__tabs", "flex-wrap")).toBe("nowrap");
    expect(cssValue(editor, ".editor__tab", "white-space")).toBe("nowrap");
    const response = "src/components/workbench/ResponsePanel.vue";
    expect(cssValue(response, ".response__tabs", "flex-wrap")).toBe("nowrap");
    expect(cssValue(response, ".response__tab", "white-space")).toBe("nowrap");
  });
});

describe("bord des contrôles et focus (DE4, AC5)", () => {
  it("DE4 : cases, radios et interrupteur éteint ont le bord de contrôle à 3:1", () => {
    expect(cssValue("src/styles.css", 'input[type="checkbox"]', "border")).toBe(
      "1px solid var(--color-border-control)",
    );
    expect(cssValue("src/styles.css", 'input[type="radio"]', "border")).toBe(
      "1px solid var(--color-border-control)",
    );
    expect(cssValue("src/components/ui/ToggleSwitch.vue", ".toggle__switch", "border")).toBe(
      "1px solid var(--color-border-control)",
    );
  });

  it("AC5 : le focus de la séparation prend la couleur de focus", () => {
    expect(
      cssValue(
        "src/components/zones/SplitHandle.vue",
        ".handle__grip:focus-visible .handle__line",
        "background",
      ),
    ).toBe("var(--color-focus)");
  });
});
