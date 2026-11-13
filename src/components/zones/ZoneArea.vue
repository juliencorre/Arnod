<script setup lang="ts">
/**
 * Zones de la fenêtre (F1 à F14) : l'arbre des zones, ou la seule zone
 * agrandie (⤢). Ctrl+W ferme l'onglet actif de la zone active, après avoir
 * enregistré la requête ; Alt+Maj+flèches déplace la séparation la plus proche
 * de la zone active (F4).
 *
 * Lot F (docs/plan-mvp-f-onglets-clavier.md) : Ctrl+Tab et Ctrl+Maj+Tab parcourent
 * les onglets de la zone active dans l'ordre d'utilisation, liste affichée tant que
 * Ctrl est tenu (1.3-c) ; Ctrl+J met le résultat dessous ou à côté (1.4-j). Les
 * touches sont lues par leur code (`event.code`), pour l'AZERTY.
 */
import { computed, onBeforeUnmount, onMounted, watch } from "vue";

import ZoneLayout from "@/components/zones/ZoneLayout.vue";
import ZoneView from "@/components/zones/ZoneView.vue";
import { useTabActions } from "@/composables/useTabActions";
import { contentProject, useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

/** Projet présenté dans une fenêtre sans onglet (fenêtre principale). */
defineProps<{ projectId?: string | null }>();

const windows = useWindowsStore();
const tabs = useTabActions();
const workbench = useWorkbenchStore();

// Les onglets ont besoin des données de leur projet (arborescence, run).
watch(
  () => [
    ...new Set(
      windows.zones.flatMap((zone) =>
        zone.tabs.flatMap((t) => {
          const project = contentProject(t.content);
          return project === null ? [] : [project];
        }),
      ),
    ),
  ],
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

/** Ctrl+Tab, Ctrl+J : `true` si la touche est traitée. */
function onShortcut(event: KeyboardEvent): boolean {
  const ctrl = event.ctrlKey || event.metaKey;
  if (ctrl && !event.altKey && event.code === "Tab") {
    if (!windows.stepTabSwitcher(event.shiftKey ? -1 : 1)) return false;
    event.preventDefault();
    return true;
  }
  if (windows.tabSwitcher !== null && (event.key === "Escape" || event.key === "Enter")) {
    event.preventDefault();
    if (event.key === "Enter") void windows.commitTabSwitcher();
    else windows.cancelTabSwitcher();
    return true;
  }
  if (ctrl && !event.altKey && !event.shiftKey && event.code === "KeyJ") {
    event.preventDefault();
    void windows.togglePlacement();
    return true;
  }
  return false;
}

/** Ctrl relâché : l'onglet choisi dans la liste de Ctrl+Tab s'ouvre (1.3-c). */
function onKeyup(event: KeyboardEvent): void {
  if (windows.tabSwitcher !== null && (event.key === "Control" || event.key === "Meta")) {
    void windows.commitTabSwitcher();
  }
}

/** La fenêtre perd le focus pendant Ctrl+Tab : la liste se ferme sans rien changer. */
function onBlur(): void {
  windows.cancelTabSwitcher();
}

function onKeydown(event: KeyboardEvent): void {
  // Déjà traité, par exemple par une séparation focalisée.
  if (event.defaultPrevented || resizeAround(event) || onShortcut(event)) return;
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "w") {
    const tab = windows.activeTab;
    if (!tab) return;
    event.preventDefault();
    void tabs.close(tab.id);
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("keyup", onKeyup);
  window.addEventListener("blur", onBlur);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("keyup", onKeyup);
  window.removeEventListener("blur", onBlur);
  windows.cancelTabSwitcher();
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
