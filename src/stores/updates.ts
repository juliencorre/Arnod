import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import type { UpdateChannel, UpdatesView } from "@/types/ipc";

/**
 * Mises à jour (lot L allégé, écran 1.6-g) : le cœur lit les versions publiées et décide si une
 * vérification est due ; le store garde sa vue. Rien n'est téléchargé ni installé : « Voir la
 * version » ouvre la page de la release dans le navigateur du système.
 */
export const useUpdatesStore = defineStore("updates", () => {
  const view = ref<UpdatesView | null>(null);
  const checking = ref(false);
  const error = ref<string | null>(null);

  async function load(): Promise<UpdatesView> {
    view.value = await ipc.updatesLoad();
    return view.value;
  }

  async function save(automatic: boolean, channel: UpdateChannel): Promise<void> {
    view.value = await ipc.updatesSave(automatic, channel);
  }

  /** « Vérifier maintenant » ; l'erreur reste affichée dans la section. */
  async function check(): Promise<void> {
    checking.value = true;
    error.value = null;
    try {
      view.value = await ipc.updatesCheck(false);
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    } finally {
      checking.value = false;
    }
  }

  /**
   * Vérification au démarrage, si elle est due : une version plus récente s'annonce par une
   * notification ; un échec reste silencieux (aucun réseau, proxy…).
   */
  async function checkAtStartup(): Promise<void> {
    try {
      const next = await ipc.updatesCheck(true);
      const known = view.value?.latest?.version;
      view.value = next;
      const latest = next.latest;
      if (latest && latest.version !== known) {
        useToastStore().show(`Arnod ${latest.version} est disponible.`, {
          action: {
            label: "Voir",
            run: () => useNetworkStore().openPreferences("updates"),
          },
        });
      }
    } catch {
      // Hors ligne ou proxy : la section « Mises à jour » le dira à la demande.
    }
  }

  async function open(url: string): Promise<void> {
    await ipc.updatesOpen(url);
  }

  return { view, checking, error, load, save, check, checkAtStartup, open };
});
