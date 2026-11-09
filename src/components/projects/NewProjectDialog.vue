<script setup lang="ts">
/** Fenêtre « Nouveau projet » (2a bis) : workspace ouvert prérempli, facultatif (R4, O8). */
import { computed } from "vue";

import ProjectForm from "@/components/projects/ProjectForm.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useDialogStore } from "@/stores/dialogs";

const dialogs = useDialogStore();
const { announceCreated } = useProjectActions();

function close(): void {
  dialogs.newProject = null;
}

const description = computed(() =>
  dialogs.newProject?.workspaceId
    ? "Il sera rangé dans le workspace ouvert, et visible dans « Tous les projets »."
    : "Il sera visible dans « Tous les projets ». Un workspace est facultatif.",
);

function onCreated(projectId: string, workspaceId: string | null): void {
  close();
  announceCreated(projectId, workspaceId);
}
</script>

<template>
  <BaseDialog
    :open="dialogs.newProject !== null"
    title="Nouveau projet"
    :description="description"
    @close="close"
  >
    <ProjectForm
      v-if="dialogs.newProject"
      :key="dialogs.newProject.workspaceId ?? 'all'"
      mode="dialog"
      :initial-workspace-id="dialogs.newProject.workspaceId"
      @created="onCreated"
      @cancel="close"
    />
  </BaseDialog>
</template>
