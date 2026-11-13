<script setup lang="ts">
/**
 * Un projet dans la barre latérale (docs/plan-barre-laterale-projet.md, B1 à B6) : une ligne
 * pour le projet et ses actions (▶ exécuter, + ajouter, ⋯ autres), visibles au survol comme au
 * focus ; sous le nom, la pastille d'environnement ; puis ses contrats, avec leurs requêtes, et
 * son référentiel de tests (docs/plan-requetes-dans-contrats.md, RC6). Replié, le projet montre
 * son environnement à droite de son nom.
 *
 * Grille commune (`--tree-*`, défini par `AppSidebar`) : le chevron du projet, puis sa pastille
 * au retrait d'un niveau, le nom après la colonne de tête ; la pastille d'environnement
 * s'aligne sur le nom, les lignes du projet sur sa pastille.
 */
import { computed, nextTick, useTemplateRef } from "vue";

import { CONTRACT_ADD_ITEMS } from "@/components/contracts/addItems";
import ContractsSection from "@/components/contracts/ContractsSection.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import TrayIcon from "@/components/ui/TrayIcon.vue";
import type { MenuItem } from "@/components/ui/menu";
import EnvironmentChip from "@/components/workbench/EnvironmentChip.vue";
import TreePanel from "@/components/workbench/TreePanel.vue";
import { ALL_PROJECTS, useProjectActions } from "@/composables/useProjectActions";
import { startProjectDrag } from "@/lib/dnd";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ProjectView, TreeNode } from "@/types/ipc";

const props = defineProps<{ project: ProjectView; workspaceId: string | null; current: boolean }>();

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const workbench = useWorkbenchStore();
const windows = useWindowsStore();
const toasts = useToastStore();
const actions = useProjectActions();
const network = useNetworkStore();

const tree = useTemplateRef<InstanceType<typeof TreePanel>>("tree");
const contracts = useTemplateRef<InstanceType<typeof ContractsSection>>("contracts");
const environment = useTemplateRef<InstanceType<typeof EnvironmentChip>>("environment");

const id = computed(() => props.project.id);
const name = computed(() => props.project.name);
const expanded = computed(() => workbench.isExpanded(id.value));
const missing = computed(() => props.project.status === "missing");
/** « Non classés » : icône de bac, jamais glissé vers un workspace (N3, R5). */
const inbox = computed(() => catalog.isInbox(id.value));

/** Nombre de requêtes du projet, une fois son arborescence lue. */
const requestCount = computed(() => {
  const count = (nodes: TreeNode[]): number =>
    nodes.reduce((sum, node) => sum + (node.kind === "folder" ? count(node.children) : 1), 0);
  const nodes = workbench.trees[id.value]?.nodes;
  return nodes ? count(nodes) : null;
});

const runLabel = computed(() => {
  const env = workbench.activeEnvironment(id.value);
  const what =
    requestCount.value === null
      ? `le projet ${name.value}`
      : countLabel(requestCount.value, "requête", "requêtes");
  return env === null ? `Exécuter ${what}` : `Exécuter ${what} sur ${env}`;
});

/** Menu « + » du projet (1.2-a ter, M1) : ce que le projet sait créer, avec ses raccourcis. */
const ADD_ITEMS: MenuItem[] = [
  { id: "request", label: "Nouvelle requête", shortcut: "Ctrl+N" },
  { id: "folder", label: "Nouveau dossier", shortcut: "Ctrl+Maj+N" },
  {
    id: "contract",
    label: "Ajouter un contrat",
    separatorBefore: true,
    children: CONTRACT_ADD_ITEMS.map((item) => ({ ...item, id: `contract:${item.id}` })),
  },
  { id: "environment", label: "Nouvel environnement…" },
];

const moreItems = computed<MenuItem[]>(() => {
  const items: MenuItem[] = [
    { id: "open", label: "Ouvrir le projet" },
    { id: "environments", label: "Gérer les environnements…", shortcut: "Ctrl+Maj+E" },
    // Lot H : réglages du projet, authentification (3.4-a).
    { id: "settings", label: "Réglages du projet…" },
  ];
  const workspace = props.workspaceId === null ? null : catalog.workspaceById(props.workspaceId);
  if (workspace) {
    items.push(
      { id: "reveal", label: `Afficher dans « ${ALL_PROJECTS} »`, separatorBefore: true },
      {
        id: "remove",
        label: `Retirer de « ${workspace.name} »`,
        hint: `Le projet reste dans « ${ALL_PROJECTS} ».`,
        tone: "danger",
      },
    );
  }
  return items;
});

/** Déplie le projet (ses environnements et son arborescence sont lus) ; `false` si impossible. */
async function ensureExpanded(): Promise<boolean> {
  if (!expanded.value) {
    try {
      await workbench.expand(id.value);
    } catch (error) {
      workbench.collapse(id.value);
      toasts.showError(error);
      return false;
    }
  }
  await nextTick();
  return true;
}

async function toggle(): Promise<void> {
  if (expanded.value) workbench.collapse(id.value);
  else await ensureExpanded();
}

/** Détachée, la barre n'a pas de vue projet : le projet se déplie (F9). */
async function choose(): Promise<void> {
  if (windows.kind === "sidebar") {
    await ensureExpanded();
    return;
  }
  await actions.openProject(id.value, props.workspaceId);
}

async function run(): Promise<void> {
  if (!(await ensureExpanded())) return;
  try {
    await workbench.runFolder(id.value, "");
  } catch (error) {
    toasts.showError(error);
  }
}

async function onAdd(item: string): Promise<void> {
  if (!(await ensureExpanded())) return;
  if (item === "request") tree.value?.newRequest();
  else if (item === "folder") tree.value?.newFolder();
  else if (item === "environment") environment.value?.manage(true);
  else if (item.startsWith("contract:")) contracts.value?.add(item.slice("contract:".length));
}

async function onMore(item: string): Promise<void> {
  if (item === "open") await actions.openProject(id.value, props.workspaceId);
  else if (item === "environments") {
    if (await ensureExpanded()) environment.value?.manage();
  } else if (item === "settings") await openSettings();
  else if (item === "reveal") await navigation.showAll(id.value);
  else if (item === "remove" && props.workspaceId !== null) {
    await actions.removeFromWorkspace(id.value, props.workspaceId);
  }
}

/** Lot H : réglages du projet sur son environnement actif (3.4-a). */
async function openSettings(): Promise<void> {
  if (!(await ensureExpanded())) return;
  const env = workbench.activeEnvironment(id.value);
  if (env === null) {
    toasts.show("Créez d'abord un environnement : les réglages s'appliquent à l'un d'eux.");
    return;
  }
  await network.openProjectSettings(id.value, env).catch(toasts.showError);
}

/** Ctrl+F : focus dans le filtre du référentiel (lot D, F5). */
async function focusFilter(): Promise<boolean> {
  if (!(await ensureExpanded())) return false;
  return (await tree.value?.focusFilter()) ?? false;
}

/** Raccourcis de la barre latérale : Ctrl+N, Ctrl+Maj+N, Ctrl+Maj+E (B6, M2) et Ctrl+F (F5). */
defineExpose({
  newRequest: () => onAdd("request"),
  newFolder: () => onAdd("folder"),
  manageEnvironments: () => onMore("environments"),
  focusFilter,
});
</script>

<template>
  <li class="project">
    <div class="project__row" :class="{ 'project__row--current': current }">
      <button
        type="button"
        class="project__chevron"
        :aria-expanded="expanded"
        :aria-label="`${expanded ? 'Replier' : 'Déplier'} ${name}`"
        :disabled="missing"
        @click="toggle"
      >
        <span aria-hidden="true">{{ expanded ? "▾" : "▸" }}</span>
      </button>
      <button
        type="button"
        class="project__main"
        :aria-current="current ? 'page' : undefined"
        :draggable="!inbox"
        @dragstart="startProjectDrag($event, id)"
        @click="choose"
      >
        <span class="project__initial" aria-hidden="true">
          <TrayIcon v-if="inbox" />
          <template v-else>{{ name.charAt(0).toUpperCase() }}</template>
        </span>
        <span class="project__name">{{ name }}</span>
        <span v-if="missing" class="project__missing" title="Dossier introuvable">
          <span aria-hidden="true">!</span>
          <span class="visually-hidden">(dossier introuvable)</span>
        </span>
      </button>
      <span v-if="!expanded && !missing" class="project__env-compact">
        <EnvironmentChip :project-id="id" compact />
      </span>
      <span v-if="expanded && !missing" class="project__actions">
        <button
          type="button"
          class="project__tool"
          :aria-label="runLabel"
          :title="runLabel"
          @click="run"
        >
          <svg width="14" height="14" viewBox="0 0 12 12" aria-hidden="true">
            <path class="project__fill" d="M4 2.8v6.4L9.2 6z" />
          </svg>
        </button>
        <BaseMenu
          :items="ADD_ITEMS"
          :label="`Ajouter au projet ${name}`"
          align="end"
          @select="onAdd"
        >
          <template #trigger="{ attrs }">
            <button
              v-bind="attrs"
              type="button"
              class="project__tool"
              :aria-label="`Ajouter au projet ${name}`"
              :title="`Ajouter au projet ${name}`"
            >
              <svg width="14" height="14" viewBox="0 0 12 12" aria-hidden="true">
                <path class="project__stroke" d="M6 2.5v7M2.5 6h7" />
              </svg>
            </button>
          </template>
        </BaseMenu>
        <BaseMenu
          :items="moreItems"
          :label="`Plus d'actions sur ${name}`"
          align="end"
          @select="onMore"
        >
          <template #trigger="{ attrs }">
            <button
              v-bind="attrs"
              type="button"
              class="project__tool"
              :aria-label="`Plus d'actions sur ${name}`"
              :title="`Plus d'actions sur ${name}`"
            >
              <svg width="14" height="14" viewBox="0 0 12 12" aria-hidden="true">
                <circle class="project__fill" cx="2.6" cy="6" r="1.1" />
                <circle class="project__fill" cx="6" cy="6" r="1.1" />
                <circle class="project__fill" cx="9.4" cy="6" r="1.1" />
              </svg>
            </button>
          </template>
        </BaseMenu>
      </span>
    </div>
    <div v-if="expanded" class="project__content">
      <div class="project__env">
        <EnvironmentChip ref="environment" :project-id="id" />
      </div>
      <ContractsSection ref="contracts" :project-id="id" />
      <TreePanel ref="tree" :project-id="id" />
    </div>
  </li>
</template>

<style scoped>
.project__row {
  display: flex;
  align-items: center;
  gap: 0;
  min-height: 2rem;
  padding: 0 calc(var(--tree-pad) - 0.125rem) 0 var(--tree-pad);
  border-radius: var(--radius-md);
}

.project__row:hover,
.project__row--current {
  background: var(--color-surface);
}

.project__chevron {
  flex: none;
  width: var(--tree-indent);
  padding: 0.375rem 0.375rem 0.375rem 0;
  border: none;
  border-radius: var(--radius-sm);
  font: inherit;
  font-size: var(--text-small);
  text-align: center;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.project__chevron:disabled {
  cursor: default;
  color: var(--color-subtle);
}

.project__main {
  box-sizing: border-box;
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--tree-gap);
  min-width: 0;
  padding: 0.375rem 0;
  border: none;
  font: inherit;
  font-size: var(--text-body);
  font-weight: 600;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.project__initial {
  display: grid;
  flex: none;
  place-items: center;
  width: calc(var(--tree-lead) - var(--tree-indent));
  height: 1.25rem;
  border-radius: var(--radius-sm);
  font: 600 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
  background: var(--color-chip);
}

.project__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project__missing {
  font-weight: 700;
  color: var(--color-danger);
}

/* EnvironmentChip a deux racines (menu et dialogue) : l'espacement passe par ce conteneur. */
.project__env-compact {
  display: flex;
  flex: 0 1 auto;
  min-width: 0;
  max-width: 7.5rem;
  margin-left: 0.5rem;
}

.project__env-compact :deep(.menu) {
  min-width: 0;
  max-width: 100%;
}

/* Actions : présentes dans l'ordre de tabulation, visibles au survol ou au focus. Cachées, elles
   ne prennent aucune largeur : le nom d'un projet n'est pas tronqué pour rien. Le repli porte
   sur les boutons, pas sur leur conteneur, qui couperait les menus ouverts. */
.project__actions {
  display: flex;
  flex: none;
  align-items: center;
}

.project__tool {
  display: grid;
  flex: none;
  place-items: center;
  width: 0;
  height: 1.5rem;
  overflow: hidden;
  opacity: 0;
  padding: 0;
  border: none;
  border-radius: var(--radius-md);
  background: none;
  cursor: pointer;
}

.project__row:hover .project__actions,
.project__row:focus-within .project__actions,
.project__row--current .project__actions {
  margin-left: 0.25rem;
}

.project__row:hover .project__tool,
.project__row:focus-within .project__tool,
.project__row--current .project__tool {
  width: 1.5rem;
  opacity: 1;
}

.project__tool:hover,
.project__tool[aria-expanded="true"] {
  background: var(--color-chip);
}

.project__fill {
  fill: var(--color-text);
}

.project__stroke {
  stroke: var(--color-text);
  stroke-width: 1.6;
  stroke-linecap: round;
}

.project__chevron:focus-visible,
.project__main:focus-visible,
.project__tool:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.project__content {
  display: flex;
  flex-direction: column;
  margin: 0.125rem 0 0.75rem;
}

/* La pastille d'environnement s'aligne sur le nom du projet. */
.project__env {
  display: flex;
  min-width: 0;
  padding: 0 var(--tree-pad) 0.375rem calc(var(--tree-pad) + var(--tree-lead) + var(--tree-gap));
}
</style>
