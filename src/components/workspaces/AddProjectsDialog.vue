<script setup lang="ts">
/**
 * « Ajouter des projets à un workspace » (2c, W6) : choix dans le référentiel ;
 * les projets déjà présents sont cochés, grisés et inactifs.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import SearchInput from "@/components/ui/SearchInput.vue";
import { countLabel } from "@/lib/format";
import { joinNames } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useToastStore } from "@/stores/toasts";

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const toasts = useToastStore();

const query = ref("");
const selected = ref<string[]>([]);

const workspace = computed(() =>
  dialogs.addProjects ? catalog.workspaceById(dialogs.addProjects.workspaceId) : null,
);
const title = computed(() =>
  workspace.value ? `Ajouter des projets à « ${workspace.value.name} »` : "Ajouter des projets",
);
const rows = computed(() => {
  const target = workspace.value;
  if (!target) return [];
  const needle = query.value.trim().toLowerCase();
  return catalog.projects
    .filter((p) => p.name.toLowerCase().includes(needle) || p.path.toLowerCase().includes(needle))
    .map((project) => {
      const present = project.workspaceIds.includes(target.id);
      const others = project.workspaceIds
        .filter((id) => id !== target.id)
        .map((id) => catalog.workspaceById(id)?.name)
        .filter((name) => name !== undefined);
      return {
        project,
        present,
        note: present
          ? "déjà dans ce workspace"
          : others.length > 0
            ? joinNames(others)
            : "Aucun workspace",
      };
    });
});
const submitLabel = computed(() =>
  selected.value.length === 0
    ? "Ajouter"
    : `Ajouter ${countLabel(selected.value.length, "projet", "projets")}`,
);
const createNote = computed(() => {
  const target = workspace.value;
  const fallback = catalog.defaultWorkspace;
  if (!target || !fallback || target.isDefault) return "ajouté ici (workspace par défaut)";
  return `ajouté ici et à « ${fallback.name} » (par défaut)`;
});

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
  <BaseDialog
    :open="dialogs.addProjects !== null"
    :title="title"
    description="Choisissez dans le référentiel. Un projet peut être dans plusieurs workspaces."
    @close="close"
  >
    <SearchInput v-model="query" label="Rechercher un projet ou un dossier" />
    <ul class="rows">
      <li v-for="row in rows" :key="row.project.id">
        <label class="row" :class="{ 'row--present': row.present }">
          <input v-if="row.present" type="checkbox" checked disabled />
          <input v-else v-model="selected" type="checkbox" :value="row.project.id" />
          <span class="row__titles">
            <span class="row__name">{{ row.project.name }}</span>
            <code class="row__path">{{ row.project.path }}</code>
          </span>
          <span class="row__note">{{ row.note }}</span>
        </label>
      </li>
      <li v-if="catalog.projects.length === 0" class="rows__empty">
        Le référentiel est vide : créez un projet.
      </li>
      <li v-else-if="rows.length === 0" class="rows__empty">Aucun projet ne correspond.</li>
    </ul>
    <template #footer>
      <div class="footer">
        <div class="footer__create">
          <BaseButton variant="link" @click="createProject">+ Créer un nouveau projet</BaseButton>
          <span class="footer__note">{{ createNote }}</span>
        </div>
        <BaseButton @click="close">Annuler</BaseButton>
        <BaseButton variant="primary" :disabled="selected.length === 0" @click="submit">
          {{ submitLabel }}
        </BaseButton>
      </div>
    </template>
  </BaseDialog>
</template>

<style scoped>
.rows {
  display: flex;
  flex-direction: column;
  max-height: 22rem;
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
  padding: 0.75rem 1rem;
  cursor: pointer;
}

.row:has(input:checked:not(:disabled)) {
  background: var(--color-accent-soft);
}

.row--present {
  cursor: default;
  color: var(--color-muted);
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

.footer {
  display: flex;
  flex: 1;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
}

.footer__create {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: flex-start;
}

.footer__note {
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
