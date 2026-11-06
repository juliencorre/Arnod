/// <reference types="vitest/config" />
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Options propres à Tauri, utilisées par `tauri dev` et `tauri build`.
  // 1. Ne pas masquer les erreurs Rust.
  clearScreen: false,
  server: {
    // 2. Tauri attend un port fixe : échouer s'il est occupé.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. Ne pas surveiller `src-tauri`.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Cibles des WebView : WebView2 (Chromium) sous Windows, WebKitGTK sous Linux.
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari15",
  },

  test: {
    environment: "happy-dom",
    include: ["tests/**/*.spec.ts"],
    // Contenu réel de la feuille de style globale, lue par tests/theme.spec.ts.
    css: { include: [/src\/styles\.css/] },
    restoreMocks: true,
  },
});
