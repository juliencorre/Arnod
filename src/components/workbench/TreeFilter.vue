<script setup lang="ts">
/**
 * Champ de filtre du référentiel de tests (docs/plan-mvp-d-barre-laterale.md, F1, F5) : le
 * champ de recherche commun (`SearchInput`, Échap efface), le compte « 2 sur 66 » et un bouton
 * qui efface. Le filtre lui-même est fait par `lib/treeFilter.ts`.
 */
import { useTemplateRef } from "vue";

import SearchInput from "@/components/ui/SearchInput.vue";

const model = defineModel<string>({ required: true });
defineProps<{ label: string; count: string | null }>();
const emit = defineEmits<{ focus: []; blur: [] }>();

const root = useTemplateRef<HTMLDivElement>("root");

function input(): HTMLInputElement | null {
  return root.value?.querySelector("input") ?? null;
}

function clear(): void {
  model.value = "";
  input()?.focus();
}

defineExpose({
  focus: () => {
    input()?.focus();
    input()?.select();
  },
});
</script>

<template>
  <div ref="root" class="filter" @focusin="emit('focus')" @focusout="emit('blur')">
    <svg class="filter__icon" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <circle cx="5" cy="5" r="3.4" />
      <path d="M7.6 7.6 10.5 10.5" />
    </svg>
    <SearchInput v-model="model" class="filter__input" :label="label" placeholder="Filtrer" />
    <span v-if="count !== null" class="filter__count" role="status">{{ count }}</span>
    <button
      v-if="model !== ''"
      type="button"
      class="filter__clear"
      aria-label="Effacer le filtre"
      title="Effacer le filtre"
      @click="clear"
    >
      ×
    </button>
  </div>
</template>

<style scoped>
.filter {
  position: relative;
  display: flex;
  align-items: center;
  margin: 0.125rem var(--tree-pad) 0.25rem calc(var(--tree-pad) + var(--tree-indent));
}

.filter__icon {
  position: absolute;
  left: 0.5rem;
  pointer-events: none;
}

.filter__icon circle,
.filter__icon path {
  fill: none;
  stroke: var(--color-muted);
  stroke-width: 1.3;
  stroke-linecap: round;
}

.filter__input {
  min-height: var(--control-sm);
  padding: 0.125rem 4.75rem 0.125rem 1.5rem;
  font-size: var(--text-body);
}

.filter__input::-webkit-search-cancel-button {
  appearance: none;
}

.filter__count {
  position: absolute;
  right: 1.75rem;
  font: var(--text-caption) var(--font-mono);
  white-space: nowrap;
  color: var(--color-muted);
  pointer-events: none;
}

.filter__clear {
  position: absolute;
  right: 0.25rem;
  display: grid;
  place-items: center;
  width: 1.25rem;
  height: 1.25rem;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.filter__clear:hover {
  background: var(--color-chip);
}

.filter__clear:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}
</style>
