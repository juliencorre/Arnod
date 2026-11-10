<script setup lang="ts">
/**
 * Formulaire d'une nature d'étape, rendu d'après ses champs déclaratifs
 * (docs/etude-catalogue-etapes.md, § 2) : une nature nouvelle n'a besoin
 * d'aucun écran nouveau. Choix en groupe radio (flèches), texte, plusieurs
 * lignes. Le cœur revérifie les valeurs à l'envoi (R3).
 */
import { computed, useId } from "vue";

import NaturePill from "@/components/steps/NaturePill.vue";
import { engineLabel, fieldValue, isVisible } from "@/lib/steps";
import type { StepField, StepNature } from "@/types/ipc";

const props = defineProps<{ nature: StepNature }>();
const values = defineModel<Record<string, string>>({ required: true });

const baseId = useId();
const visible = computed(() =>
  props.nature.form.filter((field) => isVisible(props.nature, field, values.value)),
);

function set(key: string, value: string): void {
  values.value = { ...values.value, [key]: value };
}

function fieldId(field: StepField): string {
  return `${baseId}-${field.key}`;
}

/** Flèches dans un groupe radio : la sélection suit le focus. */
function onRadioKey(event: KeyboardEvent, field: StepField): void {
  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
  if (step === undefined) return;
  event.preventDefault();
  const options = field.options;
  const current = options.indexOf(fieldValue(field, values.value));
  const next = options[(current + step + options.length) % options.length];
  if (next === undefined) return;
  set(field.key, next);
  const group = (event.currentTarget as HTMLElement).closest('[role="radiogroup"]');
  void Promise.resolve().then(() => {
    group?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  });
}

const origin = computed(() => {
  const n = props.nature;
  const engine =
    n.engine.kind === "builtin" ? `moteur ${engineLabel(n.engine)} ${n.engine.id}` : "moteur WASM";
  return `Intégrée à Arnod · ${engine} · version ${n.version}. Les vérifications sont proposées d'après la réponse reçue.`;
});
</script>

<template>
  <div class="nature-form" role="group" :aria-label="`Formulaire de la nature ${nature.name}`">
    <div class="nature-form__head">
      <NaturePill :name="nature.name" :family="nature.family" />
      <span class="nature-form__title">{{ nature.title }}</span>
    </div>
    <div class="nature-form__fields">
      <div
        v-for="field in visible"
        :key="field.key"
        class="nature-form__field"
        :class="{ 'nature-form__field--wide': field.kind === 'multiline' }"
      >
        <template v-if="field.kind === 'choice'">
          <span :id="fieldId(field)" class="nature-form__label">{{ field.label }}</span>
          <div role="radiogroup" class="segmented" :aria-labelledby="fieldId(field)">
            <button
              v-for="option in field.options"
              :key="option"
              type="button"
              role="radio"
              class="segmented__option"
              :aria-checked="fieldValue(field, values) === option"
              :tabindex="fieldValue(field, values) === option ? 0 : -1"
              @click="set(field.key, option)"
              @keydown="onRadioKey($event, field)"
            >
              {{ option }}
            </button>
          </div>
        </template>
        <template v-else>
          <label :for="fieldId(field)" class="nature-form__label">{{ field.label }}</label>
          <textarea
            v-if="field.kind === 'multiline'"
            :id="fieldId(field)"
            class="nature-form__input nature-form__input--multiline"
            rows="4"
            spellcheck="false"
            :placeholder="field.placeholder ?? ''"
            :value="fieldValue(field, values)"
            @input="set(field.key, ($event.target as HTMLTextAreaElement).value)"
          />
          <input
            v-else
            :id="fieldId(field)"
            class="nature-form__input"
            type="text"
            spellcheck="false"
            :placeholder="field.placeholder ?? ''"
            :value="fieldValue(field, values)"
            @input="set(field.key, ($event.target as HTMLInputElement).value)"
          />
        </template>
        <span v-if="field.hint" class="nature-form__hint">{{ field.hint }}</span>
      </div>
    </div>
    <p class="nature-form__origin">{{ origin }}</p>
  </div>
</template>

<style scoped>
.nature-form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0.875rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
}

.nature-form__head {
  display: flex;
  align-items: center;
  gap: 0.625rem;
}

.nature-form__title {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.nature-form__fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem 0.875rem;
}

.nature-form__field {
  display: flex;
  flex-direction: column;
  gap: 0.3125rem;
  min-width: 0;
}

.nature-form__field--wide {
  grid-column: 1 / -1;
}

.nature-form__label {
  font-size: 0.75rem;
  color: var(--color-muted);
}

.nature-form__hint {
  font-size: 0.75rem;
  color: var(--color-muted);
}

.nature-form__input {
  box-sizing: border-box;
  width: 100%;
  padding: 0.4375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.nature-form__input--multiline {
  resize: vertical;
}

.nature-form__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.nature-form__origin {
  margin: 0;
  padding-top: 0.625rem;
  border-top: 1px solid var(--color-border);
  font-size: 0.75rem;
  color: var(--color-muted);
}

.segmented {
  display: inline-flex;
  flex-wrap: wrap;
  width: max-content;
  max-width: 100%;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  overflow: hidden;
  background: var(--color-surface);
}

.segmented__option {
  padding: 0.375rem 0.75rem;
  border: 0;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.segmented__option[aria-checked="true"] {
  font-weight: 600;
  color: var(--color-bg);
  background: var(--color-text);
}

.segmented__option:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}
</style>
