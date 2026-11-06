<script setup lang="ts">
/**
 * Premier lancement (1a) : aucun projet encore. On nomme le premier ; le dossier
 * et le workspace par défaut ont déjà une valeur.
 */
import { useId } from "vue";

import ProjectForm from "@/components/projects/ProjectForm.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useCatalogStore } from "@/stores/catalog";

const catalog = useCatalogStore();
const { announceCreated } = useProjectActions();
const titleId = useId();
</script>

<template>
  <section class="first-run" :aria-labelledby="titleId">
    <div class="first-run__intro">
      <h1 :id="titleId" class="first-run__title">Créez votre premier projet</h1>
      <p class="first-run__lead">
        Donnez-lui un nom : vous écrirez votre première requête juste après.
      </p>
    </div>
    <div v-if="catalog.defaultWorkspace" class="first-run__card">
      <ProjectForm
        mode="firstRun"
        :initial-workspace-id="catalog.defaultWorkspace.id"
        @created="announceCreated"
      />
    </div>
    <p class="first-run__note">
      Une requête = un fichier YAML lisible, prêt à versionner avec Git.
    </p>
  </section>
</template>

<style scoped>
.first-run {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  min-height: 100%;
  padding: 2.5rem;
}

.first-run__intro {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  text-align: center;
}

.first-run__title {
  margin: 0;
  font-size: 1.75rem;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.first-run__lead {
  margin: 0;
  max-width: 32rem;
  font-size: 0.9375rem;
  color: var(--color-muted);
}

.first-run__card {
  box-sizing: border-box;
  width: min(32.5rem, 100%);
  padding: 1.5rem;
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-surface);
}

.first-run__note {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
