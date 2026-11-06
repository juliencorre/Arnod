<script setup lang="ts">
/**
 * Menu ⋯ d'un projet dans un workspace (2a) : ouvrir, ajouter à un autre
 * workspace (sous-menu), retirer du workspace avec « Annuler » (P5, P6, P7).
 */
import { computed } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import { useProjectActions } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import type { ProjectView } from "@/types/ipc";

const props = defineProps<{ project: ProjectView; workspaceId: string }>();

const catalog = useCatalogStore();
const actions = useProjectActions();

const workspace = computed(() => catalog.workspaceById(props.workspaceId));

const items = computed<MenuItem[]>(() => {
  const others = catalog.workspaces.filter((w) => w.id !== props.workspaceId);
  return [
    { id: "open", label: "Ouvrir" },
    {
      id: "add",
      label: "Ajouter à un autre workspace",
      disabled: others.length === 0,
      children: others.map((w) => {
        const present = props.project.workspaceIds.includes(w.id);
        return { id: `add:${w.id}`, label: w.name, checked: present, disabled: present };
      }),
    },
    {
      id: "remove",
      label: `Retirer de « ${workspace.value?.name ?? ""} »`,
      hint: "Le projet reste dans le référentiel.",
      tone: "danger",
      separatorBefore: true,
    },
  ];
});

async function onSelect(id: string): Promise<void> {
  if (id === "open") {
    await actions.openProject(props.project.id, props.workspaceId);
  } else if (id === "remove") {
    await actions.removeFromWorkspace(props.project.id, props.workspaceId);
  } else if (id.startsWith("add:")) {
    await actions.addToWorkspaces(props.project.id, [id.slice("add:".length)]);
  }
}
</script>

<template>
  <BaseMenu :items="items" :label="`Actions du projet ${project.name}`" @select="onSelect">
    <template #trigger="{ attrs }">
      <button
        v-bind="attrs"
        type="button"
        class="menu-trigger"
        :aria-label="`Actions du projet ${project.name}`"
      >
        ⋯
      </button>
    </template>
  </BaseMenu>
</template>

<style scoped>
.menu-trigger {
  display: grid;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border: 1px solid transparent;
  border-radius: 6px;
  font-size: 1.125rem;
  line-height: 1;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.menu-trigger:hover,
.menu-trigger[aria-expanded="true"] {
  border-color: var(--color-border);
  background: var(--color-surface);
}

.menu-trigger:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
