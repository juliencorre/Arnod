<script setup lang="ts">
/**
 * Point de départ « Reprendre l'existant » d'un projet (2.3-b, 1.2-d) : une collection Bruno ou
 * un contrat OpenAPI importé dans ce projet. Ouvre « Importer » sur le dossier choisi.
 */
import { useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useImportStore } from "@/stores/imports";
import { useNavigationStore } from "@/stores/navigation";

const props = defineProps<{ projectId: string }>();

const imports = useImportStore();
const navigation = useNavigationStore();
const titleId = useId();
</script>

<template>
  <section class="start" :aria-labelledby="titleId">
    <h2 :id="titleId" class="start__title">Reprendre l'existant</h2>
    <p class="start__text">
      Une collection Bruno ou un fichier OpenAPI : vos requêtes deviennent des tests.
    </p>
    <BaseButton
      class="start__action"
      @click="imports.open(props.projectId, navigation.currentWorkspaceId)"
    >
      Importer…
    </BaseButton>
  </section>
</template>

<style scoped>
.start {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  text-align: left;
  background: var(--color-surface);
}

.start__title {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.start__text {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.start__action {
  align-self: flex-start;
  margin-top: 0.5rem;
}
</style>
