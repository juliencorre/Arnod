import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useWorkbenchStore } from "@/stores/workbench";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Essais de « Non classés » : chemin généralisé (2.2-c), transfert au référentiel (2.2-b). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

const FILE = "contracts/api-boutique-dev.yaml";
const PATH = "api.boutique.dev/essai-3.yml";

/** Ouvre l'essai 3 (`GET /orders/42/tracking`) d'un projet tenu pour « Non classés ». */
async function openTrial(): Promise<{ app: AppWrapper; core: FakeCore }> {
  const snapshot = boutiqueSnapshot();
  // Le projet ouvert tient lieu de « Non classés » : ses essais montrent « Requête au point ? ».
  snapshot.inboxProjectId = BOUTIQUE_TESTS;
  const opened = await openProject((core) => {
    core.contracts.files.set(FILE, {
      name: "api.boutique.dev",
      version: "0.0.0",
      deduced: true,
      source: "openapi: 3.1.0\n",
      operations: [
        {
          key: "GET /orders/42/tracking",
          method: "GET",
          path: "/orders/42/tracking",
          summary: null,
          tag: null,
        },
      ],
    });
    core.workbench.folders.add("api.boutique.dev");
    core.workbench.requests.set(
      PATH,
      newRequest("Essai 3", {
        url: "https://api.boutique.dev/orders/42/tracking",
        contract: { file: FILE, operation: "GET /orders/42/tracking" },
      }),
    );
  }, snapshot);
  await useWorkbenchStore().openRequest(BOUTIQUE_TESTS, PATH);
  await flushPromises();
  return opened;
}

describe("chemin généralisé d'un essai (2.2-c)", () => {
  it("propose /orders/{id}/tracking ; « Généraliser » change l'adresse et l'opération", async () => {
    let core: FakeCore;
    ({ app, core } = await openTrial());
    const banner = app.get(".generalize");
    const text = banner.text().replace(/\s+/g, " ");
    await click(app, ".generalize button", "Généraliser");
    const saved = core.workbench.requests.get(PATH);
    expect({
      proposed: text.includes("/orders/{id}/tracking") && text.includes("42"),
      url: saved?.url,
      operation: saved?.contract?.operation,
      param: saved?.params.find((p) => p.kind === "path"),
      banner: app.find(".generalize").exists(),
    }).toEqual({
      proposed: true,
      url: "https://api.boutique.dev/orders/{id}/tracking",
      operation: "GET /orders/{id}/tracking",
      param: { name: "id", value: "42", enabled: true, kind: "path", description: "" },
      banner: false,
    });
  });

  it("« Garder tel quel » retire la proposition sans rien changer", async () => {
    let core: FakeCore;
    ({ app, core } = await openTrial());
    await click(app, ".generalize button", "Garder tel quel");
    expect({
      banner: app.find(".generalize").exists(),
      url: core.workbench.requests.get(PATH)?.url,
      kept: core.commands().includes("trial_keep_path"),
    }).toEqual({
      banner: false,
      url: "https://api.boutique.dev/orders/42/tracking",
      kept: true,
    });
  });
});

describe("transfert au référentiel de tests (2.2-b)", () => {
  it("nomme le test, le range, et l'essai devient son étape 1 sans lien au contrat", async () => {
    let core: FakeCore;
    ({ app, core } = await openTrial());
    await click(app, ".trial-transfer button", "Transférer au référentiel de tests");
    const dialog = app.get(".transfer");
    await dialog.get("input").setValue("Suivre une commande");
    await dialog.trigger("submit");
    await flushPromises();
    const step = core.workbench.requests.get("Suivre une commande/essai-3.yml");
    expect({
      call: core.calls.find((c) => c.cmd === "trial_transfer")?.args,
      contract: step?.contract,
      seq: step?.seq,
      gone: core.workbench.requests.has(PATH),
    }).toEqual({
      call: {
        project: BOUTIQUE_TESTS,
        path: PATH,
        name: "Suivre une commande",
        parent: "",
      },
      contract: undefined,
      seq: 1,
      gone: false,
    });
  });

  it("un nom déjà pris reste affiché dans la fenêtre, rien n'est déplacé", async () => {
    let core: FakeCore;
    ({ app, core } = await openTrial());
    core.workbench.folders.add("Clients");
    await click(app, ".trial-transfer button", "Transférer au référentiel de tests");
    const dialog = app.get(".transfer");
    await dialog.get("input").setValue("Clients");
    await dialog.trigger("submit");
    await flushPromises();
    expect({
      error: app.find(".transfer__error").text(),
      kept: core.workbench.requests.has(PATH),
    }).toEqual({ error: "« Clients » existe déjà à cet endroit.", kept: true });
  });
});
