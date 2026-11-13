<script setup lang="ts">
/**
 * Onglet d'une requête : éditeur et résultat, côte à côte ou l'un sous
 * l'autre (F6). En automatique, le résultat passe dessous quand la zone fait
 * moins de 860 px de large. Quand le résultat a sa fenêtre, l'onglet ne montre
 * que l'éditeur (docs/plan-resultat-detachable.md, R3).
 *
 * L'icône de l'en-tête du résultat, Ctrl+J et le menu Fenêtre le mettent dessous
 * ou à côté (lot F, 1.4-j) ; revenir au côté que donnerait l'automatique rend la
 * place automatique.
 */
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";

import RequestEditor from "@/components/workbench/RequestEditor.vue";
import StepBar from "@/components/workbench/StepBar.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import ResultSide from "@/components/zones/ResultSide.vue";
import { useTabActions } from "@/composables/useTabActions";
import { type ShownPlacement, useWindowsStore } from "@/stores/windows";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { Placement } from "@/types/ipc";

const props = defineProps<{
  tabId: string;
  zoneId: string;
  projectId: string;
  path: string;
  placement: Placement;
}>();

/** Largeur sous laquelle le résultat passe dessous en automatique. */
const NARROW = 860;

const workbench = useWorkbenchStore();
const windows = useWindowsStore();
const tabs = useTabActions();
const root = useTemplateRef<HTMLElement>("root");
const width = ref(Number.POSITIVE_INFINITY);
const key = computed(() => requestKey(props.projectId, props.path));
const editor = computed(() => workbench.editors[key.value] ?? null);
const below = computed(() =>
  props.placement === "auto" ? width.value < NARROW : props.placement === "below",
);

let observer: ResizeObserver | null = null;
onMounted(() => {
  if (typeof ResizeObserver !== "undefined" && root.value) {
    observer = new ResizeObserver((entries) => {
      width.value = entries[0]?.contentRect.width ?? width.value;
    });
    observer.observe(root.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());

watch(
  key,
  () => {
    void workbench.openEditor(props.projectId, props.path);
  },
  { immediate: true },
);

/** Place montrée et place automatique, pour Ctrl+J et le menu Fenêtre (1.4-j). */
const shown = computed<ShownPlacement>(() => ({
  shown: below.value ? "below" : "right",
  auto: width.value < NARROW ? "below" : "right",
}));
watch(
  shown,
  (view) => {
    windows.reportPlacement(props.zoneId, view);
  },
  { immediate: true },
);
onBeforeUnmount(() => {
  windows.reportPlacement(props.zoneId, null);
});

const placementLabel = computed(() =>
  below.value ? "Mettre le résultat à côté" : "Mettre le résultat dessous",
);
const placementTitle = computed(
  () =>
    `Résultat ${below.value ? "dessous" : "à côté"}${props.placement === "auto" ? " · auto" : ""} — ${placementLabel.value.toLowerCase()} (Ctrl+J)`,
);
</script>

<template>
  <div
    ref="root"
    class="request-tab"
    :class="{ 'request-tab--below': below, 'request-tab--alone': windows.resultDetached }"
  >
    <div v-if="editor?.loadError" class="request-tab__missing" role="alert">
      <p>{{ editor.loadError }}</p>
      <BaseButton @click="tabs.close(tabId)">Fermer l'onglet</BaseButton>
    </div>
    <p v-else-if="!editor?.draft" class="request-tab__loading">Chargement…</p>
    <template v-else>
      <div class="request-tab__editor">
        <StepBar :project-id="projectId" :path="path" />
        <RequestEditor :editor-key="key" />
      </div>
      <div v-if="!windows.resultDetached" class="request-tab__response">
        <ResultSide :editor-key="key">
          <template #tools>
            <button
              type="button"
              class="placement"
              :class="{ 'placement--below': below }"
              :aria-label="placementLabel"
              :title="placementTitle"
              aria-keyshortcuts="Control+J"
              @click="windows.togglePlacement(zoneId)"
            >
              <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
                <rect x="1.5" y="2" width="11" height="10" rx="1.5" />
                <path v-if="below" d="M1.5 7.5h11" />
                <path v-else d="M7.5 2v10" />
              </svg>
            </button>
            <button
              type="button"
              class="detach"
              aria-label="Ouvrir le résultat dans une fenêtre"
              title="Le résultat dans sa fenêtre, qui suit la requête active"
              @click="windows.detachResult()"
            >
              <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
                <rect x="1.5" y="4" width="8.5" height="8.5" rx="1.5" />
                <path d="M5 2h6a1 1 0 0 1 1 1v6" />
              </svg>
            </button>
          </template>
        </ResultSide>
      </div>
    </template>
  </div>
</template>

<style scoped>
.request-tab {
  display: flex;
  flex: 1;
  gap: 1.25rem;
  min-width: 0;
  padding: 1rem 1.25rem;
}

.request-tab--below {
  flex-direction: column;
}

.request-tab__editor {
  display: flex;
  flex: 1 1 55%;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.request-tab__response {
  flex: 1 1 45%;
  min-width: 0;
}

/* Le résultat a sa fenêtre : l'éditeur prend toute la largeur (R3). */
.request-tab--alone .request-tab__editor {
  flex-basis: 100%;
}

.detach {
  display: grid;
  place-items: center;
  width: 1.75rem;
  height: 1.625rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-muted);
  background: var(--color-surface);
  cursor: pointer;
}

.detach svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linejoin: round;
}

.detach:hover {
  color: var(--color-text);
  border-color: var(--color-border-strong);
}

.detach:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.request-tab--below .request-tab__response {
  padding-top: 1rem;
  border-top: 1px solid var(--color-border);
}

.request-tab:not(.request-tab--below) .request-tab__response {
  padding-left: 1.25rem;
  border-left: 1px solid var(--color-border);
}

.request-tab__missing,
.request-tab__loading {
  margin: 2rem auto;
  text-align: center;
  color: var(--color-muted);
}

/* Place du résultat (1.4-j) : l'icône montre la disposition, un clic la change. */
.placement {
  display: grid;
  place-items: center;
  width: 1.75rem;
  height: 1.625rem;
  border: 1px solid var(--color-accent-soft);
  border-radius: var(--radius-md);
  color: var(--color-link);
  background: var(--color-accent-soft);
  cursor: pointer;
}

.placement svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linejoin: round;
}

.placement:hover {
  border-color: var(--color-border-strong);
}

.placement:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
