import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { IpcError, ipc } from "@/lib/ipc";
import type { StartupProblem, StartupTarget } from "@/types/ipc";

import { useToastStore } from "./toasts";

/**
 * Démarrage (lot G, écrans 0.3) : erreur bloquante décrite par le cœur et ses issues.
 *
 * Aucune règle ici : le cœur classe l'erreur, rédige la cause, ce qui est intact, les issues et
 * les détails, et décide des essais automatiques (`retry`). Le store compte seulement les
 * secondes jusqu'au prochain essai et appelle les commandes.
 */
export const useStartupStore = defineStore("startup", () => {
  const toasts = useToastStore();
  const problem = ref<StartupProblem | null>(null);
  /** Une issue est en cours (boutons désactivés). */
  const busy = ref(false);
  /** Secondes avant le prochain essai automatique ; `null` s'il n'y en a pas. */
  const secondsLeft = ref<number | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  const blocked = computed(() => problem.value !== null);

  function stopTimer(): void {
    if (timer !== null) clearInterval(timer);
    timer = null;
    secondsLeft.value = null;
  }

  /** Remplace l'erreur et planifie l'essai automatique qu'elle annonce (R4). */
  function apply(next: StartupProblem | null): void {
    stopTimer();
    problem.value = next;
    const plan = next?.retry;
    if (!plan) return;
    secondsLeft.value = plan.delaySeconds;
    timer = setInterval(() => {
      const left = (secondsLeft.value ?? 1) - 1;
      secondsLeft.value = left;
      if (left <= 0) {
        stopTimer();
        void retry(true);
      }
    }, 1000);
  }

  function message(error: unknown): string {
    return error instanceof IpcError ? error.message : String(error);
  }

  /** Lit l'erreur de démarrage ; renvoie `true` si Arnod peut démarrer. */
  async function check(): Promise<boolean> {
    try {
      apply(await ipc.startupStatus());
    } catch {
      // Cœur sans cette commande : le chargement dira ce qui ne va pas.
      apply(null);
    }
    return problem.value === null;
  }

  async function run(action: () => Promise<StartupProblem | null>): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    try {
      apply(await action());
    } catch (error) {
      toasts.show(message(error), { tone: "error" });
    } finally {
      busy.value = false;
    }
  }

  /** « Réessayer », ou essai automatique quand le décompte arrive à zéro. */
  function retry(automatic = false): Promise<void> {
    return run(() => ipc.startupRetry(automatic));
  }

  /** « Repartir d'un état vide » : le fichier est renommé, jamais supprimé (R3). */
  function startEmpty(): Promise<void> {
    return run(() => ipc.startupStartEmpty());
  }

  async function copy(text: string, done: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      toasts.show(done);
    } catch {
      toasts.show("Copie impossible : sélectionnez le texte et copiez-le à la main.", {
        tone: "error",
      });
    }
  }

  function copyPath(): Promise<void> {
    return copy(problem.value?.path ?? "", "Chemin copié.");
  }

  function copyDetails(): Promise<void> {
    return copy(problem.value?.supportText ?? "", "Détails copiés.");
  }

  /** Ouvre avec le programme du système ; à défaut, l'adresse de l'issue est copiée (R6). */
  async function open(target: StartupTarget): Promise<void> {
    try {
      await ipc.startupOpen(target);
    } catch (error) {
      if (target === "issue" && problem.value) {
        await copy(
          problem.value.issueUrl,
          "Le navigateur ne s'ouvre pas : l'adresse de l'issue préremplie est copiée, collez-la dans votre navigateur.",
        );
      } else {
        toasts.show(message(error), { tone: "error" });
      }
    }
  }

  async function quit(): Promise<void> {
    stopTimer();
    try {
      await ipc.startupQuit();
    } catch (error) {
      toasts.show(message(error), { tone: "error" });
    }
  }

  /**
   * Erreur imprévue au chargement (ni fichier d'état ni fenêtres) : montrée avec la même
   * structure, sans issue autre que « Quitter ».
   */
  function unexpected(text: string): void {
    apply({
      nature: "inaccessible",
      file: "",
      title: "Arnod ne peut pas démarrer",
      message: text,
      intactLead: "",
      intactText: "",
      options: [],
      pathLabel: "",
      path: "",
      retry: null,
      details: [{ label: "Erreur technique", value: text }],
      supportText: `Arnod ne démarre pas : ${text}\n`,
      issueUrl: "",
      footer: "",
    });
  }

  /** Montre la fenêtre, créée cachée (0.3-a). */
  async function windowReady(): Promise<void> {
    try {
      await ipc.windowReady();
    } catch {
      // Le cœur la montre de lui-même après 300 ms.
    }
  }

  return {
    problem,
    blocked,
    busy,
    secondsLeft,
    check,
    retry,
    startEmpty,
    copyPath,
    copyDetails,
    open,
    quit,
    unexpected,
    windowReady,
  };
});
