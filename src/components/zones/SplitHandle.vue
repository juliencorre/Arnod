<script setup lang="ts">
/**
 * Séparation redimensionnable (F4, F5), selon le motif WAI-ARIA « window
 * splitter » : `role="separator"` focalisable, valeur en pourcentage. Un
 * séparateur focalisable est un widget interactif (ARIA 1.2) : le constat
 * SonarCloud Web:S6845 (« tabindex sur un élément non interactif ») est un faux
 * positif.
 *
 * Souris : glisser (la taille des deux côtés s'affiche), double-clic pour des
 * parts égales. Clavier : flèches (ou Alt+Maj+flèches), Début et Fin.
 * Pendant le glisser, `preview` donne la proportion à afficher ; `commit`
 * l'enregistre au relâchement. Les minimums en pixels sont respectés (Z4).
 */
import { computed, ref, useTemplateRef } from "vue";

const props = withDefaults(
  defineProps<{
    /** Séparation `row` (côte à côte, déplacement horizontal) ou `column`. */
    axis: "row" | "column";
    ratio: number;
    label: string;
    /** Taille minimale de chaque côté, en pixels. */
    minFirst?: number;
    minSecond?: number;
    /** Noms des deux côtés dans la bulle de taille (« test », « mock »). */
    names?: [string, string] | null;
  }>(),
  { minFirst: 280, minSecond: 280, names: null },
);
const emit = defineEmits<{ preview: [ratio: number | null]; commit: [ratio: number] }>();

const handle = useTemplateRef<HTMLElement>("handle");
const dragging = ref(false);
const current = ref(props.ratio);
const sizes = ref<[number, number] | null>(null);
const STEP = 0.05;

const valueNow = computed(() => Math.round((dragging.value ? current.value : props.ratio) * 100));

function containerSize(): number {
  const rect = handle.value?.parentElement?.getBoundingClientRect();
  if (!rect) return 0;
  return props.axis === "row" ? rect.width : rect.height;
}

/** Proportion bornée par 10 %-90 % et par les minimums en pixels. */
function clamp(ratio: number): number {
  const total = containerSize();
  let min = 0.1;
  let max = 0.9;
  if (total > 0) {
    min = Math.max(min, props.minFirst / total);
    max = Math.min(max, 1 - props.minSecond / total);
  }
  if (min > max) return 0.5;
  return Math.min(max, Math.max(min, ratio));
}

function showSizes(ratio: number): void {
  const total = containerSize();
  sizes.value = total > 0 ? [Math.round(ratio * total), Math.round((1 - ratio) * total)] : null;
}

function onPointerDown(event: PointerEvent): void {
  if (event.button !== 0) return;
  event.preventDefault();
  handle.value?.setPointerCapture(event.pointerId);
  handle.value?.focus();
  dragging.value = true;
  current.value = props.ratio;
  showSizes(current.value);
}

function onPointerMove(event: PointerEvent): void {
  if (!dragging.value) return;
  const rect = handle.value?.parentElement?.getBoundingClientRect();
  if (!rect) return;
  const total = props.axis === "row" ? rect.width : rect.height;
  if (total <= 0) return;
  const position = props.axis === "row" ? event.clientX - rect.left : event.clientY - rect.top;
  current.value = clamp(position / total);
  showSizes(current.value);
  emit("preview", current.value);
}

function onPointerUp(event: PointerEvent): void {
  if (!dragging.value) return;
  dragging.value = false;
  if (handle.value?.hasPointerCapture(event.pointerId)) {
    handle.value.releasePointerCapture(event.pointerId);
  }
  sizes.value = null;
  emit("preview", null);
  emit("commit", current.value);
}

function commit(ratio: number): void {
  const next = clamp(ratio);
  showSizes(next);
  emit("commit", next);
}

function onKeydown(event: KeyboardEvent): void {
  const back = props.axis === "row" ? "ArrowLeft" : "ArrowUp";
  const forward = props.axis === "row" ? "ArrowRight" : "ArrowDown";
  switch (event.key) {
    case back:
      event.preventDefault();
      commit(props.ratio - STEP);
      break;
    case forward:
      event.preventDefault();
      commit(props.ratio + STEP);
      break;
    case "Home":
      event.preventDefault();
      commit(0);
      break;
    case "End":
      event.preventDefault();
      commit(1);
      break;
    case "Enter":
      event.preventDefault();
      commit(0.5);
      break;
  }
}

const bubble = computed(() => {
  if (!sizes.value) return "";
  const [first, second] = sizes.value;
  return props.names
    ? `${props.names[0]} ${String(first)} px · ${props.names[1]} ${String(second)} px`
    : `${String(first)} px · ${String(second)} px`;
});
</script>

<template>
  <div
    ref="handle"
    class="handle"
    :class="[`handle--${axis}`, { 'handle--dragging': dragging }]"
    role="separator"
    tabindex="0"
    :aria-label="label"
    :aria-orientation="axis === 'row' ? 'vertical' : 'horizontal'"
    :aria-valuenow="valueNow"
    aria-valuemin="10"
    aria-valuemax="90"
    :aria-valuetext="`${valueNow} % · ${100 - valueNow} %`"
    title="Glisser : redimensionner · Double-clic : parts égales · Flèches : au clavier"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
    @dblclick="commit(0.5)"
    @keydown="onKeydown"
    @blur="sizes = null"
  >
    <span class="handle__line" aria-hidden="true" />
    <span v-if="bubble" class="handle__bubble" aria-hidden="true">{{ bubble }}</span>
    <slot />
  </div>
</template>

<style scoped>
.handle {
  position: relative;
  z-index: 2;
  flex: none;
  display: grid;
  place-items: center;
  touch-action: none;
  outline: none;
}

.handle--row {
  width: 7px;
  margin: 0 -3px;
  cursor: col-resize;
}

.handle--column {
  height: 7px;
  margin: -3px 0;
  cursor: row-resize;
}

.handle__line {
  background: var(--color-border);
  transition: background-color 0.12s;
}

.handle--row .handle__line {
  width: 1px;
  height: 100%;
}

.handle--column .handle__line {
  width: 100%;
  height: 1px;
}

.handle:hover .handle__line,
.handle--dragging .handle__line,
.handle:focus-visible .handle__line {
  background: var(--color-accent);
}

.handle--row:hover .handle__line,
.handle--row.handle--dragging .handle__line,
.handle--row:focus-visible .handle__line {
  width: 3px;
}

.handle--column:hover .handle__line,
.handle--column.handle--dragging .handle__line,
.handle--column:focus-visible .handle__line {
  height: 3px;
}

.handle__bubble {
  position: absolute;
  top: 50%;
  left: calc(100% + 0.5rem);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-md);
  font: var(--text-small) var(--font-mono);
  white-space: nowrap;
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
  pointer-events: none;
}

.handle--column .handle__bubble {
  top: calc(100% + 0.5rem);
  left: 50%;
}
</style>
