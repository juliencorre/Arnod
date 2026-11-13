<script setup lang="ts">
/**
 * Proposition de chemin généralisé d'un essai (écran 2.2-c) : un segment de l'adresse ressemble
 * à un identifiant ; « Généraliser » en fait un paramètre de chemin de l'opération du contrat
 * déduit, « Garder tel quel » ne le propose plus. Le cœur décide et modifie tout.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const busy = ref(false);

const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const suggestion = computed(() => workbench.generalizations[props.editorKey] ?? null);
/** Segments du chemin généralisé, les paramètres mis à part pour les montrer en gras. */
const values = computed(() => suggestion.value?.template.params ?? []);

watch(
  () => [props.editorKey, editor.value?.draft?.contract?.operation] as const,
  () => {
    void workbench.inspectTrial(props.editorKey);
  },
  { immediate: true },
);

async function act(action: "generalize" | "keep"): Promise<void> {
  const projectId = editor.value?.projectId;
  if (busy.value || !projectId) return;
  busy.value = true;
  try {
    if (action === "generalize") {
      const target = suggestion.value?.template.path ?? "";
      await workbench.generalizeTrial(props.editorKey);
      await contracts.sync(projectId);
      toasts.show(`Chemin généralisé : ${target}`);
    } else {
      await workbench.keepTrialPath(props.editorKey);
    }
  } catch (caught) {
    toasts.showError(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section v-if="suggestion" class="generalize" aria-label="Chemin généralisé proposé">
    <p class="generalize__text">
      Ce chemin ressemble à <code class="generalize__path">{{ suggestion.template.path }}</code> :
      {{ " "
      }}<template v-for="(param, index) in values" :key="param.name"
        ><template v-if="index > 0">, </template
        ><strong class="generalize__value">{{ param.value }}</strong> deviendrait le paramètre de
        chemin <code class="generalize__path">{{ param.name }}</code></template
      >, et l'endpoint servirait pour toutes les valeurs.
    </p>
    <div class="generalize__actions">
      <BaseButton :disabled="busy" @click="act('keep')">Garder tel quel</BaseButton>
      <BaseButton :disabled="busy" @click="act('generalize')">Généraliser</BaseButton>
    </div>
  </section>
</template>

<style scoped>
.generalize {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0.875rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-accent-soft);
}

.generalize__text {
  margin: 0;
  line-height: 1.6;
  color: var(--color-text);
}

.generalize__path {
  font-family: var(--font-mono);
  font-size: var(--text-small);
}

.generalize__value {
  font-weight: 600;
}

.generalize__actions {
  display: flex;
  gap: 0.5rem;
}
</style>
