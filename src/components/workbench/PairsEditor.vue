<script setup lang="ts">
/**
 * Liste de paires nom-valeur activables, avec leur description (paramètres, en-têtes, champs
 * de formulaire, EP2, EP4). `kind` restreint la liste aux paramètres de requête : les
 * paramètres de chemin, tirés de l'adresse, sont édités à part.
 *
 * Avec `textFormat`, « Éditer en texte » bascule la liste en zone de texte, une ligne par paire
 * (docs/plan-mvp-b-saisie-curl.md, 5.1-b quater) : le cœur lit et écrit le texte, chaque saisie
 * lisible met la liste à jour, « Revenir au tableau » attend un texte sans ligne illisible.
 */
import { computed, nextTick, ref, useId, useTemplateRef } from "vue";

import { useKnownVariables, vVariables } from "@/lib/variables";
import { useInputStore } from "@/stores/input";
import type { Pair, PairsFormat, ParamKind, TextError } from "@/types/ipc";

const props = defineProps<{
  label: string;
  /** Libellé du bouton d'ajout (« + Paramètre », « + En-tête »). */
  addLabel?: string;
  /** Intitulé de la colonne des valeurs. */
  valueTitle?: string;
  kind?: ParamKind;
  /** Titre de la liste, suivi du nombre de lignes (« De cette requête · 2 »). */
  title?: string;
  /** Écriture des lignes en texte ; sans elle, pas d'« Éditer en texte ». */
  textFormat?: PairsFormat;
}>();
const pairs = defineModel<Pair[]>({ required: true });

const input = useInputStore();
const known = useKnownVariables();
const ids = { title: useId(), text: useId(), hint: useId(), errors: useId() };
const textArea = useTemplateRef<HTMLTextAreaElement>("textArea");
const gutter = useTemplateRef<HTMLElement>("gutter");

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

// --- Édition en texte (R11 à R13) ---------------------------------------------------

const textMode = ref(false);
const text = ref("");
const errors = ref<TextError[]>([]);
/** Lignes au moment de passer en texte : leurs descriptions sont gardées par nom (R13). */
let previous: Pair[] = [];
let reading = 0;

const lineCount = computed(() => Math.max(1, text.value.split("\n").length));
const errorLines = computed(() => new Set(errors.value.map((e) => e.line)));
const hint = computed(() =>
  props.textFormat === "header"
    ? "Une ligne par en-tête, « nom: valeur » ; # met une ligne de côté. Ce qu'on colle depuis le navigateur ou un terminal est repris tel quel."
    : "Une ligne par paramètre, « nom=valeur » ; # met une ligne de côté. Ce qu'on colle est repris tel quel.",
);

/** Remplace les lignes affichées par `parsed`, sans toucher aux autres (paramètres de chemin). */
function apply(parsed: Pair[]): void {
  const kind = props.kind ?? "query";
  const others = pairs.value.filter((pair) => props.kind !== undefined && pair.kind !== props.kind);
  pairs.value = [...others, ...parsed.map((pair) => ({ ...pair, kind }))];
}

async function read(): Promise<boolean> {
  const format = props.textFormat;
  if (format === undefined) return true;
  const ticket = ++reading;
  const result = await input.fromText(text.value, format, previous);
  if (ticket !== reading) return false;
  errors.value = result.errors;
  if (result.errors.length === 0) apply(result.pairs);
  return result.errors.length === 0;
}

async function startText(): Promise<void> {
  const format = props.textFormat;
  if (format === undefined) return;
  previous = rows.value.map(({ pair }) => ({ ...pair }));
  text.value = await input.toText(previous, format);
  errors.value = [];
  textMode.value = true;
  await nextTick();
  textArea.value?.focus();
}

async function backToTable(): Promise<void> {
  if (await read()) {
    textMode.value = false;
  } else {
    textArea.value?.focus();
  }
}

function onText(): void {
  void read();
}

function syncGutter(): void {
  if (gutter.value && textArea.value) gutter.value.scrollTop = textArea.value.scrollTop;
}
</script>

<template>
  <div class="pairs" role="group" :aria-label="label">
    <div v-if="title !== undefined || textFormat !== undefined" class="pairs__top">
      <h3 :id="ids.title" class="pairs__title">
        <template v-if="title !== undefined">{{ title }} · </template>
        <template v-if="textMode">texte</template>
        <template v-else>{{ rows.length }}</template>
      </h3>
      <template v-if="textFormat !== undefined">
        <button v-if="textMode" type="button" class="pairs__mode" @click="backToTable">
          Revenir au tableau
        </button>
        <button v-else type="button" class="pairs__mode" @click="startText">Éditer en texte</button>
      </template>
    </div>

    <template v-if="textMode">
      <div class="pairs__text" :class="{ 'pairs__text--error': errors.length > 0 }">
        <div ref="gutter" class="pairs__gutter" aria-hidden="true">
          <span
            v-for="n in lineCount"
            :key="n"
            :class="{ 'pairs__gutter-line--error': errorLines.has(n) }"
          >
            {{ n }}
          </span>
        </div>
        <textarea
          :id="ids.text"
          ref="textArea"
          v-model="text"
          class="pairs__textarea"
          spellcheck="false"
          autocomplete="off"
          wrap="off"
          :rows="Math.max(4, lineCount)"
          :aria-label="`${label}, en texte`"
          :aria-describedby="errors.length > 0 ? `${ids.errors} ${ids.hint}` : ids.hint"
          :aria-invalid="errors.length > 0"
          @input="onText"
          @scroll="syncGutter"
        />
      </div>
      <ul v-if="errors.length > 0" :id="ids.errors" class="pairs__errors" role="alert">
        <li v-for="error in errors" :key="error.line">{{ error.message }}</li>
      </ul>
      <p :id="ids.hint" class="pairs__hint">{{ hint }}</p>
    </template>

    <template v-else>
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
          v-variables="known"
          class="pairs__input pairs__input--mono"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="Nom"
          :aria-label="`Nom, ligne ${String(position + 1)}`"
        />
        <input
          v-model="pair.value"
          v-variables="known"
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
    </template>
  </div>
</template>

<style scoped>
.pairs {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.pairs__top {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.pairs__title {
  margin: 0;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.pairs__mode {
  min-height: var(--control-sm);
  padding: 0 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background-color: var(--color-surface);
  cursor: pointer;
}

.pairs__head,
.pairs__row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) minmax(0, 1fr) auto;
  gap: 0.25rem 0.5rem;
  align-items: center;
}

.pairs__head {
  font-size: var(--text-caption);
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
  font-size: var(--text-small);
}

.pairs__input {
  min-width: 0;
  box-sizing: border-box;
  min-height: var(--control-sm);
  padding: 0.1875rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background: var(--color-surface);
}

.pairs__input--mono {
  font-family: var(--font-mono);
}

.pairs__text {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  overflow: hidden;
  background-color: var(--color-surface);
}

.pairs__text:focus-within {
  border-color: var(--color-accent);
}

.pairs__text--error {
  border-color: var(--color-danger);
}

.pairs__gutter {
  display: flex;
  flex-direction: column;
  padding: 0.5rem 0.5rem 0.5rem 0.625rem;
  overflow: hidden;
  font: var(--text-body) / 1.6 var(--font-mono);
  text-align: right;
  color: var(--color-subtle);
  background-color: var(--color-sidebar);
  user-select: none;
}

.pairs__gutter-line--error {
  font-weight: 600;
  color: var(--color-danger);
}

.pairs__textarea {
  box-sizing: border-box;
  min-width: 0;
  padding: 0.5rem 0.625rem;
  border: 0;
  font: var(--text-body) / 1.6 var(--font-mono);
  color: var(--color-text);
  background: none;
  resize: vertical;
}

.pairs__textarea:focus {
  outline: none;
}

.pairs__errors {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--text-body);
  color: var(--color-danger);
}

.pairs__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.pairs__input:focus-visible,
.pairs__check:focus-visible,
.pairs__remove:focus-visible,
.pairs__add:focus-visible,
.pairs__mode:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.pairs__remove {
  border: 0;
  background: none;
  font-size: var(--text-body);
  line-height: 1;
  color: var(--color-muted);
  cursor: pointer;
}

.pairs__add {
  align-self: flex-start;
  min-height: var(--control-sm);
  padding: 0 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  cursor: pointer;
}
</style>
