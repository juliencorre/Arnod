<script setup lang="ts">
/**
 * Projet ouvert sans onglet (fenêtre principale) : son nom, son dossier et de quoi commencer.
 * Un projet vide (ni requête ni contrat) propose quatre points de départ
 * (docs/plan-mvp-d-barre-laterale.md, D2, écran 1.2-d) : reprendre l'existant, décrire un
 * contrat, écrire une requête, configurer un environnement. Sinon, les requêtes sont dans la
 * barre latérale.
 */
import { computed, ref } from "vue";

import ContractDialog from "@/components/contracts/ContractDialog.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import EnvironmentsDialog from "@/components/workbench/EnvironmentsDialog.vue";
import { useContractAdd } from "@/composables/useContractAdd";
import { useCatalogStore } from "@/stores/catalog";
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string }>();

const catalog = useCatalogStore();
const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const toasts = useToastStore();
const project = computed(() => catalog.projectById(props.projectId));
const empty = computed(
  () =>
    (workbench.trees[props.projectId]?.nodes.length ?? 0) === 0 &&
    (contracts.lists[props.projectId]?.length ?? 0) === 0,
);
const adding = useContractAdd(() => props.projectId);
const { mode, error, busy } = adding;
const creatingEnvironment = ref(false);

async function newRequest(): Promise<void> {
  try {
    await workbench.createRequest(props.projectId, "");
  } catch (caught) {
    toasts.showError(caught);
  }
}

type Start = "import" | "contract" | "request" | "environment";

/**
 * Points de départ, dans l'ordre de la maquette. « Importer… » importe un fichier OpenAPI ;
 * les collections Bruno viendront du lot C (2.3-b).
 */
const STARTS: { id: Start; title: string; text: string; action: string }[] = [
  {
    id: "import",
    title: "Reprendre l'existant",
    text: "Un fichier OpenAPI : ses opérations deviennent des requêtes, rangées sous leur contrat.",
    action: "Importer…",
  },
  {
    id: "contract",
    title: "Décrire un contrat de service",
    text: "Ses endpoints, le corps et les réponses attendus. Vous y écrirez vos premières requêtes.",
    action: "+ Contrat",
  },
  {
    id: "request",
    title: "Écrire une requête",
    text: "Directement dans le référentiel de tests, sans contrat.",
    action: "+ Requête",
  },
  {
    id: "environment",
    title: "Configurer un environnement",
    text: "Ses variables, comme baseUrl. Vous pouvez aussi le faire au premier envoi.",
    action: "+ Environnement",
  },
];

function start(id: Start): void {
  if (id === "import") adding.choose("file");
  else if (id === "contract") adding.choose("template");
  else if (id === "request") void newRequest();
  else creatingEnvironment.value = true;
}
</script>

<template>
  <div v-if="project && empty" class="start">
    <h1 class="start__title">{{ project.name }}</h1>
    <code class="start__path">{{ project.path }}</code>
    <h2 class="start__question">Par où commencer ?</h2>
    <ul class="start__cards">
      <li v-for="item in STARTS" :key="item.id" class="start__card">
        <h3 class="start__card-title">{{ item.title }}</h3>
        <p class="start__card-text">{{ item.text }}</p>
        <BaseButton class="start__action" @click="start(item.id)">{{ item.action }}</BaseButton>
      </li>
    </ul>
    <ContractDialog
      :mode="mode"
      :error="error"
      :busy="busy"
      @close="adding.close"
      @submit="adding.submit"
    />
    <EnvironmentsDialog
      v-if="creatingEnvironment"
      :open="creatingEnvironment"
      :project-id="projectId"
      start-new
      @close="creatingEnvironment = false"
    />
  </div>
  <div v-else-if="project" class="welcome">
    <h1 class="welcome__title">{{ project.name }}</h1>
    <code class="welcome__path">{{ project.path }}</code>
    <p class="welcome__text">
      Choisissez une requête dans la barre latérale : elle s'ouvre en onglet. Glissez un onglet sur
      un bord pour diviser la fenêtre.
    </p>
    <p class="welcome__text">
      Un contrat OpenAPI ? « + Ajouter un contrat » dans la barre latérale crée ses requêtes.
    </p>
    <BaseButton variant="primary" @click="newRequest">+ Nouvelle requête</BaseButton>
  </div>
</template>

<style scoped>
.welcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.625rem;
  margin: auto;
  padding: 2rem;
  max-width: 30rem;
  text-align: center;
}

.welcome__title,
.start__title {
  margin: 0;
  font-size: var(--text-display);
  font-weight: 600;
}

.welcome__path,
.start__path {
  font: var(--text-body) var(--font-mono);
  color: var(--color-muted);
  overflow-wrap: anywhere;
}

.welcome__text {
  margin: 0.25rem 0 0.5rem;
  color: var(--color-muted);
}

/* Projet vide : quatre points de départ en cartes (1.2-d). */
.start {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  width: 100%;
  padding: 2rem 2.25rem;
}

.start__question {
  margin: 1.25rem 0 0.75rem;
  font-size: var(--text-body-large);
  font-weight: 400;
  color: var(--color-text);
}

.start__cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
  gap: 1rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.start__card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.start__card-title {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.start__card-text {
  flex: 1;
  margin: 0;
  line-height: 1.5;
  color: var(--color-muted);
}

.start__action {
  margin-top: 0.5rem;
}
</style>
