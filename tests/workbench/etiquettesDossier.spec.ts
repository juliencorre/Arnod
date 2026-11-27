/**
 * Version 2, lot B (`docs/plan-v2.md`) : étiquettes partagées (4.9-b). Une étiquette posée sur
 * un dossier vaut pour tout ce qu'il contient (R4) ; la vue d'un dossier montre les étiquettes de
 * chaque ligne et se filtre par étiquettes, les autres lignes estompées.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { hasTags, taggedTests } from "@/lib/tags";
import type { TreeNode } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function openFolderView(): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject((core) => {
    core.workbench.requests.set(
      "Clients/lister.yml",
      newRequest("Lister", { url: "{{baseUrl}}/clients", tags: ["smoke"] }),
    );
    // Un sous-dossier : « Clients » est un dossier ordinaire, pas un test.
    core.workbench.folders.add("Clients/archives");
  });
  await click(opened.app, '[role="treeitem"][data-path="Clients"]');
  return opened;
}

function rows(wrapper: AppWrapper): { name: string; tags: string[]; faded: boolean }[] {
  return wrapper.findAll(".folder__row").map((row) => ({
    name: row.get(".folder__name").text(),
    tags: row.findAll(".folder__row-tag").map((t) => t.text()),
    faded: row.classes("folder__row--faded"),
  }));
}

describe("étiquettes effectives (pur)", () => {
  const request = (tags: string[]): TreeNode => ({
    kind: "request",
    name: "r",
    path: "r.yml",
    method: "GET",
    assertionCount: 0,
    invalid: false,
    contract: null,
    url: "",
    tags,
  });
  const test = (children: TreeNode[]): TreeNode => ({
    kind: "folder",
    name: "t",
    path: "t",
    children,
    requestCount: children.length,
    test: true,
    tags: [],
    template: null,
  });

  it("un test compte s'il a une étape étiquetée ; une requête libre pour elle-même", () => {
    const folder: TreeNode = {
      kind: "folder",
      name: "d",
      path: "d",
      requestCount: 4,
      test: false,
      tags: [],
      template: null,
      children: [
        test([request(["smoke"]), request([])]),
        test([request([])]),
        request(["Smoke", "lent"]),
        request(["lent"]),
      ],
    };
    expect(taggedTests(folder, ["smoke"])).toEqual({ matching: 2, total: 4 });
    expect(taggedTests(folder, ["smoke", "lent"])).toEqual({ matching: 1, total: 4 });
    expect(hasTags(["Régression"], ["regression"])).toBe(true);
  });
});

describe("étiquettes d'un dossier (4.9-b)", () => {
  it("une étiquette posée sur le dossier vaut pour chacune de ses requêtes", async () => {
    let core: FakeCore;
    ({ app, core } = await openFolderView());
    expect(rows(app)).toEqual([
      { name: "archives", tags: [], faded: false },
      { name: "Créer", tags: [], faded: false },
      { name: "Lister", tags: ["smoke"], faded: false },
    ]);
    await click(app, ".folder__tags .tag__add");
    const input = app.get(".folder__tags .tag__input");
    await input.setValue("régression");
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "tree_set_folder_tags").at(-1)?.args).toEqual({
      project: "p-boutique-tests",
      path: "Clients",
      tags: ["régression"],
    });
    expect(core.workbench.folderTags.get("Clients")).toEqual(["régression"]);
    expect(rows(app).map((r) => r.tags)).toEqual([[], ["régression"], ["smoke", "régression"]]);
  });

  it("le filtre estompe les lignes sans l'étiquette et compte les tests", async () => {
    ({ app } = await openFolderView());
    const add = app.get<HTMLSelectElement>(".folder__filter-add");
    expect(add.findAll("option").map((o) => o.text())).toEqual(["+ Étiquette", "smoke"]);
    await add.setValue("smoke");
    await flushPromises();
    expect(app.get(".folder__filter-summary").text()).toBe(
      "1 test sur 2 · les autres sont estompés",
    );
    expect(rows(app).map((r) => [r.name, r.faded])).toEqual([
      ["archives", true],
      ["Créer", true],
      ["Lister", false],
    ]);
    // Le filtre se retire au clavier, par son bouton ×.
    await app.get('[aria-label="Retirer smoke du filtre"]').trigger("click");
    await flushPromises();
    expect(app.find(".folder__filter-summary").exists()).toBe(false);
    expect(rows(app).some((r) => r.faded)).toBe(false);
  });
});
