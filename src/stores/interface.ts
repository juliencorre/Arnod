import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { events, ipc } from "@/lib/ipc";
import { DEFAULT_SHORTCUTS, matchesKeys, type ShortcutId } from "@/lib/shortcuts";
import type { Appearance, InterfaceView, ShortcutConflict } from "@/types/ipc";

/**
 * Préférences d'interface du poste (version 1, `docs/plan-v1.md`) : apparence (1.6-f) et
 * raccourcis (1.6-f bis). Le cœur valide, enregistre et diffuse (`arnod://interface`) ; le store
 * garde sa vue, applique le thème et la police des éditeurs à la page, et dit si une touche
 * pressée est le raccourci d'une action. Tant que les préférences ne sont pas lues, les
 * raccourcis par défaut valent (`DEFAULT_SHORTCUTS`).
 */
export const useInterfaceStore = defineStore("interface", () => {
  const view = ref<InterfaceView | null>(null);
  let subscribed = false;
  /** Numéro du dernier enregistrement d'apparence demandé. */
  let saving = 0;

  const appearance = computed<Appearance>(
    () =>
      view.value?.appearance ?? {
        theme: "system",
        zoom: 100,
        editorFont: null,
        editorFontSize: 13,
      },
  );

  /** Combinaisons en vigueur de chaque action. */
  const keys = computed<Record<string, readonly string[]>>(() => {
    const current: Record<string, readonly string[]> = { ...DEFAULT_SHORTCUTS };
    for (const shortcut of view.value?.shortcuts ?? []) current[shortcut.id] = shortcut.keys;
    return current;
  });

  /** Combinaisons de l'action `id` (vide : aucun raccourci). */
  function keysOf(id: ShortcutId): readonly string[] {
    return keys.value[id] ?? [];
  }

  /** La touche pressée est le raccourci de l'action `id`. */
  function matches(event: KeyboardEvent, id: ShortcutId): boolean {
    return matchesKeys(event, keysOf(id));
  }

  /** Thème forcé et police des éditeurs, posés sur la racine du document (R1, R3). */
  function applyToPage(next: Appearance): void {
    const root = document.documentElement;
    if (next.theme === "system") delete root.dataset.theme;
    else root.dataset.theme = next.theme;
    if (next.editorFont === null) root.style.removeProperty("--editor-font");
    else root.style.setProperty("--editor-font", `"${next.editorFont}", var(--font-mono)`);
    if (next.editorFontSize === 13) root.style.removeProperty("--editor-font-size");
    else root.style.setProperty("--editor-font-size", `${String(next.editorFontSize / 16)}rem`);
  }

  function apply(next: InterfaceView): void {
    view.value = next;
    applyToPage(next.appearance);
  }

  /** Lit les préférences et suit leurs changements dans les autres fenêtres. */
  async function start(): Promise<void> {
    if (!subscribed) {
      subscribed = true;
      await events.onInterface(apply);
    }
    apply(await ipc.interfaceLoad());
  }

  /**
   * Enregistre un changement d'apparence. Il s'applique tout de suite : un second changement
   * fait avant la réponse du cœur part de celui-ci, pas de l'apparence d'avant.
   */
  async function saveAppearance(change: Partial<Appearance>): Promise<void> {
    const next = { ...appearance.value, ...change };
    const turn = ++saving;
    if (view.value) view.value = { ...view.value, appearance: next };
    applyToPage(next);
    const saved = await ipc.interfaceSaveAppearance(next);
    // Seule la réponse au dernier changement compte : une plus ancienne le défairait.
    if (turn === saving) apply(saved);
  }

  /**
   * Donne `combo` à l'action (`null` : retire son raccourci). Renvoie l'action qui tient déjà
   * la combinaison, sans rien changer, sauf `replace` (R6).
   */
  async function setShortcut(
    id: string,
    combo: string | null,
    replace = false,
  ): Promise<ShortcutConflict | null> {
    const change = await ipc.interfaceSetShortcut(id, combo, replace);
    apply(change.view);
    return change.conflict;
  }

  async function resetShortcuts(): Promise<void> {
    apply(await ipc.interfaceResetShortcuts());
  }

  return {
    view,
    appearance,
    keysOf,
    matches,
    start,
    saveAppearance,
    setShortcut,
    resetShortcuts,
  };
});
