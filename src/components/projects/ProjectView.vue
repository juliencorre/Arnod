<script setup lang="ts">
/**
 * Zone de travail de la fenêtre principale (F1 à F14) : les zones et leurs
 * onglets. Sans onglet ouvert, le projet choisi est présenté ; il est déplié
 * dans la barre latérale.
 */
import { computed, watch } from "vue";

import ZoneArea from "@/components/zones/ZoneArea.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string; workspaceId: string | null }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const catalog = useCatalogStore();
const windows = useWindowsStore();
/** Avec des onglets, le titre de la vue n'est lu que par les lecteurs d'écran. */
const hasTabs = computed(() => windows.zones.some((zone) => zone.tabs.length > 0));

watch(
  () => props.projectId,
  (projectId) => {
    workbench.expand(projectId).catch((error: unknown) => {
      toasts.showError(error);
    });
  },
  { immediate: true },
);
</script>

<template>
  <h1 v-if="hasTabs" class="visually-hidden">{{ catalog.projectById(projectId)?.name }}</h1>
  <ZoneArea :project-id="projectId" />
</template>
