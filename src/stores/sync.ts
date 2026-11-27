import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { useDrafts } from "@/composables/useDrafts";
import { IpcError, events, ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import { contentProject, useWindowsStore } from "@/stores/windows";
import type {
  MissingSecrets,
  SyncChoice,
  SyncConflicts,
  SyncLoginOptions,
  SyncView,
} from "@/types/ipc";

/** Vérification de ce qui arrive des collègues : toutes les 5 minutes (Q6). */
export const SYNC_CHECK_MS = 5 * 60 * 1000;

/**
 * Synchronisation d'un projet avec son équipe (`docs/plan-v2.md`, lots E et F, 7.1-a à 7.1-e).
 * Le cœur fait tout (Git, fusion, descriptions en clair) ; le store garde l'état de chaque
 * projet (diffusé par `arnod://synchro`), ouvre le panneau, le dialogue « Relier » et le
 * conflit, et vérifie au démarrage, toutes les 5 minutes et au retour sur la fenêtre.
 */
export const useSyncStore = defineStore("sync", () => {
  const catalog = useCatalogStore();
  const navigation = useNavigationStore();
  const windows = useWindowsStore();
  const toasts = useToastStore();

  const views = ref<Record<string, SyncView>>({});
  /** Panneau de synchronisation (7.1-b) ouvert pour ce projet. */
  const panel = ref<string | null>(null);
  /** Dialogue « Relier le projet » (7.1-a) ouvert pour ce projet. */
  const linking = ref<string | null>(null);
  /** Conflits de la récupération en attente (7.1-c). */
  const conflicts = ref<SyncConflicts | null>(null);
  /** Secrets à renseigner après une récupération (7.1-e). */
  const missing = ref<{ projectId: string; secrets: MissingSecrets[] } | null>(null);
  const loginOptions = ref<SyncLoginOptions>({ githubBrowser: false, gitlabBrowser: false });
  /** Opération en cours (récupérer, envoyer, relier) : les boutons attendent. */
  const busy = ref(false);

  /** Projet de la barre d'état : celui de l'onglet actif, sinon le projet ouvert. */
  const projectId = computed<string | null>(() => {
    const content = windows.activeTab?.content;
    const candidates = [
      content ? contentProject(content) : null,
      navigation.view.kind === "project" ? navigation.view.projectId : null,
    ];
    return (
      candidates.find(
        (id): id is string =>
          typeof id === "string" &&
          !catalog.isInbox(id) &&
          catalog.projects.some((p) => p.id === id && p.status !== "missing"),
      ) ?? null
    );
  });
  const current = computed(() => (projectId.value ? (views.value[projectId.value] ?? null) : null));

  function keep(view: SyncView): void {
    views.value = { ...views.value, [view.projectId]: view };
  }

  async function refresh(id: string): Promise<SyncView | null> {
    try {
      const view = await ipc.syncStatus(id);
      keep(view);
      return view;
    } catch {
      return null;
    }
  }

  /** Vérifie ce qui arrive (sans rien écrire) ; un projet non relié n'est que relu. */
  async function check(id: string): Promise<void> {
    const known = views.value[id] ?? (await refresh(id));
    if (!known?.remote) return;
    try {
      keep(await ipc.syncCheck(id));
    } catch {
      // L'échec est gardé dans la vue (« vérification impossible »), sans notification.
    }
  }

  let timer: ReturnType<typeof setInterval> | null = null;
  let started = false;

  /** Abonnement et vérifications (au démarrage, toutes les 5 minutes, au retour, Q6). */
  async function start(): Promise<void> {
    if (started) return;
    started = true;
    await events.onSync((view) => {
      keep(view);
    });
    try {
      loginOptions.value = await ipc.syncLoginOptions();
    } catch {
      // Sans réponse, la connexion se fait par un jeton personnel.
    }
    const tick = () => {
      if (projectId.value) void check(projectId.value);
    };
    timer = setInterval(tick, SYNC_CHECK_MS);
    window.addEventListener("focus", tick);
  }

  function stop(): void {
    if (timer !== null) clearInterval(timer);
    timer = null;
  }

  function openPanel(id: string): void {
    const view = views.value[id];
    if (view?.conflicts) {
      void openConflicts(id);
      return;
    }
    if (!view?.remote) {
      linking.value = id;
      return;
    }
    panel.value = id;
    void check(id);
  }

  async function link(
    id: string,
    url: string,
    branch: string,
    account: SyncView["account"],
  ): Promise<void> {
    busy.value = true;
    try {
      keep(await ipc.syncLink(id, url, branch, account));
      linking.value = null;
      toasts.show("✓ Projet relié et envoyé");
    } finally {
      busy.value = false;
    }
  }

  /**
   * Récupérer, récupérer puis envoyer, ou envoyer seulement (SY6). Les brouillons sont
   * enregistrés d'abord ; un brouillon non enregistré arrête tout (SY7).
   */
  async function pullPush(
    id: string,
    items: string[],
    message: string,
    pull: boolean,
    push: boolean,
  ): Promise<void> {
    const refusal = await useDrafts().saveAll();
    if (refusal !== null) {
      toasts.show(refusal, { tone: "error" });
      return;
    }
    busy.value = true;
    try {
      const view = await ipc.syncPullPush(id, items, message, pull, push);
      keep(view);
      if (view.conflicts > 0) {
        panel.value = null;
        await openConflicts(id);
        return;
      }
      if (push) toasts.show("✓ Modifications envoyées");
      else toasts.show("✓ Modifications récupérées");
      if (pull) await loadMissing(id);
    } catch (caught) {
      if (caught instanceof IpcError && caught.kind === "conflict") {
        panel.value = null;
        await openConflicts(id);
        return;
      }
      throw caught;
    } finally {
      busy.value = false;
    }
  }

  async function openConflicts(id: string): Promise<void> {
    conflicts.value = await ipc.syncConflicts(id);
  }

  async function resolve(choices: Record<string, Record<string, SyncChoice>>): Promise<void> {
    const id = conflicts.value?.projectId;
    if (!id) return;
    busy.value = true;
    try {
      keep(await ipc.syncResolve(id, choices));
      conflicts.value = null;
      toasts.show("✓ Conflit résolu : vous pouvez envoyer");
      await loadMissing(id);
    } finally {
      busy.value = false;
    }
  }

  async function abandon(): Promise<void> {
    const id = conflicts.value?.projectId;
    if (!id) return;
    keep(await ipc.syncAbandon(id));
    conflicts.value = null;
    toasts.show("Récupération abandonnée : le projet est resté tel quel.");
  }

  /** Secrets manquants après une récupération (7.1-e, R15). */
  async function loadMissing(id: string): Promise<void> {
    try {
      const secrets = await ipc.syncMissingSecrets(id);
      missing.value = secrets.length > 0 ? { projectId: id, secrets } : null;
    } catch {
      missing.value = null;
    }
  }

  async function later(): Promise<void> {
    const shown = missing.value;
    missing.value = null;
    if (!shown) return;
    for (const entry of shown.secrets)
      await ipc.syncSecretsLater(shown.projectId, entry.environment, entry.names);
  }

  /** Auteur d'un changement reçu à la dernière récupération (pastille, CF4). */
  function receivedBy(id: string, path: string): string | null {
    return views.value[id]?.received[path] ?? null;
  }

  return {
    views,
    panel,
    linking,
    conflicts,
    missing,
    loginOptions,
    busy,
    projectId,
    current,
    refresh,
    check,
    start,
    stop,
    openPanel,
    link,
    pullPush,
    openConflicts,
    resolve,
    abandon,
    loadMissing,
    later,
    receivedBy,
  };
});
