/**
 * Clavier de l'arbre du référentiel (AC1), selon le motif WAI-ARIA « tree view » : un seul arrêt
 * de tabulation, la ligne courante (`tabindex="0"`, fourni par `TREE_TAB_STOP`), les flèches haut
 * et bas passent d'une ligne visible à l'autre, Début et Fin vont à la première et à la dernière.
 * Droite et gauche (déplier, replier, aller à l'enfant ou au parent), F2, Suppr et Maj+F10 sont
 * traités par la ligne (`TreeRow`), qui connaît son nœud.
 */
import type { ComputedRef, InjectionKey } from "vue";

/** Chemin de la ligne qui porte l'arrêt de tabulation de l'arbre. */
export const TREE_TAB_STOP: InjectionKey<ComputedRef<string | null>> = Symbol("treeTabStop");

/** Lignes affichées de l'arbre `tree`, dans l'ordre (un dossier replié ne rend pas ses enfants). */
export function treeItems(tree: Element): HTMLElement[] {
  return Array.from(tree.querySelectorAll<HTMLElement>('[role="treeitem"]'));
}

/** Ligne qui contient l'élément `element` (la ligne elle-même ou un de ses descendants). */
function itemOf(element: EventTarget | null): HTMLElement | null {
  return element instanceof Element ? element.closest<HTMLElement>('[role="treeitem"]') : null;
}

/** Flèches haut et bas, Début et Fin, à brancher sur l'élément `role="tree"`. */
export function onTreeKeydown(event: KeyboardEvent): void {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
  const tree = event.currentTarget;
  const current = itemOf(event.target);
  if (!(tree instanceof Element) || !current) return;
  const items = treeItems(tree);
  const index = items.indexOf(current);
  let next: HTMLElement | undefined;
  switch (event.key) {
    case "ArrowDown":
      next = items[index + 1];
      break;
    case "ArrowUp":
      next = items[index - 1];
      break;
    case "Home":
      next = items[0];
      break;
    case "End":
      next = items.at(-1);
      break;
    default:
      return;
  }
  event.preventDefault();
  next?.focus();
}

/** Ligne du dossier parent de la ligne `item`, ou `null` à la racine. */
export function parentItem(item: HTMLElement): HTMLElement | null {
  const group = item.closest("li")?.parentElement?.closest('[role="group"]');
  return group?.parentElement?.querySelector<HTMLElement>(':scope > * > [role="treeitem"]') ?? null;
}

/** Première ligne enfant de la ligne `item` (un dossier déplié), ou `null`. */
export function firstChildItem(item: HTMLElement): HTMLElement | null {
  return (
    item.closest("li")?.querySelector<HTMLElement>(':scope > [role="group"] [role="treeitem"]') ??
    null
  );
}
