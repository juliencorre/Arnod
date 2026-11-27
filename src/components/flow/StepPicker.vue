<script setup lang="ts">
/**
 * Choix de la nature d'une étape (4.3-e « + Étape ▾ », 4.3-b bis « + » d'un lien) : un champ de
 * recherche et la liste des natures groupées. Motif combobox : le champ garde le focus, ↑ ↓
 * choisissent, Entrée prend la nature choisie, Échap ferme.
 */
import { computed, nextTick, onMounted, ref, useId } from "vue";

import { KIND_FAMILY, KIND_HINTS, PALETTE, paletteLabel } from "@/lib/flow";
import type { StepKind } from "@/types/ipc";

defineProps<{ label?: string }>();
const emit = defineEmits<{ pick: [kind: StepKind]; close: [] }>();

const id = useId();
const input = ref<HTMLInputElement | null>(null);
const query = ref("");
const active = ref(0);

/** Texte sans casse ni accents, pour la recherche. */
function plain(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

const groups = computed(() => {
  const wanted = plain(query.value.trim());
  return PALETTE.map((group) => ({
    title: group.title,
    kinds: group.kinds.filter(
      (kind) =>
        wanted === "" ||
        plain(paletteLabel(kind)).includes(wanted) ||
        plain(KIND_HINTS[kind]).includes(wanted),
    ),
  })).filter((group) => group.kinds.length > 0);
});
const options = computed(() => groups.value.flatMap((g) => g.kinds));
const activeId = computed(() => {
  const kind = options.value[active.value];
  return kind === undefined ? undefined : optionId(kind);
});

function optionId(kind: StepKind): string {
  return `${id}-${kind}`;
}

function onKeydown(event: KeyboardEvent): void {
  const count = options.value.length;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    active.value = count === 0 ? 0 : (active.value + 1) % count;
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    active.value = count === 0 ? 0 : (active.value - 1 + count) % count;
  } else if (event.key === "Enter") {
    event.preventDefault();
    const kind = options.value[active.value];
    if (kind) emit("pick", kind);
  } else if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    emit("close");
  }
}

function onInput(): void {
  active.value = 0;
}

onMounted(() => {
  void nextTick(() => input.value?.focus());
});
</script>

<template>
  <div class="picker" role="dialog" :aria-label="label ?? 'Ajouter une étape'">
    <label :for="`${id}-input`" class="visually-hidden">Nature de l'étape</label>
    <input
      :id="`${id}-input`"
      ref="input"
      v-model="query"
      class="picker__input"
      type="text"
      role="combobox"
      aria-autocomplete="list"
      aria-expanded="true"
      :aria-controls="`${id}-list`"
      :aria-activedescendant="activeId"
      placeholder="Rechercher une nature d'étape…"
      @keydown="onKeydown"
      @input="onInput"
      @blur="emit('close')"
    />
    <div :id="`${id}-list`" class="picker__list" role="listbox" aria-label="Natures d'étape">
      <div v-for="group in groups" :key="group.title" role="group" :aria-label="group.title">
        <p class="picker__group" aria-hidden="true">{{ group.title }}</p>
        <div
          v-for="kind in group.kinds"
          :id="optionId(kind)"
          :key="kind"
          class="picker__option"
          role="option"
          :aria-selected="options[active] === kind"
          @mousedown.prevent="emit('pick', kind)"
          @mouseenter="active = options.indexOf(kind)"
        >
          <span class="picker__kind" :class="`picker__kind--${KIND_FAMILY[kind]}`">{{
            paletteLabel(kind)
          }}</span>
          <span class="picker__hint">{{ KIND_HINTS[kind] }}</span>
          <span v-if="options[active] === kind" class="picker__enter" aria-hidden="true"
            >Entrée</span
          >
        </div>
      </div>
      <p v-if="options.length === 0" class="picker__empty">Aucune nature ne correspond.</p>
    </div>
  </div>
</template>

<style scoped>
.picker {
  display: flex;
  flex-direction: column;
  width: 22rem;
  max-width: calc(100vw - 2rem);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.picker__input {
  min-height: var(--control-lg);
  margin: 0.5rem;
  padding: 0 0.625rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-bg);
}

.picker__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.picker__list {
  overflow-y: auto;
  max-height: 20rem;
  padding: 0 0.25rem 0.5rem;
}

.picker__group {
  margin: 0.5rem 0.5rem 0.25rem;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.picker__option {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--control-md);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-md);
  cursor: pointer;
}

.picker__option[aria-selected="true"] {
  background: var(--color-accent-soft);
}

.picker__kind {
  flex: none;
  min-width: 5.5rem;
  padding: 0 0.375rem;
  border: 1px solid;
  border-radius: var(--radius-sm);
  font: 600 var(--text-caption) var(--font-mono);
  text-align: center;
  text-transform: uppercase;
}

.picker__kind--calls {
  border-color: var(--color-family-calls-border);
  color: var(--color-family-calls-text);
  background: var(--color-family-calls-bg);
}

.picker__kind--jobs {
  border-color: var(--color-family-jobs-border);
  color: var(--color-family-jobs-text);
  background: var(--color-family-jobs-bg);
}

.picker__kind--data {
  border-color: var(--color-family-data-border);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
}

.picker__kind--code {
  border-color: var(--color-family-code-border);
  color: var(--color-family-code-text);
  background: var(--color-family-code-bg);
}

.picker__kind--messages {
  border-color: var(--color-family-messages-border);
  color: var(--color-family-messages-text);
  background: var(--color-family-messages-bg);
}

.picker__kind--files {
  border-color: var(--color-family-files-border);
  color: var(--color-family-files-text);
  background: var(--color-family-files-bg);
}

.picker__hint {
  flex: 1;
  min-width: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.picker__enter {
  flex: none;
  font: var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.picker__empty {
  margin: 0.5rem;
  color: var(--color-muted);
}
</style>
