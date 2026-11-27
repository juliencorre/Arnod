<script setup lang="ts">
/**
 * Un test en schéma (4.3-b à 4.3-d) : palette des natures, nœuds, liens « oui » / « non », Départ
 * et fins. Au survol ou au focus d'un lien, un « + » ouvre le choix de la nature (4.3-b bis) ;
 * glisser une nature de la palette sur un lien l'insère à cet endroit, dans le vide elle crée une
 * étape libre (4.3-b ter). Après exécution, le chemin suivi est en vert, la branche non empruntée
 * en pointillé, les valeurs passées sur les liens (4.3-d). Le cœur insère et reconnecte (R3).
 */
import { computed, ref } from "vue";

import StepPicker from "@/components/flow/StepPicker.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import {
  KIND_FAMILY,
  KIND_LABELS,
  PALETTE,
  paletteLabel,
  runMark,
  stepItem,
  stepName,
  stepSummary,
} from "@/lib/flow";
import { edgePath, layoutFlow, type LayoutEdge } from "@/lib/flowLayout";
import { useFlowsStore } from "@/stores/flows";
import { useNavigationStore } from "@/stores/navigation";
import { useWorkbenchStore } from "@/stores/workbench";
import { useToastStore } from "@/stores/toasts";
import type { Flow, RunItem, StepKind, StepPlacement, TestEnd, TestView } from "@/types/ipc";

const props = defineProps<{
  projectId: string;
  path: string;
  view: TestView;
  flow: Flow;
  items: RunItem[] | undefined;
  end: TestEnd | null;
  selected: string | null;
}>();
const emit = defineEmits<{ select: [id: string] }>();

const flows = useFlowsStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();

const layout = computed(() => layoutFlow(props.flow));
const zoom = ref(1);
const canvas = ref<HTMLElement | null>(null);

function setZoom(value: number): void {
  zoom.value = Math.min(2, Math.max(0.4, Math.round(value * 10) / 10));
}

/** « Ajuster » : le schéma tient dans la largeur visible. */
function fit(): void {
  const width = canvas.value?.clientWidth ?? 0;
  if (width > 0) setZoom(Math.min(1, (width - 16) / layout.value.width));
}

function nodeItem(id: string): RunItem | null {
  return stepItem(props.items, props.path, id);
}

const played = computed(() =>
  (props.items ?? []).some((i) => i.test === props.path && i.status !== "pending"),
);

/** Un nœud a-t-il été joué ? Le départ toujours, une fin selon la fin du test. */
function reached(id: string): boolean {
  if (id === "start") return played.value;
  if (id === "end") return props.end !== null && props.end.reason === null;
  if (id === "fail") return props.end !== null && props.end.reason === "Fin en échec";
  const item = nodeItem(id);
  return item !== null && item.status !== "pending" && item.status !== "skipped";
}

/** Lien emprunté (vert), non emprunté (pointillé) ou sans exécution. */
function edgeState(edge: LayoutEdge): "taken" | "skipped" | "idle" {
  if (!played.value) return "idle";
  if (!reached(edge.from) || !reached(edge.to)) return "skipped";
  const source = nodeItem(edge.from);
  if (source?.kind === "condition" && edge.label !== null && edge.label !== "répète") {
    return (edge.label === "oui") === (source.branch === true) ? "taken" : "skipped";
  }
  return "taken";
}

/** Valeurs passées sur un lien : celles que l'étape de départ a extraites (4.3-d). */
function edgeValues(edge: LayoutEdge): string[] {
  if (edge.back || edgeState(edge) !== "taken") return [];
  const item = nodeItem(edge.from);
  if (!item || item.kind === "condition") return [];
  return item.extracted
    .filter((e) => e.value !== null)
    .slice(0, 2)
    .map((e) => `${e.name} = ${e.value ?? ""}`);
}

/** Noms de variables extraites (avant exécution), sur le lien qui part de l'étape. */
function edgeNames(edge: LayoutEdge): string[] {
  if (edge.back || played.value || edge.label === "non") return [];
  return props.view.info.find((i) => i.id === edge.from)?.extracts.slice(0, 2) ?? [];
}

function label(id: string): string {
  if (id === "start") return "Départ";
  if (id === "end") return "Fin ✓";
  if (id === "fail") return "Fin en échec";
  const step = props.flow.steps.find((s) => s.id === id);
  return step ? stepName(step, props.view.steps) : id;
}

function placementOf(edge: LayoutEdge): StepPlacement {
  return edge.after === null
    ? { kind: "start" }
    : { kind: "link", after: edge.after, otherwise: edge.otherwise };
}

/** Sélecteur ouvert sur un lien : sa place et son lien. */
const picker = ref<{ x: number; y: number; placement: StepPlacement; label: string } | null>(null);

function openPicker(edge: LayoutEdge): void {
  picker.value = {
    x: edge.mid[0],
    y: edge.mid[1],
    placement: placementOf(edge),
    label: `Insérer entre « ${label(edge.from)} » et « ${label(edge.to)} »`,
  };
}

async function add(kind: StepKind, placement: StepPlacement): Promise<void> {
  picker.value = null;
  try {
    // Le store choisit l'étape insérée (« à configurer ») : la choisir encore la quitterait.
    await flows.add(props.projectId, props.path, kind, placement);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

const dragKind = ref<StepKind | null>(null);

function onDragStart(event: DragEvent, kind: StepKind): void {
  dragKind.value = kind;
  event.dataTransfer?.setData("text/plain", kind);
}

function onEdgeDrop(edge: LayoutEdge): void {
  const kind = dragKind.value;
  dragKind.value = null;
  if (kind) void add(kind, placementOf(edge));
}

function onCanvasDrop(): void {
  const kind = dragKind.value;
  dragKind.value = null;
  if (kind) void add(kind, { kind: "free" });
}

/** Résumé d'un test arrêté (4.3-d) : l'étape et la raison. */
const stopped = computed(() => {
  if (!props.end || props.end.passed) return null;
  const id = props.end.failedStep;
  const item = id === null ? null : nodeItem(id);
  const failure =
    item?.error ??
    item?.assertions.find((a) => !a.passed && !a.ignored)?.message ??
    props.end.reason ??
    null;
  const target = item?.assertions.find((a) => !a.passed && !a.ignored)?.target;
  return {
    step: id === null ? null : label(id),
    detail: failure === null ? null : target === undefined ? failure : `${target} — ${failure}`,
    open: id === null ? null : opener(id),
  };
});

/** « Ouvrir le template » ou « Ouvrir la requête » de l'étape qui a arrêté le test (4.3-d). */
function opener(id: string): { label: string; run: () => void } | null {
  const step = props.flow.steps.find((s) => s.id === id);
  if (step?.kind === "template" && step.template !== "") {
    const template = step.template;
    return {
      label: "Ouvrir le template",
      run: () => void workbench.openFolder(props.projectId, template),
    };
  }
  const file = step?.kind === "request" || step?.kind === "wait" ? step.file : null;
  if (file) {
    return {
      label: "Ouvrir la requête",
      run: () => void workbench.openRequest(props.projectId, `${props.path}/${file}`),
    };
  }
  return null;
}
</script>

<template>
  <div class="diagram">
    <aside class="palette" aria-label="Palette des natures d'étape">
      <div v-for="group in PALETTE" :key="group.title" class="palette__group">
        <p class="palette__title">{{ group.title }}</p>
        <button
          v-for="kind in group.kinds"
          :key="kind"
          type="button"
          class="palette__item"
          :class="`palette__item--${KIND_FAMILY[kind]}`"
          draggable="true"
          :aria-label="`Ajouter une étape ${paletteLabel(kind)} à la fin`"
          @dragstart="onDragStart($event, kind)"
          @dragend="dragKind = null"
          @click="add(kind, { kind: 'end' })"
        >
          <span aria-hidden="true">⋮⋮</span> {{ paletteLabel(kind) }}
        </button>
      </div>
      <!-- Le catalogue des étapes, en lecture seule (plan V4, hors périmètre : synchronisation). -->
      <BaseButton variant="link" @click="navigation.showSteps()"
        >Parcourir le catalogue…</BaseButton
      >
      <p class="palette__hint">
        Glissez sur un lien pour insérer, ou dans le vide pour une étape libre.
      </p>
    </aside>

    <div ref="canvas" class="canvas" @dragover.prevent @drop.prevent="onCanvasDrop">
      <div
        class="board"
        :style="{
          width: `${layout.width}px`,
          height: `${layout.height}px`,
          transform: `scale(${zoom})`,
        }"
      >
        <svg
          class="links"
          :width="layout.width"
          :height="layout.height"
          aria-hidden="true"
          focusable="false"
        >
          <defs>
            <marker
              id="flow-arrow"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" class="links__head" />
            </marker>
          </defs>
          <path
            v-for="edge in layout.edges"
            :key="edge.key"
            :d="edgePath(edge)"
            class="links__path"
            :class="`links__path--${edgeState(edge)}`"
            marker-end="url(#flow-arrow)"
          />
        </svg>

        <template v-for="node in layout.nodes" :key="node.id">
          <div
            v-if="node.kind !== 'step'"
            class="cap"
            :class="`cap--${node.kind}`"
            :style="{ left: `${node.x}px`, top: `${node.y}px`, width: `${node.w}px` }"
          >
            {{
              node.kind === "start" ? "▶ Départ" : node.kind === "end" ? "✓ Fin" : "✕ Fin en échec"
            }}
          </div>
          <button
            v-else-if="flow.steps[node.index ?? 0]"
            type="button"
            class="node"
            :class="[
              `node--${KIND_FAMILY[flow.steps[node.index ?? 0]!.kind]}`,
              {
                'node--selected': selected === node.id,
                'node--skipped': played && !reached(node.id),
              },
            ]"
            :style="{
              left: `${node.x}px`,
              top: `${node.y}px`,
              width: `${node.w}px`,
              height: `${node.h}px`,
            }"
            :aria-label="`Étape ${(node.index ?? 0) + 1} : ${label(node.id)}`"
            :aria-pressed="selected === node.id"
            @click="emit('select', node.id)"
          >
            <span class="node__head">
              <span class="node__kind">{{ KIND_LABELS[flow.steps[node.index ?? 0]!.kind] }}</span>
              <span
                v-if="runMark(nodeItem(node.id))"
                class="node__mark"
                :class="`node__mark--${runMark(nodeItem(node.id))?.tone}`"
                >{{ runMark(nodeItem(node.id))?.text }}</span
              >
            </span>
            <span class="node__name">{{ label(node.id) }}</span>
            <span class="node__summary">{{
              stepSummary(flow.steps[node.index ?? 0]!, view.steps)
            }}</span>
          </button>
        </template>

        <template v-for="edge in layout.edges" :key="`plus-${edge.key}`">
          <span
            v-if="edge.label === 'oui' || edge.label === 'non' || edge.label === 'répète'"
            class="edge__label"
            :style="{
              left: `${edge.points[1]?.[0] ?? edge.mid[0]}px`,
              top: `${(edge.points[1]?.[1] ?? edge.mid[1]) - 18}px`,
            }"
            >{{ edge.label }}</span
          >
          <span
            v-for="(value, n) in [...edgeValues(edge), ...edgeNames(edge)]"
            :key="value"
            class="edge__value"
            :class="{ 'edge__value--played': played }"
            :style="{ left: `${edge.mid[0] + 18}px`, top: `${edge.mid[1] - 10 + n * 18}px` }"
            >{{ value }}</span
          >
          <button
            v-if="!edge.back"
            type="button"
            class="edge__plus"
            :class="{ 'edge__plus--drop': dragKind !== null }"
            :style="{ left: `${edge.mid[0]}px`, top: `${edge.mid[1]}px` }"
            :aria-label="`Insérer une étape entre « ${label(edge.from)} » et « ${label(edge.to)} »`"
            @click.stop="openPicker(edge)"
            @dragover.prevent.stop
            @drop.prevent.stop="onEdgeDrop(edge)"
          >
            {{ dragKind !== null ? "Insérer ici" : "+" }}
          </button>
        </template>

        <div
          v-if="picker"
          class="board__picker"
          :style="{ left: `${picker.x + 16}px`, top: `${picker.y - 12}px` }"
        >
          <StepPicker
            :label="picker.label"
            @pick="(kind) => picker && add(kind, picker.placement)"
            @close="picker = null"
          />
        </div>
      </div>
    </div>

    <div class="diagram__foot">
      <p v-if="stopped" class="diagram__stopped" role="status">
        <b>✕ Arrêtée{{ stopped.step === null ? "" : ` à « ${stopped.step} »` }}</b>
        <span v-if="stopped.detail"> · {{ stopped.detail }}</span>
        <BaseButton v-if="stopped.open" variant="link" @click="stopped.open.run()">{{
          stopped.open.label
        }}</BaseButton>
      </p>
      <p v-else-if="played" class="diagram__legend">
        Chemin suivi en vert, branche non empruntée en pointillé. Les valeurs passées d'une étape à
        l'autre s'affichent sur les liens.
      </p>
      <span class="diagram__spacer" />
      <div class="zoom" role="group" aria-label="Zoom du schéma">
        <button
          type="button"
          class="zoom__button"
          aria-label="Réduire"
          @click="setZoom(zoom - 0.1)"
        >
          −
        </button>
        <span class="zoom__value">{{ Math.round(zoom * 100) }} %</span>
        <button
          type="button"
          class="zoom__button"
          aria-label="Agrandir"
          @click="setZoom(zoom + 0.1)"
        >
          +
        </button>
        <button type="button" class="zoom__fit" @click="fit">Ajuster</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diagram {
  display: grid;
  grid-template-columns: 11rem minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 0.5rem 0.75rem;
  min-height: 24rem;
}

.palette {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  grid-row: 1 / 3;
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-sidebar);
}

.palette__group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.palette__title {
  margin: 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.palette__item {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  min-height: var(--control-sm);
  padding: 0 0.5rem;
  border: 1px solid;
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-small);
  font-weight: 600;
  text-align: left;
  cursor: grab;
}

.palette__item:focus-visible,
.node:focus-visible,
.edge__plus:focus-visible,
.zoom__button:focus-visible,
.zoom__fit:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.palette__item--calls,
.node--calls {
  border-color: var(--color-family-calls-border);
  color: var(--color-family-calls-text);
  background: var(--color-family-calls-bg);
}

.palette__item--jobs,
.node--jobs {
  border-color: var(--color-family-jobs-border);
  color: var(--color-family-jobs-text);
  background: var(--color-family-jobs-bg);
}

.palette__item--data,
.node--data {
  border-color: var(--color-family-data-border);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
}

.palette__item--code,
.node--code {
  border-color: var(--color-family-code-border);
  color: var(--color-family-code-text);
  background: var(--color-family-code-bg);
}

.palette__item--messages,
.node--messages {
  border-color: var(--color-family-messages-border);
  color: var(--color-family-messages-text);
  background: var(--color-family-messages-bg);
}

.palette__item--files,
.node--files {
  border-color: var(--color-family-files-border);
  color: var(--color-family-files-text);
  background: var(--color-family-files-bg);
}

.palette__hint {
  margin: 0.25rem 0 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.canvas {
  overflow: auto;
  min-height: 20rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-bg);
}

.board {
  position: relative;
  transform-origin: 0 0;
}

.links {
  position: absolute;
  inset: 0;
  overflow: visible;
}

.links__path {
  fill: none;
  stroke: var(--color-border-strong);
  stroke-width: 1.5;
}

.links__path--taken {
  stroke: var(--color-success);
  stroke-width: 2.5;
}

.links__path--skipped {
  stroke: var(--color-subtle);
  stroke-dasharray: 5 4;
}

.links__head {
  fill: var(--color-border-strong);
}

.cap {
  position: absolute;
  display: grid;
  place-items: center;
  height: 1.875rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
}

.cap--end {
  border-color: var(--color-success);
  color: var(--color-success);
  background: var(--color-success-soft);
}

.cap--fail {
  border-color: var(--color-danger);
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.node {
  position: absolute;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 0.125rem;
  padding: 0.375rem 0.625rem;
  border: 1px solid;
  border-radius: var(--radius-lg);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.node--selected {
  box-shadow: 0 0 0 2px var(--color-focus);
}

.node--skipped {
  border-style: dashed;
  color: var(--color-muted);
  background: var(--color-surface);
}

.node__head {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  font: 600 var(--text-caption) var(--font-mono);
  text-transform: uppercase;
}

.node__mark {
  text-transform: none;
}

.node__mark--ok {
  color: var(--color-success);
}

.node__mark--ko {
  color: var(--color-danger);
}

.node__mark--muted {
  font-weight: 400;
  color: var(--color-muted);
}

.node__name {
  overflow: hidden;
  font-size: var(--text-body);
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text);
}

.node__summary {
  overflow: hidden;
  font: var(--text-caption) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-muted);
}

.edge__label {
  position: absolute;
  padding: 0 0.25rem;
  font-size: var(--text-caption);
  font-weight: 600;
  color: var(--color-muted);
  background: var(--color-bg);
  transform: translateX(-50%);
}

.edge__value {
  position: absolute;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font: var(--text-caption) var(--font-mono);
  white-space: nowrap;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.edge__value--played {
  color: var(--color-success);
  background: var(--color-success-soft);
}

.edge__plus {
  position: absolute;
  display: grid;
  place-items: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.25rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-pill);
  font: inherit;
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
  opacity: 0;
  transform: translate(-50%, -50%);
  cursor: pointer;
}

.edge__plus:hover,
.edge__plus:focus-visible,
.edge__plus--drop {
  opacity: 1;
}

.edge__plus--drop {
  border-style: dashed;
  color: var(--color-link);
}

.board__picker {
  position: absolute;
  z-index: 2;
}

.diagram__foot {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  grid-column: 2;
}

.diagram__stopped {
  margin: 0;
  color: var(--color-danger);
}

.diagram__legend {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.diagram__spacer {
  flex: 1;
}

.zoom {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.zoom__button,
.zoom__fit {
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0 0.375rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
  cursor: pointer;
}

.zoom__value {
  min-width: 3rem;
  font: var(--text-small) var(--font-mono);
  text-align: center;
}
</style>
