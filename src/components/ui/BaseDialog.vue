<script setup lang="ts">
/**
 * Fenêtre modale sur l'élément `<dialog>` natif : focus piégé, fermeture par
 * Échap ou clic sur le fond. Le parent garde la main sur `open` : Échap et le
 * clic sur le fond émettent `close` sans fermer d'eux-mêmes. `wide` : une fenêtre
 * plus large, pour un contenu en deux colonnes (liste et formulaire). `closable` : un bouton
 * ✕ « Fermer » dans l'en-tête, pour une fenêtre sans bouton de fin (préférences, ER4).
 */
import { onMounted, useId, useTemplateRef, watch } from "vue";

const props = defineProps<{
  open: boolean;
  title: string;
  description?: string;
  wide?: boolean;
  closable?: boolean;
}>();
const emit = defineEmits<{ close: [] }>();

const dialog = useTemplateRef<HTMLDialogElement>("dialog");
const titleId = useId();
const descriptionId = useId();

function sync(open: boolean): void {
  const element = dialog.value;
  if (!element) return;
  if (open && !element.open) element.showModal();
  if (!open && element.open) element.close();
}

watch(() => props.open, sync);
onMounted(() => {
  sync(props.open);
});

function onClick(event: MouseEvent): void {
  // Le panneau couvre tout le contenu : un clic sur le `<dialog>` lui-même vise le fond.
  if (event.target === dialog.value) emit("close");
}
</script>

<template>
  <dialog
    ref="dialog"
    class="dialog"
    :class="{ 'dialog--wide': wide }"
    :aria-labelledby="titleId"
    :aria-describedby="description ? descriptionId : undefined"
    @cancel.prevent="emit('close')"
    @click="onClick"
  >
    <div class="dialog__panel">
      <div v-if="closable" class="dialog__head">
        <h2 :id="titleId" class="dialog__title">{{ title }}</h2>
        <button type="button" class="dialog__close" aria-label="Fermer" @click="emit('close')">
          ✕
        </button>
      </div>
      <h2 v-else :id="titleId" class="dialog__title">{{ title }}</h2>
      <p v-if="description" :id="descriptionId" class="dialog__description">
        {{ description }}
      </p>
      <div class="dialog__body">
        <slot />
      </div>
      <footer v-if="$slots.footer" class="dialog__footer">
        <slot name="footer" />
      </footer>
    </div>
  </dialog>
</template>

<style scoped>
.dialog {
  width: min(36rem, calc(100vw - 2rem));
  max-height: calc(100vh - 4rem);
  padding: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  color: var(--color-text);
  background: var(--color-surface);
  box-shadow: var(--shadow-dialog);
}

.dialog--wide {
  width: min(46rem, calc(100vw - 2rem));
}

.dialog::backdrop {
  background: var(--backdrop);
}

/* Fenêtre basse : le titre et les boutons restent visibles, le corps défile. */
.dialog__panel {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-height: calc(100vh - 4rem);
  padding: 1.5rem;
}

/* Marge rendue par le retrait : les contours de focus ne sont pas rognés. */
.dialog__body {
  min-height: 0;
  margin: -0.25rem;
  padding: 0.25rem;
  overflow-y: auto;
}

.dialog__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.dialog__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

/* ✕ sans bord, cible de 24 × 24 px au moins (consignes de la maquette de référence). */
.dialog__close {
  display: grid;
  place-items: center;
  min-width: var(--control-sm);
  min-height: var(--control-sm);
  padding: 0;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: var(--color-muted);
  cursor: pointer;
}

.dialog__close:hover {
  color: var(--color-text);
  background: var(--color-chip);
}

.dialog__close:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.dialog__description {
  margin: -0.5rem 0 0;
  color: var(--color-muted);
}

.dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
