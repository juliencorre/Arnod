<script setup lang="ts">
/**
 * Ajout d'un contrat (C2 à C4) : texte collé, adresse lue par le cœur, ou
 * contrat vide à partir d'un modèle. L'erreur du cœur (texte qui n'est pas un
 * contrat, adresse injoignable) s'affiche dans le dialogue.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import type { ContractDialogMode } from "@/stores/contracts";
import type { ContractSource } from "@/types/ipc";

const props = defineProps<{
  mode: ContractDialogMode | null;
  /** Erreur renvoyée par le cœur. */
  error: string | null;
  busy: boolean;
}>();
const emit = defineEmits<{ close: []; submit: [source: ContractSource] }>();

const fieldId = useId();
const errorId = useId();
const hintId = useId();
const value = ref("");

watch(
  () => props.mode,
  (mode) => {
    if (mode !== null) value.value = "";
  },
);

const copy = computed(() => {
  switch (props.mode) {
    case "url":
      return {
        title: "Contrat depuis une adresse",
        label: "Adresse du contrat",
        hint: "Adresse d'un fichier OpenAPI ou Swagger, en JSON ou en YAML.",
        submit: "Ajouter",
      };
    case "template":
      return {
        title: "Nouveau contrat",
        label: "Nom du service",
        hint: "Un contrat OpenAPI 3.0 minimal est créé, avec une opération d'exemple à compléter.",
        submit: "Créer",
      };
    default:
      return {
        title: "Coller un contrat",
        label: "Contrat OpenAPI ou Swagger",
        hint: "Texte JSON ou YAML : il est enregistré tel quel dans le dossier contracts du projet.",
        submit: "Ajouter",
      };
  }
});

function submit(): void {
  const text = value.value;
  if (text.trim() === "" || props.busy) return;
  if (props.mode === "url") emit("submit", { kind: "url", url: text.trim() });
  else if (props.mode === "template") emit("submit", { kind: "template", name: text });
  else emit("submit", { kind: "text", text });
}
</script>

<template>
  <!-- Un contrat collé se lit mieux en large ; le formulaire suit la largeur de la fenêtre. -->
  <BaseDialog
    :open="mode !== null"
    :title="copy.title"
    :wide="mode === 'text'"
    @close="emit('close')"
  >
    <form class="contract-form" novalidate @submit.prevent="submit">
      <label :for="fieldId" class="contract-form__label">{{ copy.label }}</label>
      <textarea
        v-if="mode === 'text'"
        :id="fieldId"
        v-model="value"
        class="contract-form__text"
        spellcheck="false"
        :aria-invalid="error !== null"
        :aria-describedby="error ? `${hintId} ${errorId}` : hintId"
        placeholder="openapi: 3.0.3"
      />
      <input
        v-else
        :id="fieldId"
        v-model="value"
        class="contract-form__input"
        :type="mode === 'url' ? 'url' : 'text'"
        autocomplete="off"
        spellcheck="false"
        :placeholder="mode === 'url' ? 'https://exemple.fr/openapi.json' : ''"
        :aria-invalid="error !== null"
        :aria-describedby="error ? `${hintId} ${errorId}` : hintId"
      />
      <p :id="hintId" class="contract-form__hint">{{ copy.hint }}</p>
      <p v-if="error" :id="errorId" class="contract-form__error" role="alert">{{ error }}</p>
      <div class="contract-form__actions">
        <BaseButton @click="emit('close')">Annuler</BaseButton>
        <BaseButton variant="primary" type="submit" :disabled="value.trim() === '' || busy">
          {{ copy.submit }}
        </BaseButton>
      </div>
    </form>
  </BaseDialog>
</template>

<style scoped>
.contract-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.contract-form__label {
  font-weight: 600;
}

.contract-form__input,
.contract-form__text {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.contract-form__text {
  min-height: 16rem;
  font: var(--text-body) / 1.5 var(--font-mono);
  resize: vertical;
}

.contract-form__input:focus-visible,
.contract-form__text:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.contract-form__hint {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.contract-form__error {
  margin: 0;
  color: var(--color-danger);
  overflow-wrap: anywhere;
}

.contract-form__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
</style>
