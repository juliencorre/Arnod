<script setup lang="ts">
/**
 * Carte d'un projet dans la vue d'un workspace (2a) ; signale un dossier introuvable (P10) et
 * dit le nombre de contrats de service du projet (1.1-a, docs/plan-mvp-d-barre-laterale.md, W1).
 */
import { computed, onMounted, useId } from "vue";

import ProjectMenu from "@/components/projects/ProjectMenu.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useContractsStore } from "@/stores/contracts";
import { useSidebarStore } from "@/stores/sidebar";
import type { ProjectView } from "@/types/ipc";

const props = defineProps<{ project: ProjectView; workspaceId: string }>();

const { openProject } = useProjectActions();
const contracts = useContractsStore();
const sidebar = useSidebarStore();
const titleId = useId();
const missing = computed(() => props.project.status === "missing");

/** Contrats lus par la barre latérale s'ils le sont, sinon comptés par le cœur (R5). */
const contractCount = computed<number | null>(
  () =>
    contracts.lists[props.project.id]?.length ?? sidebar.contractCounts[props.project.id] ?? null,
);

onMounted(() => {
  if (missing.value) return;
  // Le nombre est une indication : une lecture impossible ne gêne pas la carte.
  sidebar.loadContractCount(props.project.id).catch(() => undefined);
});
</script>

<template>
  <article class="card" :class="{ 'card--missing': missing }" :aria-labelledby="titleId">
    <span class="card__initial" aria-hidden="true">{{ project.name.charAt(0).toUpperCase() }}</span>
    <div class="card__titles">
      <h2 :id="titleId" class="card__name">{{ project.name }}</h2>
      <code class="card__path">{{ project.path }}</code>
      <p v-if="missing" class="card__missing">
        Dossier introuvable : déplacé ou supprimé hors d'Arnod.
      </p>
      <p v-else-if="contractCount !== null" class="card__stats">
        <template v-if="contractCount === 0">aucun contrat de service</template>
        <template v-else>
          <strong>{{ contractCount }}</strong>
          {{ contractCount > 1 ? "contrats de service" : "contrat de service" }}
        </template>
      </p>
    </div>
    <div class="card__actions">
      <BaseButton :disabled="missing" @click="openProject(project.id, workspaceId)">
        Ouvrir
      </BaseButton>
      <ProjectMenu :project="project" :workspace-id="workspaceId" />
    </div>
  </article>
</template>

<style scoped>
.card {
  display: flex;
  align-items: flex-start;
  gap: 0.875rem;
  padding: 1.125rem 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.card--missing {
  border-style: dashed;
}

.card__initial {
  display: grid;
  flex: none;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border-radius: var(--radius-md);
  font: 600 var(--text-body) var(--font-mono);
  color: var(--color-muted);
  background: var(--color-chip);
}

.card__titles {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.card__name {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.card__path {
  overflow-wrap: anywhere;
  font: var(--text-body) var(--font-mono);
  color: var(--color-muted);
}

.card__stats {
  margin: 0.75rem 0 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.card__stats strong {
  font-weight: 600;
  color: var(--color-text);
}

.card__missing {
  margin: 0.375rem 0 0;
  font-size: var(--text-body);
  color: var(--color-danger);
}

.card__actions {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}
</style>
