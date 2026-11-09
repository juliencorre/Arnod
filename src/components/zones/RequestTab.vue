<script setup lang="ts">
/**
 * Onglet d'une requête : éditeur et résultat, côte à côte ou l'un sous
 * l'autre (F6). En automatique, le résultat passe dessous quand la zone fait
 * moins de 860 px de large.
 */
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";

import ExpectedResponses from "@/components/workbench/ExpectedResponses.vue";
import RequestEditor from "@/components/workbench/RequestEditor.vue";
import ResponsePanel from "@/components/workbench/ResponsePanel.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { useTabActions } from "@/composables/useTabActions";
import { useWindowsStore } from "@/stores/windows";
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

/**
 * Côté résultat : la réponse reçue ou les réponses attendues (EP8). Un endpoint de contrat
 * s'ouvre sur ses réponses attendues ; un envoi montre la réponse.
 */
const side = ref<"response" | "expected">("response");
watch(
  () => {
    const current = editor.value;
    return current?.draft?.contract !== undefined && current.outcome === null;
  },
  (linked) => {
    if (linked) side.value = "expected";
  },
  { immediate: true },
);
watch(
  () => editor.value?.outcome,
  (outcome) => {
    if (outcome) side.value = "response";
  },
);
const expectedCount = computed(() => editor.value?.draft?.examples.length ?? 0);

/** Clic sur la place active : retour à l'automatique ; sur l'autre : choix explicite. */
function place(target: "right" | "below"): void {
  const effective = below.value ? "below" : "right";
  const next: Placement = target === effective && props.placement !== "auto" ? "auto" : target;
  void windows.setPlacement(props.zoneId, next);
}
</script>

<template>
  <div ref="root" class="request-tab" :class="{ 'request-tab--below': below }">
    <div v-if="editor?.loadError" class="request-tab__missing" role="alert">
      <p>{{ editor.loadError }}</p>
      <BaseButton @click="tabs.close(tabId)">Fermer l'onglet</BaseButton>
    </div>
    <p v-else-if="!editor?.draft" class="request-tab__loading">Chargement…</p>
    <template v-else>
      <div class="request-tab__editor">
        <RequestEditor :editor-key="key" />
      </div>
      <div class="request-tab__response">
        <div class="side">
          <div role="tablist" aria-label="Résultat" class="side__tabs">
            <button
              type="button"
              role="tab"
              class="side__tab"
              :aria-selected="side === 'response'"
              @click="side = 'response'"
            >
              Réponse
            </button>
            <button
              type="button"
              role="tab"
              class="side__tab"
              :aria-selected="side === 'expected'"
              @click="side = 'expected'"
            >
              Réponses attendues
              <span v-if="expectedCount > 0" class="side__badge">{{ expectedCount }}</span>
            </button>
          </div>
          <div class="placement" role="group" aria-label="Place du résultat">
            <button
              type="button"
              class="placement__option"
              :aria-pressed="!below"
              @click="place('right')"
            >
              À droite{{ !below && placement === "auto" ? " · auto" : "" }}
            </button>
            <button
              type="button"
              class="placement__option"
              :aria-pressed="below"
              @click="place('below')"
            >
              Dessous{{ below && placement === "auto" ? " · auto" : "" }}
            </button>
          </div>
        </div>
        <div role="tabpanel" :aria-label="side === 'response' ? 'Réponse' : 'Réponses attendues'">
          <ResponsePanel v-if="side === 'response'" :editor-key="key" />
          <ExpectedResponses v-else :editor-key="key" />
        </div>
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
  flex: 1 1 55%;
  min-width: 0;
}

.request-tab__response {
  flex: 1 1 45%;
  min-width: 0;
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

.side {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.side__tabs {
  display: flex;
  gap: 0.25rem;
}

.side__tab {
  padding: 0.5rem 0.625rem;
  border: 0;
  border-bottom: 2px solid transparent;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.side__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.side__tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.side__badge {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  border-radius: 999px;
  font-size: 0.6875rem;
  background: var(--color-chip);
}

.placement {
  display: inline-flex;
  margin-bottom: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  overflow: hidden;
}

.placement__option {
  padding: 0.1875rem 0.5rem;
  border: none;
  font: inherit;
  font-size: 0.75rem;
  color: var(--color-muted);
  background: var(--color-surface);
  cursor: pointer;
}

.placement__option[aria-pressed="true"] {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-chip);
}

.placement__option:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}
</style>
