<script setup lang="ts">
/**
 * Champ de recherche. Échap vide le champ ; s'il est déjà vide, l'événement
 * continue son chemin (par exemple pour fermer la fenêtre qui le contient).
 */
const model = defineModel<string>({ required: true });
defineProps<{ label: string; placeholder?: string }>();

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || model.value === "") return;
  event.preventDefault();
  event.stopPropagation();
  model.value = "";
}
</script>

<template>
  <input
    v-model="model"
    type="search"
    class="search"
    :aria-label="label"
    :placeholder="placeholder ?? label"
    autocomplete="off"
    spellcheck="false"
    @keydown="onKeydown"
  />
</template>

<style scoped>
.search {
  box-sizing: border-box;
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.search::placeholder {
  color: var(--color-subtle);
}

.search:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}
</style>
