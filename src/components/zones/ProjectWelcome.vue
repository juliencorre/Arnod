<script setup lang="ts">
/**
 * Projet ouvert sans onglet (fenêtre principale) : son nom, son dossier et de
 * quoi commencer. Les requêtes sont dans la barre latérale.
 */
import { computed } from "vue";

import ImportStart from "@/components/import/ImportStart.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string }>();

const catalog = useCatalogStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const project = computed(() => catalog.projectById(props.projectId));
const empty = computed(() => (workbench.trees[props.projectId]?.nodes.length ?? 0) === 0);

async function newRequest(): Promise<void> {
  try {
    await workbench.createRequest(props.projectId, "");
  } catch (error) {
    toasts.showError(error);
  }
}
</script>

<template>
  <div v-if="project" class="welcome">
    <h1 class="welcome__title">{{ project.name }}</h1>
    <code class="welcome__path">{{ project.path }}</code>
    <p class="welcome__text">
      {{
        empty
          ? "Aucune requête."
          : "Choisissez une requête dans la barre latérale : elle s'ouvre en onglet."
      }}
      Glissez un onglet sur un bord pour diviser la fenêtre.
    </p>
    <p class="welcome__text">
      Un contrat OpenAPI ? « + Ajouter un contrat » dans la barre latérale crée ses requêtes.
    </p>
    <BaseButton variant="primary" @click="newRequest">+ Nouvelle requête</BaseButton>
    <!-- Lot C - Import Bruno : point de départ « Reprendre l'existant » (2.3-b). -->
    <div v-if="empty" class="welcome__start">
      <p class="welcome__text">Par où commencer ?</p>
      <ImportStart :project-id="projectId" />
    </div>
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

.welcome__title {
  margin: 0;
  font-size: var(--text-display);
  font-weight: 600;
}

.welcome__path {
  font: var(--text-body) var(--font-mono);
  color: var(--color-muted);
  overflow-wrap: anywhere;
}

.welcome__text {
  margin: 0.25rem 0 0.5rem;
  color: var(--color-muted);
}
.welcome__start {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: min(16rem, 100%);
  margin-top: 1rem;
}
</style>
