import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { suggestTags, tagCounts } from "@/lib/tags";
import type { TreeNode } from "@/types/ipc";

import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Étiquettes proposées avec leur nombre à la saisie (écran 4.9-a). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

function request(name: string, tags: string[]): TreeNode {
  return {
    kind: "request",
    name,
    path: `${name}.yml`,
    method: "GET",
    assertionCount: 0,
    invalid: false,
    contract: null,
    url: "",
    tags,
  };
}

const NODES: TreeNode[] = [
  request("a", ["non-régression", "smoke"]),
  {
    kind: "folder",
    name: "commandes",
    path: "commandes",
    requestCount: 3,
    test: false,
    children: [
      request("b", ["Non-régression", "nominal"]),
      request("c", ["non-régression", "paiement"]),
      request("d", ["nominal"]),
    ],
  },
];

async function openTagged(): Promise<AppWrapper> {
  const opened = await openProject((core) => {
    const wb = core.workbench;
    wb.requests.set("a.yml", newRequest("Créer une commande", { tags: ["smoke"] }));
    wb.requests.set("b.yml", newRequest("Lister", { tags: ["non-régression", "nominal"] }));
    wb.requests.set("c.yml", newRequest("Lire", { tags: ["non-régression"] }));
    wb.requests.set("d.yml", newRequest("Payer", { tags: ["non-régression", "paiement"] }));
  });
  await click(opened.app, '[role="treeitem"]', "Créer une commande");
  await click(opened.app, ".tag__add");
  return opened.app;
}

function shown(wrapper: AppWrapper): string[] {
  return wrapper.findAll('[role="option"]').map((o) => o.text().replace(/\s+/g, " ").trim());
}

describe("étiquettes du projet (pur)", () => {
  it("compte les requêtes de chaque étiquette, sans casse ni accents, la plus utilisée d'abord", () => {
    expect(tagCounts(NODES)).toEqual([
      { tag: "non-régression", count: 3 },
      { tag: "nominal", count: 2 },
      { tag: "paiement", count: 1 },
      { tag: "smoke", count: 1 },
    ]);
  });

  it("propose le début d'étiquette, puis le contenu, sans celles déjà portées ; « Créer » si nouvelle", () => {
    const counts = tagCounts(NODES);
    expect([
      suggestTags(counts, "no", ["smoke"]),
      suggestTags(counts, "ment", []),
      suggestTags(counts, "NOMINAL", []),
      suggestTags(counts, "", ["nominal"]).items.map((s) => s.tag),
    ]).toEqual([
      {
        items: [
          { tag: "non-régression", count: 3 },
          { tag: "nominal", count: 2 },
        ],
        create: "no",
      },
      { items: [{ tag: "paiement", count: 1 }], create: "ment" },
      { items: [{ tag: "nominal", count: 2 }], create: null },
      ["non-régression", "paiement", "smoke"],
    ]);
  });
});

describe("saisie d'une étiquette (4.9-a)", () => {
  it("propose les étiquettes du projet avec leur nombre, puis « + Créer »", async () => {
    app = await openTagged();
    await app.get(".tag__input").setValue("no");
    await flushPromises();
    const first = app.findAll('[role="option"]')[0]?.attributes("id");
    expect({
      options: shown(app),
      firstActive: app.get(".tag__input").attributes("aria-activedescendant") === first,
    }).toEqual({
      options: ["non-régression3", "nominal1", "+ Créer « no »"],
      firstActive: true,
    });
  });

  it("Entrée ajoute la proposition choisie ; ↓ passe à la suivante", async () => {
    app = await openTagged();
    const input = app.get(".tag__input");
    await input.setValue("no");
    await input.trigger("keydown", { key: "ArrowDown" });
    await input.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(app.findAll(".tag").map((t) => t.text().replace("×", "").trim())).toEqual([
      "smoke",
      "nominal",
    ]);
  });

  it("un clic sur « + Créer » ajoute le texte tapé tel quel", async () => {
    app = await openTagged();
    await app.get(".tag__input").setValue("Recette");
    await flushPromises();
    await click(app, '[role="option"]', "Créer");
    expect(app.findAll(".tag").map((t) => t.text().replace("×", "").trim())).toEqual([
      "smoke",
      "Recette",
    ]);
  });
});
