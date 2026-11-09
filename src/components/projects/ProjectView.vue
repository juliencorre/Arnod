<script setup lang="ts">
/** Vue d'un projet (1b) : en-tête et zone de travail (arborescence, requête, réponse, run). */
import { computed, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { ALL_PROJECTS } from "@/composables/useProjectActions";
import WorkbenchView from "@/components/workbench/WorkbenchView.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";

const props = defineProps<{ projectId: string; workspaceId: string | null }>();

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const titleId = useId();

const project = computed(() => catalog.projectById(props.projectId));
const workspace = computed(() =>
  props.workspaceId === null ? null : catalog.workspaceById(props.workspaceId),
);
const initial = computed(() => project.value?.name.charAt(0).toUpperCase() ?? "");
</script>

<template>
  <section v-if="project" class="project-view" :aria-labelledby="titleId">
    <BaseButton
      variant="link"
      class="project-view__back"
      @click="navigation.showTab(workspace?.id ?? null)"
    >
      ← {{ workspace?.name ?? ALL_PROJECTS }}
    </BaseButton>
    <header class="project-view__header">
      <span class="project-view__initial" aria-hidden="true">{{ initial }}</span>
      <div>
        <h1 :id="titleId" class="project-view__title">{{ project.name }}</h1>
        <code class="project-view__path">{{ project.path }}</code>
      </div>
    </header>
    <WorkbenchView :project-id="project.id" />
  </section>
</template>

<style scoped>
.project-view {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.5rem 2.5rem 2rem;
}

.project-view__back {
  align-self: flex-start;
}

.project-view__header {
  display: flex;
  align-items: center;
  gap: 0.875rem;
}

.project-view__initial {
  display: grid;
  place-items: center;
  width: 2.5rem;
  height: 2.5rem;
  border-radius: 8px;
  font: 600 1rem var(--font-mono);
  color: var(--color-muted);
  background: var(--color-chip);
}

.project-view__title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 650;
}

.project-view__path {
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}
</style>
