<script setup lang="ts">
/**
 * « Requête au point ? » sous une requête liée à un contrat : essai de « Non classés » (écran
 * 2.2-b) ou requête d'un endpoint (3.2-c, lot J). « Transférer au référentiel de tests » ouvre une
 * fenêtre qui nomme le test et choisit où le ranger ; le cœur crée le dossier du test et y déplace
 * la requête, qui devient son étape 1 (sans lien au contrat).
 */
import { computed, ref, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ editorKey: string; projectName: string }>();

const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const ids = { name: useId(), parent: useId(), error: useId() };

const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const request = computed(() => editor.value?.draft ?? null);
const open = ref(false);
const name = ref("");
const parent = ref("");
const error = ref<string | null>(null);
const busy = ref(false);

/** Où ranger le test : la racine du référentiel, ou un dossier qui n'est pas un test. */
const places = computed<{ path: string; label: string }[]>(() => {
  const out = [{ path: "", label: "Référentiel de tests" }];
  const visit = (nodes: TreeNode[], prefix: string): void => {
    for (const node of nodes) {
      if (node.kind !== "folder" || node.test) continue;
      const label = `${prefix} / ${node.name}`;
      out.push({ path: node.path, label });
      visit(node.children, label);
    }
  };
  const projectId = editor.value?.projectId;
  visit(projectId ? (workbench.trees[projectId]?.tests ?? []) : [], "Référentiel de tests");
  return out;
});
const checked = computed(() => request.value?.assertions.filter((a) => a.enabled).length ?? 0);
const operation = computed(() => request.value?.contract?.operation ?? "");

function start(): void {
  name.value = request.value?.name ?? "";
  parent.value = "";
  error.value = null;
  open.value = true;
}

async function submit(): Promise<void> {
  const projectId = editor.value?.projectId;
  const path = editor.value?.path;
  if (busy.value || !projectId || !path || name.value.trim() === "") return;
  busy.value = true;
  error.value = null;
  try {
    const moved = await workbench.transferTrial(projectId, path, name.value.trim(), parent.value);
    await contracts.sync(projectId);
    open.value = false;
    toasts.show(`« ${moved.test} » rejoint le référentiel de tests.`);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section v-if="request?.contract" class="trial-transfer" aria-label="Requête au point ?">
    <p class="trial-transfer__title">Requête au point ?</p>
    <p class="trial-transfer__hint">
      Glissez-la dans un test, ou transférez-la au référentiel : elle devient un test.
    </p>
    <BaseButton class="trial-transfer__button" @click="start">
      Transférer au référentiel de tests
    </BaseButton>

    <BaseDialog
      :open="open"
      title="Transférer au référentiel de tests"
      :description="`Le test « ${name.trim() || request.name} » rejoint le référentiel de tests de « ${projectName} ».`"
      @close="open = false"
    >
      <form class="transfer" novalidate @submit.prevent="submit">
        <label :for="ids.name" class="transfer__label">Nom du test</label>
        <input
          :id="ids.name"
          v-model="name"
          class="transfer__input"
          type="text"
          autocomplete="off"
          :aria-invalid="error !== null"
          :aria-describedby="error ? ids.error : undefined"
        />
        <label :for="ids.parent" class="transfer__label">Ranger dans</label>
        <select :id="ids.parent" v-model="parent" class="transfer__select">
          <option v-for="place in places" :key="place.path" :value="place.path">
            {{ place.label }}
          </option>
        </select>
        <p class="transfer__note">
          Un dossier créé pour l'occasion ; un test est un dossier d'étapes.
        </p>
        <div class="transfer__summary">
          <p>
            Repris tels quels : la requête et ses
            <strong>{{ checked }} vérification{{ checked > 1 ? "s" : "" }}</strong>
            deviennent la première étape du test.
          </p>
          <p>
            Une étape n'est pas liée à un contrat : elle quitte l'endpoint
            <code>{{ operation }}</code> et ne sera plus vérifiée contre lui.
          </p>
        </div>
        <p v-if="error" :id="ids.error" class="transfer__error" role="alert">{{ error }}</p>
        <div class="transfer__actions">
          <BaseButton @click="open = false">Annuler</BaseButton>
          <BaseButton variant="primary" type="submit" :disabled="busy || name.trim() === ''">
            Transférer
          </BaseButton>
        </div>
      </form>
    </BaseDialog>
  </section>
</template>

<style scoped>
.trial-transfer {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem 1rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.trial-transfer__title {
  margin: 0;
  font-weight: 600;
}

.trial-transfer__hint {
  flex: 1;
  min-width: 12rem;
  margin: 0;
  color: var(--color-muted);
}

.transfer {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: min(32rem, 80vw);
}

.transfer__label {
  font-weight: 600;
}

.transfer__input {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

.transfer__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.transfer__select {
  min-height: var(--control-md);
}

.transfer__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.transfer__summary {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  background: var(--color-chip);
}

.transfer__summary p {
  margin: 0;
  line-height: 1.5;
}

.transfer__summary code {
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: var(--text-small);
}

.transfer__error {
  margin: 0;
  color: var(--color-danger);
}

.transfer__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
</style>
