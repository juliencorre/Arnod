<script setup lang="ts">
/**
 * Onglet « Référentiel des projets » (2e, P9) : chaque projet une fois, ses
 * workspaces en pastilles, « Ajouter à… » (P6), projet mis en évidence (P8),
 * dossier introuvable signalé (P10), « Oublier » avec « Annuler » (P11).
 */
import { computed, nextTick, onMounted, ref, useId, useTemplateRef } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import SearchInput from "@/components/ui/SearchInput.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import type { ProjectView } from "@/types/ipc";

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

function addItems(project: ProjectView): MenuItem[] {
  return catalog.workspaces.map((workspace) => {
    const present = project.workspaceIds.includes(workspace.id);
    return { id: workspace.id, label: workspace.name, checked: present, disabled: present };
  });
}

/** Workspace où ranger un nouveau projet : le dernier ouvert. */
function targetWorkspace(): string | undefined {
  return catalog.snapshot?.lastWorkspaceId ?? catalog.defaultWorkspace?.id;
}

function newProject(): void {
  const target = targetWorkspace();
  if (target) dialogs.openNewProject(target);
}

async function addExisting(): Promise<void> {
  const target = targetWorkspace();
  if (target) await addExistingProject(target);
}

onMounted(async () => {
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
  <div class="registry">
    <header class="registry__header">
      <div class="registry__intro">
        <h1 :id="titleId" class="registry__title">
          {{ countLabel(catalog.projects.length, "projet", "projets") }}
        </h1>
        <p class="registry__text">
          Chaque projet existe une seule fois ici. Les workspaces ne font que le référencer. Un
          nouveau projet va dans « {{ catalog.defaultWorkspace?.name }} » (par défaut).
        </p>
      </div>
      <div class="registry__tools">
        <SearchInput
          v-model="query"
          label="Rechercher un projet ou un dossier"
          placeholder="Rechercher…"
        />
        <BaseButton @click="addExisting">Ajouter un dossier existant…</BaseButton>
        <BaseButton variant="primary" @click="newProject">Nouveau projet</BaseButton>
      </div>
    </header>

    <div class="registry__table-wrap">
      <table ref="table" class="registry__table" :aria-labelledby="titleId">
        <thead>
          <tr>
            <th scope="col">Projet</th>
            <th scope="col">Dossier</th>
            <th scope="col">Workspaces</th>
            <th scope="col">Dernier run</th>
            <th scope="col"><span class="visually-hidden">Action</span></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="project in rows"
            :key="project.id"
            :data-project-id="project.id"
            :class="{ 'registry__row--highlight': project.id === highlightProjectId }"
            tabindex="-1"
          >
            <th scope="row" class="registry__name">{{ project.name }}</th>
            <td>
              <code class="registry__path">{{ project.path }}</code>
              <span v-if="project.status === 'missing'" class="registry__missing">
                Dossier introuvable
              </span>
            </td>
            <td>
              <ul class="chips" aria-label="Workspaces du projet">
                <li v-for="id in project.workspaceIds" :key="id" class="chip">
                  <ColorDot :color="catalog.workspaceById(id)?.color ?? 'gray'" size="sm" />
                  {{ catalog.workspaceById(id)?.name }}
                </li>
                <li v-if="project.workspaceIds.length === 0" class="registry__muted">
                  Aucun workspace
                </li>
              </ul>
            </td>
            <td class="registry__muted">—</td>
            <td class="registry__action">
              <BaseMenu
                align="end"
                :items="addItems(project)"
                :label="`Ajouter ${project.name} à un workspace`"
                @select="(workspaceId) => addToWorkspaces(project.id, [workspaceId])"
              >
                <template #trigger="{ attrs }">
                  <button
                    v-bind="attrs"
                    type="button"
                    class="registry__add"
                    :aria-label="`Ajouter ${project.name} à un workspace`"
                  >
                    Ajouter à… ▾
                  </button>
                </template>
              </BaseMenu>
              <button
                type="button"
                class="registry__forget"
                :aria-label="`Oublier ${project.name}`"
                title="Retirer du référentiel et des workspaces ; les fichiers restent sur le disque"
                @click="forgetProject(project.id)"
              >
                Oublier
              </button>
            </td>
          </tr>
          <tr v-if="rows.length === 0">
            <td colspan="5" class="registry__muted">
              {{
                catalog.projects.length === 0
                  ? "Le référentiel est vide : créez un projet."
                  : "Aucun projet ne correspond."
              }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p class="registry__muted">
      Un projet sans workspace reste ici : rien n'est perdu quand on retire un projet ou qu'on
      supprime un workspace.
    </p>
  </div>
</template>

<style scoped>
.registry {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.75rem 2rem;
}

.registry__header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
}

.registry__intro {
  flex: 1 1 28rem;
}

.registry__title {
  margin: 0 0 0.25rem;
  font-size: 1.5rem;
  font-weight: 650;
}

.registry__text {
  margin: 0;
  color: var(--color-muted);
}

.registry__tools {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  width: min(100%, 44rem);
}

.registry__tools > :first-child {
  flex: 1 1 14rem;
}

.registry__table-wrap {
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
}

.registry__table {
  width: 100%;
  border-collapse: collapse;
}

.registry__table th,
.registry__table td {
  padding: 0.75rem 1rem;
  text-align: left;
  vertical-align: middle;
}

.registry__table thead th {
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
  background: var(--color-sidebar);
}

.registry__table tbody tr + tr {
  border-top: 1px solid var(--color-border);
}

.registry__row--highlight {
  background: var(--color-accent-soft);
}

.registry__table tbody tr:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.registry__name {
  font-weight: 600;
}

.registry__path {
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}

.registry__missing {
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

.registry__muted {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.registry__action {
  text-align: right;
}

.registry__add {
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

.registry__add:focus-visible,
.registry__forget:focus-visible {
  outline: 2px solid var(--color-focus);
}

.registry__forget {
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  white-space: nowrap;
  color: var(--color-danger);
  background: none;
  cursor: pointer;
}

.registry__action {
  white-space: nowrap;
}
</style>
