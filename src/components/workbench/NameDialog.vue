<script setup lang="ts">
/** Fenêtre d'un nom : nouveau dossier, nouvelle requête, renommer. */
import { ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";

const props = defineProps<{
  open: boolean;
  title: string;
  label: string;
  initial: string;
  submitLabel: string;
  /** Erreur renvoyée par le cœur (nom déjà pris...). */
  error: string | null;
}>();
const emit = defineEmits<{ close: []; submit: [name: string] }>();

const inputId = useId();
const errorId = useId();
const name = ref(props.initial);

watch(
  () => props.open,
  (open) => {
    if (open) name.value = props.initial;
  },
);
</script>

<template>
  <BaseDialog :open="open" :title="title" @close="emit('close')">
    <form class="name-form" novalidate @submit.prevent="emit('submit', name)">
      <label :for="inputId" class="name-form__label">{{ label }}</label>
      <input
        :id="inputId"
        v-model="name"
        class="name-form__input"
        type="text"
        autocomplete="off"
        :aria-invalid="error !== null"
        :aria-describedby="error ? errorId : undefined"
      />
      <p v-if="error" :id="errorId" class="name-form__error" role="alert">{{ error }}</p>
      <div class="name-form__actions">
        <BaseButton @click="emit('close')">Annuler</BaseButton>
        <BaseButton variant="primary" type="submit" :disabled="name.trim() === ''">
          {{ submitLabel }}
        </BaseButton>
      </div>
    </form>
  </BaseDialog>
</template>

<style scoped>
.name-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 20rem;
}

.name-form__label {
  font-weight: 600;
}

.name-form__input {
  padding: 0.5rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.name-form__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.name-form__error {
  margin: 0;
  color: var(--color-danger);
}

.name-form__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
</style>
