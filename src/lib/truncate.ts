/**
 * Infobulle d'un nom coupé (DE5) : au survol, l'élément reçoit son texte entier en `title` s'il
 * dépasse sa largeur (points de suspension), et le perd sinon. Le nom accessible porte déjà le
 * texte entier ; l'infobulle sert à la souris.
 *
 * `<span v-title-if-cut class="row__name">…</span>`
 */
import type { Directive } from "vue";

const handlers = new WeakMap<HTMLElement, () => void>();

function update(element: HTMLElement): void {
  if (element.scrollWidth > element.clientWidth) {
    element.title = element.textContent.trim();
  } else {
    element.removeAttribute("title");
  }
}

export const vTitleIfCut: Directive<HTMLElement> = {
  mounted(element) {
    const handler = (): void => {
      update(element);
    };
    handlers.set(element, handler);
    element.addEventListener("mouseenter", handler);
  },
  beforeUnmount(element) {
    const handler = handlers.get(element);
    if (handler) element.removeEventListener("mouseenter", handler);
    handlers.delete(element);
  },
};
