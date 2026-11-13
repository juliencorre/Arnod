<script setup lang="ts">
/**
 * Premier lancement (docs/plan-accueil-non-classes.md, A1 à A4) : aucun projet
 * encore. « Essayer une étape » d'abord, puis créer un projet (le dossier a déjà
 * une valeur, le workspace est facultatif : O8) ou importer un projet Arnod
 * existant (P12). Une nature choisie prend la place : créer et importer se
 * réduisent à une ligne. L'entrée reste à la même hauteur dans les deux états.
 */
import { ref, useId } from "vue";

import TryStep from "@/components/home/TryStep.vue";
import ProjectForm from "@/components/projects/ProjectForm.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { useProjectActions } from "@/composables/useProjectActions";
import { useDialogStore } from "@/stores/dialogs";
import { useImportStore } from "@/stores/imports";

const { announceCreated } = useProjectActions();
const dialogs = useDialogStore();
const imports = useImportStore();
const titleId = useId();
const importTitleId = useId();
/** Une nature est choisie : son formulaire est ouvert. */
const focused = ref(false);
</script>

<template>
  <section class="first-run" :aria-labelledby="titleId">
    <div class="first-run__intro">
      <h1 :id="titleId" class="first-run__title">Bienvenue dans Arnod</h1>
      <p class="first-run__lead">
        Testez tout ce que votre application touche : API, services, bases de données, messages,
        traitements et écrans, enchaînés dans un même test.
      </p>
    </div>
    <div class="first-run__column">
      <TryStep @open="focused = $event" />
      <template v-if="!focused">
        <p class="first-run__or" aria-hidden="true"><span>ou</span></p>
        <div class="first-run__cards">
          <div class="first-run__card">
            <h2 class="first-run__card-title">Créer un projet</h2>
            <ProjectForm mode="firstRun" :initial-workspace-id="null" @created="announceCreated" />
          </div>
          <section class="first-run__import" :aria-labelledby="importTitleId">
            <span class="first-run__import-icon" aria-hidden="true">⇩</span>
            <h2 :id="importTitleId" class="first-run__import-title">Importer…</h2>
            <p class="first-run__import-text">
              Une collection Bruno, un fichier OpenAPI ou un projet Arnod : Arnod reconnaît le
              format.
            </p>
            <BaseButton @click="imports.choose(null, null)">Choisir un dossier…</BaseButton>
            <BaseButton variant="link" @click="imports.choose(null, null, true)">
              ou un fichier…
            </BaseButton>
            <p class="first-run__import-note">L'import crée le projet.</p>
          </section>
        </div>
        <p class="first-run__note">
          Un projet = un dossier sur votre poste, prêt à être partagé avec l'équipe.
        </p>
      </template>
      <div v-else class="first-run__compact">
        <span>Pas encore de projet ?</span>
        <BaseButton @click="dialogs.openNewProject(null)">Créer un projet…</BaseButton>
        <BaseButton @click="imports.open(null, null)">⇩ Importer…</BaseButton>
        <span class="first-run__compact-note">
          Un projet = un dossier sur votre poste, partagé avec l'équipe.
        </span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.first-run {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  min-height: 100%;
  padding: clamp(2rem, 9vh, 5rem) 2.5rem 2.5rem;
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
  font-size: var(--text-display);
  font-weight: 600;
  letter-spacing: -0.01em;
}

.first-run__lead {
  margin: 0;
  max-width: 44rem;
  font-size: var(--text-body-large);
  color: var(--color-muted);
}

.first-run__column {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: min(55rem, 100%);
}

.first-run__cards {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
  gap: 1rem;
}

@media (width < 48rem) {
  .first-run__cards {
    grid-template-columns: minmax(0, 1fr);
  }
}

.first-run__card {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.first-run__card-title {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.first-run__or {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin: 0;
  font-size: var(--text-body);
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
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.625rem;
  padding: 1.25rem;
  border: 1.5px dashed var(--color-border-strong);
  border-radius: var(--radius-lg);
  text-align: center;
  color: var(--color-text);
  background: var(--color-surface);
}

.first-run__import-icon {
  display: grid;
  place-items: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: var(--radius-md);
  font-weight: 700;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.first-run__import-title {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.first-run__import-note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.first-run__import-text {
  margin: 0;
  max-width: 18rem;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.first-run__note {
  margin: 0;
  font-size: var(--text-body);
  text-align: center;
  color: var(--color-muted);
}

.first-run__compact {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.625rem;
  padding-top: 0.875rem;
  border-top: 1px solid var(--color-border);
  font-size: var(--text-body);
  color: var(--color-muted);
}

.first-run__compact-note {
  margin-left: auto;
  font-size: var(--text-small);
}
</style>
