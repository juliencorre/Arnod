<script setup lang="ts">
/**
 * « Ajouter à des workspaces » pour un projet (P4, P6) : les workspaces qui le
 * contiennent déjà sont cochés et inactifs.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const { addToWorkspaces } = useProjectActions();

const selected = ref<string[]>([]);
const project = computed(() =>
  dialogs.addToWorkspaces ? catalog.projectById(dialogs.addToWorkspaces.projectId) : null,
);
const title = computed(() =>
  project.value ? `Ajouter « ${project.value.name} » à des workspaces` : "Ajouter à des workspaces",
);

watch(
  () => dialogs.addToWorkspaces,
  () => {
    selected.value = [];
  },
);

function close(): void {
  dialogs.addToWorkspaces = null;
}

async function submit(): Promise<void> {
  const current = project.value;
  if (!current || selected.value.length === 0) return;
  const workspaceIds = [...selected.value];
  close();
  await addToWorkspaces(current.id, workspaceIds);
}
</script>

<template>
  <BaseDialog
    :open="dialogs.addToWorkspaces !== null"
    :title="title"
    description="Un projet peut être dans plusieurs workspaces."
    @close="close"
  >
    <p v-if="catalog.workspaces.length === 0" class="empty">
      Aucun workspace pour l'instant : créez-en un avec « + » dans la barre des onglets.
    </p>
    <ul v-else-if="project" class="choices">
      <li v-for="workspace in catalog.workspaces" :key="workspace.id">
        <label
          class="choice"
          :class="{ 'choice--present': project.workspaceIds.includes(workspace.id) }"
        >
          <input
            v-if="project.workspaceIds.includes(workspace.id)"
            type="checkbox"
            checked
            disabled
          />
          <input v-else v-model="selected" type="checkbox" :value="workspace.id" />
          <ColorDot :color="workspace.color" size="sm" />
          <span class="choice__name">{{ workspace.name }}</span>
          <span v-if="project.workspaceIds.includes(workspace.id)" class="choice__note">
            déjà dedans
          </span>
        </label>
      </li>
    </ul>
    <template #footer>
      <BaseButton @click="close">Annuler</BaseButton>
      <BaseButton variant="primary" :disabled="selected.length === 0" @click="submit">
        Ajouter
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.empty {
  margin: 0;
  color: var(--color-muted);
}

.choices {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.choice {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem 0.625rem;
  border-radius: 6px;
  cursor: pointer;
}

.choice:hover {
  background: var(--color-sidebar);
}

.choice--present {
  cursor: default;
  color: var(--color-muted);
}

.choice__name {
  flex: 1;
}

.choice__note {
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
