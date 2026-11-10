<script setup lang="ts">
/**
 * Formulaire de création d'un projet (P1, P2, P3), partagé par le premier
 * lancement (1a) et la fenêtre « Nouveau projet » (2a bis).
 *
 * Le dossier affiché est calculé par le cœur (`<emplacement>/<nom>`) ;
 * « Changer » choisit l'emplacement parent. Rangement (O8) : le projet est
 * toujours dans « Tous les projets » ; un workspace est facultatif, choisi parmi
 * les existants ou créé sans quitter le formulaire. Les règles (nom, dossier,
 * unicité) sont vérifiées par le cœur : ses erreurs s'affichent sous le champ
 * concerné.
 */
import { computed, nextTick, onMounted, ref, useId, useTemplateRef, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import type { MenuItem } from "@/components/ui/menu";
import { ALL_PROJECTS } from "@/composables/useProjectActions";
import { IpcError } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";

const props = defineProps<{
  /** `firstRun` : carte du premier lancement ; `dialog` : fenêtre « Nouveau projet ». */
  mode: "firstRun" | "dialog";
  /** Workspace prérempli (l'onglet ouvert), ou aucun. */
  initialWorkspaceId: string | null;
  /**
   * Projet créé (`@created`). Reçu comme une prop et non comme un événement :
   * au premier lancement, le nouvel instantané démonte ce formulaire avant la
   * fin de `submit`, et Vue ignore les événements d'un composant démonté.
   */
  onCreated: (projectId: string, workspaceId: string | null) => void;
}>();
const emit = defineEmits<{ cancel: [] }>();

const catalog = useCatalogStore();
const ids = {
  name: useId(),
  nameError: useId(),
  folderLabel: useId(),
  folderError: useId(),
  storage: useId(),
  storageHint: useId(),
  newWorkspace: useId(),
  newWorkspaceError: useId(),
};
const nameInput = useTemplateRef<HTMLInputElement>("nameInput");
const newWorkspaceInput = useTemplateRef<HTMLInputElement>("newWorkspaceInput");

const name = ref("");
const location = ref<string | null>(null);
const workspaceId = ref(props.initialWorkspaceId);
const folder = ref("");
const submitting = ref(false);
const errors = ref<{ name: string | null; folder: string | null; general: string | null }>({
  name: null,
  folder: null,
  general: null,
});

const selectedWorkspace = computed(() =>
  workspaceId.value === null ? null : catalog.workspaceById(workspaceId.value),
);
const workspaceItems = computed<MenuItem[]>(() => [
  ...(catalog.workspaces.length === 0
    ? [{ id: "none", label: "Aucun workspace pour l'instant", disabled: true }]
    : catalog.workspaces.map((w) => ({ id: w.id, label: w.name }))),
  { id: "create", label: "Créer un workspace…", separatorBefore: true },
]);

// Création d'un workspace sans quitter le formulaire (1a bis).
const creatingWorkspace = ref(false);
const newWorkspaceName = ref("");
const newWorkspaceError = ref<string | null>(null);

async function onWorkspaceMenu(id: string): Promise<void> {
  if (id !== "create") {
    workspaceId.value = id;
    return;
  }
  creatingWorkspace.value = true;
  newWorkspaceName.value = "";
  newWorkspaceError.value = null;
  await nextTick();
  newWorkspaceInput.value?.focus();
}

async function createWorkspace(): Promise<void> {
  if (newWorkspaceName.value.trim() === "") return;
  try {
    workspaceId.value = await catalog.createWorkspace(newWorkspaceName.value);
    creatingWorkspace.value = false;
  } catch (error) {
    newWorkspaceError.value = message(error);
    newWorkspaceInput.value?.focus();
  }
}
const canSubmit = computed(() => name.value.trim() !== "" && !submitting.value);

// Aperçu du dossier : seule la dernière réponse compte.
let previewRequest = 0;
watch(
  [name, location],
  async ([currentName, currentLocation]) => {
    const request = ++previewRequest;
    try {
      const suggestion = await catalog.suggestFolder(currentName, currentLocation);
      if (request === previewRequest) folder.value = suggestion;
    } catch {
      // Simple aperçu : la création signalera un vrai problème.
    }
  },
  { immediate: true },
);

watch(name, () => {
  errors.value.name = null;
});

onMounted(() => {
  if (props.mode === "firstRun") nameInput.value?.focus();
});

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function changeLocation(): Promise<void> {
  errors.value.folder = null;
  try {
    const picked = await catalog.pickFolder(location.value);
    if (picked !== null) location.value = picked;
  } catch (error) {
    errors.value.folder = message(error);
  }
}

async function submit(): Promise<void> {
  if (!canSubmit.value) return;
  errors.value = { name: null, folder: null, general: null };
  submitting.value = true;
  try {
    const projectId = await catalog.createProject({
      name: name.value,
      location: location.value,
      workspaceId: workspaceId.value,
    });
    props.onCreated(projectId, workspaceId.value);
  } catch (error) {
    const kind = error instanceof IpcError ? error.kind : null;
    if (kind === "invalidName" || kind === "nameTaken") {
      errors.value.name = message(error);
      nameInput.value?.focus();
    } else if (kind === "folderNotEmpty" || kind === "alreadyRegistered" || kind === "io") {
      errors.value.folder = message(error);
    } else {
      errors.value.general = message(error);
    }
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <form class="project-form" :class="`project-form--${mode}`" novalidate @submit.prevent="submit">
    <div class="field">
      <label :for="ids.name" class="field__label">Nom du projet</label>
      <input
        :id="ids.name"
        ref="nameInput"
        v-model="name"
        class="field__input"
        type="text"
        autocomplete="off"
        spellcheck="false"
        placeholder="boutique-tests"
        :aria-invalid="errors.name !== null"
        :aria-describedby="errors.name ? ids.nameError : undefined"
      />
      <p v-if="errors.name" :id="ids.nameError" class="field__error" role="alert">
        {{ errors.name }}
      </p>
    </div>

    <div class="field">
      <span :id="ids.folderLabel" class="field__label">Dossier</span>
      <div class="folder" role="group" :aria-labelledby="ids.folderLabel">
        <code class="folder__path" data-testid="project-folder">{{ folder }}</code>
        <BaseButton
          variant="link"
          aria-label="Changer l'emplacement du dossier"
          @click="changeLocation"
        >
          Changer
        </BaseButton>
      </div>
      <p v-if="errors.folder" :id="ids.folderError" class="field__error" role="alert">
        {{ errors.folder }}
      </p>
    </div>

    <div class="field">
      <span :id="ids.storage" class="field__label">Rangement</span>
      <div
        class="storage"
        role="group"
        :aria-labelledby="ids.storage"
        :aria-describedby="ids.storageHint"
      >
        <span class="storage__chip storage__chip--all">
          {{ ALL_PROJECTS }} <span class="storage__badge">par défaut</span>
        </span>
        <span v-if="selectedWorkspace" class="storage__chip">
          <ColorDot :color="selectedWorkspace.color" size="sm" />
          {{ selectedWorkspace.name }}
          <span v-if="selectedWorkspace.id === initialWorkspaceId" class="storage__badge"
            >workspace ouvert</span
          >
          <button
            type="button"
            class="storage__remove"
            :aria-label="`Ne pas ranger dans « ${selectedWorkspace.name} »`"
            @click="workspaceId = null"
          >
            ×
          </button>
        </span>
        <BaseMenu
          v-else
          :items="workspaceItems"
          label="Ajouter à un workspace"
          @select="onWorkspaceMenu"
        >
          <template #trigger="{ attrs }">
            <button v-bind="attrs" type="button" class="storage__add">
              + Ajouter à un workspace ▾
            </button>
          </template>
        </BaseMenu>
      </div>
      <div v-if="creatingWorkspace && !selectedWorkspace" class="storage__create">
        <label :for="ids.newWorkspace" class="visually-hidden">Nom du nouveau workspace</label>
        <input
          :id="ids.newWorkspace"
          ref="newWorkspaceInput"
          v-model="newWorkspaceName"
          class="field__input"
          type="text"
          autocomplete="off"
          placeholder="E-commerce"
          :aria-invalid="newWorkspaceError !== null"
          :aria-describedby="newWorkspaceError ? ids.newWorkspaceError : undefined"
          @keydown.enter.prevent="createWorkspace"
          @keydown.esc.prevent.stop="creatingWorkspace = false"
          @input="newWorkspaceError = null"
        />
        <BaseButton :disabled="newWorkspaceName.trim() === ''" @click="createWorkspace">
          Créer
        </BaseButton>
      </div>
      <p v-if="newWorkspaceError" :id="ids.newWorkspaceError" class="field__error" role="alert">
        {{ newWorkspaceError }}
      </p>
      <p :id="ids.storageHint" class="field__hint">
        {{
          creatingWorkspace && !selectedWorkspace
            ? `Il apparaîtra dans le sélecteur, après « ${ALL_PROJECTS} ».`
            : `Un projet est toujours dans « ${ALL_PROJECTS} ». Un workspace est facultatif.`
        }}
      </p>
    </div>

    <p v-if="errors.general" class="field__error" role="alert">{{ errors.general }}</p>

    <div class="actions">
      <BaseButton v-if="mode === 'dialog'" @click="emit('cancel')">Annuler</BaseButton>
      <BaseButton
        type="submit"
        variant="primary"
        :class="{ 'actions__submit--wide': mode === 'firstRun' }"
        :disabled="!canSubmit"
      >
        Créer le projet
      </BaseButton>
      <p v-if="mode === 'firstRun'" class="actions__note">
        Vous pourrez toujours le ranger ou le déplacer plus tard.
      </p>
    </div>
  </form>
</template>

<style scoped>
.project-form {
  display: flex;
  flex-direction: column;
  gap: 1.125rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.field__label {
  font-size: 0.875rem;
  color: var(--color-muted);
}

.field__input {
  box-sizing: border-box;
  width: 100%;
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.field__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.field__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.field__error {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-danger);
}

.field__hint {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.375rem;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.folder {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
}

.folder__path {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
  font: 0.875rem var(--font-mono);
  color: var(--color-text);
}

.storage {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.storage__chip {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4375rem 0.75rem;
  border-radius: 8px;
  font-size: 0.875rem;
  font-weight: 600;
  background: var(--color-chip);
}

.storage__badge {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--color-muted);
}

.storage__remove {
  padding: 0 0.125rem;
  border: none;
  font: inherit;
  font-size: 1rem;
  line-height: 1;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.storage__add {
  padding: 0.4375rem 0.75rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.storage__add:focus-visible,
.storage__remove:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.storage__create {
  display: flex;
  gap: 0.5rem;
}

.actions__note {
  margin: 0;
  font-size: 0.8125rem;
  text-align: center;
  color: var(--color-muted);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.project-form--firstRun .actions {
  flex-direction: column;
}

.actions__submit--wide {
  width: 100%;
  padding-block: 0.75rem;
}
</style>
