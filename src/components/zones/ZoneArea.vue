<script setup lang="ts">
/**
 * Zones de la fenêtre (F1 à F14) : l'arbre des zones, ou la seule zone
 * agrandie (⤢). Ctrl+W ferme l'onglet actif de la zone active, après avoir
 * enregistré la requête ; Alt+Maj+flèches déplace la séparation la plus proche
 * de la zone active (F4).
 */
import { computed, onBeforeUnmount, onMounted, watch } from "vue";

import ZoneLayout from "@/components/zones/ZoneLayout.vue";
import ZoneView from "@/components/zones/ZoneView.vue";
import { useTabActions } from "@/composables/useTabActions";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

/** Projet présenté dans une fenêtre sans onglet (fenêtre principale). */
defineProps<{ projectId?: string | null }>();

const windows = useWindowsStore();
const tabs = useTabActions();
const workbench = useWorkbenchStore();

// Les onglets ont besoin des données de leur projet (arborescence, run).
watch(
  () => [...new Set(windows.zones.flatMap((zone) => zone.tabs.map((t) => t.content.projectId)))],
  (projects) => {
    for (const projectId of projects) {
      workbench.loadProject(projectId).catch(() => undefined);
    }
  },
  { immediate: true },
);

const maximized = computed(
  () => windows.zones.find((zone) => zone.id === windows.maximizedZone) ?? null,
);

/** Pas d'Alt+Maj+flèches, comme sur une séparation focalisée. */
const STEP = 0.05;

function resizeAround(event: KeyboardEvent): boolean {
  const horizontal = event.key === "ArrowLeft" || event.key === "ArrowRight";
  const vertical = event.key === "ArrowUp" || event.key === "ArrowDown";
  if (!event.altKey || !event.shiftKey || (!horizontal && !vertical)) return false;
  const zone = windows.activeZone;
  const split = zone ? windows.splitAround(zone.id, horizontal ? "row" : "column") : null;
  if (!split) return false;
  event.preventDefault();
  const delta = event.key === "ArrowRight" || event.key === "ArrowDown" ? STEP : -STEP;
  void windows.resize(split.id, split.ratio + delta);
  return true;
}

function onKeydown(event: KeyboardEvent): void {
  // Déjà traité, par exemple par une séparation focalisée.
  if (event.defaultPrevented || resizeAround(event)) return;
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "w") {
    const tab = windows.activeTab;
    if (!tab) return;
    event.preventDefault();
    void tabs.close(tab.id);
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="zones">
    <ZoneView v-if="maximized" :zone="maximized" :project-id="projectId ?? null" />
    <ZoneLayout
      v-else-if="windows.layout"
      :node="windows.layout.root"
      :project-id="projectId ?? null"
    />
  </div>
</template>

<style scoped>
.zones {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  height: 100%;
}
</style>
