<script setup lang="ts">
/**
 * « Nouveau workspace » (S3), ouvert sous le sélecteur de workspace : le workspace
 * s'ajoute au sélecteur et démarre vide. Échap, « Annuler » ou un clic à l'extérieur
 * ferment sans rien créer ; le focus revient au sélecteur.
 */
import { computed, nextTick, onBeforeUnmount, ref, useId, useTemplateRef, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { ALL_PROJECTS } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { useNavigationStore } from "@/stores/navigation";
import { useToastStore } from "@/stores/toasts";

const props = defineProps<{ anchor: HTMLElement | null }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const titleId = useId();
const inputId = useId();
const hintId = useId();
const errorId = useId();
const panel = useTemplateRef<HTMLElement>("panel");
const input = useTemplateRef<HTMLInputElement>("input");

const name = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);
const position = ref({ top: 0, left: 0 });
const isOpen = computed(() => dialogs.newWorkspace !== null);

function onOutside(event: PointerEvent): void {
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (panel.value?.contains(target) || props.anchor?.contains(target)) return;
  close(false);
}

watch(isOpen, async (open) => {
  if (!open) {
    document.removeEventListener("pointerdown", onOutside, true);
    return;
  }
  name.value = "";
  error.value = null;
  const rect = props.anchor?.getBoundingClientRect();
  // Aligné sur le bord gauche du sélecteur, sans sortir de la fenêtre.
  const width = Math.min(328, window.innerWidth - 16);
  position.value = {
    top: (rect?.bottom ?? 0) + 6,
    left: Math.max(8, Math.min(rect?.left ?? 8, window.innerWidth - width - 8)),
  };
  document.addEventListener("pointerdown", onOutside, true);
  await nextTick();
  input.value?.focus();
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onOutside, true);
});

/** Ferme ; `restore` rend le focus au sélecteur. */
function close(restore = true): void {
  dialogs.newWorkspace = null;
  if (restore) props.anchor?.focus();
}

async function submit(): Promise<void> {
  const request = dialogs.newWorkspace;
  if (!request || name.value.trim() === "" || submitting.value) return;
  submitting.value = true;
  try {
    const workspaceId = await catalog.createWorkspace(name.value);
    close(false);
    if (request.open) await navigation.showWorkspace(workspaceId);
    else navigation.selectManagedWorkspace(workspaceId);
    toasts.show(`Workspace « ${catalog.workspaceById(workspaceId)?.name ?? ""} » créé.`);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
    input.value?.focus();
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div
    v-if="isOpen"
    ref="panel"
    class="popover"
    role="dialog"
    :aria-labelledby="titleId"
    :style="{ top: `${position.top}px`, left: `${position.left}px` }"
    @keydown.esc.prevent.stop="close()"
  >
    <h2 :id="titleId" class="popover__title">Nouveau workspace</h2>
    <form class="popover__form" novalidate @submit.prevent="submit">
      <label :for="inputId" class="visually-hidden">Nom du workspace</label>
      <input
        :id="inputId"
        ref="input"
        v-model="name"
        class="popover__input"
        type="text"
        autocomplete="off"
        placeholder="Données de référence"
        :aria-invalid="error !== null"
        :aria-describedby="error ? `${errorId} ${hintId}` : hintId"
        @input="error = null"
      />
      <p v-if="error" :id="errorId" class="popover__error" role="alert">{{ error }}</p>
      <p :id="hintId" class="popover__hint">
        Il s'ajoute au sélecteur, vide : glissez-y des projets depuis « {{ ALL_PROJECTS }} ».
      </p>
      <div class="popover__actions">
        <BaseButton @click="close()">Annuler</BaseButton>
        <BaseButton type="submit" variant="primary" :disabled="name.trim() === '' || submitting">
          Créer
        </BaseButton>
      </div>
    </form>
  </div>
</template>

<style scoped>
.popover {
  position: fixed;
  z-index: 40;
  box-sizing: border-box;
  width: min(20.5rem, calc(100vw - 1rem));
  padding: 0.875rem 0.875rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.popover__title {
  margin: 0 0 0.625rem;
  font-size: var(--text-body);
  font-weight: 600;
}

.popover__form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.popover__input {
  box-sizing: border-box;
  width: 100%;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background: var(--color-surface);
}

.popover__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.popover__input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.popover__error {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-danger);
}

.popover__hint {
  margin: 0;
  font-size: var(--text-small);
  line-height: 1.45;
  color: var(--color-muted);
}

.popover__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.25rem;
}
</style>
