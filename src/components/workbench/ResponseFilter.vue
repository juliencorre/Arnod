<script setup lang="ts">
/**
 * Filtre JSONPath du corps (`docs/plan-mvp-a-reponse.md`, F1 à F4) : le cœur évalue
 * l'expression, les valeurs trouvées sont surlignées dans le corps ; Entrée et Maj+Entrée
 * passent d'une correspondance à l'autre, Échap vide le champ ; « + Vérification » ajoute à la
 * requête la vérification tirée de la correspondance courante.
 */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useResponseStore } from "@/stores/response";
import type { Assertion, AssertionOperator } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const store = useResponseStore();
const view = computed(() => store.views[props.editorKey] ?? null);
const input = ref<HTMLInputElement | null>(null);

const expression = computed({
  get: () => view.value?.expression ?? "",
  set: (value: string) => {
    store.setExpression(props.editorKey, value);
  },
});
const result = computed(() => view.value?.result ?? null);
const current = computed(() => store.currentMatch(props.editorKey));

const count = computed(() => {
  const found = result.value;
  if (!found || found.error !== null) return "";
  if (found.total === 0) return "Aucun résultat";
  return found.total === 1 ? "1 résultat" : `${String(found.total)} résultats`;
});

/** Libellés des opérateurs que le cœur tire d'une valeur (R6). */
const LABELS: Partial<Record<AssertionOperator, string>> = {
  equals: "égal à",
  isNull: "est null",
  length: "a pour longueur",
  exists: "existe",
};

function describe(assertion: Assertion): string {
  const label = LABELS[assertion.operator] ?? assertion.operator;
  const value = assertion.value === undefined ? "" : ` ${JSON.stringify(assertion.value)}`;
  return `${assertion.target} ${label}${value}`;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Enter") {
    event.preventDefault();
    store.move(props.editorKey, event.shiftKey ? -1 : 1);
  } else if (event.key === "Escape" && expression.value !== "") {
    event.preventDefault();
    event.stopPropagation();
    expression.value = "";
  }
}

function add(): void {
  const found = current.value;
  if (found) store.addAssertion(props.editorKey, found);
}

/** Lot E : la valeur trouvée devient une variable (extraction). */
function extract(): void {
  const found = current.value;
  if (found) store.addExtraction(props.editorKey, found);
}

defineExpose({
  /** Place le focus dans le champ du filtre (Ctrl+F). */
  focus: () => {
    input.value?.focus();
    input.value?.select();
  },
});
</script>

<template>
  <div class="filter">
    <div class="filter__row">
      <svg class="filter__icon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M2.5 3h11l-4.3 5.2v4.3l-2.4 1V8.2L2.5 3Z" />
      </svg>
      <input
        ref="input"
        v-model="expression"
        type="search"
        class="filter__input"
        aria-label="Filtre JSONPath"
        placeholder="Filtre JSONPath : $.items[*].sku"
        autocomplete="off"
        spellcheck="false"
        :aria-invalid="result?.error ? 'true' : undefined"
        :aria-describedby="result?.error ? `${editorKey}-filtre-erreur` : undefined"
        @keydown="onKeydown"
      />
      <span class="filter__count" aria-live="polite">{{ count }}</span>
      <template v-if="result && result.matches.length > 1">
        <button
          type="button"
          class="filter__step"
          aria-label="Résultat précédent"
          title="Résultat précédent (Maj+Entrée)"
          @click="store.move(editorKey, -1)"
        >
          ↑
        </button>
        <span class="filter__position">
          {{ (view?.current ?? 0) + 1 }}/{{ result.matches.length }}
        </span>
        <button
          type="button"
          class="filter__step"
          aria-label="Résultat suivant"
          title="Résultat suivant (Entrée)"
          @click="store.move(editorKey, 1)"
        >
          ↓
        </button>
      </template>
    </div>
    <p v-if="result?.error" :id="`${editorKey}-filtre-erreur`" class="filter__error" role="alert">
      {{ result.error }}
    </p>
    <template v-else-if="current">
      <div class="filter__actions">
        <BaseButton :disabled="current.assertion === null" @click="add">
          + Vérification
        </BaseButton>
        <BaseButton :disabled="current.assertion === null" @click="extract">
          Extraire dans une variable
        </BaseButton>
        <span class="filter__target"> sur {{ current.assertion?.target ?? current.path }} </span>
      </div>
      <p class="filter__hint">
        <template v-if="current.assertion">
          « + Vérification » ajoute <code>{{ describe(current.assertion) }}</code> à la requête.
        </template>
        <template v-else>
          Ce chemin ne s'écrit pas en notation pointée : il ne peut pas devenir une vérification.
        </template>
        Entrée passe au résultat suivant ; Ctrl+F ouvre le filtre.
      </p>
    </template>
    <p v-else class="filter__hint">
      Ctrl+F ouvre le filtre ; les valeurs trouvées sont surlignées dans le corps.
    </p>
  </div>
</template>

<style scoped>
.filter {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.625rem 0.75rem;
  border-top: 1px solid var(--color-border);
}

.filter__row,
.filter__actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.filter__icon {
  flex: none;
  fill: none;
  stroke: var(--color-muted);
  stroke-width: 1.4;
  stroke-linejoin: round;
}

.filter__input {
  flex: 1;
  min-width: 0;
  min-height: var(--control-md);
  box-sizing: border-box;
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: var(--text-body) var(--font-mono);
  color: var(--color-text);
  background: var(--color-surface);
}

.filter__input::placeholder {
  color: var(--color-subtle);
}

.filter__input:focus-visible {
  border-color: var(--color-accent);
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.filter__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.filter__count,
.filter__position,
.filter__target {
  flex: none;
  font-size: var(--text-small);
  color: var(--color-muted);
  white-space: nowrap;
}

.filter__target {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
}

.filter__step {
  flex: none;
  width: var(--control-sm);
  height: var(--control-sm);
  padding: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.filter__step:hover {
  border-color: var(--color-border-strong);
}

.filter__step:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.filter__error {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-danger);
}

.filter__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.filter__hint code {
  font-family: var(--font-mono);
  color: var(--color-text);
}
</style>
