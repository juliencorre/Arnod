<script setup lang="ts">
/**
 * Vue d'un workspace (2a, W3) : en-tête avec « Renommer » (W7) et « + Nouveau
 * projet » (P2), cartes des projets, carte « + Ajouter un projet » (W6).
 */
import { computed, nextTick, ref, useId, useTemplateRef } from "vue";

import ProjectCard from "@/components/projects/ProjectCard.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";

const props = defineProps<{ workspaceId: string }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const titleId = useId();
const renameId = useId();
const renameErrorId = useId();
const renameInput = useTemplateRef<HTMLInputElement>("renameInput");

const workspace = computed(() => catalog.workspaceById(props.workspaceId));
const projects = computed(() => catalog.projectsOf(props.workspaceId));
const summary = computed(() => {
  const count = projects.value.length;
  if (count === 0) return "Aucun projet pour l'instant : ajoutez-en depuis le référentiel.";
  if (count === 1) return "1 projet du référentiel, regroupé ici pour travailler ensemble.";
  return `${count} projets du référentiel, regroupés ici pour travailler ensemble.`;
});

const renaming = ref(false);
const newName = ref("");
const renameError = ref<string | null>(null);

async function startRename(): Promise<void> {
  newName.value = workspace.value?.name ?? "";
  renameError.value = null;
  renaming.value = true;
  await nextTick();
  renameInput.value?.select();
}

async function saveRename(): Promise<void> {
  try {
    await catalog.renameWorkspace(props.workspaceId, newName.value);
    renaming.value = false;
  } catch (error) {
    renameError.value = error instanceof Error ? error.message : String(error);
    renameInput.value?.focus();
  }
}

function cancelRename(): void {
  renaming.value = false;
}
</script>

<template>
  <section v-if="workspace" class="workspace-home" :aria-labelledby="titleId">
    <header class="workspace-home__header">
      <div class="workspace-home__heading">
        <p class="workspace-home__label">Workspace</p>
        <form
          v-if="renaming"
          class="rename"
          novalidate
          @submit.prevent="saveRename"
          @keydown.esc.prevent="cancelRename"
        >
          <label :for="renameId" class="visually-hidden">Nouveau nom du workspace</label>
          <input
            :id="renameId"
            ref="renameInput"
            v-model="newName"
            class="rename__input"
            type="text"
            autocomplete="off"
            :aria-invalid="renameError !== null"
            :aria-describedby="renameError ? renameErrorId : undefined"
          />
          <BaseButton type="submit" variant="primary">Enregistrer</BaseButton>
          <BaseButton @click="cancelRename">Annuler</BaseButton>
          <p v-if="renameError" :id="renameErrorId" class="rename__error" role="alert">
            {{ renameError }}
          </p>
        </form>
        <h1 v-show="!renaming" :id="titleId" class="workspace-home__title">
          <ColorDot :color="workspace.color" />
          {{ workspace.name }}
        </h1>
        <p class="workspace-home__summary">{{ summary }}</p>
      </div>
      <div class="workspace-home__actions">
        <BaseButton v-if="!renaming" @click="startRename">Renommer</BaseButton>
        <BaseButton variant="primary" @click="dialogs.openNewProject(workspace.id)">
          + Nouveau projet
        </BaseButton>
      </div>
    </header>

    <div class="workspace-home__grid">
      <ProjectCard
        v-for="project in projects"
        :key="project.id"
        :project="project"
        :workspace-id="workspace.id"
      />
      <button type="button" class="add-card" @click="dialogs.openAddProjects(workspace.id)">
        <span class="add-card__title">+ Ajouter un projet</span>
        <span class="add-card__text">nouveau, existant ou du référentiel</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.workspace-home {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1.75rem;
  padding: 2rem 2.5rem;
}

.workspace-home__header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.workspace-home__heading {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  min-width: 0;
}

.workspace-home__label {
  margin: 0;
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.workspace-home__title {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin: 0;
  font-size: 1.75rem;
  font-weight: 650;
}

.workspace-home__summary {
  margin: 0;
  color: var(--color-muted);
}

.workspace-home__actions {
  display: flex;
  gap: 0.75rem;
}

.rename {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.rename__input {
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface);
}

.rename__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.rename__error {
  flex-basis: 100%;
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-danger);
}

.workspace-home__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 22rem), 1fr));
  gap: 1rem;
}

.add-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  min-height: 7rem;
  padding: 1rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: 10px;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.add-card:hover {
  background: var(--color-surface);
}

.add-card:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.add-card__title {
  font-weight: 600;
  color: var(--color-link);
}

.add-card__text {
  font-size: 0.875rem;
}
</style>
