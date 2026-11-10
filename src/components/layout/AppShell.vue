<script setup lang="ts">
/**
 * Fenêtre principale : onglets des workspaces en haut (2b), puis barre latérale
 * (largeur réglable, détachable : F5, F9) et vue courante ; plus les fenêtres
 * partagées. « Tous les projets » et la gestion des workspaces occupent toute
 * la largeur (2d, 2e).
 */
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from "vue";

import AppSidebar from "@/components/layout/AppSidebar.vue";
import SharedDialogs from "@/components/layout/SharedDialogs.vue";
import WorkspaceTabs from "@/components/layout/WorkspaceTabs.vue";
import ManageScreen from "@/components/manage/ManageScreen.vue";
import AllProjects from "@/components/projects/AllProjects.vue";
import ProjectView from "@/components/projects/ProjectView.vue";
import StepsCatalog from "@/components/steps/StepsCatalog.vue";
import WorkspaceHome from "@/components/workspaces/WorkspaceHome.vue";
import SplitHandle from "@/components/zones/SplitHandle.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";
import { useWindowsStore } from "@/stores/windows";

const catalog = useCatalogStore();
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
/**
 * Pas de barre latérale sur le tableau « Tous les projets », dans la gestion, dans
 * le catalogue des étapes, ni détachée.
 */
const withSidebar = computed(
  () =>
    !windows.main?.sidebarDetached &&
    !manageView.value &&
    !stepsView.value &&
    !(allView.value !== null && !catalog.isFirstRun),
);

const preview = ref<number | null>(null);
const width = computed(() => preview.value ?? windows.main?.sidebarWidth ?? 260);

/** Largeur de la fenêtre, suivie pour exprimer la barre en proportion. */
const bodyWidth = ref(0);
let observer: ResizeObserver | null = null;

function measure(): void {
  bodyWidth.value = body.value?.getBoundingClientRect().width ?? 0;
}

onMounted(() => {
  measure();
  if (typeof ResizeObserver !== "undefined" && body.value) {
    observer = new ResizeObserver(measure);
    observer.observe(body.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());

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
    <WorkspaceTabs />
    <div ref="body" class="shell__body" :class="{ 'shell__body--parting': detachHover }">
      <template v-if="withSidebar">
        <div class="shell__sidebar" :style="{ width: `${width}px` }">
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
  flex: none;
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
  overflow-y: auto;
  background: var(--color-bg);
  transition:
    margin 0.2s,
    box-shadow 0.2s;
}

.shell__main > :deep(*) {
  flex: 1 0 auto;
}

/* Survol de l'icône : les panneaux s'écartent de 10 px et s'ombrent (5e). */
.shell__body--parting .shell__sidebar {
  margin-right: 5px;
  box-shadow: var(--shadow-popover);
}

.shell__body--parting .shell__main {
  margin-left: 5px;
  box-shadow: var(--shadow-popover);
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
  font-size: 0.75rem;
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
