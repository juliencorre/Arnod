<script setup lang="ts">
/**
 * Démarrage : charge le catalogue, puis affiche le premier lancement ou le
 * dernier workspace ouvert (W2). Un état illisible mis de côté est notifié ;
 * un état inutilisable (version plus récente, disque) bloque avec son message.
 */
import { onMounted, ref } from "vue";

import AppShell from "@/components/layout/AppShell.vue";
import ToastHost from "@/components/ui/ToastHost.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const startupError = ref<string | null>(null);

onMounted(async () => {
  try {
    const warning = await catalog.load();
    navigation.start();
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
  <AppShell v-else-if="catalog.snapshot" />
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
