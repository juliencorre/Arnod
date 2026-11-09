<script setup lang="ts">
/** Zone de travail d'un projet : arborescence à gauche, requête et réponse à droite. */
import { onMounted, watch } from "vue";

import ConfirmProtectedDialog from "@/components/workbench/ConfirmProtectedDialog.vue";
import EnvironmentBar from "@/components/workbench/EnvironmentBar.vue";
import RequestEditor from "@/components/workbench/RequestEditor.vue";
import ResponsePanel from "@/components/workbench/ResponsePanel.vue";
import RunPanel from "@/components/workbench/RunPanel.vue";
import TreePanel from "@/components/workbench/TreePanel.vue";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();

async function load(): Promise<void> {
  try {
    await workbench.open(props.projectId);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}

onMounted(load);
watch(() => props.projectId, load);
</script>

<template>
  <div class="bench">
    <aside class="bench__tree">
      <EnvironmentBar />
      <TreePanel />
    </aside>
    <div class="bench__main">
      <RunPanel />
      <template v-if="workbench.draft">
        <RequestEditor />
        <ResponsePanel />
      </template>
      <p v-else-if="!workbench.run" class="bench__empty">
        Choisissez une requête dans l'arborescence, ou créez-en une.
      </p>
    </div>
    <ConfirmProtectedDialog />
  </div>
</template>

<style scoped>
.bench {
  display: grid;
  grid-template-columns: minmax(14rem, 18rem) minmax(0, 1fr);
  gap: 1.5rem;
  align-items: start;
}

.bench__tree,
.bench__main {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
}

.bench__empty {
  margin: 2rem 0;
  color: var(--color-muted);
  text-align: center;
}
</style>
