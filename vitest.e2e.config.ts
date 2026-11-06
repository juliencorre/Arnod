/// <reference types="vitest/config" />
import { defineConfig } from "vite";

// Tests e2e : pilotent l'application réelle (binaire debug) via tauri-driver.
// Voir e2e/README.md. Lancer avec `npm run test:e2e` (sous Xvfb en CI).
export default defineConfig({
  test: {
    include: ["e2e/**/*.e2e.ts"],
    environment: "node",
    fileParallelism: false,
    testTimeout: 60_000,
    hookTimeout: 120_000,
  },
});
