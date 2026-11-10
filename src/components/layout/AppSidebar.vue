<script setup lang="ts">
/**
 * Barre latérale. Au premier lancement (1a) : « Aucun projet pour l'instant. ».
 * Ensuite (2a) : les projets de l'onglet actif (ceux du workspace, ou tous), chacun
 * avec ses actions, son environnement et son arborescence (`SidebarProject`,
 * docs/plan-barre-laterale-projet.md) ; « + Ajouter un projet » (W6) ; un dossier
 * introuvable est signalé (P10). Ctrl+N crée une requête et Ctrl+Maj+E ouvre les
 * environnements du projet sélectionné (B6).
 */
import { computed, onBeforeUnmount, onMounted, useId, watch } from "vue";

import SidebarProject from "@/components/layout/SidebarProject.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { contentProject, useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const windows = useWindowsStore();
const projectsLabelId = useId();

const workspaceId = computed(() => navigation.currentWorkspaceId);
const projects = computed(() =>
  workspaceId.value === null ? catalog.projects : catalog.projectsOf(workspaceId.value),
);
const openProjectId = computed(() =>
  navigation.view.kind === "project" ? navigation.view.projectId : null,
);

// Un projet replié montre son environnement : il est lu sans déplier le projet.
watch(
  () => projects.value.filter((p) => p.status !== "missing").map((p) => p.id),
  (ids) => {
    for (const id of ids) void workbench.peekEnvironments(id);
  },
  { immediate: true },
);

function add(): void {
  if (workspaceId.value === null) dialogs.openNewProject(null);
  else dialogs.openAddProjects(workspaceId.value);
}

type ProjectBlock = InstanceType<typeof SidebarProject>;
const blocks = new Map<string, ProjectBlock>();

function register(projectId: string, block: unknown): void {
  if (block) blocks.set(projectId, block as ProjectBlock);
  else blocks.delete(projectId);
}

/** Projet sélectionné : celui de l'onglet actif, sinon le projet ouvert, sinon le premier déplié. */
function selectedProject(): string | null {
  const content = windows.activeTab?.content;
  const candidates = [
    content ? contentProject(content) : null,
    openProjectId.value,
    projects.value.find((p) => workbench.isExpanded(p.id))?.id,
  ];
  return candidates.find((id) => id !== undefined && id !== null && blocks.has(id)) ?? null;
}

function onKeydown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
  const key = event.key.toLowerCase();
  const wanted =
    key === "n" && !event.shiftKey ? "request" : key === "e" && event.shiftKey ? "env" : null;
  if (wanted === null) return;
  const projectId = selectedProject();
  const block = projectId === null ? undefined : blocks.get(projectId);
  if (!block) return;
  event.preventDefault();
  if (wanted === "request") void block.newRequest();
  else void block.manageEnvironments();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <aside class="sidebar" aria-label="Barre latérale">
    <p v-if="catalog.isFirstRun" class="sidebar__empty">Aucun projet pour l'instant.</p>
    <nav v-else class="sidebar__section" :aria-labelledby="projectsLabelId">
      <h2 :id="projectsLabelId" class="sidebar__label">Projets</h2>
      <ul class="sidebar__projects">
        <SidebarProject
          v-for="project in projects"
          :key="project.id"
          :ref="(block) => register(project.id, block)"
          :project="project"
          :workspace-id="workspaceId"
          :current="project.id === openProjectId"
        />
      </ul>
      <button type="button" class="sidebar__add" @click="add">
        {{ workspaceId === null ? "+ Nouveau projet" : "+ Ajouter un projet" }}
      </button>
    </nav>
    <slot name="footer" />
  </aside>
</template>

<style scoped>
/*
 * Grille des lignes de la barre latérale (plan de la barre latérale du projet, B1) : chaque
 * ligne commence à --tree-pad ; une colonne de tête de --tree-lead (chevron et pastille d'un
 * projet, chevron et dossier, méthode, « + ») ; le libellé commence --tree-gap plus loin ;
 * chaque profondeur décale de --tree-indent, la largeur d'un chevron et de son espace.
 */
.sidebar {
  --tree-pad: 0.375rem;
  --tree-indent: 1.25rem;
  --tree-lead: 2.5rem;
  --tree-gap: 0.5rem;

  box-sizing: border-box;
  display: flex;
  flex: none;
  flex-direction: column;
  gap: 1.25rem;
  width: 100%;
  height: 100%;
  min-width: 0;
  padding: 1rem 0.5rem 0.75rem;
  background: var(--color-sidebar);
}

.sidebar__section {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.5rem;
  min-height: 0;
  overflow-y: auto;
}

.sidebar__empty {
  margin: 0;
  padding: 0 var(--tree-pad);
  font-size: 0.875rem;
  color: var(--color-muted);
}

.sidebar__label {
  margin: 0;
  padding: 0 var(--tree-pad);
  font: 600 0.65625rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.sidebar__projects {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.sidebar__add {
  margin-top: 0.25rem;
  padding: 0.5rem calc(var(--tree-pad) - 1px);
  border: 1px dashed var(--color-border-strong);
  border-radius: 6px;
  font: inherit;
  font-size: 0.875rem;
  font-weight: 600;
  text-align: left;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.sidebar__add:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}
</style>
