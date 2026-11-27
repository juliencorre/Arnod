<script setup lang="ts">
/**
 * Réponse reçue (V1 à V6) : statut, durées, corps, en-têtes et résultat des vérifications. Les
 * vérifications en échec sont détaillées sous la réponse, ignorables pour ce résultat ou
 * retirables de la requête (`docs/plan-fin-premier-usage.md`, VT1 à VT3). Le corps (format,
 * aperçu, filtre JSONPath) est dans `ResponseBody` (`docs/plan-mvp-a-reponse.md`).
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import HistoryPanel from "@/components/workbench/HistoryPanel.vue";
import ResponseBody from "@/components/workbench/ResponseBody.vue";
import SentPanel from "@/components/workbench/SentPanel.vue";
import TimelinePanel from "@/components/workbench/TimelinePanel.vue";
import NetworkFailurePanel from "@/components/workbench/NetworkFailurePanel.vue";
import { useGuard } from "@/composables/useGuard";
import { durationLabel } from "@/lib/assertions";
import { countLabel, formatBytes, formatDuration } from "@/lib/format";
import { onTablistKeydown } from "@/lib/tablist";
import { useHistoryStore } from "@/stores/history";
import { useInputStore } from "@/stores/input";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { AssertionResult } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const inputs = useInputStore();
const toasts = useToastStore();
const guard = useGuard();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const outcome = computed(() => editor.value?.outcome ?? null);
// Lot K (5.1-c) : la chronologie remplace la ligne « Attente · Réception ».
const view = ref<"body" | "headers" | "checks" | "sent" | "timeline" | "history">("body");

// Version 1 (5.1-c bis) : historique des envois de la requête, relu à chaque réponse reçue.
const histories = useHistoryStore();
const historyCount = computed(() => {
  const current = editor.value;
  return current ? (histories.viewOf(current.projectId, current.path)?.items.length ?? 0) : 0;
});
watch(
  () => [editor.value?.projectId, editor.value?.path, editor.value?.receivedAt?.getTime()] as const,
  ([projectId, path]) => {
    if (projectId !== undefined && path !== undefined)
      histories.load(projectId, path).catch(() => undefined);
  },
  { immediate: true },
);

/** Assertion de durée évaluée, telle qu'écrite, citée sous le total de la chronologie (5.1-c). */
const durationCheck = computed(() => {
  const evaluated = outcome.value?.assertions.find((a) => a.target === "responseTime");
  if (!evaluated) return null;
  const written = editor.value?.draft?.assertions.find((a) => a.id === evaluated.id);
  return written ? durationLabel(written) : evaluated.target;
});

/** Identifiant du résultat de conformité au contrat lié (`CONTRACT_CHECK`, R8). */
const CONTRACT_CHECK = "contrat";

/** Vérifications en échec qui comptent : ni réussies, ni ignorées (R7). */
const failures = computed(
  () => outcome.value?.assertions.filter((a) => !a.passed && !a.ignored) ?? [],
);
const failed = computed(() => failures.value.length);
const passed = computed(() => outcome.value?.assertions.filter((a) => a.passed).length ?? 0);

/** Lot H (3.4-g) : la réponse pose un cookie, gardé pour l'environnement. */
const setsCookie = computed(
  () => outcome.value?.response.headers.some(([n]) => n.toLowerCase() === "set-cookie") ?? false,
);

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

// --- Erreur d'envoi et son action (ER6) -------------------------------------------

/** Variables inconnues de la requête, d'après la dernière analyse de la saisie. */
const unknown = computed(() => inputs.inputs.get(props.editorKey)?.unknown ?? []);
const unknownNames = computed(() => unknown.value.map((v) => v.name).join(", "));
const environment = computed(() =>
  editor.value ? workbench.activeEnvironment(editor.value.projectId) : null,
);
/** « Environnements… » : la page des environnements du projet (3.3-a, AM43). */
function manageEnvironments(): void {
  if (editor.value) void workbench.openEnvironments(editor.value.projectId).catch(toasts.showError);
}

async function declareUnknown(): Promise<void> {
  if (!editor.value || unknown.value.length === 0) return;
  const names = unknownNames.value;
  const env = await inputs.declare(editor.value.projectId, unknown.value);
  toasts.show(`✓ ${names} ajoutée${unknown.value.length > 1 ? "s" : ""} à ${env}.`);
}
</script>

<template>
  <section class="response" aria-label="Réponse" aria-live="polite">
    <slot name="tools" />
    <p v-if="editor?.sending" class="response__info">Envoi en cours…</p>
    <!-- Lot H (5.1-i) : échec réseau expliqué, avec l'action qui règle le poste. -->
    <NetworkFailurePanel
      v-else-if="editor?.sendFailure"
      :failure="editor.sendFailure"
      :environment="workbench.activeEnvironment(editor.projectId)"
    />
    <!-- ER6 : l'erreur d'envoi propose son action (5.1-b). -->
    <div v-else-if="editor?.sendError" class="response__error" role="alert">
      <p class="response__error-title"><span aria-hidden="true">✕</span> Requête non envoyée</p>
      <p class="response__error-message">{{ editor.sendError }}</p>
      <p class="response__error-actions">
        <template v-if="unknown.length > 0">
          <BaseButton @click="guard(declareUnknown)">
            Ajouter<span class="visually-hidden"> {{ unknownNames }} à {{ environment }}</span>
          </BaseButton>
          <BaseButton @click="manageEnvironments">Environnements…</BaseButton>
        </template>
        <BaseButton v-else @click="guard(() => workbench.send(editorKey))">Réessayer</BaseButton>
      </p>
      <p v-if="unknown.length > 0" class="response__error-hint">
        « Ajouter » déclare {{ unknownNames }} dans {{ environment }} ; la requête n'est pas
        modifiée.
      </p>
    </div>
    <template v-else-if="!outcome">
      <p class="response__info">Envoyez la requête pour voir la réponse.</p>
      <!-- 5.1-c bis : les envois gardés sur le poste restent consultables après un redémarrage. -->
      <HistoryPanel
        v-if="editor && historyCount > 0"
        :project-id="editor.projectId"
        :path="editor.path"
      />
    </template>
    <template v-else>
      <div class="response__summary">
        <!-- AM28 (5.1-b) : statut en pastille, puis durée, taille ; l'environnement à droite. -->
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
          {{
            outcome.passed ? "Réussie" : `Échec · ${countLabel(failed, "assertion", "assertions")}`
          }}
        </span>
        <span v-if="outcome.slow" class="response__slow">Lente</span>
        <span v-if="outcome.sent" class="response__env" title="Environnement de l'envoi">{{
          outcome.sent.environment
        }}</span>
        <!-- 5.1-c bis : l'horloge mène à l'historique des envois. -->
        <button
          type="button"
          class="response__clock"
          :class="{ 'response__clock--alone': !outcome.sent }"
          aria-label="Historique des envois"
          title="Historique des envois"
          @click="view = 'history'"
        >
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" stroke-width="1.5" />
            <path d="M8 4.5V8l2.5 1.5" fill="none" stroke="currentColor" stroke-width="1.5" />
          </svg>
        </button>
      </div>
      <div
        role="tablist"
        aria-label="Contenu de la réponse"
        class="response__tabs"
        @keydown="onTablistKeydown"
      >
        <button
          v-for="[id, label, count] in [
            ['body', 'Corps', 0],
            ['headers', 'En-têtes', outcome.response.headers.length],
            ['checks', 'Assertions', outcome.assertions.length],
            ['sent', 'Envoyé', 0],
            ['timeline', 'Chronologie', 0],
            ['history', 'Historique', historyCount],
          ] as const"
          :key="id"
          type="button"
          role="tab"
          class="response__tab"
          :aria-selected="view === id"
          :tabindex="view === id ? 0 : -1"
          @click="view = id"
        >
          {{ label }}
          <span v-if="count" class="response__badge">{{ count }}</span>
        </button>
      </div>
      <!-- Corps : format, aperçu, actions et filtre JSONPath (docs/plan-mvp-a-reponse.md). -->
      <div v-if="view === 'body'">
        <ResponseBody :editor-key="editorKey" />
      </div>
      <!-- La note des cookies reste dans la branche des en-têtes : un `v-if` à part couperait la
           chaîne et ferait paraître la liste des assertions sous Corps et En-têtes. -->
      <template v-else-if="view === 'headers'">
        <dl class="response__headers">
          <template v-for="[name, value] in outcome.response.headers" :key="name + value">
            <dt>{{ name }}</dt>
            <dd>
              {{ value }}
              <!-- Lot H (3.4-g) : le cookie reçu est gardé pour l'environnement. -->
              <span v-if="name.toLowerCase() === 'set-cookie'" class="response__kept">gardé</span>
            </dd>
          </template>
        </dl>
        <p v-if="setsCookie" class="response__info">
          Les cookies reçus rejoignent ceux de l'environnement : ils seront renvoyés aux requêtes
          suivantes vers ce domaine.
        </p>
      </template>
      <!-- Lot E (5.1-h) : ce qui est parti, valeurs tirées et extraites. -->
      <SentPanel v-else-if="view === 'sent'" :editor-key="editorKey" />
      <HistoryPanel
        v-else-if="view === 'history' && editor"
        :project-id="editor.projectId"
        :path="editor.path"
      />
      <TimelinePanel
        v-else-if="view === 'timeline'"
        :timeline="outcome.response.timeline"
        :duration-check="durationCheck"
      />
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
            <span v-if="check.ignored" class="check__message">ignorée pour ce résultat</span>
          </li>
        </ul>
        <BaseButton @click="guard(() => workbench.proposeAssertions(editorKey))">
          Proposer des assertions
        </BaseButton>
      </div>
      <!-- Un échec est affiché une fois (DE10) : dans ce panneau dépliable, sauf sous l'onglet
           Assertions, dont la liste le montre déjà. -->
      <details
        v-if="failures.length > 0 && view !== 'checks' && view !== 'history'"
        class="failures"
        open
      >
        <summary class="failures__summary">
          <span>{{ countLabel(failed, "assertion en échec", "assertions en échec") }}</span>
          <span v-if="passed > 0" class="failures__passed">
            {{ countLabel(passed, "réussie", "réussies") }}
          </span>
        </summary>
        <ul class="failures__list" aria-label="Assertions en échec">
          <li v-for="check in failures" :key="check.id" class="failure">
            <p class="failure__title"><span aria-hidden="true">✕</span> {{ labelOf(check) }}</p>
            <p v-if="check.message" class="failure__message">{{ check.message }}</p>
            <p class="failure__actions">
              <button
                type="button"
                class="failure__link"
                :aria-label="`Ignorer ${labelOf(check)} pour ce résultat`"
                @click="guard(() => workbench.ignoreCheck(editorKey, check.id))"
              >
                Ignorer pour ce résultat
              </button>
              <button
                v-if="removable(check)"
                type="button"
                class="failure__link"
                :aria-label="`Retirer l'assertion ${labelOf(check)}`"
                @click="guard(() => workbench.removeCheck(editorKey, check.id))"
              >
                Retirer l'assertion
              </button>
            </p>
          </li>
        </ul>
      </details>
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

.response__info {
  margin: 0;
  color: var(--color-muted);
}

.response__error {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding: 0.625rem 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-danger-soft);
}

.response__error p {
  margin: 0;
}

.response__error-title {
  font-weight: 600;
  color: var(--color-danger);
}

.response__error-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.response__error-hint {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.response__summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  font-size: var(--text-body);
}

.response__status {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-pill);
  font: 600 var(--text-small) var(--font-mono);
}

.response__status--ok {
  color: var(--color-success);
  background: var(--color-success-soft);
}

.response__status--ko {
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.response__verdict--ok {
  color: var(--color-success);
}

.response__verdict--ko {
  color: var(--color-danger);
}

.response__env {
  margin-left: auto;
  color: var(--color-muted);
}

.response__clock {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: var(--radius-md);
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.response__clock--alone {
  margin-left: auto;
}

.response__clock:hover {
  color: var(--color-text);
  background-color: var(--color-sidebar);
}

.response__clock:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.response__slow {
  padding: 0 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  background: var(--color-chip);
}

/* Une rangée d'onglets, qui défile s'il le faut (DE3). */
.response__tabs {
  display: flex;
  flex-wrap: nowrap;
  gap: 0.25rem;
  overflow-x: auto;
  scrollbar-width: none;
  border-bottom: 1px solid var(--color-border);
}

.response__tabs::-webkit-scrollbar {
  display: none;
}

/* Compteur en pastille, comme dans l'éditeur (DE7). */
.response__badge {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-caption);
  background: var(--color-chip);
}

.response__tab {
  flex: none;
  padding: 0.375rem 0.375rem;
  white-space: nowrap;
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

.response__kept {
  margin-left: 0.375rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-caption);
  font-weight: 600;
  color: var(--color-link);
  background: var(--color-accent-soft);
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

/* Échecs du résultat, en un panneau dépliable (DE10). */
.failures {
  margin: 0.25rem 0 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.failures__summary {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  font-weight: 600;
  cursor: pointer;
}

.failures__summary:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.failures__passed {
  font-weight: 400;
  color: var(--color-muted);
}

.failures__list {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.failure {
  padding: 0.625rem 0.75rem;
  border-left: 3px solid var(--color-danger);
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
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  margin: 0.375rem 0 0;
  font-size: var(--text-body);
}

/* Action textuelle : couleur de lien, graisse 600, sans soulignement (ER3, 4.2-b). */
.failure__link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
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
