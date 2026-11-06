<script setup lang="ts">
/** Fenêtre « Nouveau projet » (2a bis) : workspace prérempli, ajout au défaut (R4). */
import ProjectForm from "@/components/projects/ProjectForm.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useDialogStore } from "@/stores/dialogs";

const dialogs = useDialogStore();
const { announceCreated } = useProjectActions();

function close(): void {
  dialogs.newProject = null;
}

function onCreated(projectId: string, workspaceId: string): void {
  close();
  announceCreated(projectId, workspaceId);
}
</script>

<template>
  <BaseDialog
    :open="dialogs.newProject !== null"
    title="Nouveau projet"
    description="Il sera rangé dans le workspace choisi et dans le workspace par défaut."
    @close="close"
  >
    <ProjectForm
      v-if="dialogs.newProject"
      :key="dialogs.newProject.workspaceId"
      mode="dialog"
      :initial-workspace-id="dialogs.newProject.workspaceId"
      @created="onCreated"
      @cancel="close"
    />
  </BaseDialog>
</template>
