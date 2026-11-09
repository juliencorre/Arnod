<script setup lang="ts">
/** Réponse reçue (V1 à V6) : statut, durées, corps, en-têtes et résultat des vérifications. */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { formatBytes, formatDuration } from "@/lib/format";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const outcome = computed(() => editor.value?.outcome ?? null);
const view = ref<"body" | "headers" | "checks">("body");

/** Corps JSON réindenté pour la lecture ; texte brut sinon. */
const prettyBody = computed(() => {
  const body = outcome.value?.response.body ?? "";
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
});

const failed = computed(() => outcome.value?.assertions.filter((a) => !a.passed).length ?? 0);

function messageFor(id: string): string | null {
  return editor.value?.draft?.assertions.find((a) => a.id === id)?.target ?? null;
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
      <p v-if="outcome.response.truncated" class="response__info">
        Le corps dépasse 5 Mo : seul le début est affiché.
      </p>
      <div role="tablist" aria-label="Contenu de la réponse" class="response__tabs">
        <button
          v-for="[id, label] in [
            ['body', 'Corps'],
            ['headers', `En-têtes (${outcome.response.headers.length})`],
            ['checks', `Vérifications (${outcome.assertions.length})`],
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
      <pre v-if="view === 'body'" class="response__body" tabindex="0">{{ prettyBody }}</pre>
      <dl v-else-if="view === 'headers'" class="response__headers">
        <template v-for="[name, value] in outcome.response.headers" :key="name + value">
          <dt>{{ name }}</dt>
          <dd>{{ value }}</dd>
        </template>
      </dl>
      <div v-else>
        <ul class="response__checks">
          <li
            v-for="check in outcome.assertions"
            :key="check.id"
            :class="check.passed ? 'check--ok' : 'check--ko'"
          >
            <span aria-hidden="true">{{ check.passed ? "✓" : "✕" }}</span>
            <span class="check__target">{{ messageFor(check.id) ?? check.id }}</span>
            <span v-if="check.message" class="check__message">{{ check.message }}</span>
          </li>
        </ul>
        <BaseButton @click="workbench.proposeAssertions(editorKey)">
          Proposer des vérifications
        </BaseButton>
      </div>
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
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.response__error {
  margin: 0;
  padding: 0.625rem 0.75rem;
  border-radius: 8px;
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.response__summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.875rem;
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
  border-radius: 999px;
  font-size: 0.75rem;
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
  font-size: 0.8125rem;
  color: var(--color-muted);
  cursor: pointer;
}

.response__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.response__tab:focus-visible,
.response__body:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.response__body {
  max-height: 24rem;
  margin: 0;
  padding: 0.75rem;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font: 0.8125rem var(--font-mono);
  color: var(--color-text);
  background: var(--color-sidebar);
  white-space: pre-wrap;
  word-break: break-word;
}

.response__headers {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.25rem 1rem;
  margin: 0;
  font: 0.8125rem var(--font-mono);
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
  font-size: 0.875rem;
}

.response__checks li {
  display: flex;
  gap: 0.5rem;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
}

.check--ok {
  background: var(--color-success-soft);
}

.check--ko {
  background: var(--color-danger-soft);
}

.check__target {
  font-family: var(--font-mono);
}

.check__message {
  color: var(--color-muted);
}
</style>
