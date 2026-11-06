<script setup lang="ts">
/**
 * Formulaire de création d'un projet (P1, P2, P3), partagé par le premier
 * lancement (1a) et la fenêtre « Nouveau projet » (2a bis).
 *
 * Le dossier affiché est calculé par le cœur (`<emplacement>/<nom>`) ;
 * « Changer » choisit l'emplacement parent. Les règles (nom, dossier, unicité)
 * sont vérifiées par le cœur : ses erreurs s'affichent sous le champ concerné.
 */
import { computed, onMounted, ref, useId, useTemplateRef, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import { IpcError } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";

const props = defineProps<{
  /** `firstRun` : workspace par défaut imposé ; `dialog` : workspace au choix. */
  mode: "firstRun" | "dialog";
  initialWorkspaceId: string;
}>();
const emit = defineEmits<{ created: [projectId: string, workspaceId: string]; cancel: [] }>();

const catalog = useCatalogStore();
const ids = {
  name: useId(),
  nameError: useId(),
  folderLabel: useId(),
  folderError: useId(),
  workspace: useId(),
};
const nameInput = useTemplateRef<HTMLInputElement>("nameInput");

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

const selectedWorkspace = computed(() => catalog.workspaceById(workspaceId.value));
const alsoDefault = computed(() =>
  selectedWorkspace.value && !selectedWorkspace.value.isDefault ? catalog.defaultWorkspace : null,
);
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
    emit("created", projectId, workspaceId.value);
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
      <template v-if="mode === 'dialog'">
        <label :for="ids.workspace" class="field__label">Workspace</label>
        <select :id="ids.workspace" v-model="workspaceId" class="field__input">
          <option v-for="workspace in catalog.workspaces" :key="workspace.id" :value="workspace.id">
            {{ workspace.name }}{{ workspace.isDefault ? " (par défaut)" : "" }}
          </option>
        </select>
        <p v-if="alsoDefault" class="field__hint">
          Aussi ajouté à
          <ColorDot :color="alsoDefault.color" size="sm" />
          <strong>{{ alsoDefault.name }}</strong> (par défaut)
        </p>
      </template>
      <template v-else>
        <span class="field__label">Workspace</span>
        <div v-if="selectedWorkspace" class="workspace-static">
          <ColorDot :color="selectedWorkspace.color" size="sm" />
          <span class="workspace-static__name">{{ selectedWorkspace.name }}</span>
          <span v-if="selectedWorkspace.isDefault" class="badge">par défaut</span>
        </div>
        <p class="field__hint">
          Le workspace par défaut reçoit chaque nouveau projet. Vous pourrez créer d'autres
          workspaces ensuite.
        </p>
      </template>
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

.workspace-static {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
}

.workspace-static__name {
  font-weight: 600;
}

.badge {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font-size: 0.75rem;
  color: var(--color-muted);
  background: var(--color-chip);
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
