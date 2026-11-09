<script setup lang="ts">
/**
 * Démarrage : charge le catalogue et l'état des fenêtres, puis affiche la
 * fenêtre selon son rôle (docs/plan-fenetres-zones.md) : principale (premier
 * lancement ou dernier onglet ouvert, W2), de contenu, ou barre latérale
 * détachée. Un état illisible mis de côté est notifié ; un état inutilisable
 * (version plus récente, disque) bloque avec son message. Chaque fenêtre suit
 * les modifications faites dans les autres (F11).
 */
import { onMounted, ref } from "vue";

import AppShell from "@/components/layout/AppShell.vue";
import ToastHost from "@/components/ui/ToastHost.vue";
import ContentWindow from "@/components/windows/ContentWindow.vue";
import SidebarWindow from "@/components/windows/SidebarWindow.vue";
import { events } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const startupError = ref<string | null>(null);
const ready = ref(false);

onMounted(async () => {
  try {
    const warning = await catalog.load();
    await windows.load();
    await Promise.all([catalog.subscribe(), workbench.subscribe(), contracts.subscribe()]);
    if (windows.kind !== "main") {
      // Fermeture d'une fenêtre détachée : enregistrer, puis se rattacher (Z6).
      await events.onClose(() => {
        void Promise.all(Object.keys(contracts.open).map((key) => contracts.save(key)))
          .then(() => workbench.saveAll())
          .finally(() => windows.reattach());
      });
    }
    navigation.start();
    ready.value = true;
    if (warning) toasts.show(warning.message, { tone: "error" });
  } catch (error) {
    startupError.value = error instanceof Error ? error.message : String(error);
  }
});
</script>

<template>
  <section v-if="startupError" class="startup-error" role="alert">
    <h1 class="startup-error__title">Arnod ne peut pas démarrer</h1>
    <p class="startup-error__message">{{ startupError }}</p>
  </section>
  <template v-else-if="catalog.snapshot && ready">
    <ContentWindow v-if="windows.kind === 'content'" />
    <SidebarWindow v-else-if="windows.kind === 'sidebar'" />
    <AppShell v-else />
  </template>
  <ToastHost />
</template>

<style scoped>
.startup-error {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 0.75rem;
  min-height: 100vh;
  max-width: 40rem;
  margin: 0 auto;
  padding: 2rem 1.5rem;
}

.startup-error__title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 650;
}

.startup-error__message {
  margin: 0;
  color: var(--color-danger);
}
</style>
