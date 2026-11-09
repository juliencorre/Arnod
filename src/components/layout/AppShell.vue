<script setup lang="ts">
/**
 * Mise en page : onglets des workspaces en haut (2b), puis barre latérale et vue
 * courante ; plus les fenêtres partagées (store `dialogs`). « Tous les projets »
 * et la gestion des workspaces occupent toute la largeur (2d, 2e).
 */
import { computed } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import WorkspaceTabs from "@/components/layout/WorkspaceTabs.vue";
import ManageScreen from "@/components/manage/ManageScreen.vue";
import AddToWorkspacesDialog from "@/components/projects/AddToWorkspacesDialog.vue";
import AllProjects from "@/components/projects/AllProjects.vue";
import NewProjectDialog from "@/components/projects/NewProjectDialog.vue";
import ProjectView from "@/components/projects/ProjectView.vue";
import AddProjectsDialog from "@/components/workspaces/AddProjectsDialog.vue";
import WorkspaceHome from "@/components/workspaces/WorkspaceHome.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const allView = computed(() => (navigation.view.kind === "all" ? navigation.view : null));
const workspaceView = computed(() =>
  navigation.view.kind === "workspace" ? navigation.view : null,
);
const projectView = computed(() => (navigation.view.kind === "project" ? navigation.view : null));
const manageView = computed(() => (navigation.view.kind === "manage" ? navigation.view : null));
/** Pas de barre latérale sur le tableau « Tous les projets » ni dans la gestion. */
const withSidebar = computed(
  () => !manageView.value && !(allView.value !== null && !catalog.isFirstRun),
);
</script>

<template>
  <div class="shell">
    <WorkspaceTabs />
    <div class="shell__body">
      <AppSidebar v-if="withSidebar" />
      <main class="shell__main">
        <AllProjects v-if="allView" :highlight-project-id="allView.highlightProjectId" />
        <WorkspaceHome v-else-if="workspaceView" :workspace-id="workspaceView.workspaceId" />
        <ProjectView
          v-else-if="projectView"
          :key="projectView.projectId"
          :project-id="projectView.projectId"
          :workspace-id="projectView.workspaceId"
        />
        <ManageScreen v-else-if="manageView" :workspace-id="manageView.workspaceId" />
      </main>
    </div>
    <NewProjectDialog />
    <AddToWorkspacesDialog />
    <AddProjectsDialog />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.shell__body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.shell__main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  background: var(--color-bg);
}
</style>
