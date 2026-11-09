<script setup lang="ts">
/**
 * Liste de paires nom-valeur activables, avec leur description (paramètres, en-têtes, champs
 * de formulaire, EP2, EP4). `kind` restreint la liste aux paramètres de requête : les
 * paramètres de chemin, tirés de l'adresse, sont édités à part.
 */
import { computed } from "vue";

import type { Pair, ParamKind } from "@/types/ipc";

const props = defineProps<{
  label: string;
  /** Libellé du bouton d'ajout (« + Paramètre », « + En-tête »). */
  addLabel?: string;
  /** Intitulé de la colonne des valeurs. */
  valueTitle?: string;
  kind?: ParamKind;
}>();
const pairs = defineModel<Pair[]>({ required: true });

/** Lignes affichées, avec leur place dans la liste complète. */
const rows = computed(() =>
  pairs.value
    .map((pair, index) => ({ pair, index }))
    .filter(({ pair }) => props.kind === undefined || pair.kind === props.kind),
);

function add(): void {
  pairs.value = [
    ...pairs.value,
    { name: "", value: "", enabled: true, kind: props.kind ?? "query", description: "" },
  ];
}

function remove(index: number): void {
  pairs.value = pairs.value.filter((_, i) => i !== index);
}

function nameOf(pair: Pair, position: number): string {
  return pair.name.trim() === "" ? `la ligne ${String(position + 1)}` : pair.name;
}
</script>

<template>
  <div class="pairs" role="group" :aria-label="label">
    <div v-if="rows.length > 0" class="pairs__head" aria-hidden="true">
      <span />
      <span>Nom</span>
      <span>{{ valueTitle ?? "Valeur" }}</span>
      <span />
    </div>
    <div
      v-for="({ pair, index }, position) in rows"
      :key="index"
      class="pairs__row"
      :class="{ 'pairs__row--off': !pair.enabled }"
    >
      <input
        v-model="pair.enabled"
        type="checkbox"
        :aria-label="`${pair.enabled ? 'Désactiver' : 'Activer'} ${nameOf(pair, position)}`"
        class="pairs__check"
      />
      <input
        v-model="pair.name"
        class="pairs__input pairs__input--mono"
        type="text"
        autocomplete="off"
        spellcheck="false"
        placeholder="Nom"
        :aria-label="`Nom, ligne ${String(position + 1)}`"
      />
      <input
        v-model="pair.value"
        class="pairs__input pairs__input--mono"
        type="text"
        autocomplete="off"
        spellcheck="false"
        :placeholder="valueTitle ?? 'Valeur'"
        :aria-label="`Valeur, ligne ${String(position + 1)}`"
      />
      <button
        type="button"
        class="pairs__remove"
        :aria-label="`Supprimer ${nameOf(pair, position)}`"
        @click="remove(index)"
      >
        ✕
      </button>
      <input
        v-model="pair.description"
        class="pairs__input pairs__description"
        type="text"
        autocomplete="off"
        placeholder="Description"
        :aria-label="`Description, ligne ${String(position + 1)}`"
      />
    </div>
    <button type="button" class="pairs__add" @click="add">{{ addLabel ?? "+ Ligne" }}</button>
  </div>
</template>

<style scoped>
.pairs {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.pairs__head,
.pairs__row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) minmax(0, 1fr) auto;
  gap: 0.25rem 0.5rem;
  align-items: center;
}

.pairs__head {
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.pairs__row {
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--color-border);
}

.pairs__row--off .pairs__input {
  color: var(--color-muted);
}

.pairs__description {
  grid-column: 2 / 4;
  font-size: 0.75rem;
}

.pairs__input {
  min-width: 0;
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.pairs__input--mono {
  font-family: var(--font-mono);
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
  font-size: 0.875rem;
  line-height: 1;
  color: var(--color-muted);
  cursor: pointer;
}

.pairs__add {
  align-self: flex-start;
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  background: var(--color-surface);
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  cursor: pointer;
}
</style>
