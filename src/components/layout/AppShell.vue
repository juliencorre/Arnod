<script setup lang="ts">
/**
 * Fenêtre principale : barre latérale (largeur réglable, détachable : F5, F9) coiffée
 * du sélecteur de workspace, à la hauteur des onglets de contenu (S1, S8), puis vue
 * courante ; plus les fenêtres partagées. « Tous les projets » garde la barre latérale,
 * d'où l'on ouvre un projet (2e, 2f). La gestion des workspaces et le catalogue des
 * étapes occupent toute la largeur (2d) : le sélecteur passe alors dans une rangée en
 * haut.
 *
 * Ctrl+\ (code de la touche, pour l'AZERTY) ou le menu Fenêtre masquent la barre
 * latérale, gardé dans windows.json (lot F, 1.4-j) ; masquée, la rangée du haut porte
 * le bouton qui la rouvre.
 */
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import SharedDialogs from "@/components/layout/SharedDialogs.vue";
import WorkspaceSwitcher from "@/components/layout/WorkspaceSwitcher.vue";
import ManageScreen from "@/components/manage/ManageScreen.vue";
import AllProjects from "@/components/projects/AllProjects.vue";
import ProjectView from "@/components/projects/ProjectView.vue";
import StepsCatalog from "@/components/steps/StepsCatalog.vue";
import WorkspaceHome from "@/components/workspaces/WorkspaceHome.vue";
import SplitHandle from "@/components/zones/SplitHandle.vue";
import { useNavigationStore } from "@/stores/navigation";
import { useWindowsStore } from "@/stores/windows";

const navigation = useNavigationStore();
const windows = useWindowsStore();
const body = useTemplateRef<HTMLElement>("body");

const allView = computed(() => (navigation.view.kind === "all" ? navigation.view : null));
const workspaceView = computed(() =>
  navigation.view.kind === "workspace" ? navigation.view : null,
);
const projectView = computed(() => (navigation.view.kind === "project" ? navigation.view : null));
const manageView = computed(() => (navigation.view.kind === "manage" ? navigation.view : null));
const stepsView = computed(() => navigation.view.kind === "steps");
/** Pas de barre latérale dans la gestion, dans le catalogue des étapes, ni détachée. */
const sidebarPossible = computed(
  () => !windows.main?.sidebarDetached && !manageView.value && !stepsView.value,
);
/** Barre latérale affichée : possible et pas masquée (Ctrl+\, 1.4-j). */
const withSidebar = computed(() => sidebarPossible.value && !windows.sidebarHidden);

/** Ctrl+\ masque ou affiche la barre latérale (1.4-j). */
function onKeydown(event: KeyboardEvent): void {
  if (event.defaultPrevented || !(event.ctrlKey || event.metaKey)) return;
  if (event.altKey || event.shiftKey || event.code !== "Backslash") return;
  event.preventDefault();
  void windows.toggleSidebar();
}

const preview = ref<number | null>(null);
const width = computed(() => preview.value ?? windows.main?.sidebarWidth ?? 260);

/** Largeur de la fenêtre, suivie pour exprimer la barre en proportion. */
const bodyWidth = ref(0);
let observer: ResizeObserver | null = null;

function measure(): void {
  bodyWidth.value = body.value?.getBoundingClientRect().width ?? 0;
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  measure();
  if (typeof ResizeObserver !== "undefined" && body.value) {
    observer = new ResizeObserver(measure);
    observer.observe(body.value);
  }
});
onBeforeUnmount(() => {
  observer?.disconnect();
  window.removeEventListener("keydown", onKeydown);
});

/** Proportion de la barre latérale dans la fenêtre, pour la séparation. */
const ratio = computed(() => (bodyWidth.value > 0 ? width.value / bodyWidth.value : 0.2));

function onPreview(next: number | null): void {
  measure();
  preview.value = next === null ? null : Math.round(next * bodyWidth.value);
}

function onCommit(next: number): void {
  preview.value = null;
  measure();
  if (bodyWidth.value <= 0) return;
  void windows.setSidebarWidth(Math.round(next * bodyWidth.value));
}

/** Icône au survol de la séparation : la barre et le contenu s'écartent (5e). */
const detachHover = ref(false);
</script>

<template>
  <div class="shell">
    <div ref="body" class="shell__body" :class="{ 'shell__body--parting': detachHover }">
      <template v-if="withSidebar">
        <div class="shell__sidebar" :style="{ width: `${width}px` }">
          <div class="shell__switcher">
            <WorkspaceSwitcher />
          </div>
          <AppSidebar />
        </div>
        <SplitHandle
          axis="row"
          class="shell__handle"
          :ratio="ratio"
          :min-first="200"
          :min-second="400"
          label="Largeur de la barre latérale"
          @preview="onPreview"
          @commit="onCommit"
        >
          <button
            type="button"
            class="shell__detach"
            aria-label="Détacher la barre latérale dans une fenêtre"
            title="Détacher dans une fenêtre (Ctrl+Maj+D)"
            @pointerdown.stop
            @mouseenter="detachHover = true"
            @mouseleave="detachHover = false"
            @focus="detachHover = true"
            @blur="detachHover = false"
            @click="windows.detachSidebar()"
          >
            ⧉
          </button>
        </SplitHandle>
      </template>
      <div class="shell__column">
        <div v-if="!withSidebar" class="shell__switcher shell__switcher--top">
          <button
            v-if="sidebarPossible"
            type="button"
            class="shell__show-sidebar"
            aria-label="Afficher la barre latérale"
            title="Afficher la barre latérale (Ctrl+\)"
            aria-keyshortcuts="Control+Backslash"
            @click="windows.toggleSidebar()"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
              <rect x="1.5" y="2" width="11" height="10" rx="1.5" />
              <path d="M5 2v10" />
            </svg>
          </button>
          <WorkspaceSwitcher />
        </div>
        <main class="shell__main">
          <AllProjects v-if="allView" :highlight-project-id="allView.highlightProjectId" />
          <WorkspaceHome v-else-if="workspaceView" :workspace-id="workspaceView.workspaceId" />
          <ProjectView
            v-else-if="projectView"
            :project-id="projectView.projectId"
            :workspace-id="projectView.workspaceId"
          />
          <ManageScreen v-else-if="manageView" :workspace-id="manageView.workspaceId" />
          <StepsCatalog v-else-if="stepsView" />
        </main>
      </div>
    </div>
    <SharedDialogs />
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

.shell__sidebar {
  display: flex;
  flex: none;
  flex-direction: column;
  min-width: 0;
  background: var(--color-sidebar);
  transition:
    margin 0.2s,
    box-shadow 0.2s;
}

.shell__sidebar > :deep(.sidebar) {
  flex: 1;
  height: auto;
  min-height: 0;
}

/* Rangée du sélecteur : à la hauteur de la rangée d'onglets d'une zone (.zone__bar). */
.shell__switcher {
  display: flex;
  flex: none;
  align-items: center;
  height: 2.375rem;
  padding: 0 0.5rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.shell__column {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  transition:
    margin 0.2s,
    box-shadow 0.2s;
}

.shell__main {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  background: var(--color-bg);
}

.shell__main > :deep(*) {
  flex: 1 0 auto;
}

/* Survol de l'icône : les panneaux s'écartent de 10 px et s'ombrent (5e). */
.shell__body--parting .shell__sidebar {
  margin-right: 5px;
  box-shadow: var(--shadow-popover);
}

.shell__body--parting .shell__column {
  margin-left: 5px;
  box-shadow: var(--shadow-popover);
}

.shell__show-sidebar {
  display: grid;
  flex: none;
  place-items: center;
  width: 1.625rem;
  height: 1.625rem;
  margin-right: 0.375rem;
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.shell__show-sidebar svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linejoin: round;
}

.shell__show-sidebar:hover {
  color: var(--color-text);
  background: var(--color-chip);
}

.shell__show-sidebar:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.shell__detach {
  position: absolute;
  top: 3.5rem;
  left: 50%;
  display: grid;
  place-items: center;
  width: 1.625rem;
  height: 1.625rem;
  margin-left: -0.8125rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 50%;
  font: inherit;
  font-size: var(--text-small);
  color: var(--color-text);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
}

.shell__handle:hover .shell__detach,
.shell__handle:focus-within .shell__detach,
.shell__detach:focus-visible {
  opacity: 1;
}

.shell__detach:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
