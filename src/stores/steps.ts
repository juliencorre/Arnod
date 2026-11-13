import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { ipc } from "@/lib/ipc";
import type { Detection, StepCatalog, StepNature, TrialResult } from "@/types/ipc";

/**
 * Catalogue des natures d'étape et essais sans projet
 * (docs/plan-accueil-non-classes.md). Le cœur lit les définitions, reconnaît ce
 * que l'on colle, vérifie les valeurs et enregistre l'essai dans « Non classés » ;
 * ce store ne fait que relayer.
 */
export const useStepsStore = defineStore("steps", () => {
  const catalog = ref<StepCatalog | null>(null);
  const natures = computed<StepNature[]>(() => catalog.value?.natures ?? []);
  /** Envoi d'essai en cours, pour l'annuler. */
  const sendingId = ref<string | null>(null);

  /** Charge le catalogue une seule fois. */
  async function load(): Promise<void> {
    catalog.value ??= await ipc.stepsCatalog();
  }

  function nature(id: string | null): StepNature | null {
    return natures.value.find((n) => n.id === id) ?? null;
  }

  function detect(input: string): Promise<Detection | null> {
    return ipc.stepsDetect(input);
  }

  /**
   * Envoie l'essai ; enregistré dans « Non classés » seulement s'il reçoit une réponse (R12).
   * `secrets` : ceux d'une commande cURL collée, gardés au coffre par le cœur.
   */
  async function send(
    natureId: string,
    subject: string,
    values: Record<string, string>,
    secrets: Record<string, string> = {},
  ): Promise<TrialResult> {
    const execId = crypto.randomUUID();
    sendingId.value = execId;
    try {
      return await ipc.trialSend(execId, natureId, subject, values, secrets);
    } finally {
      sendingId.value = null;
    }
  }

  async function cancel(): Promise<void> {
    if (sendingId.value !== null) await ipc.requestCancel(sendingId.value);
  }

  return { catalog, natures, sendingId, load, nature, detect, send, cancel };
});
