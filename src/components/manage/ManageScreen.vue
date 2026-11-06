<script setup lang="ts">
/**
 * Écran de gestion (2d, 2e, W10) : onglets « Workspaces » et « Référentiel des
 * projets ». Flèches gauche et droite entre les onglets.
 */
import { nextTick, useTemplateRef } from "vue";

import RegistryTab from "@/components/manage/RegistryTab.vue";
import WorkspacesTab from "@/components/manage/WorkspacesTab.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { type ManageTab, useNavigationStore } from "@/stores/navigation";

const props = defineProps<{
  tab: ManageTab;
  workspaceId: string;
  highlightProjectId: string | null;
}>();

const navigation = useNavigationStore();
const tablist = useTemplateRef<HTMLElement>("tablist");

const TABS: { id: ManageTab; label: string }[] = [
  { id: "workspaces", label: "Workspaces" },
  { id: "registry", label: "Référentiel des projets" },
];

async function onTabKeydown(event: KeyboardEvent): Promise<void> {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  const index = TABS.findIndex((tab) => tab.id === props.tab);
  const next = TABS[(index + (event.key === "ArrowRight" ? 1 : -1) + TABS.length) % TABS.length];
  if (!next) return;
  navigation.selectTab(next.id);
  await nextTick();
  tablist.value?.querySelector<HTMLElement>('[aria-selected="true"]')?.focus();
}
</script>

<template>
  <div class="manage">
    <div class="manage__bar">
      <div ref="tablist" class="manage__tabs" role="tablist" aria-label="Gestion">
        <button
          v-for="item in TABS"
          :id="`manage-tab-${item.id}`"
          :key="item.id"
          type="button"
          role="tab"
          class="manage__tab"
          :aria-selected="item.id === tab"
          :aria-controls="`manage-panel-${item.id}`"
          :tabindex="item.id === tab ? 0 : -1"
          @click="navigation.selectTab(item.id)"
          @keydown="onTabKeydown"
        >
          {{ item.label }}
        </button>
      </div>
      <BaseButton variant="link" @click="navigation.closeManage()">Retour au workspace</BaseButton>
    </div>
    <section
      :id="`manage-panel-${tab}`"
      class="manage__panel"
      role="tabpanel"
      :aria-labelledby="`manage-tab-${tab}`"
    >
      <WorkspacesTab v-if="tab === 'workspaces'" :workspace-id="workspaceId" />
      <RegistryTab v-else :highlight-project-id="highlightProjectId" />
    </section>
  </div>
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

.manage__tabs {
  display: flex;
  gap: 1.5rem;
}

.manage__tab {
  padding: 0.625rem 0.25rem;
  border: none;
  border-bottom: 2px solid transparent;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.manage__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 650;
  color: var(--color-text);
}

.manage__tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.manage__panel {
  display: flex;
  flex: 1;
  flex-direction: column;
}
</style>
