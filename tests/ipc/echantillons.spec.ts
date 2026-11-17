/**
 * Le faux cœur reproduit le vrai : pour le projet de `openProject`, `tree_load` et `request_read`
 * rendent exactement les échantillons écrits par le cœur (`echantillons.json`, généré par
 * `src-tauri/src/ts_mirror.rs` ; MA1 de docs/plan-qualite-mvp.md).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { ipc } from "@/lib/ipc";
import type { RequestDoc, TreeView } from "@/types/ipc";

import { installFakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
import samples from "./echantillons.json";

afterEach(() => {
  clearMocks();
});

function install(): void {
  const core = installFakeCore(boutiqueSnapshot());
  const wb = core.workbench;
  wb.folders.add("Clients");
  wb.requests.set("Clients/lister.yml", newRequest("Lister", { url: "{{baseUrl}}/clients" }));
  wb.requests.set("Clients/creer.yml", newRequest("Créer", { method: "POST" }));
  wb.requests.set("sante.yml", newRequest("Santé"));
  wb.unreadable.add("casse.yml");
}

describe("faux cœur et échantillons du cœur (MA1)", () => {
  it("tree_load rend l'arborescence du cœur", async () => {
    install();
    const view: TreeView = await ipc.treeLoad(BOUTIQUE_TESTS);
    expect(view).toEqual(samples.treeLoad as TreeView);
  });

  it("request_read rend la requête du cœur", async () => {
    install();
    const request: RequestDoc = await ipc.requestRead(BOUTIQUE_TESTS, "Clients/lister.yml");
    expect(request).toEqual(samples.requestRead as RequestDoc);
  });
});
