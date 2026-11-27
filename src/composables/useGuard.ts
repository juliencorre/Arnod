/**
 * Garde d'action (IN8) : un bouton branché sur une action asynchrone du store passe par
 * `guard`, qui notifie tout refus du cœur au lieu de le laisser muet.
 *
 * `@click="guard(() => workbench.cancelRun(projectId))"`
 */
import { useToastStore } from "@/stores/toasts";

export function useGuard(): (action: () => unknown) => void {
  const toasts = useToastStore();
  return (action) => {
    try {
      const result = action();
      if (result instanceof Promise) {
        result.catch((error: unknown) => {
          toasts.showError(error);
        });
      }
    } catch (error) {
      toasts.showError(error);
    }
  };
}
