// Aperçu de l'application sur le faux cœur (tests/apercu), pour les captures Playwright de
// scripts/apercu.mjs. Hors build et hors CI.
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

export default defineConfig({
  root: fileURLToPath(new URL("./tests/apercu", import.meta.url)),
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  server: { port: 1430, strictPort: false },
});
