<script setup lang="ts">
/**
 * Avant / Après d'un dossier (4.7-c, 4.7-c bis) : « Avant le dossier » et « Avant chaque test »
 * en tête de la vue, « Après chaque test » et « Après le dossier » à la fin ; l'ordre de la liste
 * est l'ordre d'exécution. Chaque phase a ses étapes (requête, test existant, script, variable,
 * template) et leur dernier résultat. Le cœur les écrit (`hooks_*`) et les joue (R13, R14).
 */
import { computed, ref, watch } from "vue";

import StepPicker from "@/components/flow/StepPicker.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import RowRemoveButton from "@/components/ui/RowRemoveButton.vue";
import { useGuard } from "@/composables/useGuard";
import { KIND_LABELS, runMark, stepName, stepSummary } from "@/lib/flow";
import { useFlowsStore } from "@/stores/flows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { FlowStep, HookPhase, RunItem, StepKind } from "@/types/ipc";

const props = defineProps<{
  projectId: string;
  path: string;
  /** Phases montrées ici : celles d'avant (en tête) ou d'après (à la fin). */
  side: "before" | "after";
  /** Nombre de tests directs, pour « avant chacun des 4 tests ». */
  tests: number;
}>();

const flows = useFlowsStore();
const workbench = useWorkbenchStore();
const guard = useGuard();
const view = computed(() => flows.hooks[flows.folderKey(props.projectId, props.path)] ?? null);
const PHASES = computed<HookPhase[]>(() =>
  props.side === "before" ? ["beforeAll", "beforeEach"] : ["afterEach", "afterAll"],
);
const TITLES: Record<HookPhase, string> = {
  beforeAll: "Avant le dossier",
  beforeEach: "Avant chaque test",
  afterEach: "Après chaque test",
  afterAll: "Après le dossier",
};

function hint(phase: HookPhase): string {
  const name = props.path.split("/").at(-1) ?? props.path;
  const tests = `${String(props.tests)} test${props.tests > 1 ? "s" : ""}`;
  switch (phase) {
    case "beforeAll":
      return `une fois, au début de ${name}`;
    case "beforeEach":
      return `avant chacun des ${tests} de ${name} ; les sous-dossiers ont les leurs`;
    case "afterEach":
      return `après chacun des ${tests}, même en échec`;
    case "afterAll":
      return `une fois, à la fin de ${name}, même en cas d'échec`;
  }
}

watch(
  () => [props.projectId, props.path, workbench.trees[props.projectId]] as const,
  () => {
    if (props.side === "before")
      void flows.loadHooks(props.projectId, props.path).catch(() => undefined);
  },
  { immediate: true },
);

/** Phase dont le choix de nature est ouvert. */
const picking = ref<HookPhase | null>(null);
/** Natures d'une étape d'Avant / Après. */
const HOOK_KINDS: StepKind[] = ["request", "test", "script", "variable", "template"];

function add(phase: HookPhase, kind: StepKind): void {
  picking.value = null;
  if (!HOOK_KINDS.includes(kind)) return;
  guard(() => flows.addHook(props.projectId, props.path, phase, kind));
}

/** Ligne du run d'une étape d'Avant / Après. */
function item(phase: HookPhase, step: FlowStep): RunItem | null {
  return (
    workbench.runs[props.projectId]?.items.find(
      (i) => i.hook === phase && i.folder === props.path && i.stepId === step.id,
    ) ?? null
  );
}

function result(phase: HookPhase, step: FlowStep): { text: string; tone: string } {
  const found = item(phase, step);
  if (found === null) return { text: "—", tone: "none" };
  if (found.status === "skipped") return { text: "— Non joué", tone: "none" };
  const mark = runMark(found);
  const label =
    found.status === "passed"
      ? "✓ Réussi"
      : found.status === "failed" || found.status === "error"
        ? "✕ Échec"
        : (mark?.text ?? "—");
  return { text: label, tone: mark?.tone ?? "none" };
}

function detail(phase: HookPhase, step: FlowStep): string | null {
  const found = item(phase, step);
  if (!found || (found.status !== "failed" && found.status !== "error")) return null;
  const check = found.assertions.find((a) => !a.passed && !a.ignored);
  return found.error ?? (check ? `${check.target} ${check.message ?? ""}` : null);
}

function open(step: FlowStep): void {
  const file = step.kind === "request" ? step.file : null;
  if (file !== null) void workbench.openRequest(props.projectId, `${props.path}/${file}`);
}

/** Étape en cours de modification sur place (script, variable, test, template). */
const editing = ref<string | null>(null);

function update(phase: HookPhase, step: FlowStep): void {
  const current = view.value;
  if (!current) return;
  const hooks = JSON.parse(JSON.stringify(current.hooks)) as typeof current.hooks;
  hooks[phase] = hooks[phase].map((s) => (s.id === step.id ? step : s));
  guard(() => flows.saveHooks(props.projectId, props.path, hooks));
}

function edited(phase: HookPhase, step: FlowStep, change: (s: FlowStep) => void): void {
  const copy = JSON.parse(JSON.stringify(step)) as FlowStep;
  change(copy);
  update(phase, copy);
}

const tests = computed(() => {
  const out: string[] = [];
  const walk = (nodes: NonNullable<(typeof workbench.trees)[string]>["tests"]): void => {
    for (const node of nodes) {
      if (node.kind !== "folder") continue;
      if (node.test) out.push(node.path);
      walk(node.children);
    }
  };
  walk(workbench.trees[props.projectId]?.tests ?? []);
  return out;
});
</script>

<template>
  <div v-if="view" class="hooks">
    <section
      v-for="phase in PHASES"
      :key="phase"
      class="hooks__phase"
      :aria-label="`${TITLES[phase]} de ${path.split('/').at(-1)}`"
    >
      <p class="hooks__title">
        <b>{{ TITLES[phase] }}</b> <span class="hooks__hint">{{ hint(phase) }}</span>
      </p>
      <ul class="hooks__list">
        <li v-for="step in view.hooks[phase]" :key="step.id" class="hooks__row">
          <span class="hooks__kind">{{
            step.kind === "request"
              ? (view.requests.find((r) => r.file === step.file)?.method ?? "REQ")
              : KIND_LABELS[step.kind]
          }}</span>
          <button
            type="button"
            class="hooks__name"
            @click="
              step.kind === 'request'
                ? open(step)
                : (editing = editing === step.id ? null : step.id)
            "
          >
            {{ stepName(step, view.requests) }}
          </button>
          <span class="hooks__content">{{ stepSummary(step, view.requests) }}</span>
          <span :class="`hooks__result hooks__result--${result(phase, step).tone}`">{{
            result(phase, step).text
          }}</span>
          <span v-if="detail(phase, step)" class="hooks__detail">{{ detail(phase, step) }}</span>
          <RowRemoveButton
            :label="`Retirer ${stepName(step, view.requests)} de « ${TITLES[phase]} »`"
            @click="guard(() => flows.removeHook(projectId, path, phase, step.id))"
          />
          <div v-if="editing === step.id" class="hooks__edit">
            <textarea
              v-if="step.kind === 'script'"
              class="hooks__code"
              :aria-label="`JavaScript de ${stepName(step, view.requests)}`"
              :value="step.code"
              @change="
                edited(phase, step, (s) => {
                  if (s.kind === 'script') s.code = ($event.target as HTMLTextAreaElement).value;
                })
              "
            />
            <template v-else-if="step.kind === 'variable'">
              <input
                class="hooks__input"
                aria-label="Variable"
                :value="step.variable"
                @change="
                  edited(phase, step, (s) => {
                    if (s.kind === 'variable')
                      s.variable = ($event.target as HTMLInputElement).value;
                  })
                "
              />
              <input
                class="hooks__input"
                aria-label="Valeur"
                :value="step.value"
                @change="
                  edited(phase, step, (s) => {
                    if (s.kind === 'variable') s.value = ($event.target as HTMLInputElement).value;
                  })
                "
              />
            </template>
            <select
              v-else-if="step.kind === 'test'"
              class="hooks__input"
              aria-label="Test du référentiel"
              :value="step.test"
              @change="
                edited(phase, step, (s) => {
                  if (s.kind === 'test') s.test = ($event.target as HTMLSelectElement).value;
                })
              "
            >
              <option value="" disabled>Choisir un test…</option>
              <option v-for="t in tests" :key="t" :value="t">{{ t }}</option>
            </select>
          </div>
        </li>
      </ul>
      <p class="hooks__add">
        <span v-if="view.hooks[phase].length === 0" class="hooks__hint">Aucune étape ·</span>
        <BaseButton
          variant="link"
          :aria-expanded="picking === phase"
          @click="picking = picking === phase ? null : phase"
          >+ Étape</BaseButton
        >
      </p>
      <div v-if="picking === phase" class="hooks__picker">
        <StepPicker
          :label="`Ajouter une étape à « ${TITLES[phase]} »`"
          @pick="(kind) => add(phase, kind)"
          @close="picking = null"
        />
      </div>
    </section>
  </div>
</template>

<style scoped>
.hooks {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.hooks__phase {
  position: relative;
  padding: 0.5rem 0.75rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: var(--radius-lg);
}

.hooks__title {
  margin: 0;
  font-size: var(--text-small);
  text-transform: none;
}

.hooks__title b {
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.hooks__hint {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.hooks__list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0.25rem 0 0;
  padding: 0;
  list-style: none;
}

.hooks__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--control-md);
}

.hooks__kind {
  flex: none;
  min-width: 4rem;
  font: 700 var(--text-caption) var(--font-mono);
  text-transform: uppercase;
  color: var(--color-muted);
}

.hooks__name {
  padding: 0;
  border: 0;
  font: inherit;
  font-weight: 600;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.hooks__name:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.hooks__content {
  flex: 1;
  min-width: 0;
  font: var(--text-small) var(--font-mono);
  color: var(--color-muted);
}

.hooks__result--ok {
  color: var(--color-success);
}

.hooks__result--ko,
.hooks__detail {
  color: var(--color-danger);
}

.hooks__result--none,
.hooks__result--muted {
  color: var(--color-muted);
}

.hooks__detail {
  font-size: var(--text-small);
}

.hooks__edit {
  display: flex;
  flex-basis: 100%;
  gap: 0.5rem;
}

.hooks__code,
.hooks__input {
  flex: 1;
  min-height: var(--control-md);
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: var(--text-small) var(--font-mono);
  color: var(--color-text);
  background-color: var(--color-bg);
}

.hooks__code {
  min-height: 5rem;
}

.hooks__add {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  margin: 0.25rem 0 0;
}

.hooks__picker {
  position: absolute;
  z-index: 3;
  left: 0.75rem;
}
</style>
