<script setup lang="ts">
/** Liste de paires nom-valeur activables (paramètres, en-têtes, champs de formulaire). */
import type { Pair } from "@/types/ipc";

defineProps<{ label: string; namePlaceholder?: string; valuePlaceholder?: string }>();
const pairs = defineModel<Pair[]>({ required: true });

function add(): void {
  pairs.value = [...pairs.value, { name: "", value: "", enabled: true }];
}

function remove(index: number): void {
  pairs.value = pairs.value.filter((_, i) => i !== index);
}
</script>

<template>
  <div class="pairs" role="group" :aria-label="label">
    <div v-for="(pair, index) in pairs" :key="index" class="pairs__row">
      <input
        v-model="pair.enabled"
        type="checkbox"
        :aria-label="`Activer la ligne ${index + 1}`"
        class="pairs__check"
      />
      <input
        v-model="pair.name"
        class="pairs__input"
        type="text"
        autocomplete="off"
        :placeholder="namePlaceholder ?? 'Nom'"
        :aria-label="`Nom, ligne ${index + 1}`"
      />
      <input
        v-model="pair.value"
        class="pairs__input"
        type="text"
        autocomplete="off"
        :placeholder="valuePlaceholder ?? 'Valeur'"
        :aria-label="`Valeur, ligne ${index + 1}`"
      />
      <button
        type="button"
        class="pairs__remove"
        :aria-label="`Supprimer la ligne ${index + 1}`"
        @click="remove(index)"
      >
        ×
      </button>
    </div>
    <button type="button" class="pairs__add" @click="add">+ Ajouter une ligne</button>
  </div>
</template>

<style scoped>
.pairs {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.pairs__row {
  display: grid;
  grid-template-columns: auto 1fr 1.5fr auto;
  gap: 0.5rem;
  align-items: center;
}

.pairs__input {
  min-width: 0;
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font: 0.8125rem var(--font-mono);
  color: var(--color-text);
  background: var(--color-surface);
}

.pairs__input:focus-visible,
.pairs__check:focus-visible,
.pairs__remove:focus-visible,
.pairs__add:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.pairs__remove {
  border: 0;
  background: none;
  font-size: 1.125rem;
  line-height: 1;
  color: var(--color-muted);
  cursor: pointer;
}

.pairs__add {
  align-self: flex-start;
  padding: 0.25rem 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}
</style>
