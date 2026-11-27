<script setup lang="ts">
/**
 * Résultat d'une requête à jeu de données (4.7-b) : « Cas 4 sur 12 · 11 ✓ 1 ✗ », un bouton par
 * cas, la réponse du cas choisi dessous. Un cas en échec se lit d'une ligne (« ✕ Cas 4 : status
 * attendu 200, reçu 400 ») ; « Ignorer ce cas pour ce résultat » ne vaut que pour l'affichage,
 * « Retirer le cas » le retire du tableau. Comptage d'affichage seulement.
 */
import { computed } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { useDatasetsStore } from "@/stores/datasets";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const datasets = useDatasetsStore();
const guard = useGuard();
const outcome = computed(() => datasets.outcomes[props.editorKey] ?? null);
const current = computed(() => datasets.selected[props.editorKey] ?? 1);
const ignored = computed(() => datasets.ignored[props.editorKey] ?? []);
const failures = computed(() => datasets.failures(props.editorKey));
const passed = computed(
  () => outcome.value?.cases.filter((c) => c.outcome?.passed === true).length ?? 0,
);
const shown = computed(() => outcome.value?.cases.find((c) => c.number === current.value) ?? null);

/** Raison de l'échec du cas choisi : sa première assertion en échec, ou son erreur. */
const reason = computed(() => {
  const found = shown.value;
  if (!found || found.outcome?.passed === true) return null;
  if (found.error !== null) return found.error;
  const check = found.outcome?.assertions.find((a) => !a.passed && !a.ignored);
  if (!check) return "en échec";
  return check.target === ""
    ? (check.message ?? "en échec")
    : `${check.target} ${check.message ?? ""}`;
});

function choose(number: number): void {
  datasets.select(props.editorKey, number);
  const found = outcome.value?.cases.find((c) => c.number === number);
  const editor = workbench.editors[props.editorKey];
  if (editor && found?.outcome) editor.outcome = found.outcome;
}

async function removeCase(): Promise<void> {
  const editor = workbench.editors[props.editorKey];
  const set = datasets.sets[props.editorKey];
  if (!editor || !set?.link.editable) return;
  const rows = set.rows.filter((row) => row.some((c) => c.trim() !== ""));
  rows.splice(current.value - 1, 1);
  await datasets.save(props.editorKey, editor.projectId, editor.path, {
    columns: set.columns,
    rows,
  });
  datasets.setOutcome(props.editorKey, null);
}
</script>

<template>
  <div v-if="outcome" class="cases" role="group" aria-label="Cas du jeu de données">
    <p class="cases__head">
      <b>Cas {{ current }}</b> sur {{ outcome.cases.length }}
      <span class="cases__ok">{{ passed }} ✓</span>
      <span v-if="failures.length > 0" class="cases__ko">{{ failures.length }} ✕</span>
      <span v-if="ignored.length > 0" class="cases__muted">{{ ignored.length }} ignoré(s)</span>
      <button
        type="button"
        class="cases__arrow"
        aria-label="Cas précédent"
        :disabled="current <= 1"
        @click="choose(current - 1)"
      >
        ‹
      </button>
      <button
        type="button"
        class="cases__arrow"
        aria-label="Cas suivant"
        :disabled="current >= outcome.cases.length"
        @click="choose(current + 1)"
      >
        ›
      </button>
    </p>
    <div class="cases__list">
      <button
        v-for="c in outcome.cases"
        :key="c.number"
        type="button"
        class="cases__chip"
        :class="{
          'cases__chip--ko': c.outcome?.passed !== true && !ignored.includes(c.number),
          'cases__chip--current': c.number === current,
        }"
        :aria-pressed="c.number === current"
        :aria-label="`Cas ${c.number} ${c.outcome?.passed === true ? 'réussi' : 'en échec'}`"
        @click="choose(c.number)"
      >
        {{ c.outcome?.passed === true ? "✓" : "✕" }} {{ c.number }}
      </button>
    </div>
    <p v-if="reason && !ignored.includes(current)" class="cases__failure" role="status">
      <b>✕ Cas {{ current }} : {{ reason }}</b>
      <span class="cases__values">
        {{ shown?.values.map(([k, v]) => `${k} ${v}`).join(" · ") }}
      </span>
      <BaseButton variant="link" @click="datasets.ignore(editorKey, current)"
        >Ignorer ce cas pour ce résultat</BaseButton
      >
      <BaseButton
        v-if="datasets.sets[editorKey]?.link.editable"
        variant="link"
        @click="guard(removeCase)"
        >Retirer le cas</BaseButton
      >
    </p>
  </div>
</template>

<style scoped>
.cases {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.cases__head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
}

.cases__ok {
  color: var(--color-success);
}

.cases__ko,
.cases__failure {
  color: var(--color-danger);
}

.cases__muted,
.cases__values {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.cases__arrow {
  min-width: 1.5rem;
  min-height: 1.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-sm);
  font: inherit;
  background-color: var(--color-surface);
  cursor: pointer;
}

.cases__list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.cases__chip {
  min-width: 2.25rem;
  min-height: 1.5rem;
  padding: 0 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-pill);
  font: var(--text-small) var(--font-mono);
  color: var(--color-success);
  background-color: var(--color-surface);
  cursor: pointer;
}

.cases__chip--ko {
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.cases__chip--current {
  background-color: var(--color-accent-soft);
}

.cases__arrow:focus-visible,
.cases__chip:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.cases__failure {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
  margin: 0;
}
</style>
