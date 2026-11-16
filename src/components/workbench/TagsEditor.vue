<script setup lang="ts">
/**
 * Étiquettes d'une requête (EP1, écran 4.9-a) : pastilles retirables et champ d'ajout qui propose
 * les étiquettes du projet avec leur nombre de requêtes, puis « + Créer « … » ».
 *
 * Motif « combobox » : le champ garde le focus ; ↑ ↓ choisir, Entrée ajouter la proposition
 * choisie (sinon le texte tapé), Échap annuler ; quitter le champ ajoute le texte tapé.
 */
import { computed, nextTick, ref, useId, useTemplateRef, watch } from "vue";

import { placeBelow, type Point } from "@/components/ui/menu";
import { suggestTags, tagCounts } from "@/lib/tags";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string; labelledby: string }>();
const tags = defineModel<string[]>({ required: true });

const workbench = useWorkbenchStore();
const listId = useId();
const adding = ref(false);
const text = ref("");
/** Option active : index dans `options` (-1 : aucune). */
const active = ref(-1);
const input = useTemplateRef<HTMLInputElement>("input");
const popover = useTemplateRef<HTMLElement>("popover");
const position = ref<Point | null>(null);

const counts = computed(() => tagCounts(workbench.trees[props.projectId]?.nodes ?? []));
const suggestions = computed(() => suggestTags(counts.value, text.value, tags.value));

/** Options de la liste : étiquettes proposées, puis la création. */
interface Option {
  tag: string;
  count: number | null;
}
const options = computed<Option[]>(() => [
  ...suggestions.value.items.map((s) => ({ tag: s.tag, count: s.count })),
  ...(suggestions.value.create === null ? [] : [{ tag: suggestions.value.create, count: null }]),
]);
const open = computed(() => adding.value && options.value.length > 0);

function optionId(index: number): string {
  return `${listId}-${String(index)}`;
}

async function start(): Promise<void> {
  adding.value = true;
  text.value = "";
  active.value = -1;
  await nextTick();
  input.value?.focus();
}

function add(tag: string): void {
  const value = tag.trim();
  const key = value.toLowerCase();
  if (value !== "" && !tags.value.some((t) => t.toLowerCase() === key)) {
    tags.value.push(value);
  }
}

function finish(tag: string | null): void {
  if (!adding.value) return;
  if (tag !== null) add(tag);
  adding.value = false;
  text.value = "";
  position.value = null;
}

function remove(index: number): void {
  tags.value.splice(index, 1);
}

function onKeydown(event: KeyboardEvent): void {
  const count = options.value.length;
  switch (event.key) {
    case "ArrowDown":
      if (count > 0) active.value = (active.value + 1) % count;
      break;
    case "ArrowUp":
      if (count > 0) active.value = active.value <= 0 ? count - 1 : active.value - 1;
      break;
    case "Enter":
      finish(options.value[active.value]?.tag ?? text.value);
      break;
    case "Escape":
      finish(null);
      break;
    default:
      return;
  }
  event.preventDefault();
}

function place(): void {
  const field = input.value;
  const el = popover.value;
  if (!field || !el) return;
  position.value = placeBelow(
    field.getBoundingClientRect(),
    { width: el.offsetWidth, height: el.offsetHeight },
    { width: window.innerWidth, height: window.innerHeight },
    "start",
  );
}

// Un texte tapé met en avant la première proposition, comme la maquette ; vide, aucune.
watch(text, (value) => {
  active.value = value.trim() === "" ? -1 : 0;
});
watch([open, options], () => {
  if (open.value) void nextTick(place);
});

function style(): Record<string, string> {
  const p = position.value;
  return p
    ? { top: `${String(p.top)}px`, left: `${String(p.left)}px` }
    : { visibility: "hidden", top: "0", left: "0" };
}
</script>

<template>
  <ul class="tags" :aria-labelledby="labelledby">
    <li v-for="(tag, index) in tags" :key="tag" class="tag">
      {{ tag }}
      <button
        type="button"
        class="tag__remove"
        :aria-label="`Retirer l'étiquette ${tag}`"
        @click="remove(index)"
      >
        ×
      </button>
    </li>
    <li>
      <input
        v-if="adding"
        ref="input"
        v-model="text"
        class="tag__input"
        type="text"
        role="combobox"
        aria-label="Nouvelle étiquette"
        aria-autocomplete="list"
        :aria-expanded="open"
        :aria-controls="listId"
        :aria-activedescendant="open && active >= 0 ? optionId(active) : undefined"
        autocomplete="off"
        @keydown="onKeydown"
        @blur="finish(text)"
      />
      <button v-else type="button" class="tag__add" @click="start">+ Étiquette</button>
      <div v-if="open" ref="popover" class="tag-list" :style="style()">
        <ul :id="listId" role="listbox" aria-label="Étiquettes du projet" class="tag-list__items">
          <li
            v-for="(option, index) in options"
            :id="optionId(index)"
            :key="`${option.count === null ? '+' : ''}${option.tag}`"
            role="option"
            :aria-selected="index === active"
            class="tag-list__option"
            :class="{
              'tag-list__option--active': index === active,
              'tag-list__option--create': option.count === null,
            }"
            @mousedown.prevent
            @mousemove="active = index"
            @click="finish(option.tag)"
          >
            <template v-if="option.count === null">+ Créer « {{ option.tag }} »</template>
            <template v-else>
              <span class="tag-list__name">{{ option.tag }}</span>
              <span class="tag-list__count" :aria-label="`${String(option.count)} requêtes`">{{
                option.count
              }}</span>
            </template>
          </li>
        </ul>
      </div>
    </li>
  </ul>
</template>

<style scoped>
.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-body);
  color: var(--color-text);
  background: var(--color-chip);
}

.tag__remove,
.tag__add {
  border: 0;
  background: none;
  font: inherit;
  cursor: pointer;
}

.tag__remove {
  padding: 0 0.125rem;
  color: var(--color-muted);
}

.tag__add {
  min-height: var(--control-sm);
  padding: 0 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font-size: var(--text-body);
  color: var(--color-text);
  background: var(--color-surface);
}

.tag__input {
  box-sizing: border-box;
  width: 9rem;
  min-height: var(--control-sm);
  padding: 0.1875rem 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background-color: var(--color-surface);
}

.tag__input:focus-visible,
.tag__remove:focus-visible,
.tag__add:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

/* Position fixe : `top` et `left` viennent de place(), sous le champ. */
.tag-list {
  position: fixed;
  z-index: 20;
  width: max-content;
  min-width: 15rem;
  max-width: min(22rem, 90vw);
  padding: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.tag-list__items {
  margin: 0;
  padding: 0;
  list-style: none;
}

.tag-list__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  min-height: var(--control-sm);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-md);
  font-size: var(--text-body);
  color: var(--color-text);
  cursor: pointer;
}

.tag-list__option--active {
  background: var(--color-accent-soft);
}

.tag-list__option--create {
  margin-top: 0.25rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-border);
  border-radius: 0 0 var(--radius-md) var(--radius-md);
  font-weight: 600;
  color: var(--color-link);
}

.tag-list__option--create:first-child {
  margin-top: 0;
  border-top: 0;
}

.tag-list__count {
  font-size: var(--text-small);
  font-variant-numeric: tabular-nums;
  color: var(--color-muted);
}
</style>
