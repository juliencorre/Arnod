<script setup lang="ts">
/** Mise en page : barre latérale et vue courante de la zone principale. */
import { computed } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import FirstRun from "@/components/projects/FirstRun.vue";
import WorkspaceHome from "@/components/workspaces/WorkspaceHome.vue";
import { useNavigationStore } from "@/stores/navigation";

const navigation = useNavigationStore();
const workspaceId = computed(() =>
  navigation.view.kind === "workspace" ? navigation.view.workspaceId : null,
);
</script>

<template>
  <div class="shell">
    <AppSidebar />
    <main class="shell__main">
      <FirstRun v-if="navigation.view.kind === 'firstRun'" />
      <WorkspaceHome v-else-if="workspaceId !== null" :workspace-id="workspaceId" />
    </main>
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
