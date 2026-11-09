<script setup lang="ts">
/**
 * Contrats d'un projet, sous ses requêtes dans la barre latérale (C1 à C4, C11).
 * « + Contrat » propose d'importer un fichier, de coller un contrat, de le lire
 * à une adresse ou d'en créer un vide ; un contrat choisi s'ouvre en onglet.
 */
import { computed, onMounted, ref } from "vue";

import ContractDialog from "@/components/contracts/ContractDialog.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
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

const ADD_ITEMS: MenuItem[] = [
  { id: "file", label: "Importer un fichier…", hint: "OpenAPI 3 ou Swagger 2, JSON ou YAML" },
  { id: "text", label: "Coller un contrat…" },
  { id: "url", label: "Depuis une adresse…" },
  { id: "template", label: "Nouveau contrat…", separatorBefore: true },
];

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
    <div class="contracts__head">
      <h3 class="contracts__title">Contrats</h3>
      <BaseMenu :items="ADD_ITEMS" label="Ajouter un contrat" @select="onAdd">
        <template #trigger="{ attrs }">
          <button
            type="button"
            class="contracts__tool"
            v-bind="attrs"
            :aria-label="`Ajouter un contrat à ${projectName}`"
          >
            + Contrat
          </button>
        </template>
      </BaseMenu>
    </div>
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
          <span v-if="contract.version" class="contract-row__version">{{ contract.version }}</span>
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
    <p v-else class="contracts__empty">Aucun contrat : « + Contrat » en ajoute un.</p>
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
  gap: 0.25rem;
  margin-top: 0.5rem;
  min-width: 0;
}

.contracts__head {
  display: flex;
  align-items: baseline;
  gap: 0.625rem;
  padding: 0 0.25rem;
}

.contracts__title {
  margin: 0;
  font-size: 0.6875rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.contracts__tool {
  padding: 0.125rem 0;
  border: none;
  font: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.contracts__tool:focus-visible,
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

.contracts__empty {
  margin: 0.25rem 0.25rem 0.5rem;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.contract-row {
  display: flex;
  align-items: center;
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
  gap: 0.375rem;
  min-width: 0;
  padding: 0.3125rem 0.25rem 0.3125rem 0.5rem;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.contract-row__badge {
  min-width: 2.5rem;
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
