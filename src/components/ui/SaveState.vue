<script setup lang="ts">
/**
 * État d'un enregistrement automatique (ER2), comme celui d'une requête : « ✓ Enregistré »,
 * « Modifications en attente… », « Enregistrement… », ou « Non enregistré · Réessayer ».
 */
import type { AutosaveState } from "@/composables/useAutosave";

defineProps<{ state: AutosaveState; error?: string | null }>();
const emit = defineEmits<{ retry: [] }>();

const LABELS: Record<AutosaveState, string> = {
  saved: "✓ Enregistré",
  pending: "Modifications en attente…",
  saving: "Enregistrement…",
  error: "Non enregistré",
};
</script>

<template>
  <p class="save-state" :class="`save-state--${state}`" role="status" :title="error ?? undefined">
    {{ LABELS[state] }}
    <button v-if="state === 'error'" type="button" class="save-state__retry" @click="emit('retry')">
      Réessayer
    </button>
  </p>
</template>

<style scoped>
.save-state {
  display: flex;
  flex: none;
  gap: 0.375rem;
  align-items: center;
  margin: 0;
  white-space: nowrap;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.save-state--error {
  color: var(--color-danger);
}

.save-state__retry {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.save-state__retry:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}
</style>
