<script setup lang="ts">
/**
 * Retrait d'une ligne (ER8) : une seule forme dans l'application, × en texte secondaire, rouge
 * au survol, cible de 24 × 24 px (AC4). `label` donne le nom accessible (« Supprimer l'en-tête
 * Accept »), aussi montré en bulle. L'ajout d'une ligne est un `BaseButton` « + Nom ».
 */
defineProps<{ label: string }>();
</script>

<template>
  <button type="button" class="row-remove" :aria-label="label" :title="label">×</button>
</template>

<style scoped>
.row-remove {
  display: inline-grid;
  flex: none;
  place-items: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: none;
  font: inherit;
  font-size: var(--text-body);
  line-height: 1;
  color: var(--color-muted);
  cursor: pointer;
}

.row-remove:hover {
  color: var(--color-danger);
}

.row-remove:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
