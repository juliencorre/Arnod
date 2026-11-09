<script setup lang="ts">
/**
 * Barre latérale. Au premier lancement (1a) : « Aucun projet pour l'instant. ».
 * Ensuite (2a) : les projets de l'onglet actif (ceux du workspace, ou tous) et
 * « + Ajouter un projet » (W6) ; un dossier introuvable est signalé (P10).
 */
import { computed, useId } from "vue";

import { useProjectActions } from "@/composables/useProjectActions";
import { startProjectDrag } from "@/lib/dnd";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const { openProject } = useProjectActions();
const projectsLabelId = useId();

const workspaceId = computed(() => navigation.currentWorkspaceId);
const projects = computed(() =>
  workspaceId.value === null ? catalog.projects : catalog.projectsOf(workspaceId.value),
);
const openProjectId = computed(() =>
  navigation.view.kind === "project" ? navigation.view.projectId : null,
);

function add(): void {
  if (workspaceId.value === null) dialogs.openNewProject(null);
  else dialogs.openAddProjects(workspaceId.value);
}
</script>

<template>
  <aside class="sidebar" aria-label="Barre latérale">
    <p v-if="catalog.isFirstRun" class="sidebar__empty">Aucun projet pour l'instant.</p>
    <nav v-else class="sidebar__section" :aria-labelledby="projectsLabelId">
      <h2 :id="projectsLabelId" class="sidebar__label">Projets</h2>
      <ul class="sidebar__projects">
        <li v-for="project in projects" :key="project.id">
          <button
            type="button"
            class="sidebar__project"
            :class="{ 'sidebar__project--open': project.id === openProjectId }"
            :aria-current="project.id === openProjectId ? 'page' : undefined"
            draggable="true"
            @dragstart="startProjectDrag($event, project.id)"
            @click="openProject(project.id, workspaceId)"
          >
            <span class="sidebar__initial" aria-hidden="true">{{
              project.name.charAt(0).toUpperCase()
            }}</span>
            <span class="sidebar__project-name">{{ project.name }}</span>
            <span
              v-if="project.status === 'missing'"
              class="sidebar__missing"
              title="Dossier introuvable"
            >
              <span aria-hidden="true">!</span>
              <span class="visually-hidden">(dossier introuvable)</span>
            </span>
          </button>
        </li>
      </ul>
      <button type="button" class="sidebar__add" @click="add">
        {{ workspaceId === null ? "+ Nouveau projet" : "+ Ajouter un projet" }}
      </button>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  box-sizing: border-box;
  display: flex;
  flex: none;
  flex-direction: column;
  gap: 1.25rem;
  width: 260px;
  padding: 1rem 0.75rem;
  border-right: 1px solid var(--color-border);
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
  padding: 0 0.5rem;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.sidebar__label {
  margin: 0;
  padding: 0 0.5rem;
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

.sidebar__project {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.375rem 0.5rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 0.875rem;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.sidebar__project:hover {
  background: var(--color-surface);
}

.sidebar__project--open {
  font-weight: 600;
  background: var(--color-surface);
}

.sidebar__project:focus-visible,
.sidebar__add:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.sidebar__initial {
  display: grid;
  flex: none;
  place-items: center;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 4px;
  font: 600 0.6875rem var(--font-mono);
  color: var(--color-muted);
  background: var(--color-chip);
}

.sidebar__project-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar__missing {
  font-weight: 700;
  color: var(--color-danger);
}

.sidebar__add {
  margin-top: 0.25rem;
  padding: 0.5rem 0.625rem;
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
</style>
