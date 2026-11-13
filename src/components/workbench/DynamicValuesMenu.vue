<script setup lang="ts">
/**
 * Liste des valeurs dynamiques (écran 5.1-h) : s'ouvre sous le curseur à la frappe de `{{$` dans
 * un champ qui accepte les variables (`v-variables`) de `root`, et se filtre avec ce qui suit le
 * `$`. ↑ ↓ choisir, Entrée ou Tab insérer `{{$nom}}`, Échap fermer ; un clic insère aussi.
 *
 * Motif « combobox » : le champ garde le focus et porte `aria-expanded`, `aria-controls` et
 * `aria-activedescendant` vers la liste (`listbox`).
 */
import { computed, nextTick, onBeforeUnmount, ref, useId, useTemplateRef, watch } from "vue";

import { placeBelow, type Point } from "@/components/ui/menu";
import {
  caretBox,
  dynamicQuery,
  filterDynamic,
  insertDynamic,
  type DynamicQuery,
} from "@/lib/dynamicValues";
import { useInputStore } from "@/stores/input";
import type { DynamicValue } from "@/types/ipc";

const props = defineProps<{ root: HTMLElement | null }>();

type Field = HTMLInputElement | HTMLTextAreaElement;

const inputs = useInputStore();
const listId = useId();
const popover = useTemplateRef<HTMLElement>("popover");
const field = ref<Field | null>(null);
const at = ref<DynamicQuery | null>(null);
const active = ref(0);
const position = ref<Point | null>(null);

const items = computed<DynamicValue[]>(() =>
  at.value ? filterDynamic(inputs.dynamicValues, at.value.query) : [],
);
const open = computed(() => field.value !== null && at.value !== null);

function optionId(index: number): string {
  return `${listId}-${String(index)}`;
}

function isField(target: EventTarget | null): target is Field {
  return (
    (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) &&
    target.dataset.variables !== undefined
  );
}

function close(): void {
  const f = field.value;
  if (f) {
    f.removeAttribute("aria-expanded");
    f.removeAttribute("aria-controls");
    f.removeAttribute("aria-activedescendant");
    f.removeAttribute("aria-autocomplete");
  }
  field.value = null;
  at.value = null;
  position.value = null;
}

/** Relit la saisie au curseur du champ `f` : ouvre, filtre ou ferme la liste. */
function check(f: Field): void {
  const caret = f.selectionStart ?? f.value.length;
  const query = f.selectionStart === f.selectionEnd ? dynamicQuery(f.value, caret) : null;
  if (!query) {
    close();
    return;
  }
  if (field.value !== f) {
    close();
    field.value = f;
    active.value = 0;
    void inputs.loadDynamicValues();
  } else if (at.value?.query !== query.query) {
    active.value = 0;
  }
  at.value = query;
  f.setAttribute("aria-autocomplete", "list");
  f.setAttribute("aria-expanded", "true");
  f.setAttribute("aria-controls", listId);
  void nextTick(place);
}

function place(): void {
  const f = field.value;
  const at_ = at.value;
  const el = popover.value;
  if (!f || !at_ || !el) return;
  const anchor = caretBox(f, at_.start);
  position.value = placeBelow(
    anchor,
    { width: el.offsetWidth, height: el.offsetHeight },
    { width: window.innerWidth, height: window.innerHeight },
    "start",
  );
}

function choose(index: number): void {
  const f = field.value;
  const query = at.value;
  const item = items.value[index];
  if (!f || !query || !item) return;
  const caret = f.selectionStart ?? f.value.length;
  const next = insertDynamic(f.value, query, caret, item.name);
  close();
  f.value = next.text;
  f.setSelectionRange(next.caret, next.caret);
  f.dispatchEvent(new Event("input", { bubbles: true }));
  f.focus();
}

function onInput(event: Event): void {
  if (isField(event.target)) check(event.target);
}

function onKeydown(event: KeyboardEvent): void {
  if (!open.value || event.target !== field.value) return;
  const count = items.value.length;
  let handled = true;
  switch (event.key) {
    case "ArrowDown":
      if (count > 0) active.value = (active.value + 1) % count;
      break;
    case "ArrowUp":
      if (count > 0) active.value = (active.value - 1 + count) % count;
      break;
    case "Enter":
    case "Tab":
      if (count > 0) choose(active.value);
      else handled = false;
      break;
    case "Escape":
      close();
      break;
    default:
      handled = false;
  }
  if (handled) {
    event.preventDefault();
    event.stopPropagation();
  }
}

/** Le curseur a bougé sans frappe (clic, flèches gauche et droite) : la saisie est relue. */
function onCaret(event: Event): void {
  if (isField(event.target) && event.target === field.value) check(event.target);
}

function onFocusOut(event: FocusEvent): void {
  if (event.target === field.value) close();
}

watch(active, (index) => {
  field.value?.setAttribute("aria-activedescendant", optionId(index));
  void nextTick(() => {
    document.getElementById(optionId(index))?.scrollIntoView({ block: "nearest" });
  });
});
watch(open, (isOpen) => {
  if (isOpen) field.value?.setAttribute("aria-activedescendant", optionId(active.value));
});
watch(items, () => {
  void nextTick(place);
});

watch(
  () => props.root,
  (root, previous) => {
    if (previous) detach(previous);
    if (root) {
      root.addEventListener("input", onInput);
      root.addEventListener("keydown", onKeydown, true);
      root.addEventListener("keyup", onCaret);
      root.addEventListener("click", onCaret);
      root.addEventListener("focusout", onFocusOut);
    }
  },
  { immediate: true },
);

function detach(root: HTMLElement): void {
  root.removeEventListener("input", onInput);
  root.removeEventListener("keydown", onKeydown, true);
  root.removeEventListener("keyup", onCaret);
  root.removeEventListener("click", onCaret);
  root.removeEventListener("focusout", onFocusOut);
}

onBeforeUnmount(() => {
  if (props.root) detach(props.root);
  close();
});

function style(): Record<string, string> {
  const p = position.value;
  return p
    ? { top: `${String(p.top)}px`, left: `${String(p.left)}px` }
    : { visibility: "hidden", top: "0", left: "0" };
}
</script>

<template>
  <div v-if="open" ref="popover" class="dynamic-values" :style="style()">
    <p class="dynamic-values__head">
      <span class="dynamic-values__title">Valeurs dynamiques</span>
      <span class="dynamic-values__note">tirée à chaque envoi</span>
    </p>
    <ul :id="listId" role="listbox" aria-label="Valeurs dynamiques" class="dynamic-values__list">
      <li
        v-for="(item, index) in items"
        :id="optionId(index)"
        :key="item.name"
        role="option"
        :aria-selected="index === active"
        class="dynamic-values__option"
        :class="{ 'dynamic-values__option--active': index === active }"
        @mousedown.prevent
        @mousemove="active = index"
        @click="choose(index)"
      >
        <span class="dynamic-values__name">{{ item.name }}</span>
        <span class="dynamic-values__desc">{{ item.description }}</span>
        <span class="dynamic-values__example">{{ item.example }}</span>
      </li>
    </ul>
    <p v-if="items.length === 0" class="dynamic-values__empty">
      Aucune valeur dynamique « ${{ at?.query }} » : le texte partira tel quel.
    </p>
    <p class="dynamic-values__help">
      Mêmes noms que Bruno : une collection importée garde ses valeurs. ↑ ↓ choisir · Entrée insérer
      · Échap fermer
    </p>
  </div>
</template>

<style scoped>
/* Position fixe : `top` et `left` viennent de place(), sous le `{{` du champ. */
.dynamic-values {
  position: fixed;
  z-index: 20;
  width: min(30rem, 90vw);
  box-sizing: border-box;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
  overflow: hidden;
}

.dynamic-values__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.75rem;
  margin: 0;
  padding: 0.5rem 0.75rem 0.375rem;
}

.dynamic-values__title {
  font: 600 var(--text-caption) / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.dynamic-values__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.dynamic-values__list {
  max-height: 14rem;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.dynamic-values__option {
  display: grid;
  grid-template-columns: 10rem minmax(0, 1fr) minmax(0, 9rem);
  align-items: center;
  gap: 0.75rem;
  min-height: var(--control-md);
  padding: 0 0.75rem;
  font-size: var(--text-body);
  color: var(--color-text);
  cursor: pointer;
}

.dynamic-values__option--active {
  background: var(--color-accent-soft);
}

.dynamic-values__name {
  font: 600 var(--text-small) / 1.4 var(--font-mono);
}

.dynamic-values__desc,
.dynamic-values__example {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dynamic-values__example {
  text-align: right;
  font: var(--text-small) / 1.4 var(--font-mono);
  color: var(--color-muted);
}

.dynamic-values__empty {
  margin: 0;
  padding: 0.5rem 0.75rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.dynamic-values__help {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--color-border);
  font-size: var(--text-small);
  line-height: 1.5;
  color: var(--color-muted);
}
</style>
