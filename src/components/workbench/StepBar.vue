<script setup lang="ts">
/**
 * Barre d'une étape (maquette 4.1-b, docs/plan-mvp-e-tests-etapes.md, NT2) : au-dessus d'une
 * requête rangée dans un test, les étapes du test numérotées (l'étape ouverte en évidence),
 * « + Étape » pour ajouter la suivante, et le test lui-même à ouvrir. Rien pour une requête hors
 * d'un test. L'arborescence vient du cœur (`TreeNode.test`).
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";

import { testOf } from "@/lib/tree";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();

const folder = computed(() => props.path.split("/").slice(0, -1).join("/"));

/** Dossier de la requête, s'il est un test. */
const test = computed(() => testOf(workbench.trees[props.projectId]?.nodes ?? [], props.path));

const steps = computed(() =>
  (test.value?.children ?? []).filter(
    (node): node is Extract<TreeNode, { kind: "request" }> => node.kind === "request",
  ),
);
const current = computed(() => steps.value.findIndex((s) => s.path === props.path));
const hint = computed(() =>
  current.value <= 0
    ? "Un test est un dossier d'étapes ; celle-ci est la première."
    : "Elle reçoit les valeurs extraites des étapes précédentes.",
);

/*
 * Une rangée qui défile (DE8, Q11) : l'étape ouverte reste visible, et un fondu marque le bord coupé.
 */
const list = useTemplateRef<HTMLOListElement>("list");
const cut = ref({ start: false, end: false });

function measure(): void {
  const el = list.value;
  if (!el) return;
  cut.value = {
    start: el.scrollLeft > 1,
    end: el.scrollLeft + el.clientWidth < el.scrollWidth - 1,
  };
}

async function reveal(): Promise<void> {
  await nextTick();
  list.value
    ?.querySelector<HTMLElement>('[aria-current="step"]')
    ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  measure();
}

let observer: ResizeObserver | null = null;
onMounted(() => {
  void reveal();
  if (typeof ResizeObserver !== "undefined" && list.value) {
    observer = new ResizeObserver(measure);
    observer.observe(list.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());
watch(current, () => void reveal());

async function addStep(): Promise<void> {
  try {
    await workbench.addStep(props.projectId, folder.value);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}
</script>

<template>
  <nav v-if="test" class="steps" :aria-label="`Étapes du test ${test.name}`">
    <button
      type="button"
      class="steps__test"
      :title="`Ouvrir le test « ${test.name} »`"
      @click="workbench.openTest(projectId, test.path)"
    >
      TEST
    </button>
    <span class="steps__label">Étapes :</span>
    <ol
      ref="list"
      class="steps__list"
      :class="{ 'steps__list--cut-start': cut.start, 'steps__list--cut-end': cut.end }"
      @scroll="measure"
    >
      <li v-for="(step, index) in steps" :key="step.path">
        <button
          type="button"
          class="steps__chip"
          :aria-current="index === current ? 'step' : undefined"
          @click="workbench.openRequest(projectId, step.path)"
        >
          {{ index + 1 }} · {{ step.name }}
        </button>
      </li>
    </ol>
    <button type="button" class="steps__add" @click="addStep">+ Étape</button>
    <span class="steps__hint" :title="hint">
      {{ hint }}
    </span>
  </nav>
</template>

<style scoped>
.steps {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-body);
}

/* Cible de 24 × 24 px au moins (AC4, WCAG 2.5.8). */
.steps__test {
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0 0.375rem;
  border: 0;
  border-radius: var(--radius-sm);
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
  cursor: pointer;
}

/* Le libellé, le test et « + Étape » gardent leur place ; l'aide se coupe la première. */
.steps__test,
.steps__label,
.steps__add {
  flex: none;
  white-space: nowrap;
}

.steps__label {
  color: var(--color-muted);
}

/* Une seule rangée qui défile (DE8, Q11) ; le bord coupé s'estompe. */
.steps__list {
  display: flex;
  flex-wrap: nowrap;
  flex: 0 1 auto;
  min-width: 0;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  overflow-x: auto;
  scrollbar-width: none;
  list-style: none;
}

.steps__list::-webkit-scrollbar {
  display: none;
}

/* Le masque ne lit que l'opacité : toute couleur pleine convient. */
.steps__list--cut-end {
  --fade: linear-gradient(to right, var(--color-text) calc(100% - 2rem), transparent);
}

.steps__list--cut-start {
  --fade: linear-gradient(to left, var(--color-text) calc(100% - 2rem), transparent);
}

.steps__list--cut-start.steps__list--cut-end {
  --fade: linear-gradient(
    to right,
    transparent,
    var(--color-text) 2rem,
    var(--color-text) calc(100% - 2rem),
    transparent
  );
}

.steps__list--cut-start,
.steps__list--cut-end {
  -webkit-mask-image: var(--fade);
  mask-image: var(--fade);
}

.steps__chip {
  min-height: 1.5rem;
  white-space: nowrap;
  padding: 0 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-small);
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.steps__chip[aria-current="step"] {
  border-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-accent-soft);
}

.steps__add {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.steps__hint {
  flex: 0 10 auto;
  min-width: 0;
  margin-left: auto;
  overflow: hidden;
  font-size: var(--text-small);
  white-space: nowrap;
  text-overflow: ellipsis;
  color: var(--color-muted);
}

:is(.steps__test, .steps__chip, .steps__add):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
