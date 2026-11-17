/** Choix du secret gardé d'un onglet à l'autre (plan qualité, lot C : IN4). */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { RequestDoc } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 750));
  await flushPromises();
}

function saved(core: FakeCore): RequestDoc[] {
  return core.calls
    .filter((c) => c.cmd === "request_save")
    .map((c) => c.args.request as RequestDoc);
}

describe("« Laisser dans le test » (4.2-c, IN4)", () => {
  it("le choix tient quand on revient sur l'onglet", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set("rembourser.yml", newRequest("Rembourser", { method: "POST" }));
    }));
    await click(app, '[role="treeitem"]', "Rembourser");
    await click(app, ".editor__tab", "Auth");
    await app.get(".auth select").setValue("bearer");
    await app.get('.auth input[type="text"]').setValue("eyJhbGciOi.xyz");
    await settle();
    await click(app, ".secret button", "Laisser dans le test");
    await settle();
    expect(saved(core).at(-1)?.auth.token).toBe("eyJhbGciOi.xyz");

    // Un autre onglet, puis retour sur « Rembourser ».
    await click(app, '[role="treeitem"]', "Santé");
    await settle();
    await click(app, '[role="treeitem"]', "Rembourser");
    await settle();
    const before = saved(core).length;
    await app.get(".editor__url").setValue("{{baseUrl}}/remboursements");
    await settle();

    expect({
      notice: app.find(".secret").exists(),
      label: app.get(".editor__saved").text(),
      savedAfterEdit: saved(core).length - before,
    }).toEqual({ notice: false, label: "✓ Enregistré", savedAfterEdit: 1 });
  });
});
