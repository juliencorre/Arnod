<script setup lang="ts">
/**
 * Contrats d'un projet, sous ses requêtes dans la barre latérale (C1 à C4, C11).
 * Le « + » de la section propose d'importer un fichier, de coller un contrat, de le
 * lire à une adresse ou d'en créer un vide ; un contrat choisi s'ouvre en onglet.
 * Les lignes suivent la grille de la barre latérale (plan de la barre latérale du
 * projet, B5) ; `add` est exposé au menu « + » du projet.
 */
import { computed, onMounted, ref } from "vue";

import { CONTRACT_ADD_ITEMS } from "@/components/contracts/addItems";
import ContractDialog from "@/components/contracts/ContractDialog.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import SectionHeading from "@/components/ui/SectionHeading.vue";
import type { MenuItem } from "@/components/ui/menu";
import { useCatalogStore } from "@/stores/catalog";
import { type ContractDialogMode, useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ContractSource, ContractSummary } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const contracts = useContractsStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const list = computed(() => contracts.lists[props.projectId] ?? []);

const mode = ref<ContractDialogMode | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);

const ADD_ITEMS = CONTRACT_ADD_ITEMS;

onMounted(() => {
  if (!(props.projectId in contracts.lists)) {
    contracts.loadList(props.projectId).catch((caught: unknown) => {
      toasts.showError(caught);
    });
  }
});

function isSelected(contract: ContractSummary): boolean {
  const content = windows.activeTab?.content;
  return (
    content?.kind === "contract" &&
    content.projectId === props.projectId &&
    content.path === contract.path
  );
}

defineExpose({ add: onAdd });

function onAdd(id: string): void {
  if (id === "file") {
    void contracts.importFile(props.projectId);
    return;
  }
  error.value = null;
  mode.value = id as ContractDialogMode;
}

async function submit(source: ContractSource): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    const view = await contracts.add(props.projectId, source);
    mode.value = null;
    toasts.show(`Contrat « ${view.name} » ajouté.`);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}

function itemsFor(contract: ContractSummary): MenuItem[] {
  return [
    { id: "open", label: "Ouvrir" },
    { id: "run", label: "Exécuter le contrat", disabled: contract.error !== null },
    { id: "delete", label: "Mettre à la corbeille", tone: "danger", separatorBefore: true },
  ];
}

async function onItem(id: string, contract: ContractSummary): Promise<void> {
  try {
    if (id === "open") await contracts.show(props.projectId, contract.path);
    else if (id === "run") await workbench.runContract(props.projectId, contract.path);
    else if (id === "delete") await contracts.remove(props.projectId, contract.path, contract.name);
  } catch (caught) {
    toasts.showError(caught);
  }
}
</script>

<template>
  <section class="contracts" :aria-label="`Contrats de ${projectName}`">
    <SectionHeading icon="contracts" title="Contrats de service" :count="list.length">
      <BaseMenu :items="ADD_ITEMS" label="Ajouter un contrat" align="end" @select="onAdd">
        <template #trigger="{ attrs }">
          <button
            type="button"
            class="contracts__tool"
            v-bind="attrs"
            :aria-label="`Ajouter un contrat à ${projectName}`"
            :title="`Ajouter un contrat à ${projectName}`"
          >
            <svg width="14" height="14" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M6 2.5v7M2.5 6h7" />
            </svg>
          </button>
        </template>
      </BaseMenu>
    </SectionHeading>
    <ul v-if="list.length > 0" class="contracts__list">
      <li
        v-for="contract in list"
        :key="contract.path"
        class="contract-row"
        :class="{ 'contract-row--selected': isSelected(contract) }"
      >
        <button
          type="button"
          class="contract-row__main"
          :aria-current="isSelected(contract) ? 'page' : undefined"
          :title="contract.error ?? `${contract.operationCount} opération(s)`"
          @click="contracts.show(projectId, contract.path)"
        >
          <span class="contract-row__badge" aria-hidden="true">API</span>
          <span class="contract-row__name">{{ contract.name }}</span>
          <span v-if="contract.version && !contract.deduced" class="contract-row__version">{{
            contract.version
          }}</span>
          <span v-if="contract.error" class="contract-row__invalid" aria-label="illisible">⚠</span>
        </button>
        <BaseMenu
          :items="itemsFor(contract)"
          :label="`Actions pour le contrat ${contract.name}`"
          align="end"
          @select="(id) => onItem(id, contract)"
        >
          <template #trigger="{ attrs }">
            <button
              type="button"
              class="contract-row__more"
              v-bind="attrs"
              :aria-label="`Actions pour le contrat ${contract.name}`"
            >
              ⋯
            </button>
          </template>
        </BaseMenu>
      </li>
    </ul>
    <BaseMenu v-else :items="ADD_ITEMS" label="Ajouter un contrat" @select="onAdd">
      <template #trigger="{ attrs }">
        <button type="button" class="contracts__ghost" v-bind="attrs">
          <span class="contracts__lead" aria-hidden="true">+</span>
          Ajouter un contrat
        </button>
      </template>
    </BaseMenu>
    <ContractDialog
      :mode="mode"
      :error="error"
      :busy="busy"
      @close="mode = null"
      @submit="submit"
    />
  </section>
</template>

<style scoped>
.contracts {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.contracts__tool {
  display: grid;
  place-items: center;
  width: 1.625rem;
  height: 1.625rem;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: none;
  cursor: pointer;
}

.contracts__tool path {
  stroke: var(--color-text);
  stroke-width: 1.6;
  stroke-linecap: round;
}

.contracts__tool:hover,
.contracts__tool[aria-expanded="true"] {
  background: var(--color-chip);
}

.contracts__tool:focus-visible,
.contracts__ghost:focus-visible,
.contract-row__main:focus-visible,
.contract-row__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: 6px;
}

.contracts__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

/* Ligne d'ajout d'un projet sans contrat, sur la grille des lignes. */
.contracts__ghost {
  display: flex;
  align-items: center;
  gap: var(--tree-gap);
  width: 100%;
  min-height: 1.75rem;
  padding: 0 var(--tree-pad) 0 calc(var(--tree-pad) + var(--tree-indent));
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: left;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.contracts__ghost:hover {
  background: var(--color-accent-soft);
}

.contracts__lead {
  box-sizing: border-box;
  flex: none;
  width: var(--tree-lead);
  padding-left: 0.125rem;
}

.contract-row {
  display: flex;
  align-items: center;
  padding-left: calc(var(--tree-pad) + var(--tree-indent));
  border-radius: 6px;
}

.contract-row:hover,
.contract-row--selected {
  background: var(--color-accent-soft);
}

.contract-row__main {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--tree-gap);
  min-width: 0;
  min-height: 1.75rem;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.contract-row__badge {
  flex: none;
  width: var(--tree-lead);
  font: 700 0.6875rem var(--font-mono);
  color: var(--color-link);
}

.contract-row__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.contract-row__version {
  margin-left: auto;
  font: 0.6875rem var(--font-mono);
  color: var(--color-muted);
}

.contract-row__invalid {
  color: var(--color-danger);
}

.contract-row__more {
  border: 0;
  background: none;
  padding: 0.125rem 0.5rem;
  color: var(--color-muted);
  cursor: pointer;
}
</style>
