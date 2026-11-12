<script setup lang="ts">
/**
 * Corps d'un endpoint (EP3) : type commun, variantes nommées. La variante sélectionnée part à
 * l'envoi (R2) ; « + Variante » copie la sélectionnée.
 */
import { computed, useId } from "vue";

import PairsEditor from "@/components/workbench/PairsEditor.vue";
import type { Body, BodyKind, BodyVariant } from "@/types/ipc";

const body = defineModel<Body>({ required: true });

const KINDS: { id: BodyKind; label: string }[] = [
  { id: "none", label: "Aucun" },
  { id: "json", label: "JSON" },
  { id: "xml", label: "XML" },
  { id: "text", label: "Texte" },
  { id: "form", label: "Formulaire" },
];

const typeId = useId();
const nameId = useId();

const current = computed<BodyVariant | null>(
  () => body.value.variants[body.value.selected] ?? body.value.variants[0] ?? null,
);
/** Variantes montrées dès qu'il y en a plusieurs, ou une nommée. */
const named = computed(
  () => body.value.variants.length > 1 || body.value.variants.some((v) => v.title !== ""),
);
const editable = computed(() => body.value.kind !== "none" && body.value.kind !== "other");

function setKind(kind: BodyKind): void {
  const value = body.value;
  value.kind = kind;
  if (kind !== "none" && value.variants.length === 0) {
    value.variants.push({ title: "", content: "", form: [] });
  }
  if (value.selected >= value.variants.length) value.selected = 0;
}

function addVariant(): void {
  const value = body.value;
  const base = current.value ?? { title: "", content: "", form: [] };
  const first = value.variants[0];
  if (!named.value && first) first.title = "Variante 1";
  value.variants.push({
    title: `Variante ${String(value.variants.length + 1)}`,
    content: base.content,
    form: base.form.map((pair) => ({ ...pair })),
  });
  value.selected = value.variants.length - 1;
}

function removeVariant(): void {
  const value = body.value;
  if (value.variants.length < 2) return;
  value.variants.splice(value.selected, 1);
  value.selected = Math.max(0, value.selected - 1);
}

function select(index: number): void {
  body.value.selected = index;
}
</script>

<template>
  <div class="body">
    <div class="body__bar">
      <label :for="typeId" class="sr-only">Type de corps</label>
      <select
        :id="typeId"
        class="body__type"
        :value="body.kind"
        @change="setKind(($event.target as HTMLSelectElement).value as BodyKind)"
      >
        <option v-for="k in KINDS" :key="k.id" :value="k.id">{{ k.label }}</option>
        <option v-if="body.kind === 'other'" value="other">Autre ({{ body.other }})</option>
      </select>
      <template v-if="editable">
        <div v-if="named" class="body__variants" role="group" aria-label="Variantes du corps">
          <button
            v-for="(variant, index) in body.variants"
            :key="index"
            type="button"
            class="body__variant"
            :aria-pressed="index === body.selected"
            @click="select(index)"
          >
            {{ variant.title || `Variante ${String(index + 1)}` }}
          </button>
        </div>
        <button type="button" class="body__action" @click="addVariant">+ Variante</button>
      </template>
    </div>

    <p v-if="body.kind === 'none'" class="hint">Cette requête part sans corps.</p>
    <p v-else-if="body.kind === 'other'" class="hint">
      Corps « {{ body.other }} » conservé dans le fichier ; Arnod ne l'envoie pas.
    </p>
    <template v-else-if="current">
      <div v-if="named" class="body__name">
        <label :for="nameId">Nom de la variante</label>
        <input :id="nameId" v-model="current.title" type="text" autocomplete="off" />
        <button
          v-if="body.variants.length > 1"
          type="button"
          class="body__action"
          @click="removeVariant"
        >
          Supprimer la variante
        </button>
      </div>
      <PairsEditor
        v-if="body.kind === 'form'"
        v-model="current.form"
        label="Champs du formulaire"
        add-label="+ Champ"
      />
      <textarea
        v-else
        v-model="current.content"
        class="body__content"
        spellcheck="false"
        aria-label="Contenu du corps"
      />
      <p v-if="named" class="hint">La variante sélectionnée part à l'envoi.</p>
    </template>
  </div>
</template>

<style scoped>
.body {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
}

.body__bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.body__type,
.body__name input,
.body__content {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background-color: var(--color-surface);
}

.body__variants {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  padding-left: 0.5rem;
  border-left: 1px solid var(--color-border);
}

.body__variant,
.body__action {
  min-height: var(--control-sm);
  padding: 0 0.75rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-pill);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.body__action {
  border-radius: var(--radius-md);
}

.body__variant[aria-pressed="true"] {
  border-color: var(--color-text);
  background: var(--color-chip);
  font-weight: 600;
}

.body__name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.body__name input {
  flex: 1 1 12rem;
  min-width: 0;
}

.body__content {
  min-height: 11rem;
  font: var(--text-body) var(--font-mono);
  resize: vertical;
}

.hint {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

:is(.body__type, .body__name input, .body__content, .body__variant, .body__action):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}

/* La flèche de la liste déroulante garde sa place (styles.css). */
.body__type {
  padding-right: 1.75rem;
}
</style>
