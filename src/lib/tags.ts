/**
 * Étiquettes proposées à la saisie avec leur nombre (écran 4.9-a) : comptage d'affichage des
 * étiquettes des requêtes du projet (fournies par le cœur dans l'arborescence) et propositions qui
 * répondent au texte tapé. Sans casse ni accents ; la casse la plus utilisée est montrée.
 */
import type { TreeNode } from "@/types/ipc";

/** Étiquette du projet et nombre de requêtes qui la portent. */
export interface TagCount {
  tag: string;
  count: number;
}

/** En minuscules, sans accents, pour comparer. */
export function foldTag(tag: string): string {
  return tag.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

/** Étiquettes des requêtes de `nodes`, de la plus utilisée à la moins utilisée, puis par nom. */
export function tagCounts(nodes: readonly TreeNode[]): TagCount[] {
  const byKey = new Map<string, { count: number; spellings: Map<string, number> }>();
  const visit = (list: readonly TreeNode[]): void => {
    for (const node of list) {
      if (node.kind === "folder") {
        visit(node.children);
        continue;
      }
      for (const tag of new Set(node.tags)) {
        const key = foldTag(tag);
        const entry = byKey.get(key) ?? { count: 0, spellings: new Map<string, number>() };
        entry.count += 1;
        entry.spellings.set(tag, (entry.spellings.get(tag) ?? 0) + 1);
        byKey.set(key, entry);
      }
    }
  };
  visit(nodes);
  return [...byKey.values()]
    .map(({ count, spellings }) => {
      // L'orthographe la plus utilisée est montrée.
      let tag = "";
      let uses = 0;
      for (const [spelling, n] of spellings) {
        if (n > uses) {
          tag = spelling;
          uses = n;
        }
      }
      return { tag, count };
    })
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag, "fr"));
}

/** Propositions pour le texte tapé `query` : début d'étiquette d'abord, puis contenu. */
export interface TagSuggestions {
  items: TagCount[];
  /** Texte à créer (« + Créer « no » ») ; `null` si vide ou déjà une étiquette du projet. */
  create: string | null;
}

/** Nombre de propositions montrées. */
export const MAX_TAG_SUGGESTIONS = 8;

/**
 * Étiquettes du projet qui répondent à `query`, sans celles que la requête porte déjà (`taken`).
 * Texte vide : les plus utilisées.
 */
export function suggestTags(
  counts: readonly TagCount[],
  query: string,
  taken: readonly string[],
): TagSuggestions {
  const q = foldTag(query.trim());
  const takenKeys = new Set(taken.map(foldTag));
  const free = counts.filter((c) => !takenKeys.has(foldTag(c.tag)));
  const starts = free.filter((c) => foldTag(c.tag).startsWith(q));
  const contains =
    q === "" ? [] : free.filter((c) => !foldTag(c.tag).startsWith(q) && foldTag(c.tag).includes(q));
  const exists = counts.some((c) => foldTag(c.tag) === q) || takenKeys.has(q);
  return {
    items: [...starts, ...contains].slice(0, MAX_TAG_SUGGESTIONS),
    create: q === "" || exists ? null : query.trim(),
  };
}
