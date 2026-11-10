<script setup lang="ts">
/**
 * Résultat d'une requête : la réponse reçue ou les réponses attendues (EP8). Partagé par
 * l'onglet de la requête et par l'onglet de résultat de sa fenêtre
 * (docs/plan-resultat-detachable.md). Le créneau `tools` reçoit les commandes de l'onglet
 * (place du résultat, fenêtre).
 */
import { computed, ref, watch } from "vue";

import ExpectedResponses from "@/components/workbench/ExpectedResponses.vue";
import ResponsePanel from "@/components/workbench/ResponsePanel.vue";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);

/**
 * Un endpoint de contrat s'ouvre sur ses réponses attendues ; un envoi montre la réponse.
 */
const side = ref<"response" | "expected">("response");
watch(
  () => {
    const current = editor.value;
    const linked = current?.draft?.contract !== undefined && current.outcome === null;
    return [props.editorKey, linked] as const;
  },
  ([key, linked], previous) => {
    if (linked) side.value = "expected";
    // Une autre requête (onglet de résultat qui suit la requête active) : sa réponse.
    else if (previous && previous[0] !== key) side.value = "response";
  },
  { immediate: true },
);
watch(
  () => [editor.value?.outcome, editor.value?.sending] as const,
  ([outcome, sending]) => {
    if (outcome || sending === true) side.value = "response";
  },
);
const expectedCount = computed(() => editor.value?.draft?.examples.length ?? 0);
</script>

<template>
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
    <div class="side__tools">
      <slot name="tools" />
    </div>
  </div>
  <div role="tabpanel" :aria-label="side === 'response' ? 'Réponse' : 'Réponses attendues'">
    <ResponsePanel v-if="side === 'response'" :editor-key="editorKey" />
    <ExpectedResponses v-else :editor-key="editorKey" />
  </div>
</template>

<style scoped>
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

.side__tools {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  margin-bottom: 0.375rem;
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
</style>
