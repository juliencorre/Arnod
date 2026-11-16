/**
 * Filtre d'affichage du référentiel de tests (docs/plan-mvp-d-barre-laterale.md, F1 à F4, R3, R4 ;
 * décision Q4 du plan d'ensemble : filtre fait dans l'interface). Le texte saisi est découpé en
 * termes, qui se cumulent : `#mot` cherche une étiquette qui commence par `mot`, une méthode en
 * capitales (`GET`) la méthode, tout autre terme le nom ou l'adresse, sans tenir compte de la casse
 * ni des accents.
 */
import type { TreeNode } from "@/types/ipc";

const METHODS = new Set(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]);

/** Filtre lu depuis le texte saisi. */
export interface TreeFilter {
  /** Termes cherchés dans le nom ou l'adresse, repliés (minuscules, sans accents). */
  words: string[];
  /** Débuts d'étiquette, repliés, sans `#`. */
  tags: string[];
  /** Méthodes exigées (`GET`). */
  methods: string[];
}

/** Arborescence filtrée et compte (« 2 sur 66 », R4). */
export interface FilteredTree {
  nodes: TreeNode[];
  matched: number;
  total: number;
}

/** Morceau d'un nom, surligné s'il correspond au filtre. */
export interface NamePart {
  text: string;
  hit: boolean;
}

type RequestNode = Extract<TreeNode, { kind: "request" }>;

/** Un caractère en minuscules, sans accent. */
function foldChar(char: string): string {
  return char.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

/** Texte replié (minuscules, sans accents). */
export function fold(text: string): string {
  return Array.from(text, foldChar).join("");
}

/** Lit le texte saisi ; `null` s'il ne contient aucun terme. */
export function parseFilter(text: string): TreeFilter | null {
  const filter: TreeFilter = { words: [], tags: [], methods: [] };
  for (const term of text.split(/\s+/)) {
    if (term === "" || term === "#") continue;
    if (term.startsWith("#")) filter.tags.push(fold(term.slice(1)));
    else if (METHODS.has(term)) filter.methods.push(term);
    else filter.words.push(fold(term));
  }
  const empty = filter.words.length + filter.tags.length + filter.methods.length === 0;
  return empty ? null : filter;
}

/** La requête correspond-elle à chaque terme du filtre ? */
export function matches(node: RequestNode, filter: TreeFilter): boolean {
  if (filter.methods.some((method) => method !== node.method)) return false;
  const tags = node.tags.map(fold);
  if (!filter.tags.every((tag) => tags.some((t) => t.startsWith(tag)))) return false;
  const name = fold(node.name);
  const url = fold(node.url);
  return filter.words.every((word) => name.includes(word) || url.includes(word));
}

/** Un terme n'est trouvé que dans l'adresse : la ligne le signale (« adresse »). */
export function matchedByUrl(node: RequestNode, filter: TreeFilter): boolean {
  const name = fold(node.name);
  return filter.words.some((word) => !name.includes(word));
}

/** Arborescence sans ce qui ne correspond pas : requêtes retenues et dossiers qui en ont (R3). */
export function filterTree(nodes: TreeNode[], filter: TreeFilter): FilteredTree {
  let total = 0;
  let matched = 0;
  const walk = (list: TreeNode[]): TreeNode[] => {
    const kept: TreeNode[] = [];
    for (const node of list) {
      if (node.kind === "request") {
        total += 1;
        if (matches(node, filter)) {
          matched += 1;
          kept.push(node);
        }
        continue;
      }
      const children = walk(node.children);
      if (children.length > 0) kept.push({ ...node, children });
    }
    return kept;
  };
  const kept = walk(nodes);
  return { nodes: kept, matched, total };
}

/**
 * Morceaux du nom, le premier passage trouvé de chaque terme surligné. Les positions sont celles
 * du nom affiché : le repli se fait caractère par caractère.
 */
export function highlight(name: string, filter: TreeFilter | null): NamePart[] {
  if (filter === null || filter.words.length === 0) return [{ text: name, hit: false }];
  const chars = Array.from(name);
  const folded = chars.map(foldChar);
  // Début de chaque caractère dans le texte replié.
  const starts: number[] = [];
  let at = 0;
  for (const part of folded) {
    starts.push(at);
    at += part.length;
  }
  const text = folded.join("");
  const hits = new Array<boolean>(chars.length).fill(false);
  for (const word of filter.words) {
    const from = text.indexOf(word);
    if (from < 0) continue;
    const to = from + word.length;
    starts.forEach((start, index) => {
      if (start >= from && start < to) hits[index] = true;
    });
  }
  const parts: NamePart[] = [];
  chars.forEach((char, index) => {
    const hit = hits[index] ?? false;
    const last = parts.at(-1);
    if (last?.hit === hit) last.text += char;
    else parts.push({ text: char, hit });
  });
  return parts;
}
