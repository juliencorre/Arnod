<script setup lang="ts">
/**
 * Fenêtre modale sur l'élément `<dialog>` natif : focus piégé, fermeture par
 * Échap ou clic sur le fond. Le parent garde la main sur `open` : Échap et le
 * clic sur le fond émettent `close` sans fermer d'eux-mêmes.
 */
import { onMounted, useId, useTemplateRef, watch } from "vue";

const props = defineProps<{ open: boolean; title: string; description?: string }>();
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
    :aria-labelledby="titleId"
    :aria-describedby="description ? descriptionId : undefined"
    @cancel.prevent="emit('close')"
    @click="onClick"
  >
    <div class="dialog__panel">
      <h2 :id="titleId" class="dialog__title">{{ title }}</h2>
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
  border-radius: 14px;
  color: var(--color-text);
  background: var(--color-surface);
  box-shadow: var(--shadow-dialog);
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
  font-size: 1.25rem;
  font-weight: 650;
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
