<script setup lang="ts">
/**
 * Démarrage : vérifie d'abord que l'état se lit (lot G, `startup_status`), puis charge le
 * catalogue et l'état des fenêtres et affiche la fenêtre selon son rôle
 * (docs/plan-fenetres-zones.md) : principale (premier lancement ou dernier onglet ouvert, W2),
 * de contenu, ou barre latérale détachée. Une erreur bloquante s'explique avec ses issues
 * (0.3-b) ; réglée, le démarrage reprend. Tant que rien n'est prêt, un squelette de l'écran
 * attendu (0.3-a), visible seulement si la fenêtre, créée cachée, est montrée après 300 ms.
 * Chaque fenêtre suit les modifications faites dans les autres (F11).
 */
import { computed, nextTick, onMounted, ref, watch } from "vue";

import AppShell from "@/components/layout/AppShell.vue";
import StartupProblemScreen from "@/components/startup/StartupProblemScreen.vue";
import StartupSkeleton from "@/components/startup/StartupSkeleton.vue";
import ToastHost from "@/components/ui/ToastHost.vue";
import ContentWindow from "@/components/windows/ContentWindow.vue";
import SidebarWindow from "@/components/windows/SidebarWindow.vue";
import { useDrafts } from "@/composables/useDrafts";
import { events, ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useNavigationStore } from "@/stores/navigation";
import { useStartupStore } from "@/stores/startup";
import { useToastStore } from "@/stores/toasts";
import { useUpdatesStore } from "@/stores/updates";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const startup = useStartupStore();
const toasts = useToastStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const drafts = useDrafts();
const ready = ref(false);
let booting = false;

/** Écran attendu par le squelette, connu une fois les fenêtres lues. */
const skeletonKind = computed(() => (windows.kind === "main" ? "main" : windows.kind));

/** « Ouverture de « Boutique »… » : le workspace rouvert, une fois le catalogue lu. */
const caption = computed(() => {
  const id = catalog.snapshot?.lastWorkspaceId;
  const name = id ? catalog.workspaceById(id)?.name : null;
  return name ? `Ouverture de « ${name} »…` : null;
});

async function boot(): Promise<void> {
  if (booting) return;
  booting = true;
  try {
    const warning = await catalog.load();
    await windows.load();
    await Promise.all([
      catalog.subscribe(),
      workbench.subscribe(),
      contracts.subscribe(),
      // Lot L : progression d'une mise à jour, annoncée par la fenêtre principale.
      useUpdatesStore().subscribe(windows.kind === "main"),
    ]);
    if (windows.kind !== "main") {
      // Fermeture d'une fenêtre détachée : enregistrer, puis se rattacher (Z6).
      // Un brouillon non enregistré garde la fenêtre ouverte et le dit (R6, IN3).
      await events.onClose(() => {
        void drafts.reattach();
      });
    }
    navigation.start();
    ready.value = true;
    if (warning) toasts.show(warning.message, { tone: "error" });
  } catch (error) {
    startup.unexpected(error instanceof Error ? error.message : String(error));
  } finally {
    booting = false;
  }
}

/** La fenêtre, créée cachée, apparaît une fois l'écran rendu : prêt, ou l'erreur. */
async function reveal(): Promise<void> {
  await nextTick();
  await startup.windowReady();
}

// Erreur réglée (réessai, état mis de côté) : le démarrage reprend.
watch(
  () => startup.blocked,
  (blocked, before) => {
    if (before && !blocked && !ready.value) void boot().then(reveal);
  },
);

/**
 * Sortie d'Arnod (fenêtre principale fermée, « Fichier › Quitter ») : chaque fenêtre enregistre
 * ses brouillons puis répond au cœur ; un brouillon non enregistré garde Arnod ouvert et le dit
 * (plan qualité, R6, IN3). Abonné avant le démarrage : un écran d'erreur répond aussi.
 */
async function answerQuit(): Promise<void> {
  let saved = false;
  try {
    saved = await drafts.saveAllOrSay();
  } catch (error) {
    toasts.showError(error);
  }
  await ipc.windowQuitReady(saved);
}

onMounted(async () => {
  await events.onQuit(() => {
    void answerQuit();
  });
  if (await startup.check()) await boot();
  await reveal();
  // Lot L (1.6-g) : une version plus récente s'annonce, si la vérification est due.
  if (ready.value && windows.kind === "main") void useUpdatesStore().checkAtStartup();
});
</script>

<template>
  <StartupProblemScreen v-if="startup.problem" />
  <template v-else-if="catalog.snapshot && ready">
    <ContentWindow v-if="windows.kind === 'content'" />
    <SidebarWindow v-else-if="windows.kind === 'sidebar'" />
    <AppShell v-else />
  </template>
  <StartupSkeleton v-else :kind="skeletonKind" :caption="caption" />
  <ToastHost />
</template>
