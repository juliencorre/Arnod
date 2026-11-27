import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { RequestDoc } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

/** Lot K (4.2-c) : secret saisi en clair dans l'authentification. */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** Laisse passer la relecture de la saisie (250 ms) et l'enregistrement automatique (600 ms). */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 750));
  await flushPromises();
}

async function bearer(token: string): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject((core) => {
    core.workbench.requests.set("rembourser.yml", newRequest("Rembourser", { method: "POST" }));
  });
  await click(opened.app, '[role="treeitem"]', "Rembourser");
  await click(opened.app, ".editor__tab", "Auth");
  await opened.app.get(".auth select").setValue("bearer");
  await opened.app.get('.auth input[type="text"]').setValue(token);
  await settle();
  return opened;
}

function saved(core: FakeCore): RequestDoc[] {
  return core.calls
    .filter((c) => c.cmd === "request_save")
    .map((c) => c.args.request as RequestDoc);
}

describe("secret saisi en clair (4.2-c)", () => {
  it("le signale et n'enregistre rien tant que l'utilisateur n'a pas choisi", async () => {
    let core: FakeCore;
    ({ app, core } = await bearer("eyJhbGciOi.xyz"));
    expect({
      title: app.get(".secret__title").text(),
      note: app.get(".secret__note").text(),
      saved: saved(core).filter((r) => r.auth.token === "eyJhbGciOi.xyz").length,
      label: app.get(".editor__saved").text(),
    }).toEqual({
      title: "Ce jeton serait partagé avec l'équipe",
      note: "Le champ deviendra {{ token }}, valeur rangée dans le coffre du système, jamais envoyée à l'équipe.",
      saved: 0,
      label: "Non enregistré : choisissez où va le jeton",
    });
  });

  it("« Mettre dans une variable locale » range la valeur au coffre et écrit {{token}}", async () => {
    let core: FakeCore;
    ({ app, core } = await bearer("eyJhbGciOi.xyz"));
    await click(app, ".secret button", "Mettre dans une variable locale");
    await settle();
    const local = core.calls.find((c) => c.cmd === "local_set")?.args;
    expect({
      local,
      declared: core.workbench.envs.find((e) => e.name === "Local")?.locals.includes("token"),
      field: app.get<HTMLInputElement>('.auth input[type="text"]').element.value,
      written: saved(core).at(-1)?.auth.token,
      notice: app.find(".secret").exists(),
    }).toEqual({
      local: expect.objectContaining({
        environment: "Local",
        name: "token",
        value: "eyJhbGciOi.xyz",
        secret: true,
      }) as unknown,
      declared: true,
      field: "{{token}}",
      written: "{{token}}",
      notice: false,
    });
  });

  it("« Laisser dans la requête » écrit la valeur telle quelle ; une référence n'est pas signalée", async () => {
    let core: FakeCore;
    ({ app, core } = await bearer("eyJhbGciOi.xyz"));
    await click(app, ".secret button", "Laisser dans la requête");
    await settle();
    const kept = saved(core).at(-1)?.auth.token;
    await app.get('.auth input[type="text"]').setValue("{{jeton}}");
    await settle();
    expect({ kept, notice: app.find(".secret").exists() }).toEqual({
      kept: "eyJhbGciOi.xyz",
      notice: false,
    });
  });
});
