import { defineStore } from "pinia";
import { ref } from "vue";

/** Action proposée dans une notification (« Annuler », « Ajouter à un autre… »). */
export interface ToastAction {
  label: string;
  run: () => unknown;
}

/** Notification affichée en bas de la fenêtre. */
export interface Toast {
  id: number;
  message: string;
  tone: "info" | "error";
  action: ToastAction | null;
}

/** Durée d'affichage, prolongée tant que la notification est survolée ou a le focus. */
export const TOAST_DURATION_MS = 6000;

/**
 * Notifications (R6) : une seule à la fois, la plus récente remplace la précédente.
 * Elles remplacent les fenêtres de confirmation quand rien n'est perdu.
 */
export const useToastStore = defineStore("toasts", () => {
  const current = ref<Toast | null>(null);
  let nextId = 1;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function clearTimer(): void {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
  }

  function schedule(): void {
    clearTimer();
    timer = setTimeout(dismiss, TOAST_DURATION_MS);
  }

  /** Affiche une notification, avec une action facultative. */
  function show(
    message: string,
    options: { action?: ToastAction; tone?: Toast["tone"] } = {},
  ): void {
    current.value = {
      id: nextId++,
      message,
      tone: options.tone ?? "info",
      action: options.action ?? null,
    };
    schedule();
  }

  /** Affiche une erreur (message prêt à afficher, par exemple celui d'une `IpcError`). */
  function showError(error: unknown): void {
    show(error instanceof Error ? error.message : String(error), { tone: "error" });
  }

  function dismiss(): void {
    clearTimer();
    current.value = null;
  }

  /** Suspend la disparition (survol, focus). */
  function pause(): void {
    clearTimer();
  }

  /** Reprend la disparition après une pause. */
  function resume(): void {
    if (current.value !== null) schedule();
  }

  /** Exécute l'action de la notification courante, puis la ferme. Un échec est notifié. */
  async function runAction(): Promise<void> {
    const action = current.value?.action;
    if (!action) return;
    dismiss();
    try {
      await action.run();
    } catch (error) {
      showError(error);
    }
  }

  return { current, show, showError, dismiss, pause, resume, runAction };
});
