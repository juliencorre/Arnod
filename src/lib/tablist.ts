/**
 * Clavier d'une liste d'onglets selon le motif WAI-ARIA « tabs » (AC3) : un seul arrêt de
 * tabulation (l'onglet choisi, `tabindex="0"`, les autres `-1`), flèches gauche et droite
 * (haut et bas si `aria-orientation="vertical"`), Début et Fin. L'activation est automatique :
 * l'onglet atteint est cliqué, son gestionnaire `@click` reste la seule façon de choisir.
 *
 * À brancher sur l'élément `role="tablist"` : `@keydown="onTablistKeydown"`.
 */
export function onTablistKeydown(event: KeyboardEvent): void {
  const list = event.currentTarget;
  if (!(list instanceof HTMLElement)) return;
  const tabs = Array.from(list.children).filter(
    (child): child is HTMLElement =>
      child instanceof HTMLElement &&
      child.getAttribute("role") === "tab" &&
      !child.hasAttribute("disabled"),
  );
  // L'onglet qui a reçu la touche (celui qui a le focus) : la cible de l'événement.
  const current = event.target instanceof Element ? event.target.closest('[role="tab"]') : null;
  const index = tabs.findIndex((tab) => tab === current);
  if (index < 0) return;
  const vertical = list.getAttribute("aria-orientation") === "vertical";
  const last = tabs.length - 1;
  let next: number;
  switch (event.key) {
    case vertical ? "ArrowUp" : "ArrowLeft":
      next = index === 0 ? last : index - 1;
      break;
    case vertical ? "ArrowDown" : "ArrowRight":
      next = index === last ? 0 : index + 1;
      break;
    case "Home":
      next = 0;
      break;
    case "End":
      next = last;
      break;
    default:
      return;
  }
  event.preventDefault();
  const target = tabs[next];
  if (!target) return;
  target.click();
  target.focus();
}
