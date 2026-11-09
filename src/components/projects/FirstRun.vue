<script setup lang="ts">
/**
 * Premier lancement (1a) : aucun projet encore. On nomme le premier ; le dossier
 * a déjà une valeur et le rangement dans un workspace est facultatif (O8). Ou
 * l'on importe un projet Arnod existant (P12).
 */
import { useId } from "vue";

import ProjectForm from "@/components/projects/ProjectForm.vue";
import { useProjectActions } from "@/composables/useProjectActions";

const { announceCreated, addExistingProject } = useProjectActions();
const titleId = useId();
const importTitleId = useId();
</script>

<template>
  <section class="first-run" :aria-labelledby="titleId">
    <div class="first-run__intro">
      <h1 :id="titleId" class="first-run__title">Créez votre premier projet</h1>
      <p class="first-run__lead">
        Donnez-lui un nom : vous écrirez votre première requête juste après.
      </p>
    </div>
    <div class="first-run__card">
      <ProjectForm mode="firstRun" :initial-workspace-id="null" @created="announceCreated" />
    </div>
    <p class="first-run__or" aria-hidden="true"><span>ou</span></p>
    <button
      type="button"
      class="first-run__import"
      :aria-labelledby="importTitleId"
      @click="addExistingProject(null)"
    >
      <span class="first-run__import-icon" aria-hidden="true">⇩</span>
      <span>
        <span :id="importTitleId" class="first-run__import-title">Importer…</span>
        <span class="first-run__import-text">
          Un projet Arnod existant, par exemple après un clone Git : choisissez son dossier.
        </span>
      </span>
    </button>
    <p class="first-run__note">
      Un projet = un dossier sur votre poste, prêt à être partagé avec l'équipe.
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

.first-run__or {
  display: flex;
  align-items: center;
  gap: 1rem;
  width: min(32.5rem, 100%);
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.first-run__or::before,
.first-run__or::after {
  flex: 1;
  height: 1px;
  content: "";
  background: var(--color-border);
}

.first-run__import {
  box-sizing: border-box;
  display: flex;
  gap: 0.875rem;
  width: min(32.5rem, 100%);
  padding: 1rem 1.25rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: 12px;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.first-run__import:hover {
  background: var(--color-surface);
}

.first-run__import:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.first-run__import-icon {
  display: grid;
  flex: none;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border-radius: 8px;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.first-run__import-title {
  display: block;
  font-weight: 600;
}

.first-run__import-text {
  display: block;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.first-run__note {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}
</style>
