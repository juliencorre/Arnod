// Mesures de performance de l'interface (tests/perf) : l'application réelle sur le faux cœur
// gonflé, en build de production, pilotée par scripts/perf-interface.mjs. Hors build de
// l'application ; lancé chaque semaine par .github/workflows/qualite.yml.
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

export default defineConfig({
  root: fileURLToPath(new URL("./tests/perf", import.meta.url)),
  base: "./",
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  build: {
    outDir: process.env.PERF_DIST ?? fileURLToPath(new URL("./dist-perf", import.meta.url)),
    emptyOutDir: true,
    // Comme le build de l'application sous Windows (WebView2).
    target: "chrome105",
    sourcemap: true,
  },
  server: { port: 1440, strictPort: false },
});
