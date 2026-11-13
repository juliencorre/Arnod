/** Filtre du référentiel de tests (docs/plan-mvp-d-barre-laterale.md, F2, R3, R4). */
import { describe, expect, it } from "vitest";

import { filterTree, highlight, matchedByUrl, matches, parseFilter } from "@/lib/treeFilter";
import type { HttpMethod, TreeNode } from "@/types/ipc";

type RequestNode = Extract<TreeNode, { kind: "request" }>;

function request(
  name: string,
  path: string,
  extra: { method?: HttpMethod; url?: string; tags?: string[] } = {},
): RequestNode {
  return {
    kind: "request",
    name,
    path,
    method: extra.method ?? "GET",
    assertionCount: 0,
    invalid: false,
    contract: null,
    url: extra.url ?? "",
    tags: extra.tags ?? [],
  };
}

function folder(path: string, children: TreeNode[]): TreeNode {
  return {
    kind: "folder",
    name: path.split("/").at(-1) ?? path,
    path,
    children,
    requestCount: children.length,
    test: false,
  };
}

const LISTER = request("Lister les commandes", "commandes/v2/lister.yml", {
  url: "{{baseUrl}}/orders?statut=ouvert",
  tags: ["smoke"],
});
const CHANGER = request("Changer le statut", "commandes/v2/changer.yml", {
  method: "PATCH",
  url: "{{baseUrl}}/orders/{{id}}/status",
  tags: ["critique", "commandes"],
});
const SANTE = request("Santé", "sante.yml", { url: "{{baseUrl}}/health" });
const TREE: TreeNode[] = [
  folder("commandes", [folder("commandes/v2", [LISTER, CHANGER]), folder("commandes/vide", [])]),
  SANTE,
];

describe("lecture du filtre", () => {
  it("vide ou blanc : pas de filtre", () => {
    expect(parseFilter("")).toBeNull();
    expect(parseFilter("   ")).toBeNull();
    expect(parseFilter("#")).toBeNull();
  });

  it("mots repliés, étiquettes sans #, méthodes en capitales", () => {
    expect(parseFilter("Santé #Critique PATCH get")).toEqual({
      words: ["sante", "get"],
      tags: ["critique"],
      methods: ["PATCH"],
    });
  });
});

describe("correspondance (F2)", () => {
  it("un mot cherche dans le nom et l'adresse, sans casse ni accents", () => {
    const filter = parseFilter("SANTE");
    expect(filter && matches(SANTE, filter)).toBe(true);
    const status = parseFilter("stat");
    expect(status && matches(LISTER, status)).toBe(true);
    expect(status && matchedByUrl(LISTER, status)).toBe(true);
    expect(status && matchedByUrl(CHANGER, status)).toBe(false);
  });

  it("#étiquette : une étiquette qui commence par le mot", () => {
    const filter = parseFilter("#crit");
    expect(filter && matches(CHANGER, filter)).toBe(true);
    expect(filter && matches(LISTER, filter)).toBe(false);
  });

  it("une méthode en capitales filtre la méthode", () => {
    const filter = parseFilter("PATCH");
    expect(filter && matches(CHANGER, filter)).toBe(true);
    expect(filter && matches(LISTER, filter)).toBe(false);
  });

  it("les termes se cumulent", () => {
    const filter = parseFilter("orders GET");
    expect(filter && matches(LISTER, filter)).toBe(true);
    expect(filter && matches(CHANGER, filter)).toBe(false);
  });
});

describe("arborescence filtrée (R3, R4)", () => {
  it("garde les requêtes retenues et les dossiers qui en ont ; compte N sur M", () => {
    const filter = parseFilter("stat");
    if (!filter) throw new Error("filtre attendu");
    const result = filterTree(TREE, filter);
    expect(result.matched).toBe(2);
    expect(result.total).toBe(3);
    expect(result.nodes).toHaveLength(1);
    const commandes = result.nodes[0];
    expect(commandes?.kind === "folder" && commandes.children.map((c) => c.path)).toEqual([
      "commandes/v2",
    ]);
  });

  it("aucun résultat : une arborescence vide", () => {
    const filter = parseFilter("introuvable");
    if (!filter) throw new Error("filtre attendu");
    expect(filterTree(TREE, filter)).toEqual({ nodes: [], matched: 0, total: 3 });
  });
});

describe("surlignage", () => {
  it("surligne le passage trouvé, accents compris", () => {
    expect(highlight("Changer le statut", parseFilter("stat"))).toEqual([
      { text: "Changer le ", hit: false },
      { text: "stat", hit: true },
      { text: "ut", hit: false },
    ]);
    expect(highlight("Santé", parseFilter("TE"))).toEqual([
      { text: "San", hit: false },
      { text: "té", hit: true },
    ]);
    expect(highlight("Santé", null)).toEqual([{ text: "Santé", hit: false }]);
  });
});
