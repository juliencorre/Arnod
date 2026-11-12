<script setup lang="ts">
/**
 * Barre latérale détachée (F9) : sélecteur de workspace et projets ; elle
 * pilote les fenêtres qui suivent la sélection (Z5). « Rattacher » la remet
 * dans la fenêtre principale.
 */
import { computed } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import SharedDialogs from "@/components/layout/SharedDialogs.vue";
import WorkspaceSwitcher from "@/components/layout/WorkspaceSwitcher.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { countLabel } from "@/lib/format";
import { useWindowsStore } from "@/stores/windows";

const windows = useWindowsStore();
const pilot = computed(
  () => `Pilote ${countLabel(windows.followers, "fenêtre de contenu", "fenêtres de contenu")}`,
);
</script>

<template>
  <div class="sidebar-window">
    <div class="sidebar-window__switcher">
      <WorkspaceSwitcher />
    </div>
    <div class="sidebar-window__body">
      <AppSidebar />
    </div>
    <footer class="sidebar-window__footer">
      <span class="sidebar-window__pilot"
        ><span class="sidebar-window__dot" aria-hidden="true" />{{ pilot }}</span
      >
      <BaseButton @click="windows.reattach()">Rattacher</BaseButton>
    </footer>
    <SharedDialogs />
  </div>
</template>

<style scoped>
.sidebar-window {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--color-sidebar);
}

.sidebar-window__switcher {
  display: flex;
  flex: none;
  align-items: center;
  height: 2.375rem;
  padding: 0 0.5rem;
  border-bottom: 1px solid var(--color-border);
}

.sidebar-window__body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.sidebar-window__footer {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--color-border);
}

.sidebar-window__pilot {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.sidebar-window__dot {
  width: 0.4375rem;
  height: 0.4375rem;
  border-radius: 50%;
  background: var(--color-success);
}
</style>
