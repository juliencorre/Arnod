<script setup lang="ts">
/**
 * Onglet « Workspaces » (2d, W7 à W11) : liste à gauche, détail à droite (nom,
 * couleur, défaut, projets) et suppression avec « Annuler ».
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import ToggleSwitch from "@/components/ui/ToggleSwitch.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";
import type { WorkspaceColor } from "@/types/ipc";

const props = defineProps<{ workspaceId: string }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const { removeFromWorkspace } = useProjectActions();
const listLabelId = useId();
const nameId = useId();
const nameErrorId = useId();
const projectsLabelId = useId();

/** Libellés français des couleurs (W8). */
const COLOR_LABELS: Record<WorkspaceColor, string> = {
  gray: "Gris",
  orange: "Orange",
  blue: "Bleu",
  green: "Vert",
  purple: "Violet",
  red: "Rouge",
  yellow: "Jaune",
  teal: "Turquoise",
};

const workspace = computed(() => catalog.workspaceById(props.workspaceId));
const projects = computed(() => catalog.projectsOf(props.workspaceId));
const colorItems = computed<MenuItem[]>(() =>
  (Object.keys(COLOR_LABELS) as WorkspaceColor[]).map((color) => ({
    id: color,
    label: COLOR_LABELS[color],
    checked: workspace.value?.color === color,
  })),
);

const name = ref("");
const nameError = ref<string | null>(null);
watch(
  () => workspace.value?.name,
  (current) => {
    name.value = current ?? "";
    nameError.value = null;
  },
  { immediate: true },
);

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function saveName(): Promise<void> {
  const current = workspace.value;
  if (!current || name.value.trim() === current.name) return;
  try {
    await catalog.renameWorkspace(current.id, name.value);
    nameError.value = null;
  } catch (error) {
    nameError.value = message(error);
  }
}

async function setColor(color: string): Promise<void> {
  try {
    await catalog.setWorkspaceColor(props.workspaceId, color as WorkspaceColor);
  } catch (error) {
    toasts.showError(error);
  }
}

async function setDefault(value: boolean): Promise<void> {
  if (!value) return;
  try {
    await catalog.setDefaultWorkspace(props.workspaceId);
    toasts.show(`« ${workspace.value?.name ?? ""} » est le workspace par défaut.`);
  } catch (error) {
    toasts.showError(error);
  }
}

async function deleteWorkspace(): Promise<void> {
  const current = workspace.value;
  if (!current) return;
  const kept = countLabel(current.projectIds.length, "projet conservé", "projets conservés");
  try {
    const undoToken = await catalog.deleteWorkspace(current.id);
    toasts.show(`Workspace « ${current.name} » supprimé · ${kept} dans le référentiel.`, {
      action: {
        label: "Annuler",
        run: async () => {
          await catalog.undo(undoToken);
          navigation.selectManagedWorkspace(current.id);
        },
      },
    });
  } catch (error) {
    toasts.showError(error);
  }
}
</script>

<template>
  <div class="workspaces-tab">
    <nav class="list" :aria-labelledby="listLabelId">
      <h2 :id="listLabelId" class="list__label">
        {{ countLabel(catalog.workspaces.length, "workspace", "workspaces") }}
      </h2>
      <ul class="list__items">
        <li v-for="item in catalog.workspaces" :key="item.id">
          <button
            type="button"
            class="list__item"
            :class="{ 'list__item--selected': item.id === workspaceId }"
            :aria-current="item.id === workspaceId ? 'true' : undefined"
            @click="navigation.selectManagedWorkspace(item.id)"
          >
            <ColorDot :color="item.color" size="sm" />
            <span class="list__name">{{ item.name }}</span>
            <span v-if="item.isDefault" class="badge">par défaut</span>
            <span class="list__count">{{
              countLabel(item.projectIds.length, "projet", "projets")
            }}</span>
          </button>
        </li>
      </ul>
      <BaseButton variant="link" @click="dialogs.openNewWorkspace({ open: false })">
        + Nouveau workspace
      </BaseButton>
      <p class="list__note">
        Le workspace par défaut reçoit chaque nouveau projet. Il ne peut pas être supprimé tant
        qu'il a ce rôle.
      </p>
    </nav>

    <div v-if="workspace" class="detail">
      <form class="detail__name" novalidate @submit.prevent="saveName">
        <label :for="nameId" class="detail__label">Nom</label>
        <div class="detail__name-row">
          <ColorDot :color="workspace.color" />
          <input
            :id="nameId"
            v-model="name"
            class="detail__input"
            type="text"
            autocomplete="off"
            :aria-invalid="nameError !== null"
            :aria-describedby="nameError ? nameErrorId : undefined"
            @blur="saveName"
          />
          <BaseMenu :items="colorItems" label="Couleur du workspace" @select="setColor">
            <template #trigger="{ attrs }">
              <button v-bind="attrs" type="button" class="detail__color">Couleur ▾</button>
            </template>
          </BaseMenu>
        </div>
        <p v-if="nameError" :id="nameErrorId" class="detail__error" role="alert">
          {{ nameError }}
        </p>
      </form>

      <div class="detail__default">
        <ToggleSwitch
          :model-value="workspace.isDefault"
          :disabled="workspace.isDefault"
          @update:model-value="setDefault"
        >
          Workspace par défaut
          <span class="detail__muted">(actuellement : {{ catalog.defaultWorkspace?.name }})</span>
        </ToggleSwitch>
      </div>

      <section class="projects" :aria-labelledby="projectsLabelId">
        <div class="projects__head">
          <h2 :id="projectsLabelId" class="detail__label">Projets · {{ projects.length }}</h2>
          <BaseButton @click="dialogs.openAddProjects(workspace.id)"
            >+ Ajouter un projet</BaseButton
          >
        </div>
        <ul v-if="projects.length > 0" class="projects__list">
          <li v-for="project in projects" :key="project.id" class="projects__row">
            <span class="projects__titles">
              <span class="projects__name">{{ project.name }}</span>
              <code class="projects__path">{{ project.path }}</code>
            </span>
            <BaseButton
              variant="link"
              class="projects__remove"
              :aria-label="`Retirer ${project.name} de ${workspace.name}`"
              @click="removeFromWorkspace(project.id, workspace.id)"
            >
              Retirer
            </BaseButton>
          </li>
        </ul>
        <p v-else class="detail__muted">Aucun projet : ajoutez-en depuis le référentiel.</p>
      </section>

      <div class="detail__delete">
        <BaseButton variant="danger" :disabled="workspace.isDefault" @click="deleteWorkspace">
          Supprimer ce workspace
        </BaseButton>
        <p class="detail__muted">
          {{
            workspace.isDefault
              ? "Le workspace par défaut ne peut pas être supprimé : définissez d'abord un autre workspace par défaut."
              : "Seul le regroupement disparaît : les projets restent dans le référentiel et dans leurs autres workspaces."
          }}
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.workspaces-tab {
  display: flex;
  flex: 1;
  flex-wrap: wrap;
}

.list {
  box-sizing: border-box;
  display: flex;
  flex: 0 1 22rem;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1.5rem 1.25rem;
  border-right: 1px solid var(--color-border);
  background: var(--color-sidebar);
}

.list__label {
  margin: 0;
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.list__items {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.list__item {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  padding: 0.625rem 0.75rem;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.list__item--selected {
  border-color: var(--color-border);
  background: var(--color-surface);
}

.list__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.list__name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
}

.list__count {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.list__note {
  margin: auto 0 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.badge {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font-size: 0.75rem;
  color: var(--color-muted);
  background: var(--color-chip);
}

.detail {
  box-sizing: border-box;
  display: flex;
  flex: 999 1 32rem;
  flex-direction: column;
  gap: 1.5rem;
  min-width: 0;
  padding: 1.5rem 2rem;
}

.detail__label {
  margin: 0;
  font-size: 0.875rem;
  font-weight: 400;
  color: var(--color-muted);
}

.detail__name {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.detail__name-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.detail__input {
  flex: 1;
  max-width: 28rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font: inherit;
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
}

.detail__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.detail__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.detail__color {
  padding: 0.5rem 0.625rem;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.detail__color:hover,
.detail__color[aria-expanded="true"] {
  border-color: var(--color-border);
}

.detail__error {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-danger);
}

.detail__muted {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.projects {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.projects__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.projects__head .detail__label {
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.projects__list {
  margin: 0;
  padding: 0;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  list-style: none;
  background: var(--color-surface);
}

.projects__row {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1rem;
}

.projects__row + .projects__row {
  border-top: 1px solid var(--color-border);
}

.projects__titles {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.projects__name {
  font-weight: 600;
}

.projects__path {
  overflow-wrap: anywhere;
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}

.projects__remove {
  color: var(--color-danger);
}

.detail__delete {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 1rem;
  margin-top: auto;
  padding-top: 1.25rem;
  border-top: 1px solid var(--color-border);
}
</style>
