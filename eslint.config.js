import skipFormatting from "@vue/eslint-config-prettier/skip-formatting";
import { defineConfigWithVueTs, vueTsConfigs } from "@vue/eslint-config-typescript";
import pluginVue from "eslint-plugin-vue";

export default defineConfigWithVueTs(
  {
    name: "arnod/ignores",
    // docs/design-system/ : types de documentation du design system, hors projet TypeScript.
    ignores: ["dist/**", "src-tauri/target/**", "src-tauri/gen/**", "coverage/**", "docs/**"],
  },
  pluginVue.configs["flat/recommended"],
  vueTsConfigs.strictTypeChecked,
  {
    name: "arnod/rules",
    rules: {
      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/restrict-template-expressions": ["error", { allowNumber: true }],
    },
  },
  {
    name: "arnod/tests",
    files: ["tests/**/*.ts"],
    rules: {
      // Les mocks IPC renvoient volontairement des valeurs hétérogènes.
      "@typescript-eslint/only-throw-error": "off",
    },
  },
  // Prettier gère la mise en forme : on désactive les règles de style concurrentes.
  skipFormatting,
);
