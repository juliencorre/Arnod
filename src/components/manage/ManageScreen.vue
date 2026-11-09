<script setup lang="ts">
/**
 * « Gérer les workspaces » (2d, O11), ouvert depuis le menu d'un onglet :
 * liste des workspaces et détail du workspace choisi. « Retour » ramène à
 * l'onglet d'où l'on venait.
 */
import { computed, useId } from "vue";

import WorkspacesTab from "@/components/manage/WorkspacesTab.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { ALL_PROJECTS } from "@/composables/useProjectActions";
import { useNavigationStore } from "@/stores/navigation";

defineProps<{ workspaceId: string | null }>();

const navigation = useNavigationStore();
const titleId = useId();
const backLabel = computed(() =>
  navigation.currentWorkspaceId === null ? `Retour à « ${ALL_PROJECTS} »` : "Retour au workspace",
);
</script>

<template>
  <section class="manage" :aria-labelledby="titleId">
    <div class="manage__bar">
      <h1 :id="titleId" class="manage__title">Workspaces</h1>
      <BaseButton variant="link" @click="navigation.closeManage()">{{ backLabel }}</BaseButton>
    </div>
    <div class="manage__panel">
      <WorkspacesTab :workspace-id="workspaceId" />
    </div>
  </section>
</template>

<style scoped>
.manage {
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.manage__bar {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 2rem 0;
  border-bottom: 1px solid var(--color-border);
}

.manage__title {
  margin: 0;
  padding: 0.625rem 0.25rem;
  border-bottom: 2px solid var(--color-accent);
  font-size: 1rem;
  font-weight: 650;
}

.manage__panel {
  display: flex;
  flex: 1;
  flex-direction: column;
}
</style>
