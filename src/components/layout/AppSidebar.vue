<script setup lang="ts">
/**
 * Barre latérale. Au premier lancement (1a) : la marque et le workspace par
 * défaut créé automatiquement. Ensuite (2a) : le sélecteur de workspaces (2b),
 * les projets du workspace courant et « + Ajouter un projet » (W6).
 */
import { computed, useId } from "vue";

import ColorDot from "@/components/ui/ColorDot.vue";
import WorkspaceSwitcher from "@/components/workspaces/WorkspaceSwitcher.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const { openProject } = useProjectActions();
const labelId = useId();
const projectsLabelId = useId();

const isFirstRun = computed(() => navigation.view.kind === "firstRun");
const workspace = computed(() =>
  navigation.currentWorkspaceId === null
    ? null
    : catalog.workspaceById(navigation.currentWorkspaceId),
);
const projectCount = computed(() => workspace.value?.projectIds.length ?? 0);
const projects = computed(() => (workspace.value ? catalog.projectsOf(workspace.value.id) : []));
const openProjectId = computed(() =>
  navigation.view.kind === "project" ? navigation.view.projectId : null,
);
</script>

<template>
  <aside class="sidebar" aria-label="Barre latérale">
    <template v-if="isFirstRun">
      <div class="sidebar__brand">
        <span class="sidebar__logo" aria-hidden="true" />
        Arnod
      </div>
      <section v-if="workspace" class="sidebar__section" :aria-labelledby="labelId">
        <h2 :id="labelId" class="sidebar__label">Workspace</h2>
        <div class="sidebar__workspace">
          <ColorDot :color="workspace.color" size="sm" />
          <span class="sidebar__name">{{ workspace.name }}</span>
          <span v-if="workspace.isDefault" class="sidebar__badge">par défaut</span>
          <span class="sidebar__count">
            <span class="visually-hidden">Projets : </span>{{ projectCount }}
          </span>
        </div>
        <p class="sidebar__note">Créé automatiquement. Votre premier projet y sera rangé.</p>
      </section>
    </template>

    <template v-else-if="workspace">
      <WorkspaceSwitcher :workspace="workspace" />
      <nav class="sidebar__section sidebar__section--scroll" :aria-labelledby="projectsLabelId">
        <h2 :id="projectsLabelId" class="sidebar__label">Projets</h2>
        <ul class="sidebar__projects">
          <li v-for="project in projects" :key="project.id">
            <button
              type="button"
              class="sidebar__project"
              :class="{ 'sidebar__project--open': project.id === openProjectId }"
              :aria-current="project.id === openProjectId ? 'page' : undefined"
              @click="openProject(project.id, workspace.id)"
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
        <button type="button" class="sidebar__add" @click="dialogs.openAddProjects(workspace.id)">
          + Ajouter un projet
        </button>
      </nav>
    </template>
  </aside>
</template>

<style scoped>
/*
 * La barre latérale ne défile pas : le menu du sélecteur de workspace (2b),
 * plus large qu'elle, doit déborder sur la zone principale. Un `overflow-y`
 * ici le rognerait (l'axe horizontal passe alors aussi en `auto`) et le focus
 * de sa recherche ferait défiler la barre de côté. Seule la liste des projets
 * défile (`sidebar__section--scroll`).
 */
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

.sidebar__brand {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0 0.375rem;
  font-size: 0.9375rem;
  font-weight: 700;
}

.sidebar__logo {
  width: 1.375rem;
  height: 1.375rem;
  border-radius: 6px;
  background: var(--color-accent);
}

.sidebar__section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.sidebar__section--scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.sidebar__label {
  margin: 0;
  padding: 0 0.5rem;
  font: 600 0.65625rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.sidebar__workspace {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font-size: 0.8125rem;
  background: var(--color-surface);
}

.sidebar__name {
  font-weight: 600;
}

.sidebar__badge {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font-size: 0.65625rem;
  color: var(--color-muted);
  background: var(--color-chip);
}

.sidebar__count {
  margin-left: auto;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.sidebar__note {
  margin: 0;
  padding: 0 0.5rem;
  font-size: 0.75rem;
  line-height: 1.5;
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
