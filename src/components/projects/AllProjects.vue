<script setup lang="ts">
/**
 * Onglet épinglé « Tous les projets » (2e, O1) : chaque projet une fois, ses
 * workspaces en pastilles, sa dernière exécution (O12), « Ajouter à… » (P6) et
 * « Oublier » avec « Annuler » (P11) ; dossier introuvable signalé (P10). Une
 * ligne se glisse sur un onglet pour ajouter le projet au workspace (O7). Sans
 * projet : le premier lancement (1a).
 */
import { computed, nextTick, onMounted, ref, useId, useTemplateRef, watch } from "vue";

import TryStep from "@/components/home/TryStep.vue";
import FirstRun from "@/components/projects/FirstRun.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import SearchInput from "@/components/ui/SearchInput.vue";
import TrayIcon from "@/components/ui/TrayIcon.vue";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { startProjectDrag } from "@/lib/dnd";
import { formatDayTime } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import type { ProjectLastRun, ProjectView } from "@/types/ipc";

const props = defineProps<{ highlightProjectId: string | null }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const { addToWorkspaces, addExistingProject, forgetProject } = useProjectActions();
const titleId = useId();
const table = useTemplateRef<HTMLElement>("table");

const query = ref("");
const rows = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return catalog.projects.filter(
    (p) => p.name.toLowerCase().includes(needle) || p.path.toLowerCase().includes(needle),
  );
});

const lastRuns = ref(new Map<string, ProjectLastRun>());

async function loadLastRuns(): Promise<void> {
  try {
    lastRuns.value = new Map((await catalog.lastRuns()).map((run) => [run.projectId, run]));
  } catch {
    // Colonne d'information : sans elle, « — » s'affiche.
  }
}

watch(() => catalog.projects.length, loadLastRuns);

function addItems(project: ProjectView): MenuItem[] {
  if (catalog.workspaces.length === 0) {
    return [{ id: "none", label: "Aucun workspace : créez-en un avec « + »", disabled: true }];
  }
  return catalog.workspaces.map((workspace) => {
    const present = project.workspaceIds.includes(workspace.id);
    return { id: workspace.id, label: workspace.name, checked: present, disabled: present };
  });
}

onMounted(async () => {
  void loadLastRuns();
  if (props.highlightProjectId === null) return;
  await nextTick();
  const row = table.value?.querySelector<HTMLElement>(
    `[data-project-id="${props.highlightProjectId}"]`,
  );
  row?.scrollIntoView({ block: "center" });
  row?.focus();
});
</script>

<template>
  <FirstRun v-if="catalog.isFirstRun" />
  <div v-else class="all">
    <header class="all__header">
      <div class="all__intro">
        <h1 :id="titleId" class="all__title">
          {{ ALL_PROJECTS }} <span class="all__count">· {{ catalog.projects.length }}</span>
        </h1>
        <p class="all__text">
          Chaque projet connu d'Arnod, une seule fois. Glissez une ligne sur le sélecteur de
          workspace pour ajouter le projet à un workspace.
        </p>
      </div>
      <div class="all__tools">
        <SearchInput
          v-model="query"
          label="Rechercher un projet ou un dossier"
          placeholder="Rechercher…"
        />
        <BaseButton @click="addExistingProject(null)">Importer…</BaseButton>
        <BaseButton variant="primary" @click="dialogs.openNewProject(null)"
          >Nouveau projet</BaseButton
        >
      </div>
    </header>

    <div class="all__try">
      <TryStep note="L'essai est gardé dans « Non classés »." />
    </div>

    <div class="all__table-wrap">
      <table ref="table" class="all__table" :aria-labelledby="titleId">
        <thead>
          <tr>
            <th scope="col">Projet</th>
            <th scope="col">Dossier</th>
            <th scope="col">Workspaces</th>
            <th scope="col">Dernière exécution</th>
            <th scope="col"><span class="visually-hidden">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="project in rows"
            :key="project.id"
            :data-project-id="project.id"
            :class="{
              'all__row--highlight': project.id === highlightProjectId,
              'all__row--inbox': catalog.isInbox(project.id),
            }"
            class="all__row"
            tabindex="-1"
            :draggable="!catalog.isInbox(project.id)"
            @dragstart="startProjectDrag($event, project.id)"
          >
            <th scope="row" class="all__name">
              <span v-if="catalog.isInbox(project.id)" class="all__inbox">
                <TrayIcon />{{ project.name }}
              </span>
              <template v-else>{{ project.name }}</template>
            </th>
            <td>
              <code class="all__path">{{ project.path }}</code>
              <span v-if="project.status === 'missing'" class="all__missing">
                Dossier introuvable
              </span>
            </td>
            <td>
              <ul class="chips" aria-label="Workspaces du projet">
                <li v-for="id in project.workspaceIds" :key="id" class="chip">
                  <ColorDot :color="catalog.workspaceById(id)?.color ?? 'gray'" size="sm" />
                  {{ catalog.workspaceById(id)?.name }}
                </li>
                <li v-if="catalog.isInbox(project.id)" class="all__muted">Hors workspaces</li>
                <li v-else-if="project.workspaceIds.length === 0" class="all__muted">
                  Aucun workspace
                </li>
              </ul>
            </td>
            <td class="all__run">
              <template v-if="lastRuns.get(project.id)">
                <span
                  :class="
                    lastRuns.get(project.id)?.passed === lastRuns.get(project.id)?.total
                      ? 'all__run--ok'
                      : 'all__run--ko'
                  "
                >
                  {{ lastRuns.get(project.id)?.passed }}/{{ lastRuns.get(project.id)?.total }}
                </span>
                <span class="all__muted">
                  · {{ formatDayTime(lastRuns.get(project.id)?.startedAt ?? 0) }}</span
                >
              </template>
              <span v-else class="all__muted">—</span>
            </td>
            <td class="all__action">
              <BaseMenu
                v-if="!catalog.isInbox(project.id)"
                align="end"
                :items="addItems(project)"
                :label="`Ajouter ${project.name} à un workspace`"
                @select="(workspaceId) => addToWorkspaces(project.id, [workspaceId])"
              >
                <template #trigger="{ attrs }">
                  <button
                    v-bind="attrs"
                    type="button"
                    class="all__add"
                    :aria-label="`Ajouter ${project.name} à un workspace`"
                  >
                    Ajouter à… ▾
                  </button>
                </template>
              </BaseMenu>
              <button
                type="button"
                class="all__forget"
                :aria-label="`Oublier ${project.name}`"
                :title="`Retirer de « ${ALL_PROJECTS} » et des workspaces ; les fichiers restent sur le disque`"
                @click="forgetProject(project.id)"
              >
                Oublier
              </button>
            </td>
          </tr>
          <tr v-if="rows.length === 0">
            <td colspan="5" class="all__muted">Aucun projet ne correspond.</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p class="all__muted">
      Un projet sans workspace reste ici : rien n'est perdu quand on retire un projet ou qu'on
      supprime un workspace.
    </p>
  </div>
</template>

<style scoped>
.all__try {
  padding: 0.875rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
}

.all__inbox {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--color-text);
}

.all__row--inbox > * {
  background: var(--color-accent-soft);
}

.all {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.75rem 2rem;
}

.all__header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
}

.all__intro {
  flex: 1 1 28rem;
}

.all__title {
  margin: 0 0 0.25rem;
  font-size: 1.5rem;
  font-weight: 650;
}

.all__count {
  font-size: 1rem;
  font-weight: 400;
  color: var(--color-muted);
}

.all__row[draggable="true"] {
  cursor: grab;
}

.all__run {
  white-space: nowrap;
  font-size: 0.875rem;
}

.all__run--ok {
  font: 600 0.875rem var(--font-mono);
  color: var(--color-success);
}

.all__run--ko {
  font: 600 0.875rem var(--font-mono);
  color: var(--color-danger);
}

.all__text {
  margin: 0;
  color: var(--color-muted);
}

.all__tools {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  width: min(100%, 44rem);
}

.all__tools > :first-child {
  flex: 1 1 14rem;
}

.all__table-wrap {
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
}

.all__table {
  width: 100%;
  border-collapse: collapse;
}

.all__table th,
.all__table td {
  padding: 0.75rem 1rem;
  text-align: left;
  vertical-align: middle;
}

.all__table thead th {
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
  background: var(--color-sidebar);
}

.all__table tbody tr + tr {
  border-top: 1px solid var(--color-border);
}

.all__row--highlight {
  background: var(--color-accent-soft);
}

.all__table tbody tr:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.all__name {
  font-weight: 600;
}

.all__path {
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}

.all__missing {
  display: block;
  font-size: 0.8125rem;
  color: var(--color-danger);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.125rem 0.625rem;
  border: 1px solid var(--color-border);
  border-radius: 999px;
  font-size: 0.8125rem;
}

.all__muted {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.all__action {
  text-align: right;
}

.all__add {
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  font-weight: 600;
  white-space: nowrap;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.all__add:focus-visible,
.all__forget:focus-visible {
  outline: 2px solid var(--color-focus);
}

.all__forget {
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  white-space: nowrap;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.all__action {
  white-space: nowrap;
}
</style>
