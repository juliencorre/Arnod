<script setup lang="ts">
/**
 * Liste de paires nom-valeur activables (paramètres, en-têtes, champs de formulaire).
 * `withKind` ajoute le choix de l'emplacement d'un paramètre : adresse ou chemin (`:nom`, S8).
 */
import type { Pair, ParamKind } from "@/types/ipc";

defineProps<{
  label: string;
  namePlaceholder?: string;
  valuePlaceholder?: string;
  withKind?: boolean;
}>();
const pairs = defineModel<Pair[]>({ required: true });

const KINDS: { id: ParamKind; label: string }[] = [
  { id: "query", label: "Adresse" },
  { id: "path", label: "Chemin" },
];

function add(): void {
  pairs.value = [...pairs.value, { name: "", value: "", enabled: true, kind: "query" }];
}

function remove(index: number): void {
  pairs.value = pairs.value.filter((_, i) => i !== index);
}
</script>

<template>
  <div class="pairs" role="group" :aria-label="label">
    <div
      v-for="(pair, index) in pairs"
      :key="index"
      class="pairs__row"
      :class="{ 'pairs__row--kind': withKind }"
    >
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
      <select
        v-if="withKind"
        v-model="pair.kind"
        class="pairs__input pairs__kind"
        :aria-label="`Emplacement, ligne ${index + 1}`"
      >
        <option v-for="k in KINDS" :key="k.id" :value="k.id">{{ k.label }}</option>
      </select>
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

.pairs__row--kind {
  grid-template-columns: auto 1fr 1.5fr auto auto;
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

.pairs__kind {
  font-family: inherit;
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
