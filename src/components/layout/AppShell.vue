<script setup lang="ts">
/**
 * Mise en page : barre latérale et vue courante de la zone principale, plus
 * les fenêtres modales partagées (store `dialogs`).
 */
import { computed } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import ManageScreen from "@/components/manage/ManageScreen.vue";
import AddToWorkspacesDialog from "@/components/projects/AddToWorkspacesDialog.vue";
import FirstRun from "@/components/projects/FirstRun.vue";
import NewProjectDialog from "@/components/projects/NewProjectDialog.vue";
import ProjectView from "@/components/projects/ProjectView.vue";
import AddProjectsDialog from "@/components/workspaces/AddProjectsDialog.vue";
import NewWorkspaceDialog from "@/components/workspaces/NewWorkspaceDialog.vue";
import WorkspaceHome from "@/components/workspaces/WorkspaceHome.vue";
import { useNavigationStore } from "@/stores/navigation";

const navigation = useNavigationStore();
const workspaceView = computed(() =>
  navigation.view.kind === "workspace" ? navigation.view : null,
);
const projectView = computed(() => (navigation.view.kind === "project" ? navigation.view : null));
const manageView = computed(() => (navigation.view.kind === "manage" ? navigation.view : null));
</script>

<template>
  <div class="shell">
    <AppSidebar v-if="!manageView" />
    <main class="shell__main">
      <FirstRun v-if="navigation.view.kind === 'firstRun'" />
      <WorkspaceHome v-else-if="workspaceView" :workspace-id="workspaceView.workspaceId" />
      <ProjectView
        v-else-if="projectView"
        :key="projectView.projectId"
        :project-id="projectView.projectId"
        :workspace-id="projectView.workspaceId"
      />
      <ManageScreen
        v-else-if="manageView"
        :tab="manageView.tab"
        :workspace-id="manageView.workspaceId"
        :highlight-project-id="manageView.highlightProjectId"
      />
    </main>
    <NewProjectDialog />
    <AddToWorkspacesDialog />
    <AddProjectsDialog />
    <NewWorkspaceDialog />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100vh;
}

.shell__main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  background: var(--color-bg);
}
</style>
