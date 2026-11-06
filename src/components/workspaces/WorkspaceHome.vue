<script setup lang="ts">
/**
 * Vue d'un workspace (2a). Pour l'instant l'en-tête seul ; les actions et les
 * cartes de projets arrivent avec T18.
 */
import { computed, useId } from "vue";

import ColorDot from "@/components/ui/ColorDot.vue";
import { useCatalogStore } from "@/stores/catalog";

const props = defineProps<{ workspaceId: string }>();

const catalog = useCatalogStore();
const titleId = useId();

const workspace = computed(() => catalog.workspaceById(props.workspaceId));
const summary = computed(() => {
  const count = workspace.value?.projectIds.length ?? 0;
  if (count === 0) return "Aucun projet pour l'instant : ajoutez-en depuis le référentiel.";
  if (count === 1) return "1 projet du référentiel, regroupé ici pour travailler ensemble.";
  return `${count} projets du référentiel, regroupés ici pour travailler ensemble.`;
});
</script>

<template>
  <section v-if="workspace" class="workspace-home" :aria-labelledby="titleId">
    <header class="workspace-home__header">
      <p class="workspace-home__label">Workspace</p>
      <h1 :id="titleId" class="workspace-home__title">
        <ColorDot :color="workspace.color" />
        {{ workspace.name }}
      </h1>
      <p class="workspace-home__summary">{{ summary }}</p>
    </header>
  </section>
</template>

<style scoped>
.workspace-home {
  box-sizing: border-box;
  padding: 2rem 2.5rem;
}

.workspace-home__header {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.workspace-home__label {
  margin: 0;
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.workspace-home__title {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin: 0;
  font-size: 1.75rem;
  font-weight: 650;
}

.workspace-home__summary {
  margin: 0;
  color: var(--color-muted);
}
</style>
