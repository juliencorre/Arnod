<script setup lang="ts">
/**
 * Contrats d'un projet, en tête de la barre latérale (C1 à C4, C11), et leurs requêtes
 * (docs/plan-requetes-dans-contrats.md, RC1 à RC5) : un contrat se déplie en ressources, une
 * ressource en opérations « MÉTHODE /chemin », une opération en requêtes liées ; « Hors contrat »
 * regroupe celles dont l'opération manque (R4). Un clic sur une opération ouvre son endpoint, ou
 * crée sa requête ; la requête de l'onglet actif est révélée (RC4) et, repliée, mise en évidence
 * sur sa ligne visible la plus proche (RC2). Le « + » de la section propose d'importer un
 * fichier, de coller un contrat, de le lire à une adresse ou d'en créer un vide. Les lignes
 * suivent la grille de la barre latérale (B5) ; `add` est exposé au menu « + » du projet.
 *
 * Lot D (docs/plan-mvp-d-barre-laterale.md) : la section se replie, vide elle l'est par défaut
 * (D1, R6). La ligne d'une opération est son endpoint (Q4) : le nombre de ses réponses attendues,
 * son statut et son menu ⋯ ; dépliée, elle liste ses réponses attendues par code, puis les autres
 * requêtes de l'opération (E1 à E3).
 */
import { computed, onMounted, ref, watch } from "vue";

import { CONTRACT_ADD_ITEMS } from "@/components/contracts/addItems";
import { useContractAdd } from "@/composables/useContractAdd";
import ContractDialog from "@/components/contracts/ContractDialog.vue";
import LinkedRequestRow from "@/components/contracts/LinkedRequestRow.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import SectionHeading from "@/components/ui/SectionHeading.vue";
import type { MenuItem } from "@/components/ui/menu";
import NameDialog from "@/components/workbench/NameDialog.vue";
import StatusCount from "@/components/workbench/StatusCount.vue";
import StatusMark from "@/components/workbench/StatusMark.vue";
import { countStatuses } from "@/lib/status";
import { useCatalogStore } from "@/stores/catalog";
import { type ContractNode, useContractsStore } from "@/stores/contracts";
import { useSidebarStore } from "@/stores/sidebar";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type {
  ContractSummary,
  ExampleRef,
  LinkedView,
  OperationNode,
  RequestStatus,
  ResourceView,
} from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const contracts = useContractsStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();

/** Dernière exécution de l'endpoint d'une opération, sa première requête (ST1, Q4 du lot D). */
function endpointStatus(operation: OperationNode): RequestStatus | null {
  const endpoint = operation.requests[0];
  return endpoint ? (workbench.statuses[props.projectId]?.[endpoint.path] ?? null) : null;
}

/** Réponses attendues de l'endpoint (E1). */
function examplesOf(operation: OperationNode): ExampleRef[] {
  return operation.requests[0]?.examples ?? [];
}

/** L'opération se déplie : réponses attendues de l'endpoint, ou autres requêtes (E2). */
function expandable(operation: OperationNode): boolean {
  return examplesOf(operation).length > 0 || operation.requests.length > 1;
}

/** Famille d'un code, pour sa couleur (comme `ExpectedResponses`). */
function family(status: number | null): string {
  return status === null ? "none" : `${String(Math.floor(status / 100))}xx`;
}

function exampleLabel(example: ExampleRef): string {
  return `${example.status === null ? "" : String(example.status)} ${example.name}`.trim();
}

/** Réponse ouverte depuis la barre latérale, sur l'endpoint de l'onglet actif (E3). */
function exampleShown(operation: OperationNode, example: ExampleRef): boolean {
  const endpoint = operation.requests[0];
  const shown = sidebar.shownExample;
  return (
    endpoint !== undefined &&
    endpoint.path === activeRequest.value &&
    shown?.key === requestKey(props.projectId, endpoint.path) &&
    shown.index === example.index
  );
}

/** Ouvre l'endpoint sur ses réponses attendues, la réponse choisie (E3). */
async function openExample(operation: OperationNode, example: ExampleRef): Promise<void> {
  const endpoint = operation.requests[0];
  if (endpoint) await sidebar.openExample(props.projectId, endpoint.path, example.index);
}
const toasts = useToastStore();
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const list = computed(() => contracts.lists[props.projectId] ?? []);

const sidebar = useSidebarStore();
const adding = useContractAdd(() => props.projectId);
const { mode, error, busy } = adding;
/** Requête liée en cours de renommage (RC5). */
const renaming = ref<LinkedView | null>(null);
const renameError = ref<string | null>(null);

const ADD_ITEMS = CONTRACT_ADD_ITEMS;

/** Section dépliée : vide, repliée par défaut (D1, R6). */
const open = computed(() =>
  sidebar.isSectionOpen(props.projectId, "contracts", list.value.length === 0),
);

function toggleSection(): void {
  sidebar.setSectionOpen(props.projectId, "contracts", !open.value);
}

onMounted(() => {
  if (!(props.projectId in contracts.lists)) {
    contracts.loadList(props.projectId).catch((caught: unknown) => {
      toasts.showError(caught);
    });
  }
});

// --- Dépliage et sélection --------------------------------------------------

function contractNode(contract: ContractSummary): ContractNode {
  return { kind: "contract", path: contract.path };
}

function resourceNode(contract: ContractSummary, resource: ResourceView): ContractNode {
  return { kind: "resource", path: contract.path, resource: resource.path };
}

function operationNode(contract: ContractSummary, operation: OperationNode): ContractNode {
  return { kind: "operation", path: contract.path, operation: operation.key };
}

function unplacedNode(contract: ContractSummary): ContractNode {
  return { kind: "unplaced", path: contract.path };
}

function isOpen(node: ContractNode): boolean {
  return contracts.isExpanded(props.projectId, node);
}

function toggle(node: ContractNode): void {
  contracts.toggleExpanded(props.projectId, node);
}

/** Requête de ce projet affichée dans l'onglet actif de la fenêtre. */
const activeRequest = computed(() => {
  const content = windows.activeTab?.content;
  return content?.kind === "request" && content.projectId === props.projectId ? content.path : null;
});

function holds(requests: LinkedView[]): boolean {
  return activeRequest.value !== null && requests.some((r) => r.path === activeRequest.value);
}

function operationHolds(operation: OperationNode): boolean {
  return holds(operation.requests);
}

/**
 * Ligne de l'opération mise en évidence : son endpoint est la requête active (la ligne est
 * l'endpoint, Q4 du lot D), ou elle est repliée sur la requête active (RC2).
 */
function operationSelected(contract: ContractSummary, operation: OperationNode): boolean {
  const endpoint = operation.requests[0];
  if (endpoint !== undefined && endpoint.path === activeRequest.value) return true;
  return !isOpen(operationNode(contract, operation)) && operationHolds(operation);
}

function resourceHolds(resource: ResourceView): boolean {
  return resource.operations.some(operationHolds);
}

function contractHolds(contract: ContractSummary): boolean {
  return holds(contract.unplaced) || contract.resources.some(resourceHolds);
}

/** Onglet du contrat actif, ou contrat replié sur la requête active (RC2). */
function contractSelected(contract: ContractSummary): boolean {
  const content = windows.activeTab?.content;
  const tab =
    content?.kind === "contract" &&
    content.projectId === props.projectId &&
    content.path === contract.path;
  return tab || (!isOpen(contractNode(contract)) && contractHolds(contract));
}

// Requête active ouverte ailleurs (onglet du contrat, autre fenêtre, essai) : son contrat se
// déplie, une fois la liste à jour si elle ne l'est pas encore (RC4).
let pendingReveal: string | null = null;

function tryReveal(): void {
  if (pendingReveal !== null && contracts.reveal(props.projectId, pendingReveal)) {
    pendingReveal = null;
    if (!open.value) sidebar.setSectionOpen(props.projectId, "contracts", true);
  }
}

watch(
  activeRequest,
  (path) => {
    pendingReveal = path;
    tryReveal();
  },
  { immediate: true },
);
watch(list, tryReveal);

// --- Actions -----------------------------------------------------------------

defineExpose({ add: onAdd });

function onAdd(id: string): void {
  if (!open.value) sidebar.setSectionOpen(props.projectId, "contracts", true);
  adding.choose(id);
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

/** Une requête peut être créée pour l'opération (méthode envoyable, contrat lisible). */
function creatable(contract: ContractSummary, operation: OperationNode): boolean {
  return operation.supported && contract.error === null;
}

/**
 * Ouvre l'endpoint de l'opération (RC2), en aperçu sur un simple clic (lot F, 1.3-a), ou crée
 * sa requête (RC3).
 */
async function onOperation(
  contract: ContractSummary,
  operation: OperationNode,
  preview = true,
): Promise<void> {
  const endpoint = operation.requests[0];
  if (endpoint) await workbench.openRequest(props.projectId, endpoint.path, preview);
  else if (preview && creatable(contract, operation)) {
    await contracts.createForOperation(props.projectId, contract.path, operation.key);
  }
}

function operationLabel(contract: ContractSummary, operation: OperationNode): string {
  const what = `${operation.method} ${operation.path}`;
  if (operation.requests.length > 0) return `Ouvrir ${what}`;
  return creatable(contract, operation) ? `Créer la requête de ${what}` : what;
}

/** Actions de l'endpoint, sur la ligne de son opération (RC5, Q4 du lot D). */
const ENDPOINT_ITEMS: MenuItem[] = [
  { id: "rename", label: "Renommer" },
  { id: "duplicate", label: "Dupliquer" },
  { id: "delete", label: "Mettre à la corbeille", tone: "danger", separatorBefore: true },
];

async function onEndpoint(id: string, operation: OperationNode): Promise<void> {
  const endpoint = operation.requests[0];
  if (endpoint) await onRequest(id, endpoint);
}

/** Actions d'une requête liée (RC5) : celles de l'arborescence. */
async function onRequest(id: string, request: LinkedView): Promise<void> {
  try {
    if (id === "rename") {
      renameError.value = null;
      renaming.value = request;
    } else if (id === "duplicate") await workbench.duplicate(props.projectId, request.path);
    else if (id === "delete") {
      await workbench.remove(props.projectId, request.path, false);
      toasts.show(`« ${request.name} » mis à la corbeille.`);
    }
  } catch (caught) {
    toasts.showError(caught);
  }
}

async function rename(name: string): Promise<void> {
  const request = renaming.value;
  if (request === null) return;
  try {
    await workbench.rename(props.projectId, request.path, name);
    renaming.value = null;
  } catch (caught) {
    renameError.value = caught instanceof Error ? caught.message : String(caught);
  }
}
</script>

<template>
  <section class="contracts" :aria-label="`Contrats de ${projectName}`">
    <SectionHeading
      icon="contracts"
      title="Contrats de service"
      :count="list.length"
      :expanded="open"
      @toggle="toggleSection"
    >
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
    <ul v-if="open && list.length > 0" class="contracts__list">
      <li v-for="contract in list" :key="contract.path" class="contracts__item">
        <div class="contract-row" :class="{ 'contract-row--selected': contractSelected(contract) }">
          <button
            type="button"
            class="contract-row__chevron"
            :aria-expanded="isOpen(contractNode(contract))"
            :aria-label="`${isOpen(contractNode(contract)) ? 'Replier' : 'Déplier'} le contrat ${contract.name}`"
            @click="toggle(contractNode(contract))"
          >
            <span aria-hidden="true">{{ isOpen(contractNode(contract)) ? "▾" : "▸" }}</span>
          </button>
          <button
            type="button"
            class="contract-row__main"
            :aria-current="contractSelected(contract) ? 'page' : undefined"
            :title="contract.error ?? `${contract.operationCount} opération(s)`"
            @click="contracts.show(projectId, contract.path, true)"
            @dblclick="contracts.show(projectId, contract.path)"
          >
            <span class="contract-row__badge" aria-hidden="true">API</span>
            <span class="contract-row__name">{{ contract.name }}</span>
            <span v-if="contract.version && !contract.deduced" class="contract-row__version">{{
              contract.version
            }}</span>
            <span v-if="contract.error" class="contract-row__invalid" aria-label="illisible"
              >⚠</span
            >
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
        </div>
        <ul
          v-if="isOpen(contractNode(contract))"
          class="contracts__group"
          :aria-label="`Ressources du contrat ${contract.name}`"
        >
          <li v-for="resource in contract.resources" :key="resource.path">
            <div
              class="crow"
              :class="{
                'crow--selected':
                  !isOpen(resourceNode(contract, resource)) && resourceHolds(resource),
              }"
              style="--depth: 2"
            >
              <button
                type="button"
                class="crow__main crow__resource"
                :aria-expanded="isOpen(resourceNode(contract, resource))"
                @click="toggle(resourceNode(contract, resource))"
              >
                <span class="crow__lead" aria-hidden="true">
                  <span class="crow__chevron">{{
                    isOpen(resourceNode(contract, resource)) ? "▾" : "▸"
                  }}</span>
                  <svg class="crow__folder" width="14" height="14" viewBox="0 0 14 14">
                    <path d="M1.5 3.5h4l1.2 1.4h5.8v6.6h-11z" />
                  </svg>
                </span>
                <span class="crow__name">{{ resource.path }}</span>
                <StatusCount
                  class="crow__count"
                  :count="
                    countStatuses(
                      resource.operations.flatMap((o) => o.requests.map((r) => r.path)),
                      workbench.statuses[projectId],
                    )
                  "
                  :fallback="resource.operations.length"
                />
              </button>
            </div>
            <ul v-if="isOpen(resourceNode(contract, resource))" class="contracts__group">
              <li v-for="operation in resource.operations" :key="operation.key">
                <div
                  class="crow crow--operation"
                  :class="{
                    'crow--selected': operationSelected(contract, operation),
                  }"
                  style="--depth: 2"
                >
                  <button
                    v-if="expandable(operation)"
                    type="button"
                    class="crow__toggle"
                    :aria-expanded="isOpen(operationNode(contract, operation))"
                    :aria-label="`${isOpen(operationNode(contract, operation)) ? 'Replier' : 'Déplier'} ${operation.method} ${operation.path}`"
                    @click="toggle(operationNode(contract, operation))"
                  >
                    <span aria-hidden="true">{{
                      isOpen(operationNode(contract, operation)) ? "▾" : "▸"
                    }}</span>
                  </button>
                  <span v-else class="crow__toggle" aria-hidden="true" />
                  <button
                    type="button"
                    class="crow__main"
                    :class="{ 'crow__main--empty': operation.requests.length === 0 }"
                    :aria-label="operationLabel(contract, operation)"
                    :title="operation.summary ?? `${operation.method} ${operation.path}`"
                    :disabled="operation.requests.length === 0 && !creatable(contract, operation)"
                    @click="onOperation(contract, operation)"
                    @dblclick="
                      operation.requests.length > 0 && onOperation(contract, operation, false)
                    "
                  >
                    <span
                      class="crow__lead crow__method"
                      :class="`crow__method--${operation.method.toLowerCase()}`"
                      aria-hidden="true"
                    >
                      {{ operation.method }}
                    </span>
                    <span class="crow__name" aria-hidden="true">{{ operation.path }}</span>
                    <span
                      v-if="operation.requests.length === 0 && creatable(contract, operation)"
                      class="crow__hint"
                      aria-hidden="true"
                      >Créer</span
                    >
                    <template v-else-if="operation.requests.length > 0">
                      <span
                        v-if="examplesOf(operation).length > 0"
                        class="crow__examples"
                        :title="`${String(examplesOf(operation).length)} réponse(s) attendue(s)`"
                        aria-hidden="true"
                        >{{ examplesOf(operation).length }}</span
                      >
                      <StatusMark
                        class="crow__mark"
                        aria-hidden="true"
                        :status="endpointStatus(operation)"
                      />
                    </template>
                  </button>
                  <BaseMenu
                    v-if="operation.requests[0]"
                    :items="ENDPOINT_ITEMS"
                    :label="`Actions pour ${operation.requests[0].name}`"
                    align="end"
                    @select="(id) => onEndpoint(id, operation)"
                  >
                    <template #trigger="{ attrs }">
                      <button
                        type="button"
                        class="crow__more"
                        v-bind="attrs"
                        :aria-label="`Actions pour ${operation.requests[0].name}`"
                      >
                        ⋯
                      </button>
                    </template>
                  </BaseMenu>
                </div>
                <ul
                  v-if="expandable(operation) && isOpen(operationNode(contract, operation))"
                  class="contracts__group"
                  :aria-label="`Réponses attendues et requêtes de ${operation.method} ${operation.path}`"
                >
                  <li v-for="example in examplesOf(operation)" :key="`e${example.index}`">
                    <div
                      class="crow"
                      :class="{ 'crow--shown': exampleShown(operation, example) }"
                      style="--depth: 3"
                    >
                      <button
                        type="button"
                        class="crow__main crow__example"
                        :aria-label="`Ouvrir la réponse attendue ${exampleLabel(example)}`"
                        @click="openExample(operation, example)"
                      >
                        <span
                          class="crow__lead crow__code"
                          :class="`crow__code--${family(example.status)}`"
                          aria-hidden="true"
                          >{{ example.status ?? "—" }}</span
                        >
                        <span class="crow__name" aria-hidden="true">{{
                          example.name || "Sans nom"
                        }}</span>
                      </button>
                    </div>
                  </li>
                  <LinkedRequestRow
                    v-for="request in operation.requests.slice(1)"
                    :key="request.path"
                    :request="request"
                    :project-id="projectId"
                    :depth="3"
                    @action="onRequest"
                  />
                </ul>
              </li>
            </ul>
          </li>
          <li v-if="contract.unplaced.length > 0">
            <div
              class="crow"
              :class="{
                'crow--selected': !isOpen(unplacedNode(contract)) && holds(contract.unplaced),
              }"
              style="--depth: 2"
            >
              <button
                type="button"
                class="crow__main crow__resource"
                :aria-expanded="isOpen(unplacedNode(contract))"
                :title="
                  contract.error
                    ? 'Contrat illisible : ses requêtes ne sont pas rangées.'
                    : 'Requêtes dont l\'opération n\'est plus dans le contrat.'
                "
                @click="toggle(unplacedNode(contract))"
              >
                <span class="crow__lead" aria-hidden="true">
                  <span class="crow__chevron">{{
                    isOpen(unplacedNode(contract)) ? "▾" : "▸"
                  }}</span>
                  <svg class="crow__folder" width="14" height="14" viewBox="0 0 14 14">
                    <path d="M1.5 3.5h4l1.2 1.4h5.8v6.6h-11z" />
                  </svg>
                </span>
                <span class="crow__name">Hors contrat</span>
                <StatusCount
                  class="crow__count"
                  :count="
                    countStatuses(
                      contract.unplaced.map((r) => r.path),
                      workbench.statuses[projectId],
                    )
                  "
                  :fallback="contract.unplaced.length"
                />
              </button>
            </div>
            <ul v-if="isOpen(unplacedNode(contract))" class="contracts__group">
              <LinkedRequestRow
                v-for="request in contract.unplaced"
                :key="request.path"
                :request="request"
                :project-id="projectId"
                :depth="3"
                show-method
                @action="onRequest"
              />
            </ul>
          </li>
        </ul>
      </li>
    </ul>
    <BaseMenu v-else-if="open" :items="ADD_ITEMS" label="Ajouter un contrat" @select="onAdd">
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
      @close="adding.close"
      @submit="adding.submit"
    />
    <NameDialog
      :open="renaming !== null"
      title="Renommer"
      label="Nouveau nom"
      :initial="renaming?.name ?? ''"
      submit-label="Renommer"
      :error="renameError"
      @close="renaming = null"
      @submit="rename"
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
  border-radius: var(--radius-md);
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
.contract-row__chevron:focus-visible,
.contract-row__main:focus-visible,
.contract-row__more:focus-visible,
.crow__toggle:focus-visible,
.crow__main:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: var(--radius-md);
}

.contracts__list,
.contracts__group {
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
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  font-weight: 400;
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
  border-radius: var(--radius-md);
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
  font-size: var(--text-body);
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

/* Colonne de tête du contrat : son chevron (la largeur d'un retrait), puis « API ». */
.contract-row__chevron {
  flex: none;
  width: var(--tree-indent);
  min-height: 1.75rem;
  padding: 0 0.375rem 0 0;
  border: none;
  font: inherit;
  font-size: var(--text-small);
  text-align: center;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.contract-row__badge {
  flex: none;
  width: calc(var(--tree-lead) - var(--tree-indent));
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-link);
}

.contract-row__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.contract-row__version {
  margin-left: auto;
  font: var(--text-caption) var(--font-mono);
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

/* Ressources, opérations et groupe « Hors contrat » : même grille que l'arborescence. Une
   opération porte son chevron dans le retrait qui précède sa méthode. */
.crow {
  display: flex;
  align-items: center;
  padding-left: calc(var(--tree-pad) + var(--tree-indent) * var(--depth));
  padding-right: 0.5rem;
  border-radius: var(--radius-md);
}

.crow:hover,
.crow--selected {
  background: var(--color-accent-soft);
}

.crow__toggle {
  flex: none;
  align-self: stretch;
  width: var(--tree-indent);
  padding: 0;
  border: none;
  font: inherit;
  font-size: var(--text-small);
  text-align: center;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.crow__main {
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
  font-size: var(--text-body);
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.crow__main:disabled {
  cursor: default;
}

/* Opération sans requête : estompée, « Créer » au survol (RC3). */
.crow__main--empty {
  color: var(--color-muted);
}

.crow__lead {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.375rem;
  width: var(--tree-lead);
}

.crow__chevron {
  width: 0.875rem;
  font-size: var(--text-small);
  text-align: center;
  color: var(--color-muted);
}

.crow__folder {
  flex: none;
}

.crow__folder path {
  fill: var(--color-chip);
  stroke: var(--color-subtle);
  stroke-width: 1;
}

.crow__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.crow__mark {
  margin-left: auto;
}

/* Nombre de réponses attendues de l'endpoint (E1). */
.crow__examples {
  margin-left: auto;
  font: var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.crow__examples + .crow__mark {
  margin-left: 0.375rem;
}

/* ⋯ de l'endpoint : dans l'ordre de tabulation, visible au survol ou au focus ; caché, il ne
   prend aucune largeur (comme les actions d'un projet), le chemin garde sa place. */
.crow__more {
  flex: none;
  width: 0;
  overflow: hidden;
  opacity: 0;
  border: 0;
  background: none;
  padding: 0.125rem 0;
  color: var(--color-muted);
  cursor: pointer;
}

.crow:hover .crow__more,
.crow:focus-within .crow__more,
.crow__more[aria-expanded="true"] {
  width: 1.5rem;
  opacity: 1;
}

/* Réponse attendue : code coloré par famille (comme ExpectedResponses), puis son nom (E2). */
.crow__code {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.crow__code--2xx {
  color: var(--color-success);
}

.crow__code--3xx {
  color: var(--color-link);
}

.crow__code--4xx,
.crow__code--5xx {
  color: var(--color-danger);
}

.crow--shown {
  background: var(--color-chip);
}

.crow__example {
  font-size: var(--text-small);
}

.crow__count {
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.crow__hint {
  margin-left: auto;
  font-size: var(--text-small);
  font-weight: 400;
  color: var(--color-link);
  opacity: 0;
}

.crow:hover .crow__hint,
.crow__main:focus-visible .crow__hint {
  opacity: 1;
}

.crow__method {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.crow__main--empty .crow__method {
  opacity: 0.7;
}

.crow__method--get {
  color: var(--color-success);
}

.crow__method--post,
.crow__method--put,
.crow__method--patch {
  color: var(--color-link);
}

.crow__method--delete {
  color: var(--color-danger);
}
</style>
