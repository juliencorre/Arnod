<script setup lang="ts">
/** Fenêtre « Nouveau workspace » (2b, 2d, W5) : il démarre vide. */
import { ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";

const emit = defineEmits<{ created: [workspaceId: string] }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const inputId = useId();
const hintId = useId();
const errorId = useId();

const name = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);

watch(
  () => dialogs.newWorkspace,
  () => {
    name.value = "";
    error.value = null;
  },
);

function close(): void {
  dialogs.newWorkspace = null;
}

async function submit(): Promise<void> {
  const request = dialogs.newWorkspace;
  if (!request || name.value.trim() === "" || submitting.value) return;
  submitting.value = true;
  try {
    const workspaceId = await catalog.createWorkspace(name.value);
    close();
    if (request.open) await navigation.showWorkspace(workspaceId);
    emit("created", workspaceId);
    toasts.show(`Workspace « ${catalog.workspaceById(workspaceId)?.name ?? ""} » créé.`);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <BaseDialog :open="dialogs.newWorkspace !== null" title="Nouveau workspace" @close="close">
    <form id="new-workspace-form" class="form" novalidate @submit.prevent="submit">
      <label :for="inputId" class="form__label">Nom du workspace</label>
      <input
        :id="inputId"
        v-model="name"
        class="form__input"
        type="text"
        autocomplete="off"
        placeholder="Migration paiements"
        :aria-invalid="error !== null"
        :aria-describedby="error ? `${errorId} ${hintId}` : hintId"
      />
      <p v-if="error" :id="errorId" class="form__error" role="alert">{{ error }}</p>
      <p :id="hintId" class="form__hint">
        Il démarre vide : ajoutez-y des projets du référentiel. Entrée pour créer.
      </p>
    </form>
    <template #footer>
      <BaseButton @click="close">Annuler</BaseButton>
      <BaseButton
        type="submit"
        form="new-workspace-form"
        variant="primary"
        :disabled="name.trim() === '' || submitting"
      >
        Créer
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.form__label {
  font-size: 0.875rem;
  color: var(--color-muted);
}

.form__input {
  box-sizing: border-box;
  width: 100%;
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.form__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.form__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.form__error {
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-danger);
}

.form__hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
