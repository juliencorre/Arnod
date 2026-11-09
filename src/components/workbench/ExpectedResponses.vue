<script setup lang="ts">
/**
 * Réponses attendues d'un endpoint (EP8, EP9) : exemples complets (code, texte, nom,
 * description, en-têtes, corps) enregistrés dans le fichier OpenCollection. Ajouter,
 * dupliquer, supprimer avec « Annuler », modifier, ou reprendre la dernière réponse reçue.
 */
import { computed, ref, useId } from "vue";

import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { Example, ExampleBodyKind } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const examples = computed<Example[]>(() => editor.value?.draft?.examples ?? []);

/** Réponse ouverte en modification, par son index ; `null` : la liste. */
const selected = ref<number | null>(null);
const current = computed(() =>
  selected.value === null ? null : (examples.value[selected.value] ?? null),
);

const BODY_KINDS: { id: ExampleBodyKind; label: string }[] = [
  { id: "none", label: "Aucun" },
  { id: "json", label: "JSON" },
  { id: "xml", label: "XML" },
  { id: "text", label: "Texte" },
  { id: "html", label: "HTML" },
];
const ids = { code: useId(), text: useId(), name: useId(), desc: useId(), body: useId() };

/** Famille d'un code, pour sa pastille. */
function family(status: number | null): string {
  if (status === null) return "none";
  return `${String(Math.floor(status / 100))}xx`;
}

function label(example: Example): string {
  return `${example.status === null ? "" : String(example.status)} ${example.name}`.trim();
}

const last = computed(() => {
  const current = editor.value;
  const response = current?.outcome?.response;
  if (!current || !response) return null;
  const at = current.receivedAt;
  const time = at
    ? ` · ${at.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`
    : "";
  return `${String(response.status)}${time}`;
});

function add(): void {
  examples.value.push({
    name: "Nouvelle réponse",
    description: "",
    status: 200,
    statusText: "OK",
    headers: [{ name: "Content-Type", value: "application/json" }],
    bodyKind: "json",
    body: "{\n}",
  });
  selected.value = examples.value.length - 1;
}

async function addFromLast(): Promise<void> {
  const index = await workbench.addExampleFromResponse(props.editorKey);
  if (index !== null) selected.value = index;
}

function duplicate(index: number): void {
  const source = examples.value[index];
  if (!source) return;
  const copy: Example = {
    ...source,
    name: `${source.name} (copie)`,
    headers: source.headers.map((h) => ({ ...h })),
  };
  examples.value.splice(index + 1, 0, copy);
  if (selected.value !== null) selected.value = index + 1;
}

/** Supprime sans confirmation : la notification propose « Annuler » (R6 des projets). */
function remove(index: number): void {
  const list = examples.value;
  const [removed] = list.splice(index, 1);
  selected.value = null;
  if (!removed) return;
  toasts.show(`Réponse ${label(removed)} supprimée.`, {
    action: {
      label: "Annuler",
      run: () => {
        list.splice(Math.min(index, list.length), 0, removed);
      },
    },
  });
}

function setStatus(text: string): void {
  const example = current.value;
  if (!example) return;
  const status = Number.parseInt(text, 10);
  example.status = Number.isFinite(status) ? status : null;
}
</script>

<template>
  <section class="expected" aria-label="Réponses attendues">
    <template v-if="current === null">
      <p v-if="examples.length === 0" class="hint">Aucune réponse attendue.</p>
      <ul v-else class="expected__list">
        <li v-for="(example, index) in examples" :key="index" class="expected__row">
          <span class="code" :class="`code--${family(example.status)}`">{{
            example.status ?? "—"
          }}</span>
          <button
            type="button"
            class="expected__open"
            :aria-label="`Modifier la réponse ${label(example)}`"
            @click="selected = index"
          >
            <span class="expected__name">{{ example.name || "Sans nom" }}</span>
            <span class="expected__text">{{ example.statusText }}</span>
            <span class="expected__desc">{{ example.description || "Sans description" }}</span>
          </button>
          <button
            type="button"
            class="icon"
            :aria-label="`Dupliquer la réponse ${label(example)}`"
            @click="duplicate(index)"
          >
            ⧉
          </button>
          <button
            type="button"
            class="icon"
            :aria-label="`Supprimer la réponse ${label(example)}`"
            @click="remove(index)"
          >
            ✕
          </button>
          <span class="expected__chevron" aria-hidden="true">›</span>
        </li>
      </ul>
      <div class="expected__actions">
        <button type="button" class="action action--strong" @click="add">+ Réponse</button>
        <button type="button" class="action" :disabled="last === null" @click="addFromLast">
          Depuis la dernière réponse reçue<template v-if="last"> · {{ last }}</template>
        </button>
      </div>
      <p class="hint">
        Chaque réponse attendue est un exemple complet : code, en-têtes et corps. Elle s'enregistre
        dans les exemples du fichier OpenCollection.
      </p>
    </template>

    <template v-else>
      <div class="expected__bar">
        <button type="button" class="action" @click="selected = null">← Réponses attendues</button>
        <span class="expected__spacer" />
        <button type="button" class="action" @click="duplicate(selected ?? 0)">⧉ Dupliquer</button>
        <button type="button" class="action action--danger" @click="remove(selected ?? 0)">
          ✕ Supprimer
        </button>
      </div>
      <div class="grid">
        <div class="field">
          <label :for="ids.code">Code</label>
          <input
            :id="ids.code"
            class="input input--mono"
            :class="`code--${family(current.status)}`"
            type="number"
            min="100"
            max="599"
            :value="current.status ?? ''"
            @input="setStatus(($event.target as HTMLInputElement).value)"
          />
        </div>
        <div class="field">
          <label :for="ids.text">Texte du statut</label>
          <input
            :id="ids.text"
            v-model="current.statusText"
            class="input input--mono"
            type="text"
          />
        </div>
        <div class="field field--wide">
          <label :for="ids.name">Nom</label>
          <input :id="ids.name" v-model="current.name" class="input" type="text" />
        </div>
      </div>
      <div class="field">
        <label :for="ids.desc">Description</label>
        <input :id="ids.desc" v-model="current.description" class="input" type="text" />
      </div>
      <div class="expected__section">
        <h3 class="section">En-têtes de la réponse</h3>
        <button type="button" class="action" @click="current.headers.push({ name: '', value: '' })">
          + En-tête
        </button>
      </div>
      <div v-for="(header, index) in current.headers" :key="index" class="header-row">
        <input
          v-model="header.name"
          class="input input--mono"
          type="text"
          placeholder="Nom"
          :aria-label="`Nom de l'en-tête ${String(index + 1)}`"
        />
        <input
          v-model="header.value"
          class="input input--mono"
          type="text"
          placeholder="Valeur"
          :aria-label="`Valeur de l'en-tête ${String(index + 1)}`"
        />
        <button
          type="button"
          class="icon"
          :aria-label="`Supprimer l'en-tête ${header.name || String(index + 1)}`"
          @click="current.headers.splice(index, 1)"
        >
          ✕
        </button>
      </div>
      <div class="expected__section">
        <label class="section" :for="ids.body">Corps de la réponse</label>
        <select v-model="current.bodyKind" class="input" aria-label="Type du corps de la réponse">
          <option v-for="k in BODY_KINDS" :key="k.id" :value="k.id">{{ k.label }}</option>
          <option v-if="current.bodyKind === 'binary'" value="binary">Binaire</option>
        </select>
      </div>
      <textarea
        v-if="current.bodyKind !== 'none'"
        :id="ids.body"
        v-model="current.body"
        class="input expected__body"
        spellcheck="false"
      />
    </template>
  </section>
</template>

<style scoped>
.expected {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  min-width: 0;
}

.expected__list {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.expected__row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto auto;
  gap: 0.5rem;
  align-items: start;
  padding: 0.625rem 0.25rem;
  border-bottom: 1px solid var(--color-border);
}

.expected__open {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.125rem 0.5rem;
  padding: 0;
  border: 0;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.expected__name {
  font-weight: 600;
}

.expected__text {
  font: 0.75rem var(--font-mono);
  color: var(--color-muted);
}

.expected__desc {
  flex-basis: 100%;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.expected__chevron {
  color: var(--color-muted);
}

.code {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font: 600 0.8125rem var(--font-mono);
  color: var(--color-text);
  background: var(--color-chip);
}

.code--2xx {
  color: var(--color-success);
  background: var(--color-success-soft);
}

.code--4xx {
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.code--5xx {
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.expected__actions,
.expected__bar,
.expected__section {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.expected__section {
  justify-content: space-between;
  margin-top: 0.25rem;
}

.expected__spacer {
  flex: 1;
}

.action {
  padding: 0.375rem 0.75rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.action:disabled {
  color: var(--color-subtle);
  cursor: default;
}

.action--strong {
  font-weight: 600;
  color: var(--color-surface);
  background: var(--color-text);
  border-color: var(--color-text);
}

.action--danger {
  color: var(--color-danger);
}

.icon {
  padding: 0 0.25rem;
  border: 0;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.grid {
  display: grid;
  grid-template-columns: minmax(5rem, 0.6fr) minmax(7rem, 1fr) minmax(8rem, 1.4fr);
  gap: 0.5rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.input {
  min-width: 0;
  padding: 0.4375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.input--mono {
  font-family: var(--font-mono);
}

.header-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr) auto;
  gap: 0.5rem;
  align-items: center;
}

.section {
  margin: 0;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-text);
}

.expected__body {
  min-height: 9rem;
  font: 0.8125rem var(--font-mono);
  resize: vertical;
}

.hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

:is(.expected__open, .icon, .action, .input):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
