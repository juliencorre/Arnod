/**
 * Enregistrement automatique d'un formulaire hors requête (ER2, Q5) : environnements et réglages
 * du projet s'enregistrent comme une requête, `AUTOSAVE_MS` après la dernière modification.
 * `flush()` enregistre tout de suite ce qui attend ; une fenêtre l'appelle avant de se fermer ou
 * de changer d'élément, et ne se ferme pas sur un échec : rien n'est jeté.
 *
 * `source` rend le brouillon (comparé sérialisé) ; `save` l'enregistre. `reset()` déclare le
 * brouillon courant comme référence (chargé, ou remplacé par le cœur) : rien à enregistrer.
 */
import { onBeforeUnmount, readonly, ref, watch, type Ref } from "vue";

import { AUTOSAVE_MS } from "@/stores/workbench";

export type AutosaveState = "saved" | "pending" | "saving" | "error";

export interface Autosave {
  state: Readonly<Ref<AutosaveState>>;
  error: Readonly<Ref<string | null>>;
  reset: () => void;
  flush: () => Promise<boolean>;
}

export function useAutosave(source: () => unknown, save: () => Promise<void>): Autosave {
  const state = ref<AutosaveState>("saved");
  const error = ref<string | null>(null);
  /** Brouillon de référence, sérialisé ; `null` tant que rien n'est chargé. */
  let baseline: string | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let running: Promise<boolean> | null = null;

  const snapshot = (): string => JSON.stringify(source() ?? null);

  function clear(): void {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  }

  function schedule(): void {
    clear();
    timer = setTimeout(() => void flush(), AUTOSAVE_MS);
  }

  function reset(): void {
    clear();
    baseline = source() === null || source() === undefined ? null : snapshot();
    state.value = "saved";
    error.value = null;
  }

  async function flush(): Promise<boolean> {
    clear();
    if (running) await running;
    const text = snapshot();
    if (baseline === null || text === baseline) {
      if (state.value !== "error") state.value = "saved";
      return state.value !== "error";
    }
    state.value = "saving";
    running = (async () => {
      try {
        await save();
        baseline = text;
        error.value = null;
        if (snapshot() === text) state.value = "saved";
        else {
          state.value = "pending";
          schedule();
        }
        return true;
      } catch (caught) {
        error.value = caught instanceof Error ? caught.message : String(caught);
        state.value = "error";
        return false;
      } finally {
        running = null;
      }
    })();
    return running;
  }

  watch(
    source,
    () => {
      if (baseline === null) return;
      if (snapshot() === baseline) {
        clear();
        if (state.value === "pending") state.value = "saved";
        return;
      }
      state.value = "pending";
      schedule();
    },
    { deep: true },
  );

  // Une fenêtre retirée sans passer par sa fermeture enregistre encore ce qui attend.
  onBeforeUnmount(() => {
    if (timer !== null) void flush();
  });

  return { state: readonly(state), error: readonly(error), reset, flush };
}
