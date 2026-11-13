<script setup lang="ts">
/**
 * Erreur bloquante au démarrage (lot G, 0.3-b, 0.3-b bis, 0.3-b ter) : la cause en clair, ce
 * qui est intact, deux issues avec leurs conséquences (la recommandée en premier), le fichier
 * concerné, les détails pour le support repliés, puis « Quitter ». Même structure pour toute
 * erreur bloquante ; tout le texte vient du cœur (`startup::describe`).
 */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useStartupStore } from "@/stores/startup";
import type { StartupOption } from "@/types/ipc";

const startup = useStartupStore();
const expanded = ref(false);

const problem = computed(() => startup.problem);

function choose(option: StartupOption): void {
  switch (option.action) {
    case "retry":
      void startup.retry();
      break;
    case "startEmpty":
      void startup.startEmpty();
      break;
    case "openReleases":
      void startup.open("releases");
      break;
    case "openFolder":
      void startup.open("folder");
      break;
  }
}

/** Ligne du prochain essai automatique, sous l'issue « Réessayer » (R4). */
function retryLine(option: StartupOption): string | null {
  if (option.action !== "retry") return null;
  if (startup.busy && startup.secondsLeft === null && problem.value?.retry) {
    return "Nouvel essai en cours…";
  }
  if (startup.secondsLeft !== null) {
    return `Arnod réessaie seul : prochain essai dans ${String(startup.secondsLeft)} s.`;
  }
  return option.note;
}
</script>

<template>
  <main v-if="problem" class="startup-problem" aria-labelledby="startup-problem-title">
    <div class="startup-problem__column">
      <header class="startup-problem__header" role="alert">
        <span class="startup-problem__icon" aria-hidden="true">!</span>
        <div class="startup-problem__heading">
          <h1 id="startup-problem-title" class="startup-problem__title">{{ problem.title }}</h1>
          <p class="startup-problem__message">{{ problem.message }}</p>
        </div>
      </header>

      <p v-if="problem.intactLead" class="startup-problem__intact">
        <span class="startup-problem__check" aria-hidden="true">✓</span>
        <span>
          <strong>{{ problem.intactLead }}</strong> {{ problem.intactText }}
        </span>
      </p>

      <section
        v-if="problem.options.length > 0"
        class="startup-problem__choices"
        aria-labelledby="startup-problem-choices"
      >
        <h2 id="startup-problem-choices" class="startup-problem__question">
          Que voulez-vous faire ?
        </h2>
        <article
          v-for="option in problem.options"
          :key="option.action"
          class="startup-problem__option"
          :class="{ 'startup-problem__option--recommended': option.recommended }"
        >
          <div class="startup-problem__option-text">
            <h3 class="startup-problem__option-title">
              {{ option.title }}
              <span v-if="option.recommended" class="startup-problem__badge">Recommandé</span>
            </h3>
            <p class="startup-problem__option-description">{{ option.description }}</p>
            <p v-if="retryLine(option)" class="startup-problem__option-note" aria-live="polite">
              {{ retryLine(option) }}
            </p>
          </div>
          <BaseButton
            class="startup-problem__option-button"
            :variant="option.recommended ? 'primary' : 'secondary'"
            :disabled="startup.busy"
            @click="choose(option)"
          >
            {{ option.button }}
          </BaseButton>
        </article>
      </section>

      <div v-if="problem.path && !expanded" class="startup-problem__path">
        <span class="startup-problem__path-label">{{ problem.pathLabel }}</span>
        <code class="startup-problem__path-value" :title="problem.path">{{ problem.path }}</code>
        <span class="startup-problem__path-actions">
          <BaseButton variant="link" @click="startup.copyPath()">Copier</BaseButton>
          <span class="startup-problem__separator" aria-hidden="true"></span>
          <BaseButton variant="link" @click="startup.open('folder')">Ouvrir le dossier</BaseButton>
        </span>
      </div>

      <section class="startup-problem__support">
        <div class="startup-problem__support-bar">
          <button
            type="button"
            class="startup-problem__toggle"
            :aria-expanded="expanded"
            aria-controls="startup-problem-details"
            @click="expanded = !expanded"
          >
            <span aria-hidden="true">{{ expanded ? "▾" : "▸" }}</span>
            Détails pour le support
          </button>
          <span class="startup-problem__support-actions">
            <BaseButton variant="link" @click="startup.copyDetails()">Copier le détail</BaseButton>
            <BaseButton v-if="problem.issueUrl" variant="link" @click="startup.open('issue')">
              Signaler le problème…
            </BaseButton>
          </span>
        </div>
        <div v-if="expanded" id="startup-problem-details" class="startup-problem__details">
          <dl class="startup-problem__rows">
            <template v-for="row in problem.details" :key="row.label">
              <dt class="startup-problem__row-label">{{ row.label }}</dt>
              <dd class="startup-problem__row-value">{{ row.value }}</dd>
            </template>
          </dl>
          <p v-if="problem.issueUrl" class="startup-problem__privacy">
            « Signaler le problème… » ouvre une issue GitHub préremplie avec ce détail, dans votre
            navigateur : rien n'est envoyé sans vous.
          </p>
        </div>
      </section>

      <footer class="startup-problem__footer">
        <span class="startup-problem__version">{{ problem.footer }}</span>
        <BaseButton @click="startup.quit()">Quitter</BaseButton>
      </footer>
    </div>
  </main>
</template>

<style scoped>
.startup-problem {
  box-sizing: border-box;
  height: 100vh;
  overflow-y: auto;
  padding: 2rem 1.5rem;
  background: var(--color-bg);
  color: var(--color-text);
}

.startup-problem__column {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 40rem;
  min-height: 100%;
  margin: 0 auto;
  justify-content: center;
}

.startup-problem__header {
  display: flex;
  gap: 0.875rem;
  align-items: flex-start;
}

.startup-problem__icon {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: var(--control-lg);
  height: var(--control-lg);
  border-radius: var(--radius-lg);
  background: var(--color-accent-soft);
  color: var(--color-link);
  font-size: var(--text-title);
  font-weight: 600;
}

.startup-problem__heading {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  min-width: 0;
}

.startup-problem__title {
  margin: 0;
  font-size: var(--text-display);
  font-weight: 600;
  line-height: 1.25;
}

.startup-problem__message {
  margin: 0;
  font-size: var(--text-body-large);
  line-height: 1.5;
  color: var(--color-muted);
}

.startup-problem__intact {
  display: flex;
  gap: 0.625rem;
  margin: 0;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  background: var(--color-success-soft);
  color: var(--color-success);
  line-height: 1.5;
}

.startup-problem__check {
  font-weight: 600;
}

.startup-problem__choices {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.startup-problem__question {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.startup-problem__option {
  display: flex;
  gap: 1rem;
  align-items: center;
  padding: 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.startup-problem__option--recommended {
  border-color: var(--color-text);
}

.startup-problem__option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.startup-problem__option-title {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.startup-problem__badge {
  padding: 0.0625rem 0.5rem;
  border-radius: var(--radius-pill);
  background: var(--color-chip);
  color: var(--color-link);
  font-size: var(--text-small);
  font-weight: 600;
}

.startup-problem__option-description,
.startup-problem__option-note {
  margin: 0;
  line-height: 1.5;
  color: var(--color-muted);
}

.startup-problem__option-note {
  font-size: var(--text-small);
}

.startup-problem__option-button {
  flex: none;
}

.startup-problem__path {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.startup-problem__path-label {
  color: var(--color-muted);
  font-size: var(--text-small);
}

.startup-problem__path-value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
}

.startup-problem__path-actions,
.startup-problem__support-actions {
  display: inline-flex;
  gap: 0.25rem;
  align-items: center;
}

.startup-problem__separator {
  align-self: stretch;
  width: 1px;
  margin: 0.375rem 0;
  background: var(--color-border);
}

.startup-problem__support {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.startup-problem__support-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  justify-content: space-between;
}

.startup-problem__toggle {
  display: inline-flex;
  gap: 0.375rem;
  align-items: center;
  min-height: var(--control-sm);
  padding: 0;
  border: 0;
  background: none;
  color: var(--color-text);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

.startup-problem__toggle:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.startup-problem__details {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.startup-problem__rows {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.375rem 2rem;
  margin: 0;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  background: var(--color-sidebar);
}

.startup-problem__row-label {
  color: var(--color-muted);
  font-size: var(--text-small);
}

.startup-problem__row-value {
  margin: 0;
  min-width: 0;
  overflow-wrap: anywhere;
  font-family: var(--font-mono);
  font-size: var(--text-small);
}

.startup-problem__privacy {
  margin: 0;
  color: var(--color-muted);
  font-size: var(--text-small);
  line-height: 1.5;
}

.startup-problem__footer {
  display: flex;
  gap: 1rem;
  align-items: center;
  justify-content: space-between;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border);
}

.startup-problem__version {
  color: var(--color-muted);
  font-size: var(--text-small);
}
</style>
