<script setup lang="ts">
/**
 * Fenêtre de contenu détachée (F8, F10) : en-tête avec le chemin de l'onglet
 * actif, « Suit la sélection » ou « Épinglée », « Rattacher » ; puis ses zones.
 */
import { computed, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import ConfirmProtectedDialog from "@/components/workbench/ConfirmProtectedDialog.vue";
import ZoneArea from "@/components/zones/ZoneArea.vue";
import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { contractKey, useContractsStore } from "@/stores/contracts";
import { useWindowsStore } from "@/stores/windows";
import { findRequest, requestKey, useWorkbenchStore } from "@/stores/workbench";

const catalog = useCatalogStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();

/** Projet, dossiers et nom de l'onglet actif. */
const crumbs = computed<string[]>(() => {
  const content = windows.activeTab?.content;
  if (!content) return [];
  if (content.kind === "result") return ["Résultat"];
  const project = catalog.projectById(content.projectId)?.name ?? "projet";
  if (content.kind === "run") return [project, "Run"];
  if (content.kind === "contract") {
    const name =
      contracts.open[contractKey(content.projectId, content.path)]?.view?.name ??
      content.path.split("/").at(-1) ??
      "";
    return [project, "Contrats", name];
  }
  const draft = workbench.editors[requestKey(content.projectId, content.path)]?.draft;
  const node = findRequest(workbench.trees[content.projectId]?.nodes ?? [], content.path);
  const folders = content.path.split("/").slice(0, -1);
  const name = draft?.name ?? node?.name ?? content.path.split("/").at(-1) ?? "";
  return [project, ...folders, name];
});

const follow = computed(() => windows.me?.follow ?? true);

watch(
  crumbs,
  (path) => {
    ipc.windowSetTitle(path.at(-1) ?? null).catch(() => undefined);
  },
  { immediate: true },
);
</script>

<template>
  <div class="content-window">
    <header class="content-window__bar">
      <nav class="content-window__crumbs" aria-label="Emplacement">
        <template v-for="(crumb, index) in crumbs" :key="index">
          <span v-if="index > 0" class="content-window__sep" aria-hidden="true">›</span>
          <span :class="{ 'content-window__current': index === crumbs.length - 1 }">{{
            crumb
          }}</span>
        </template>
      </nav>
      <span
        v-if="windows.resultOnly"
        class="content-window__follow content-window__follow--static"
        title="Montre le résultat de la requête active, dans la dernière fenêtre utilisée."
      >
        ⇆ Suit la requête active
      </span>
      <button
        v-else
        type="button"
        class="content-window__follow"
        :aria-pressed="follow"
        :title="
          follow
            ? 'Les requêtes choisies dans la barre latérale s\'ouvrent ici. Cliquer pour épingler.'
            : 'Épinglée : la sélection s\'ouvre ailleurs. Cliquer pour suivre la sélection.'
        "
        @click="windows.setFollow(!follow)"
      >
        {{ follow ? "⇆ Suit la sélection" : "Épinglée" }}
      </button>
      <BaseButton @click="windows.reattach()">Rattacher</BaseButton>
    </header>
    <main class="content-window__main">
      <ZoneArea />
    </main>
    <ConfirmProtectedDialog />
  </div>
</template>

<style scoped>
.content-window {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.content-window__bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: 0.75rem;
  height: 2.75rem;
  padding: 0 0.75rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.content-window__crumbs {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 0.375rem;
  min-width: 0;
  overflow: hidden;
  font-size: var(--text-body);
  white-space: nowrap;
  color: var(--color-muted);
}

.content-window__current {
  font-weight: 600;
  color: var(--color-text);
}

.content-window__sep {
  color: var(--color-subtle);
}

.content-window__follow {
  padding: 0.25rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-pill);
  font: inherit;
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
}

.content-window__follow[aria-pressed="true"] {
  border-color: var(--color-toast-bg);
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
}

.content-window__follow--static {
  border-color: var(--color-toast-bg);
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
  cursor: default;
}

.content-window__follow:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.content-window__main {
  display: flex;
  flex: 1;
  min-height: 0;
  background: var(--color-bg);
}
</style>
