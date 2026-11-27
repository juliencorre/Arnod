<script setup lang="ts">
/**
 * Onglet d'un test (4.2-a, 4.3-a à 4.3-e, 4.3-g) : le test en liste ou en schéma, ses étapes de
 * natures variées et leur enchaînement, le panneau de l'étape choisie, « Si une étape échoue » et
 * le dernier résultat. Le cœur joue le schéma (R5 à R8) ; ce composant affiche et compte, et
 * enregistre seul le brouillon du schéma (`flow_save`).
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import FlowDiagram from "@/components/flow/FlowDiagram.vue";
import FlowList from "@/components/flow/FlowList.vue";
import StepPanel from "@/components/flow/StepPanel.vue";
import StepPicker from "@/components/flow/StepPicker.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import SaveState from "@/components/ui/SaveState.vue";
import { useAutosave } from "@/composables/useAutosave";
import { KIND_LABELS, PALETTE, flowCounts, paletteLabel } from "@/lib/flow";
import { countLabel, formatAgo } from "@/lib/format";
import { onTablistKeydown } from "@/lib/tablist";
import { useFlowsStore, type FlowMode } from "@/stores/flows";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { OnFailure, StepKind } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const workbench = useWorkbenchStore();
const flows = useFlowsStore();
const toasts = useToastStore();
const key = computed(() => flows.key(props.projectId, props.path));
const view = computed(() => workbench.tests[key.value] ?? null);
const flow = computed(() => flows.drafts[key.value] ?? null);
const parents = computed(() => props.path.split("/").slice(0, -1));
/** Message d'un test illisible ou disparu. */
const error = ref<string | null>(null);
const selected = computed(() => flows.selected[key.value] ?? null);
const selectedStep = computed(() => flow.value?.steps.find((s) => s.id === selected.value) ?? null);
const mode = computed<FlowMode>(() => flows.mode(props.projectId, props.path));

const autosave = useAutosave(
  () => flows.drafts[key.value] ?? null,
  () => flows.save(props.projectId, props.path),
);

async function load(): Promise<void> {
  try {
    // Ce qui attend d'être enregistré passe d'abord (la relecture remplacerait le brouillon).
    await autosave.flush();
    await flows.load(props.projectId, props.path);
    autosave.reset();
    error.value = null;
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
}

// Relu à l'ouverture et à chaque changement de l'arborescence (étape ajoutée, renommée…).
watch(
  () => [props.path, workbench.trees[props.projectId]] as const,
  () => void load(),
  { immediate: true },
);

const run = computed(() => workbench.runs[props.projectId] ?? null);
const items = computed(() => run.value?.items.filter((i) => i.test === props.path));
const end = computed(() => run.value?.ends.find((e) => e.test === props.path) ?? null);
const statuses = computed(() => workbench.statuses[props.projectId] ?? {});

/** Résumé du dernier passage : étapes exécutées, réussies, et quand (statuts des étapes). */
const last = computed(() => {
  const steps = view.value?.steps ?? [];
  const known = steps.map((s) => statuses.value[s.path]).filter((s) => s !== undefined);
  if (known.length === 0) return null;
  const passed = known.filter((s) => s.passed).length;
  const at = Math.max(...known.map((s) => s.at));
  return { passed, total: steps.length, ok: passed === steps.length, at };
});

/** Verdict du test dans le run montré (R4). */
const verdict = computed(() => run.value?.tests.find((t) => t.path === props.path) ?? null);

const running = computed(() => run.value !== null && !run.value.finished && verdict.value !== null);

async function guard(work: () => Promise<unknown>): Promise<void> {
  try {
    await work();
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

const runTest = (): Promise<void> =>
  guard(async () => {
    await autosave.flush();
    await workbench.runFolder(props.projectId, props.path);
  });

/** « + Étape ▾ » ouvert (4.3-e). */
const picking = ref(false);

/** Une requête reprend « + Étape » : nouvelle requête ouverte, curseur dans l'adresse (NT2). */
async function addKind(kind: StepKind): Promise<void> {
  picking.value = false;
  await guard(async () => {
    await autosave.flush();
    if (kind === "request") {
      await workbench.addStep(props.projectId, props.path);
      return;
    }
    await flows.add(props.projectId, props.path, kind, { kind: "end" });
    autosave.reset();
  });
}

function select(id: string): void {
  flows.select(props.projectId, props.path, selected.value === id ? null : id);
}

function setMode(value: FlowMode): void {
  flows.setMode(props.projectId, props.path, value);
}

const onFailure = computed({
  get: () => flow.value?.onFailure ?? "stop",
  set: (value: OnFailure) => {
    flows.edit(props.projectId, props.path, (f) => {
      f.onFailure = value;
    });
  },
});

/** Ctrl+Z : annule la dernière insertion (4.3-b quater). */
function onKeydown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement | null;
  if (target?.closest("input, textarea, select, [contenteditable]")) return;
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.code === "KeyZ") {
    void guard(async () => {
      if (await flows.undoInsert(props.projectId, props.path)) autosave.reset();
    }).then(() => undefined);
    event.preventDefault();
  }
}
const root = ref<HTMLElement | null>(null);
onMounted(() => root.value?.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => {
  root.value?.removeEventListener("keydown", onKeydown);
  void autosave.flush();
});

/** Raccourcis d'ajout sous la liste (4.3-a). */
const QUICK: StepKind[] = PALETTE.flatMap((g) => g.kinds);
</script>

<template>
  <section ref="root" class="test" :aria-label="`Test ${view?.name ?? ''}`">
    <p v-if="error" class="test__error" role="alert">{{ error }}</p>
    <template v-else-if="view && flow">
      <header class="test__head">
        <span class="test__badge">TEST</span>
        <span v-for="parent in parents" :key="parent" class="test__parent"
          >{{ parent }} <span aria-hidden="true">›</span></span
        >
        <h2 class="test__title">{{ view.name }}</h2>
        <span v-if="view.test" class="test__counts">{{ flowCounts(flow) }}</span>
        <SaveState
          v-if="view.test"
          :state="autosave.state.value"
          :error="autosave.error.value"
          @retry="autosave.flush()"
        />
        <span class="test__spacer" />
        <div
          v-if="view.test"
          class="test__modes"
          role="tablist"
          aria-label="Affichage du test"
          @keydown="onTablistKeydown"
        >
          <button
            v-for="m in ['list', 'diagram'] as const"
            :key="m"
            type="button"
            role="tab"
            class="test__mode"
            :aria-selected="mode === m"
            :tabindex="mode === m ? 0 : -1"
            @click="setMode(m)"
          >
            {{ m === "list" ? "Liste" : "Schéma" }}
          </button>
        </div>
        <div v-if="view.test || view.steps.length === 0" class="test__add-menu">
          <BaseButton :aria-expanded="picking" @click="picking = !picking">+ Étape ▾</BaseButton>
          <div v-if="picking" class="test__picker">
            <StepPicker @pick="addKind" @close="picking = false" />
          </div>
        </div>
        <BaseButton variant="primary" :disabled="running" @click="runTest">
          ▶ {{ view.test ? "Exécuter le test" : "Exécuter le dossier" }}
        </BaseButton>
      </header>

      <p class="test__summary">
        <template v-if="view.test">
          {{ countLabel(flow.steps.length, "étape", "étapes") }}, chacune dit où l'on va ensuite
        </template>
        <template v-else>Ce dossier range des dossiers : ce n'est pas un test.</template>
        <template v-if="verdict && verdict.failedStep !== null">
          ·
          <span class="test__ko">
            ✕ dernière exécution en échec à l'étape {{ verdict.failedStep }},
            {{ verdict.passed }}/{{ verdict.steps }} étapes
          </span>
        </template>
        <template v-else-if="end && !end.passed">
          ·
          <span class="test__ko"
            >✕ dernière exécution en échec : {{ end.reason ?? "étape en échec" }}</span
          >
        </template>
        <template v-else-if="verdict?.status === 'passed'">
          ·
          <span class="test__ok">
            ✓ dernière exécution réussie, {{ verdict.passed }}/{{ verdict.steps }} étapes</span
          ><template v-if="run?.finishedAt">, {{ formatAgo(run.finishedAt) }}</template>
        </template>
        <template v-else-if="last">
          ·
          <span :class="last.ok ? 'test__ok' : 'test__ko'">
            {{ last.ok ? "✓ dernière exécution réussie" : "✕ dernière exécution en échec" }},
            {{ last.passed }}/{{ last.total }} étapes</span
          >, {{ formatAgo(last.at) }}
        </template>
        <template v-if="view.test"> · les valeurs extraites passent aux étapes suivantes</template>
      </p>

      <div v-if="view.test" class="test__body" :class="{ 'test__body--panel': selectedStep }">
        <div class="test__main">
          <FlowList
            v-if="mode === 'list'"
            :project-id="projectId"
            :path="path"
            :view="view"
            :flow="flow"
            :items="items"
            :selected="selected"
            @select="select"
          />
          <FlowDiagram
            v-else
            :project-id="projectId"
            :path="path"
            :view="view"
            :flow="flow"
            :items="items"
            :end="end"
            :selected="selected"
            @select="select"
          />
          <div v-if="mode === 'list'" class="test__quick" aria-label="Ajouter une étape">
            <button
              v-for="kind in QUICK"
              :key="kind"
              type="button"
              class="test__link"
              @click="addKind(kind)"
            >
              + {{ kind === "request" ? "Requête" : paletteLabel(kind) }}
            </button>
          </div>
          <p v-if="mode === 'list'" class="test__note">
            Les étapes s'exécutent dans l'ordre ; une condition renvoie vers une autre étape, et
            chaque étape indique où l'on va ensuite. Glisser ⋮⋮ réordonne ; les renvois suivent
            l'étape, pas son numéro.
          </p>
          <label class="test__failure">
            <span>Si une étape échoue</span>
            <select v-model="onFailure" class="test__select">
              <option value="stop">Arrêter le test</option>
              <option value="continue">Continuer (le test reste en échec)</option>
            </select>
          </label>
        </div>
        <StepPanel
          v-if="selectedStep"
          :key="selectedStep.id"
          :project-id="projectId"
          :path="path"
          :view="view"
          :flow="flow"
          :step="selectedStep"
        />
      </div>
      <p v-if="view.steps.length === 0 && !view.test" class="test__add">
        <button type="button" class="test__link" @click="addKind('request')">
          + Ajouter une étape
        </button>
        <span class="test__hint">une nouvelle requête, ouverte aussitôt</span>
      </p>
      <p v-if="selectedStep" class="test__sr" aria-live="polite">
        Étape choisie : {{ KIND_LABELS[selectedStep.kind] }}
      </p>
    </template>
    <p v-else class="test__loading">Chargement…</p>
  </section>
</template>

<style scoped>
.test {
  container-type: inline-size;
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
  padding: 1rem 1.25rem;
  font-size: var(--text-body);
}

.test__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.test__badge {
  padding: 0 0.375rem;
  border-radius: var(--radius-sm);
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
}

.test__parent,
.test__counts {
  color: var(--color-muted);
}

.test__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.test__spacer {
  flex: 1;
}

.test__modes {
  display: flex;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
}

.test__mode {
  min-height: var(--control-md);
  padding: 0 0.75rem;
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.test__mode[aria-selected="true"] {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-accent-soft);
}

.test__mode:focus-visible,
.test__link:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.test__add-menu {
  position: relative;
}

.test__picker {
  position: absolute;
  top: calc(100% + 0.25rem);
  right: 0;
  z-index: 3;
}

.test__summary {
  margin: 0;
  color: var(--color-muted);
}

.test__ok {
  color: var(--color-success);
}

.test__ko {
  color: var(--color-danger);
}

.test__error {
  margin: 0;
  color: var(--color-danger);
}

.test__loading {
  color: var(--color-muted);
}

.test__body {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 0.75rem;
  margin-top: 0.5rem;
}

.test__body--panel {
  grid-template-columns: minmax(0, 1fr) minmax(18rem, 22rem);
}

.test__main {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.test__quick {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem 0.875rem;
  padding-left: 0.75rem;
}

.test__link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.test__note,
.test__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.test__failure {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.test__select {
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-bg);
}

.test__add {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.75rem;
  margin: 0.25rem 0 0;
  padding-left: 0.75rem;
}

.test__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}

@container (max-width: 900px) {
  .test__body--panel {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
