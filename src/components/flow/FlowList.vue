<script setup lang="ts">
/**
 * Un test en liste (4.3-a) : étapes numérotées, nature, données (« extrait », « utilise »),
 * enchaînement (« puis → 2 », « si oui → 4 · sinon → 6 ») et dernier résultat. ⋮⋮ réordonne, au
 * glisser ou par ↑ ↓ ; les renvois suivent l'étape, pas son numéro (R3). Affichage seulement.
 */
import { computed, ref } from "vue";

import {
  KIND_FAMILY,
  KIND_LABELS,
  chain,
  runMark,
  stepItem,
  stepName,
  stepRequest,
  stepSummary,
} from "@/lib/flow";
import { useFlowsStore } from "@/stores/flows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { Flow, FlowStep, RunItem, TestView } from "@/types/ipc";

const props = defineProps<{
  projectId: string;
  path: string;
  view: TestView;
  flow: Flow;
  items: RunItem[] | undefined;
  selected: string | null;
}>();
const emit = defineEmits<{ select: [id: string] }>();

const flows = useFlowsStore();
const workbench = useWorkbenchStore();
const statuses = computed(() => workbench.statuses[props.projectId] ?? {});

function info(step: FlowStep) {
  return props.view.info.find((i) => i.id === step.id) ?? null;
}

/** Dernier résultat : ligne du run montré, sinon statut de sa dernière exécution (requête). */
function mark(step: FlowStep): { text: string; tone: string } | null {
  const fromRun = runMark(stepItem(props.items, props.path, step.id));
  if (fromRun) return fromRun;
  const request = stepRequest(step, props.view.steps);
  if (step.kind !== "request" || !request) return null;
  const status = statuses.value[request.path];
  if (!status) return null;
  return status.passed ? { text: "✓", tone: "ok" } : { text: "✕", tone: "ko" };
}

const dragging = ref<string | null>(null);

function onKeydown(event: KeyboardEvent, index: number, id: string): void {
  if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
  event.preventDefault();
  const to = event.key === "ArrowUp" ? index - 1 : index + 1;
  flows.move(props.projectId, props.path, id, to);
  // Le focus suit l'étape déplacée.
  requestAnimationFrame(() => {
    document.querySelector<HTMLElement>(`[data-step-handle="${CSS.escape(id)}"]`)?.focus();
  });
}

function onDrop(index: number): void {
  if (dragging.value !== null) flows.move(props.projectId, props.path, dragging.value, index);
  dragging.value = null;
}
</script>

<template>
  <ol class="steps" aria-label="Étapes du test">
    <li
      v-for="(step, index) in flow.steps"
      :key="step.id"
      class="step"
      :class="{ 'step--selected': selected === step.id }"
      @dragover.prevent
      @drop.prevent="onDrop(index)"
    >
      <button
        type="button"
        class="step__handle"
        draggable="true"
        :data-step-handle="step.id"
        :aria-label="`Déplacer l'étape ${index + 1} (↑ ↓)`"
        @dragstart="dragging = step.id"
        @dragend="dragging = null"
        @keydown="onKeydown($event, index, step.id)"
      >
        ⋮⋮
      </button>
      <span class="step__number" aria-hidden="true">{{ index + 1 }}</span>
      <span
        v-if="step.kind === 'request'"
        class="step__method"
        :class="`step__method--${(stepRequest(step, view.steps)?.method ?? 'get').toLowerCase()}`"
        >{{ stepRequest(step, view.steps)?.method ?? "?" }}</span
      >
      <span v-else class="step__kind" :class="`step__kind--${KIND_FAMILY[step.kind]}`">{{
        KIND_LABELS[step.kind]
      }}</span>
      <button
        type="button"
        class="step__name"
        :aria-label="`Étape ${index + 1} : ${stepName(step, view.steps)}`"
        :aria-pressed="selected === step.id"
        @click="emit('select', step.id)"
      >
        {{ stepName(step, view.steps) }}
      </button>
      <code class="step__url">{{ stepSummary(step, view.steps) }}</code>
      <span class="step__spacer" />
      <span v-if="info(step)?.inputs.length" class="step__uses"
        >utilise
        {{
          info(step)
            ?.inputs.map((i) => i.name)
            .join(", ")
        }}</span
      >
      <span
        v-for="name in info(step)?.extracts ?? []"
        :key="name"
        class="step__extract"
        :title="`Valeur extraite : ${name}`"
        >→ {{ name }}</span
      >
      <span class="step__chain">
        <span v-for="link in chain(flow, index)" :key="link.label" class="step__link"
          >{{ link.label }} <b>→ {{ link.target }}</b></span
        >
      </span>
      <span v-if="mark(step)" class="step__mark" :class="`step__mark--${mark(step)?.tone}`">{{
        mark(step)?.text
      }}</span>
    </li>
  </ol>
</template>

<style scoped>
.steps {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.step {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  min-height: var(--control-lg);
  padding: 0 0.75rem 0 0.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.step--selected {
  border-color: var(--color-border-strong);
  background: var(--color-accent-soft);
}

.step__handle {
  flex: none;
  display: grid;
  place-items: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: grab;
}

.step__handle:focus-visible,
.step__name:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.step__number {
  display: grid;
  flex: none;
  place-items: center;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: var(--radius-pill);
  font: 600 var(--text-caption) var(--font-mono);
  background: var(--color-chip);
}

.step__method {
  flex: none;
  width: 4.5rem;
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.step__method--get {
  color: var(--color-success);
}

.step__method--post,
.step__method--put,
.step__method--patch {
  color: var(--color-link);
}

.step__method--delete {
  color: var(--color-danger);
}

.step__kind {
  flex: none;
  width: 4.5rem;
  overflow: hidden;
  padding: 0 0.25rem;
  border: 1px solid;
  border-radius: var(--radius-sm);
  font: 600 var(--text-caption) var(--font-mono);
  text-align: center;
  text-overflow: ellipsis;
  text-transform: uppercase;
  white-space: nowrap;
}

.step__kind--jobs {
  border-color: var(--color-family-jobs-border);
  color: var(--color-family-jobs-text);
  background: var(--color-family-jobs-bg);
}

.step__kind--data {
  border-color: var(--color-family-data-border);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
}

.step__kind--code {
  border-color: var(--color-family-code-border);
  color: var(--color-family-code-text);
  background: var(--color-family-code-bg);
}

.step__kind--messages {
  border-color: var(--color-family-messages-border);
  color: var(--color-family-messages-text);
  background: var(--color-family-messages-bg);
}

.step__kind--files {
  border-color: var(--color-family-files-border);
  color: var(--color-family-files-text);
  background: var(--color-family-files-bg);
}

.step__name {
  flex: none;
  min-width: 9rem;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  text-align: left;
  color: var(--color-text);
  cursor: pointer;
}

.step__name:hover {
  text-decoration: underline;
}

.step__url {
  overflow: hidden;
  min-width: 0;
  font: var(--text-small) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-muted);
}

.step__spacer {
  flex: 1;
}

.step__uses {
  flex: none;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.step__extract {
  flex: none;
  padding: 0 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.step__chain {
  display: flex;
  flex: none;
  gap: 0.5rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.step__link b {
  font-weight: 600;
  color: var(--color-text);
}

.step__mark {
  flex: none;
  min-width: 5rem;
  font: 700 var(--text-small) var(--font-mono);
  text-align: right;
}

.step__mark--ok {
  color: var(--color-success);
}

.step__mark--ko {
  color: var(--color-danger);
}

.step__mark--muted {
  font-weight: 400;
  color: var(--color-muted);
}

@container (max-width: 720px) {
  .step__url,
  .step__uses {
    display: none;
  }
}
</style>
