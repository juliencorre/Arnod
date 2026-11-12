<script setup lang="ts">
/**
 * Vérifications d'une requête (V2) : opérateurs d'OpenCollection et de Bruno, activables une
 * à une ; la conformité au contrat lié est rappelée (R8 du format OpenCollection).
 */
import type { Assertion, AssertionOperator, AssertionResult, Scalar } from "@/types/ipc";

const props = defineProps<{ linked: boolean; results: AssertionResult[] | null }>();
const assertions = defineModel<Assertion[]>({ required: true });

/** Opérateurs d'OpenCollection et de Bruno (R4), dans l'ordre du menu. */
const OPERATORS: { id: AssertionOperator; label: string }[] = [
  { id: "equals", label: "est égal à" },
  { id: "notEquals", label: "est différent de" },
  { id: "contains", label: "contient" },
  { id: "notContains", label: "ne contient pas" },
  { id: "startsWith", label: "commence par" },
  { id: "endsWith", label: "se termine par" },
  { id: "lessThan", label: "est inférieur à" },
  { id: "lessOrEqual", label: "est inférieur ou égal à" },
  { id: "greaterThan", label: "est supérieur à" },
  { id: "greaterOrEqual", label: "est supérieur ou égal à" },
  { id: "between", label: "est entre (min, max)" },
  { id: "in", label: "est parmi (a, b, …)" },
  { id: "notIn", label: "n'est pas parmi (a, b, …)" },
  { id: "length", label: "a pour longueur" },
  { id: "exists", label: "existe" },
  { id: "isUndefined", label: "n'existe pas" },
  { id: "isNull", label: "est null" },
  { id: "isEmpty", label: "est vide" },
  { id: "isNotEmpty", label: "n'est pas vide" },
  { id: "isTruthy", label: "est vrai (truthy)" },
  { id: "isFalsy", label: "est faux (falsy)" },
  { id: "isNumber", label: "est un nombre" },
  { id: "isText", label: "est un texte" },
  { id: "isBoolean", label: "est un booléen" },
  { id: "isArray", label: "est une liste" },
  { id: "isJson", label: "est un objet JSON" },
  { id: "matches", label: "correspond à l'expression (non évaluée)" },
  { id: "notMatches", label: "ne correspond pas à l'expression (non évaluée)" },
];
const NO_VALUE: AssertionOperator[] = [
  "exists",
  "isUndefined",
  "isNull",
  "isEmpty",
  "isNotEmpty",
  "isTruthy",
  "isFalsy",
  "isNumber",
  "isText",
  "isBoolean",
  "isArray",
  "isJson",
];

/** Identifiant du résultat de conformité au contrat lié (`CONTRACT_CHECK`, R8). */
const CONTRACT_CHECK = "contrat";

/**
 * Origine d'une vérification (VT4), connue pendant la session : une vérification relue d'un
 * fichier est « manuelle » (Q2 de `docs/plan-fin-premier-usage.md`).
 */
const ORIGINS: Record<Assertion["origin"], string> = {
  proposed: "proposée",
  manual: "manuelle",
  default: "par défaut",
};

function setOperator(index: number, operator: AssertionOperator): void {
  const assertion = assertions.value[index];
  if (!assertion) return;
  assertion.operator = operator;
  assertion.origin = "manual";
  if (NO_VALUE.includes(operator)) assertion.value = null;
}

function scalarText(value: Scalar | null | undefined): string {
  return value === null || value === undefined ? "" : String(value);
}

/** Convertit la saisie : nombre ou booléen si le texte en a la forme, sinon texte. */
function parseScalar(text: string): Scalar {
  if (text === "true") return true;
  if (text === "false") return false;
  const number = Number(text);
  return text.trim() !== "" && Number.isFinite(number) ? number : text;
}

function setValue(index: number, text: string): void {
  const assertion = assertions.value[index];
  if (assertion) {
    assertion.value = parseScalar(text);
    assertion.origin = "manual";
  }
}

function add(): void {
  assertions.value.push({
    id: `manuelle-${crypto.randomUUID().slice(0, 8)}`,
    target: "status",
    operator: "equals",
    value: 200,
    origin: "manual",
    enabled: true,
  });
}

function remove(index: number): void {
  assertions.value.splice(index, 1);
}

/** Résultat du dernier envoi ; une vérification ignorée pour ce run n'en a pas (R7). */
function resultFor(id: string): boolean | null {
  const found = props.results?.find((a) => a.id === id);
  return found && !found.ignored ? found.passed : null;
}
</script>

<template>
  <div class="stack">
    <p v-if="assertions.length === 0 && !linked" class="hint">
      Sans vérification, la requête réussit si le statut est inférieur à 400.
    </p>
    <p
      v-if="linked"
      class="assertion assertion--contract"
      :class="{
        'assertion--ok': resultFor(CONTRACT_CHECK) === true,
        'assertion--ko': resultFor(CONTRACT_CHECK) === false,
      }"
    >
      La réponse est conforme au contrat (statut prévu, corps conforme au schéma) : vérifié à chaque
      envoi tant que la requête est liée.
    </p>
    <ul class="assertions">
      <li
        v-for="(assertion, index) in assertions"
        :key="assertion.id"
        class="assertion"
        :class="{
          'assertion--ok': assertion.enabled && resultFor(assertion.id) === true,
          'assertion--ko': assertion.enabled && resultFor(assertion.id) === false,
          'assertion--off': !assertion.enabled,
        }"
      >
        <input
          v-model="assertion.enabled"
          type="checkbox"
          class="assertion__check"
          :aria-label="`Activer la vérification ${index + 1}`"
        />
        <input
          v-model="assertion.target"
          type="text"
          spellcheck="false"
          :aria-label="`Cible de la vérification ${index + 1}`"
        />
        <select
          :value="assertion.operator"
          :aria-label="`Opérateur de la vérification ${index + 1}`"
          @change="
            setOperator(index, ($event.target as HTMLSelectElement).value as AssertionOperator)
          "
        >
          <option v-for="o in OPERATORS" :key="o.id" :value="o.id">{{ o.label }}</option>
        </select>
        <input
          v-if="!NO_VALUE.includes(assertion.operator)"
          :value="scalarText(assertion.value)"
          type="text"
          :aria-label="`Valeur de la vérification ${index + 1}`"
          @input="setValue(index, ($event.target as HTMLInputElement).value)"
        />
        <span v-else />
        <span class="assertion__origin">{{ ORIGINS[assertion.origin] }}</span>
        <button
          type="button"
          class="assertion__remove"
          :aria-label="`Supprimer la vérification ${index + 1}`"
          @click="remove(index)"
        >
          ✕
        </button>
      </li>
    </ul>
    <button type="button" class="link" @click="add">+ Ajouter une vérification</button>
  </div>
</template>

<style scoped>
.stack {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
}

.hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.assertions {
  display: flex;
  flex-direction: column;
  align-self: stretch;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.assertion {
  display: grid;
  grid-template-columns: auto 1.4fr 1.2fr 1fr auto auto;
  gap: 0.375rem;
  align-items: center;
  padding: 0.25rem;
  border-radius: 8px;
}

.assertion--ok {
  background: var(--color-success-soft);
}

.assertion--ko {
  background: var(--color-danger-soft);
}

.assertion--off input[type="text"],
.assertion--off select {
  color: var(--color-muted);
}

.assertion--contract {
  display: block;
  align-self: stretch;
  margin: 0;
  padding: 0.5rem 0.625rem;
  font-size: 0.8125rem;
  border: 1px dashed var(--color-border-strong);
}

.assertion input[type="text"],
.assertion select {
  min-width: 0;
  padding: 0.3125rem 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.assertion__origin {
  font-size: 0.75rem;
  color: var(--color-muted);
}

.assertion__remove,
.link {
  border: 0;
  background: none;
  font: inherit;
  cursor: pointer;
}

.assertion__remove {
  font-size: 0.875rem;
  color: var(--color-muted);
}

.link {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
}

:is(.assertion input, .assertion select, .assertion__remove, .link):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
