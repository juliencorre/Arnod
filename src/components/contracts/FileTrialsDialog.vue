<script setup lang="ts">
/**
 * « Ranger dans un projet » (écran 2.2-d) : les essais d'un contrat déduit de « Non classés »
 * vont dans un projet, sous une copie du contrat ou sous un contrat du projet (chaque essai
 * rejoint l'opération qui sert son chemin). Le cœur range ; la notification propose « Annuler ».
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import type { ContractSummary } from "@/types/ipc";

const props = defineProps<{ open: boolean; projectId: string; contract: ContractSummary }>();
const emit = defineEmits<{ close: [] }>();

const catalog = useCatalogStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const ids = { project: useId(), contract: useId(), error: useId() };

/** Projets qui peuvent recevoir les essais : tous sauf « Non classés ». */
const projects = computed(() => catalog.projects.filter((p) => p.id !== props.projectId));
const target = ref("");
/** Contrat choisi dans le projet ; vide : une copie du contrat déduit. */
const into = ref("");
const error = ref<string | null>(null);
const busy = ref(false);

const count = computed(() =>
  props.contract.resources.reduce(
    (n, r) => n + r.operations.reduce((m, o) => m + o.requests.length, 0),
    props.contract.unplaced.length,
  ),
);
const targetContracts = computed(() =>
  (contracts.lists[target.value] ?? []).filter((c) => c.error === null),
);
const note = computed(() =>
  into.value === ""
    ? "Le contrat déduit et ses essais sont copiés tels quels dans le projet."
    : "Chaque essai rejoint l'opération qui sert son chemin ; son adresse prend {{baseUrl}}. Le contrat du projet n'est pas modifié ; un essai sans opération reste dans « Non classés ».",
);
const targetName = computed(() => projects.value.find((p) => p.id === target.value)?.name ?? "");

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    target.value = projects.value[0]?.id ?? "";
    into.value = "";
    error.value = null;
  },
  { immediate: true },
);
watch(target, (id) => {
  into.value = "";
  if (id !== "") void contracts.loadList(id).catch(() => undefined);
});

async function submit(): Promise<void> {
  if (busy.value || target.value === "") return;
  busy.value = true;
  error.value = null;
  const inbox = props.projectId;
  const to = target.value;
  const name = targetName.value;
  try {
    const filed = await contracts.fileTrials(
      inbox,
      props.contract.path,
      to,
      into.value === "" ? null : into.value,
    );
    emit("close");
    const n = filed.moved.length;
    const rest =
      filed.left.length > 0
        ? ` ; ${String(filed.left.length)} sans opération restent dans « Non classés »`
        : "";
    toasts.show(
      `${String(n)} essai${n > 1 ? "s" : ""} rangé${n > 1 ? "s" : ""} dans ${name} › ${filed.contractName}${rest}.`,
      {
        action: {
          label: "Annuler",
          run: () => contracts.undoFiling(inbox, to, filed.undoToken).catch(toasts.showError),
        },
      },
    );
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog
    :open="open"
    title="Ranger dans un projet"
    :description="`${String(count)} essai${count > 1 ? 's' : ''} de « ${contract.name} » quittent « Non classés ».`"
    @close="emit('close')"
  >
    <form class="filing" novalidate @submit.prevent="submit">
      <label :for="ids.project" class="filing__label">Projet</label>
      <select :id="ids.project" v-model="target" class="filing__select">
        <option v-for="project in projects" :key="project.id" :value="project.id">
          {{ project.name }}
        </option>
      </select>
      <label :for="ids.contract" class="filing__label">Contrat de service</label>
      <select :id="ids.contract" v-model="into" class="filing__select">
        <option value="">Nouveau contrat : copie de « {{ contract.name }} »</option>
        <option v-for="c in targetContracts" :key="c.path" :value="c.path">
          {{ c.name }}{{ c.deduced ? "" : ` v${c.version}` }}
        </option>
      </select>
      <p class="filing__note">{{ note }}</p>
      <p v-if="error" :id="ids.error" class="filing__error" role="alert">{{ error }}</p>
      <div class="filing__actions">
        <BaseButton @click="emit('close')">Annuler</BaseButton>
        <BaseButton variant="primary" type="submit" :disabled="busy || target === ''">
          Ranger
        </BaseButton>
      </div>
    </form>
  </BaseDialog>
</template>

<style scoped>
.filing {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: min(30rem, 80vw);
}

.filing__label {
  font-weight: 600;
}

.filing__select {
  min-height: var(--control-md);
}

.filing__note {
  margin: 0;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  line-height: 1.5;
  background: var(--color-chip);
}

.filing__error {
  margin: 0;
  color: var(--color-danger);
}

.filing__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
</style>
