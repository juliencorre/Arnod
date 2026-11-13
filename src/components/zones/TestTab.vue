<script setup lang="ts">
/**
 * Onglet d'un test (maquette 4.2-a, docs/plan-mvp-e-tests-etapes.md, TE2, TE3) : le test est un
 * dossier d'étapes, jouées dans l'ordre par ▶ et arrêtées à la première en échec. Chaque étape
 * montre sa méthode, son chemin, ses valeurs extraites, ses vérifications et son dernier
 * résultat (ligne du run montré, sinon statut de sa dernière exécution). Le cœur calcule tout ;
 * ce composant ne fait que compter pour l'affichage.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { countLabel, formatAgo, formatDuration } from "@/lib/format";
import { useToastStore } from "@/stores/toasts";
import { contentKey } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { RunItem, StepView } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const key = computed(() =>
  contentKey({ kind: "test", projectId: props.projectId, path: props.path }),
);
const view = computed(() => workbench.tests[key.value] ?? null);
const parents = computed(() => props.path.split("/").slice(0, -1));
/** Message d'un test illisible ou disparu. */
const error = ref<string | null>(null);

async function load(): Promise<void> {
  try {
    await workbench.loadTest(props.projectId, props.path);
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

/** Ligne du run montré pour une étape, si ce run l'a exécutée. */
function runItem(step: StepView): RunItem | null {
  return workbench.runs[props.projectId]?.items.find((i) => i.path === step.path) ?? null;
}

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
const verdict = computed(
  () => workbench.runs[props.projectId]?.tests.find((t) => t.path === props.path) ?? null,
);

const running = computed(() => {
  const run = workbench.runs[props.projectId];
  return run !== null && run !== undefined && !run.finished && verdict.value !== null;
});

/** Chemin de l'adresse, sans la variable de base (`{{baseUrl}}`). */
function shortUrl(url: string): string {
  return url.replace(/^\{\{\s*baseUrl\s*\}\}/, "") || url;
}

type Mark = { text: string; tone: "ok" | "ko" | "muted"; label: string };

/** Dernier résultat d'une étape : ✓ ou ✕ (jamais la couleur seule), statut HTTP et durée. */
function mark(step: StepView): Mark | null {
  const item = runItem(step);
  if (item) {
    const http = item.httpStatus === null ? "" : ` ${String(item.httpStatus)}`;
    const time = item.durationMs === null ? "" : ` · ${formatDuration(item.durationMs)}`;
    switch (item.status) {
      case "passed":
        return { text: `✓${http}${time}`, tone: "ok", label: "réussie" };
      case "failed":
      case "error":
        return { text: `✕${http}${time}`, tone: "ko", label: "en échec" };
      case "skipped":
        return { text: "non jouée", tone: "muted", label: "non jouée" };
      case "running":
        return { text: "en cours…", tone: "muted", label: "en cours" };
      case "pending":
        return { text: "en attente", tone: "muted", label: "en attente" };
      case "cancelled":
        return { text: "annulée", tone: "muted", label: "annulée" };
    }
  }
  const status = statuses.value[step.path];
  if (!status) return null;
  return status.passed
    ? { text: "✓", tone: "ok", label: "réussie" }
    : { text: "✕", tone: "ko", label: "en échec" };
}

async function guard(work: () => Promise<unknown>): Promise<void> {
  try {
    await work();
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

const addStep = (): Promise<void> => guard(() => workbench.addStep(props.projectId, props.path));
const run = (): Promise<void> => guard(() => workbench.runFolder(props.projectId, props.path));
</script>

<template>
  <section class="test" :aria-label="`Test ${view?.name ?? ''}`">
    <p v-if="error" class="test__error" role="alert">{{ error }}</p>
    <template v-else-if="view">
      <header class="test__head">
        <span class="test__badge">TEST</span>
        <span v-for="parent in parents" :key="parent" class="test__parent"
          >{{ parent }} <span aria-hidden="true">›</span></span
        >
        <h2 class="test__title">{{ view.name }}</h2>
        <span class="test__spacer" />
        <BaseButton v-if="view.test || view.steps.length === 0" @click="addStep">
          + Étape
        </BaseButton>
        <BaseButton variant="primary" :disabled="running" @click="run">
          ▶ {{ view.test ? "Exécuter le test" : "Exécuter le dossier" }}
        </BaseButton>
      </header>

      <p class="test__summary">
        <template v-if="view.test">
          {{ countLabel(view.steps.length, "étape", "étapes") }}, exécutées dans l'ordre
        </template>
        <template v-else>Ce dossier range des dossiers : ce n'est pas un test.</template>
        <template v-if="verdict && verdict.failedStep !== null">
          ·
          <span class="test__ko">
            ✕ dernière exécution en échec à l'étape {{ verdict.failedStep }},
            {{ verdict.passed }}/{{ verdict.steps }} étapes
          </span>
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

      <ol v-if="view.test" class="steps" aria-label="Étapes du test">
        <li v-for="(step, index) in view.steps" :key="step.path" class="step">
          <span class="step__number" aria-hidden="true">{{ index + 1 }}</span>
          <span
            class="step__method"
            :class="`step__method--${(step.method ?? 'get').toLowerCase()}`"
            >{{ step.method ?? "?" }}</span
          >
          <button
            type="button"
            class="step__name"
            :aria-label="`Étape ${index + 1} : ${step.name}`"
            @click="workbench.openRequest(projectId, step.path)"
          >
            {{ step.name }}
          </button>
          <code class="step__url">{{ shortUrl(step.url) }}</code>
          <span class="step__spacer" />
          <span
            v-for="name in step.extractions"
            :key="name"
            class="step__extract"
            :title="`Valeur extraite : ${name}`"
            >→ {{ name }}</span
          >
          <span class="step__checks">{{
            countLabel(step.assertionCount, "vérification", "vérifications")
          }}</span>
          <span
            v-if="mark(step)"
            class="step__mark"
            :class="`step__mark--${mark(step)?.tone}`"
            :aria-label="mark(step)?.label"
            >{{ mark(step)?.text }}</span
          >
        </li>
      </ol>
      <p v-if="view.test || view.steps.length === 0" class="test__add">
        <button type="button" class="test__link" @click="addStep">+ Ajouter une étape</button>
        <span class="test__hint"
          >une nouvelle requête, ouverte aussitôt, à la suite des autres</span
        >
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

.test__parent {
  color: var(--color-muted);
}

.test__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.test__spacer,
.step__spacer {
  flex: 1;
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

.steps {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0.5rem 0 0;
  padding: 0;
  list-style: none;
}

.step {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-height: var(--control-lg);
  padding: 0 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
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
  width: 3.5rem;
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

.step__name:focus-visible,
.test__link:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.step__url {
  overflow: hidden;
  min-width: 0;
  font: var(--text-small) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
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

.step__checks {
  flex: none;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.step__mark {
  flex: none;
  min-width: 6rem;
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

.test__add {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.75rem;
  margin: 0.25rem 0 0;
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

.test__hint {
  color: var(--color-muted);
}

@container (max-width: 640px) {
  .step__url,
  .step__checks {
    display: none;
  }
}
</style>
