<script setup lang="ts">
/**
 * Barre d'une étape (maquette 4.1-b, docs/plan-mvp-e-tests-etapes.md, NT2) : au-dessus d'une
 * requête rangée dans un test, les étapes du test numérotées (l'étape ouverte en évidence),
 * « + Étape » pour ajouter la suivante, et le test lui-même à ouvrir. Rien pour une requête hors
 * d'un test. L'arborescence vient du cœur (`TreeNode.test`).
 */
import { computed } from "vue";

import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();

const folder = computed(() => props.path.split("/").slice(0, -1).join("/"));

/** Dossier de la requête, s'il est un test. */
const test = computed(() => {
  if (folder.value === "") return null;
  const find = (nodes: TreeNode[]): TreeNode | null => {
    for (const node of nodes) {
      if (node.kind !== "folder") continue;
      if (node.path === folder.value) return node;
      const found = find(node.children);
      if (found) return found;
    }
    return null;
  };
  const node = find(workbench.trees[props.projectId]?.nodes ?? []);
  return node?.kind === "folder" && node.test ? node : null;
});

const steps = computed(() =>
  (test.value?.children ?? []).filter(
    (node): node is Extract<TreeNode, { kind: "request" }> => node.kind === "request",
  ),
);
const current = computed(() => steps.value.findIndex((s) => s.path === props.path));

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
    <ol class="steps__list">
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
    <span class="steps__hint">
      {{
        current <= 0
          ? "Un test est un dossier d'étapes ; celle-ci est la première."
          : "Elle reçoit les valeurs extraites des étapes précédentes."
      }}
    </span>
  </nav>
</template>

<style scoped>
.steps {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-body);
}

.steps__test {
  padding: 0 0.375rem;
  border: 0;
  border-radius: var(--radius-sm);
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
  cursor: pointer;
}

.steps__label {
  color: var(--color-muted);
}

.steps__list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.steps__chip {
  min-height: 1.5rem;
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
  border-color: var(--color-text);
  font-weight: 600;
  color: var(--color-bg);
  background: var(--color-text);
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
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

:is(.steps__test, .steps__chip, .steps__add):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
