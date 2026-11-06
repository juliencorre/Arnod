<script setup lang="ts">
/**
 * « Ajouter un projet à un workspace » (2c revu, W6) : d'abord les deux actions
 * principales, créer un projet (P2) ou importer un dossier projet existant
 * (P12) ; ensuite, s'il y en a, les projets du référentiel absents de ce
 * workspace, à cocher. La recherche n'apparaît que pour une longue liste.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import SearchInput from "@/components/ui/SearchInput.vue";
import { countLabel } from "@/lib/format";
import { joinNames, useProjectActions } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useToastStore } from "@/stores/toasts";

/** Au-delà de ce nombre de projets proposés, la liste du référentiel se filtre. */
const SEARCH_THRESHOLD = 6;

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const toasts = useToastStore();
const { addExistingProject } = useProjectActions();

const createTitleId = useId();
const createTextId = useId();
const importTitleId = useId();
const importTextId = useId();
const registryTitleId = useId();

const query = ref("");
const selected = ref<string[]>([]);

const workspace = computed(() =>
  dialogs.addProjects ? catalog.workspaceById(dialogs.addProjects.workspaceId) : null,
);
const title = computed(() =>
  workspace.value ? `Ajouter un projet à « ${workspace.value.name} »` : "Ajouter un projet",
);
/** R4 : un projet créé ou importé va aussi dans le workspace par défaut. */
const defaultNote = computed(() => {
  const target = workspace.value;
  const fallback = catalog.defaultWorkspace;
  if (!target || !fallback || target.isDefault) return null;
  return `Un projet créé ou importé est aussi ajouté à « ${fallback.name} » (par défaut).`;
});
/** Projets du référentiel qui ne sont pas encore dans ce workspace. */
const candidates = computed(() => {
  const target = workspace.value;
  if (!target) return [];
  return catalog.projects
    .filter((project) => !project.workspaceIds.includes(target.id))
    .map((project) => {
      const others = project.workspaceIds
        .map((id) => catalog.workspaceById(id)?.name)
        .filter((name) => name !== undefined);
      return { project, note: others.length > 0 ? joinNames(others) : "Aucun workspace" };
    });
});
const searchable = computed(() => candidates.value.length > SEARCH_THRESHOLD);
const rows = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return candidates.value.filter(
    ({ project }) =>
      project.name.toLowerCase().includes(needle) || project.path.toLowerCase().includes(needle),
  );
});
const submitLabel = computed(() =>
  selected.value.length === 0
    ? "Ajouter"
    : `Ajouter ${countLabel(selected.value.length, "projet", "projets")}`,
);

watch(
  () => dialogs.addProjects,
  () => {
    query.value = "";
    selected.value = [];
  },
);

function close(): void {
  dialogs.addProjects = null;
}

function createProject(): void {
  const target = workspace.value;
  close();
  if (target) dialogs.openNewProject(target.id);
}

/** Dossier projet existant, par exemple après un clone Git (P12). */
async function importExisting(): Promise<void> {
  const target = workspace.value;
  close();
  if (target) await addExistingProject(target.id);
}

async function submit(): Promise<void> {
  const target = workspace.value;
  if (!target || selected.value.length === 0) return;
  const projectIds = [...selected.value];
  close();
  try {
    await catalog.addProjects(target.id, projectIds);
    const added =
      projectIds.length === 1
        ? `« ${catalog.projectById(projectIds[0] ?? "")?.name ?? ""} » ajouté`
        : `${countLabel(projectIds.length, "projet", "projets")} ajoutés`;
    toasts.show(`${added} à « ${target.name} ».`);
  } catch (error) {
    toasts.showError(error);
  }
}
</script>

<template>
  <BaseDialog :open="dialogs.addProjects !== null" :title="title" @close="close">
    <div class="content">
      <div class="choices">
        <button
          type="button"
          class="choice"
          :aria-labelledby="createTitleId"
          :aria-describedby="createTextId"
          @click="createProject"
        >
          <span class="choice__icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20">
              <path d="M12 5v14M5 12h14" />
            </svg>
          </span>
          <span :id="createTitleId" class="choice__title">Créer un nouveau projet</span>
          <span :id="createTextId" class="choice__text">
            Un dossier neuf avec son manifeste arnod.yml.
          </span>
        </button>
        <button
          type="button"
          class="choice"
          :aria-labelledby="importTitleId"
          :aria-describedby="importTextId"
          @click="importExisting"
        >
          <span class="choice__icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20">
              <path
                d="M3 7.5A1.5 1.5 0 0 1 4.5 6H9l2 2h8.5A1.5 1.5 0 0 1 21 9.5v8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z"
              />
            </svg>
          </span>
          <span :id="importTitleId" class="choice__title">Importer un projet existant…</span>
          <span :id="importTextId" class="choice__text">
            Un dossier contenant déjà arnod.yml, par exemple un clone Git.
          </span>
        </button>
      </div>
      <p v-if="defaultNote" class="note">{{ defaultNote }}</p>

      <section v-if="candidates.length > 0" class="registry" :aria-labelledby="registryTitleId">
        <h3 :id="registryTitleId" class="registry__title">Ou reprendre un projet du référentiel</h3>
        <p class="registry__text">
          Déjà connus d'Arnod. Un projet peut être dans plusieurs workspaces.
        </p>
        <SearchInput
          v-if="searchable"
          v-model="query"
          label="Filtrer les projets du référentiel"
          placeholder="Filtrer par nom ou dossier…"
        />
        <ul class="rows">
          <li v-for="row in rows" :key="row.project.id">
            <label class="row">
              <input v-model="selected" type="checkbox" :value="row.project.id" />
              <span class="row__titles">
                <span class="row__name">{{ row.project.name }}</span>
                <code class="row__path">{{ row.project.path }}</code>
              </span>
              <span class="row__note">{{ row.note }}</span>
            </label>
          </li>
          <li v-if="rows.length === 0" class="rows__empty">Aucun projet ne correspond.</li>
        </ul>
      </section>
    </div>

    <template #footer>
      <BaseButton @click="close">Annuler</BaseButton>
      <BaseButton
        v-if="candidates.length > 0"
        variant="primary"
        :disabled="selected.length === 0"
        @click="submit"
      >
        {{ submitLabel }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.choices {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.choice {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  column-gap: 0.875rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: var(--color-surface);
  cursor: pointer;
  transition:
    background-color 120ms ease,
    border-color 120ms ease;
}

.choice:hover {
  border-color: var(--color-accent);
  background: var(--color-accent-soft);
}

.choice:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.choice__icon {
  display: flex;
  grid-row: span 2;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 8px;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.choice:hover .choice__icon {
  background: var(--color-surface);
}

.choice__icon svg {
  fill: none;
  stroke: currentcolor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.choice__title {
  font-weight: 600;
}

.choice__text {
  font-size: 0.875rem;
  color: var(--color-muted);
}

.note {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.registry {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  margin-top: 0.75rem;
  padding-top: 1.25rem;
  border-top: 1px solid var(--color-border);
}

.registry__title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
}

.registry__text {
  margin: -0.375rem 0 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.rows {
  display: flex;
  flex-direction: column;
  max-height: 13rem;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  list-style: none;
}

.rows li + li {
  border-top: 1px solid var(--color-border);
}

.row {
  display: flex;
  align-items: center;
  gap: 0.875rem;
  padding: 0.625rem 1rem;
  cursor: pointer;
}

.row:has(input:checked) {
  background: var(--color-accent-soft);
}

.row__titles {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.row__name {
  font-weight: 500;
}

.row__path {
  overflow-wrap: anywhere;
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}

.row__note {
  max-width: 45%;
  font-size: 0.8125rem;
  text-align: right;
  color: var(--color-muted);
}

.rows__empty {
  padding: 1rem;
  color: var(--color-muted);
}
</style>
