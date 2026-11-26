/**
 * Focus rendu (AC2) : une action qui remplace la vue ou ouvre un onglet (création d'un projet,
 * essai, ▶) retire souvent l'élément qui avait le focus, qui retombe alors sur `body`. Après
 * chaque changement de vue ou d'onglet actif, le focus resté sur `body` va à l'onglet actif de la
 * zone active, sinon à la zone principale `main` (qui porte `tabindex="-1"`).
 */
import { nextTick, watch, type Ref } from "vue";

import { useNavigationStore } from "@/stores/navigation";
import { useWindowsStore } from "@/stores/windows";

/**
 * Le focus est perdu : sur `body`, nulle part, ou sur `main` lui-même, où une précédente reprise
 * l'a posé en attendant l'onglet (un essai change la vue, puis ouvre son onglet).
 */
function lost(main: HTMLElement): boolean {
  const active = document.activeElement;
  return active === null || active === document.body || active === main;
}

export function useFocusRescue(main: Readonly<Ref<HTMLElement | null>>): void {
  const navigation = useNavigationStore();
  const windows = useWindowsStore();
  watch(
    () => [navigation.view, windows.activeZone?.id, windows.activeZone?.activeTab] as const,
    async () => {
      await nextTick();
      const root = main.value;
      if (!root || !lost(root)) return;
      const tab =
        root.querySelector<HTMLElement>('.zone--active [role="tab"][aria-selected="true"]') ??
        root.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
      if (tab) tab.focus();
      else if (document.activeElement !== root) root.focus();
    },
    { flush: "post" },
  );
}
