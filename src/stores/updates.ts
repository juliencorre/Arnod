import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { events, ipc } from "@/lib/ipc";
import { useContractsStore } from "@/stores/contracts";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { UpdateChannel, UpdatesView } from "@/types/ipc";

/**
 * Mises à jour (écran 1.6-g). Le cœur lit les versions publiées et décide si une vérification
 * est due (lot L allégé) ; il télécharge, vérifie et installe une version installable (lot L,
 * `docs/plan-mvp-l-mises-a-jour.md`). Le store garde sa vue, remplacée à chaque événement
 * `arnod://mise-a-jour` ; sinon « Voir la version » ouvre la page de la release.
 */
export const useUpdatesStore = defineStore("updates", () => {
  const view = ref<UpdatesView | null>(null);
  const checking = ref(false);
  const error = ref<string | null>(null);
  let subscribed = false;
  /** Seule la fenêtre principale annonce une version prête. */
  let announce = false;

  const install = computed(() => view.value?.install ?? { state: "idle" as const });

  /** Applique une vue du cœur ; annonce la fin d'un téléchargement (MJ2). */
  function apply(next: UpdatesView): void {
    const before = view.value?.install.state;
    view.value = next;
    if (announce && before === "downloading" && next.install.state === "ready") {
      const version = next.install.version;
      useToastStore().show(`Arnod ${version} est prête.`, {
        action: { label: "Redémarrer maintenant", run: () => installNow() },
      });
    }
  }

  /**
   * Suit le téléchargement lancé dans n'importe quelle fenêtre (R8) ; `main` : cette fenêtre
   * annonce la version prête par une notification.
   */
  async function subscribe(main: boolean): Promise<void> {
    announce = main;
    if (subscribed) return;
    subscribed = true;
    await events.onUpdates(apply);
  }

  async function load(): Promise<UpdatesView> {
    const next = await ipc.updatesLoad();
    apply(next);
    return next;
  }

  async function save(automatic: boolean, channel: UpdateChannel): Promise<void> {
    apply(await ipc.updatesSave(automatic, channel));
  }

  /** « Vérifier maintenant » ; l'erreur reste affichée dans la section. */
  async function check(): Promise<void> {
    checking.value = true;
    error.value = null;
    try {
      apply(await ipc.updatesCheck(false));
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    } finally {
      checking.value = false;
    }
  }

  /**
   * Vérification au démarrage, si elle est due : une version plus récente s'annonce par une
   * notification, avec « Installer » si elle s'installe depuis Arnod (MJ1), « Voir » sinon. Un
   * échec reste silencieux (aucun réseau, proxy…).
   */
  async function checkAtStartup(): Promise<void> {
    try {
      const known = view.value?.latest?.version;
      const next = await ipc.updatesCheck(true);
      apply(next);
      const latest = next.latest;
      if (latest && latest.version !== known) {
        useToastStore().show(`Arnod ${latest.version} est disponible.`, {
          action: next.installable
            ? { label: "Installer", run: () => download() }
            : {
                label: "Voir",
                run: () => useNetworkStore().openPreferences("updates"),
              },
        });
      }
    } catch {
      // Hors ligne ou proxy : la section « Mises à jour » le dira à la demande.
    }
  }

  /**
   * Télécharge et vérifie la version trouvée (MJ1) ; la progression arrive par l'événement. Un
   * échec est gardé par le cœur (`install.state === "failed"`) et rejeté pour être notifié.
   */
  async function download(): Promise<void> {
    apply(await ipc.updatesDownload());
  }

  /** Arrête le téléchargement, oublie le paquet (R5). */
  async function cancel(): Promise<void> {
    apply(await ipc.updatesCancel());
  }

  /** « Au prochain lancement » (MJ3, R9). */
  async function installOnExit(): Promise<void> {
    apply(await ipc.updatesInstall(false));
  }

  /**
   * « Redémarrer maintenant » (MJ2) : enregistre les contrats et requêtes modifiés de la
   * fenêtre (R6), puis installe ; Arnod se ferme et redémarre. Un brouillon resté non enregistré
   * (échec, conflit, secret en attente) arrête l'installation.
   */
  async function installNow(): Promise<void> {
    const contracts = useContractsStore();
    const workbench = useWorkbenchStore();
    const saved = await Promise.all(Object.keys(contracts.open).map((key) => contracts.save(key)));
    let failed = saved.includes(false);
    try {
      await workbench.saveAll();
    } catch {
      // L'éditeur garde et montre l'échec (« Non enregistré · Réessayer »).
      failed = true;
    }
    const unsaved = failed || Object.keys(workbench.editors).some((key) => workbench.isDirty(key));
    if (unsaved) {
      throw new Error(
        "Des modifications ne sont pas enregistrées : enregistrez-les ou annulez-les, puis redémarrez.",
      );
    }
    apply(await ipc.updatesInstall(true));
  }

  async function open(url: string): Promise<void> {
    await ipc.updatesOpen(url);
  }

  return {
    view,
    install,
    checking,
    error,
    subscribe,
    load,
    save,
    check,
    checkAtStartup,
    download,
    cancel,
    installOnExit,
    installNow,
    open,
  };
});
