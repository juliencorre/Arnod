<script setup lang="ts">
/**
 * Réponse reçue (V1 à V6) : statut, durées, corps, en-têtes et résultat des vérifications. Les
 * vérifications en échec sont détaillées sous la réponse, ignorables pour ce résultat ou
 * retirables de la requête (`docs/plan-fin-premier-usage.md`, VT1 à VT3). Le corps (format,
 * aperçu, filtre JSONPath) est dans `ResponseBody` (`docs/plan-mvp-a-reponse.md`).
 */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import ResponseBody from "@/components/workbench/ResponseBody.vue";
import SentPanel from "@/components/workbench/SentPanel.vue";
import { formatBytes, formatDuration } from "@/lib/format";
import { useWorkbenchStore } from "@/stores/workbench";
import type { AssertionResult } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const outcome = computed(() => editor.value?.outcome ?? null);
const view = ref<"body" | "headers" | "checks" | "sent">("body");

/** Identifiant du résultat de conformité au contrat lié (`CONTRACT_CHECK`, R8). */
const CONTRACT_CHECK = "contrat";

/** Vérifications en échec qui comptent : ni réussies, ni ignorées (R7). */
const failures = computed(
  () => outcome.value?.assertions.filter((a) => !a.passed && !a.ignored) ?? [],
);
const failed = computed(() => failures.value.length);

/** Libellé d'un résultat : cible de la vérification, ou conformité au contrat lié (R8). */
function labelOf(check: AssertionResult): string {
  if (check.id === CONTRACT_CHECK) return "Conformité au contrat";
  return (
    check.target ||
    (editor.value?.draft?.assertions.find((a) => a.id === check.id)?.target ?? check.id)
  );
}

/** La vérification est encore dans la requête : elle peut en être retirée (R8). */
function removable(check: AssertionResult): boolean {
  return (
    check.id !== CONTRACT_CHECK &&
    (editor.value?.draft?.assertions.some((a) => a.id === check.id) ?? false)
  );
}
</script>

<template>
  <section class="response" aria-label="Réponse" aria-live="polite">
    <slot name="tools" />
    <p v-if="editor?.sending" class="response__info">Envoi en cours…</p>
    <p v-else-if="editor?.sendError" class="response__error" role="alert">
      {{ editor.sendError }}
    </p>
    <p v-else-if="!outcome" class="response__info">Envoyez la requête pour voir la réponse.</p>
    <template v-else>
      <div class="response__summary">
        <span
          class="response__status"
          :class="outcome.response.status < 400 ? 'response__status--ok' : 'response__status--ko'"
        >
          {{ outcome.response.status }} {{ outcome.response.statusText }}
        </span>
        <span>{{ formatDuration(outcome.response.totalMs) }}</span>
        <span>{{ formatBytes(outcome.response.sizeBytes) }}</span>
        <span
          class="response__verdict"
          :class="outcome.passed ? 'response__verdict--ok' : 'response__verdict--ko'"
        >
          {{ outcome.passed ? "Réussie" : `Échec · ${failed} vérification(s)` }}
        </span>
        <span v-if="outcome.slow" class="response__slow">Lente</span>
      </div>
      <p class="response__timeline">
        Attente {{ formatDuration(outcome.response.waitMs) }} · Réception
        {{ formatDuration(outcome.response.receiveMs) }}
      </p>
      <div role="tablist" aria-label="Contenu de la réponse" class="response__tabs">
        <button
          v-for="[id, label] in [
            ['body', 'Corps'],
            ['headers', `En-têtes (${outcome.response.headers.length})`],
            ['checks', `Vérifications (${outcome.assertions.length})`],
            ['sent', 'Envoyé'],
          ] as const"
          :key="id"
          type="button"
          role="tab"
          class="response__tab"
          :aria-selected="view === id"
          @click="view = id"
        >
          {{ label }}
        </button>
      </div>
      <!-- Corps : format, aperçu, actions et filtre JSONPath (docs/plan-mvp-a-reponse.md). -->
      <div v-if="view === 'body'">
        <ResponseBody :editor-key="editorKey" />
      </div>
      <dl v-else-if="view === 'headers'" class="response__headers">
        <template v-for="[name, value] in outcome.response.headers" :key="name + value">
          <dt>{{ name }}</dt>
          <dd>{{ value }}</dd>
        </template>
      </dl>
      <!-- Lot E (5.1-h) : ce qui est parti, valeurs tirées et extraites. -->
      <SentPanel v-else-if="view === 'sent'" :editor-key="editorKey" />
      <div v-else>
        <ul class="response__checks">
          <li
            v-for="check in outcome.assertions"
            :key="check.id"
            :class="check.passed ? 'check--ok' : check.ignored ? 'check--ignored' : 'check--ko'"
          >
            <span aria-hidden="true">{{ check.passed ? "✓" : check.ignored ? "–" : "✕" }}</span>
            <span class="check__target">{{ labelOf(check) }}</span>
            <span v-if="check.message" class="check__message">{{ check.message }}</span>
            <span v-if="check.ignored" class="check__message">ignorée pour ce run</span>
          </li>
        </ul>
        <BaseButton @click="workbench.proposeAssertions(editorKey)">
          Proposer des vérifications
        </BaseButton>
      </div>
      <ul v-if="failures.length > 0" class="failures" aria-label="Vérifications en échec">
        <li v-for="check in failures" :key="check.id" class="failure">
          <p class="failure__title"><span aria-hidden="true">✕</span> {{ labelOf(check) }}</p>
          <p v-if="check.message" class="failure__message">{{ check.message }}</p>
          <p class="failure__actions">
            <button
              type="button"
              class="failure__link"
              :aria-label="`Ignorer ${labelOf(check)} pour ce run`"
              @click="workbench.ignoreCheck(editorKey, check.id)"
            >
              Ignorer pour ce run
            </button>
            <template v-if="removable(check)">
              ·
              <button
                type="button"
                class="failure__link"
                :aria-label="`Retirer la vérification ${labelOf(check)}`"
                @click="workbench.removeCheck(editorKey, check.id)"
              >
                Retirer la vérification
              </button>
            </template>
          </p>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.response {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.response__info,
.response__timeline {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.response__error {
  margin: 0;
  padding: 0.625rem 0.75rem;
  border-radius: var(--radius-md);
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.response__summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  font-size: var(--text-body);
}

.response__status {
  font-weight: 700;
}

.response__status--ok,
.response__verdict--ok {
  color: var(--color-success);
}

.response__status--ko,
.response__verdict--ko {
  color: var(--color-danger);
}

.response__slow {
  padding: 0 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  background: var(--color-chip);
}

.response__tabs {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--color-border);
}

.response__tab {
  padding: 0.375rem 0.75rem;
  border: 0;
  border-bottom: 2px solid transparent;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  cursor: pointer;
}

.response__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.response__tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.response__headers {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.25rem 1rem;
  margin: 0;
  font: var(--text-body) var(--font-mono);
}

.response__headers dt {
  font-weight: 600;
}

.response__headers dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.response__checks {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0 0 0.5rem;
  padding: 0;
  list-style: none;
  font-size: var(--text-body);
}

.response__checks li {
  display: flex;
  gap: 0.5rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-md);
}

.check--ok {
  background: var(--color-success-soft);
}

.check--ko {
  background: var(--color-danger-soft);
}

.check--ignored {
  color: var(--color-muted);
}

.failures {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0.25rem 0 0;
  padding: 0;
  list-style: none;
}

.failure {
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  background: var(--color-danger-soft);
}

.failure__title {
  margin: 0;
  font-weight: 700;
  color: var(--color-danger);
}

.failure__message {
  margin: 0.25rem 0 0;
  font: var(--text-body) var(--font-mono);
  color: var(--color-text);
}

.failure__actions {
  margin: 0.375rem 0 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.failure__link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  text-decoration: underline;
  color: var(--color-link);
  cursor: pointer;
}

.failure__link:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.check__target {
  font-family: var(--font-mono);
}

.check__message {
  color: var(--color-muted);
}
</style>
