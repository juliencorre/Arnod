<script setup lang="ts">
/**
 * Catalogue des étapes (docs/plan-accueil-non-classes.md, K1, K2) : les natures
 * d'étape disponibles sur ce poste, leur famille, leur moteur et leur fiche.
 * Lecture seule : la synchronisation viendra plus tard
 * (docs/etude-catalogue-etapes.md).
 */
import { computed, onMounted, ref, useId } from "vue";

import NaturePill from "@/components/steps/NaturePill.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { FAMILY_LABELS, engineLabel, fieldKindLabel, ruleLabel } from "@/lib/steps";
import { useNavigationStore } from "@/stores/navigation";
import { useStepsStore } from "@/stores/steps";
import { useToastStore } from "@/stores/toasts";

const steps = useStepsStore();
const navigation = useNavigationStore();
const toasts = useToastStore();
const ids = { title: useId(), detail: useId() };

const selectedId = ref<string | null>(null);
const selected = computed(() => steps.nature(selectedId.value) ?? steps.natures[0] ?? null);
const source = computed(() => {
  const s = steps.catalog?.source;
  if (!s) return "";
  return `${s.name} · ${s.updated} · ${s.synced ? "synchronisé" : "définitions intégrées"}`;
});

onMounted(() => {
  steps.load().catch((cause: unknown) => {
    toasts.showError(cause);
  });
});
</script>

<template>
  <section class="catalog" :aria-labelledby="ids.title">
    <header class="catalog__header">
      <BaseButton variant="link" @click="navigation.closeSteps()">← Retour</BaseButton>
      <div class="catalog__title-row">
        <h1 :id="ids.title" class="catalog__title">Catalogue des étapes</h1>
        <span v-if="source" class="catalog__source">{{ source }}</span>
      </div>
      <p class="catalog__text">
        Une nature d'étape est une définition, comme celles d'un antivirus : son formulaire, ce
        qu'Arnod reconnaît dans ce que vous collez, son moteur. Les natures ci-dessous sont
        intégrées à Arnod ; les autres viendront d'un catalogue signé, sans nouvelle version de
        l'application.
      </p>
    </header>
    <div class="catalog__body">
      <table class="catalog__table" :aria-labelledby="ids.title">
        <thead>
          <tr>
            <th scope="col">Nature d'étape</th>
            <th scope="col">Famille</th>
            <th scope="col">Moteur</th>
            <th scope="col">Éditeur</th>
            <th scope="col">Version</th>
            <th scope="col">État</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="nature in steps.natures"
            :key="nature.id"
            :class="{ 'catalog__row--selected': selected?.id === nature.id }"
          >
            <th scope="row">
              <button
                type="button"
                class="catalog__name"
                :aria-pressed="selected?.id === nature.id"
                :aria-controls="ids.detail"
                @click="selectedId = nature.id"
              >
                {{ nature.name }}
                <span class="catalog__muted">{{ nature.title }}</span>
              </button>
            </th>
            <td>
              <NaturePill :name="FAMILY_LABELS[nature.family]" :family="nature.family" />
            </td>
            <td class="catalog__mono">{{ engineLabel(nature.engine) }}</td>
            <td>{{ nature.publisher }}</td>
            <td class="catalog__mono">{{ nature.version }}</td>
            <td class="catalog__state">
              {{ nature.origin === "builtin" ? "✓ Intégrée" : "✓ Dans le catalogue" }}
            </td>
          </tr>
        </tbody>
      </table>

      <aside v-if="selected" :id="ids.detail" class="detail" :aria-label="`Fiche ${selected.name}`">
        <p class="detail__kicker">
          Nature d'étape · <span>{{ FAMILY_LABELS[selected.family] }}</span>
        </p>
        <h2 class="detail__title">{{ selected.name }}</h2>
        <p class="detail__meta">
          {{ selected.version }} · {{ selected.publisher }} · moteur
          {{ engineLabel(selected.engine) }}
          <template v-if="selected.engine.kind === 'builtin'">{{ selected.engine.id }}</template>
        </p>
        <p class="detail__text">{{ selected.description }}</p>
        <h3 class="detail__section">Ce qu'elle demande</h3>
        <dl class="detail__fields">
          <dt>{{ selected.subject.label }}</dt>
          <dd class="catalog__mono">{{ selected.subject.placeholder || "texte" }}</dd>
          <template v-for="field in selected.form" :key="field.key">
            <dt>{{ field.label }}</dt>
            <dd>{{ fieldKindLabel(field) }}</dd>
          </template>
        </dl>
        <h3 class="detail__section">Ce qu'Arnod reconnaît</h3>
        <ul class="detail__rules">
          <li v-for="(rule, index) in selected.detect" :key="index">{{ ruleLabel(rule) }}</li>
        </ul>
        <p class="detail__note">
          Le bouton de l'essai devient « {{ selected.action }} ». Les vérifications sans code sont
          proposées d'après le résultat reçu.
        </p>
      </aside>
    </div>
  </section>
</template>

<style scoped>
.catalog {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.5rem 2rem;
}

.catalog__header {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
}

.catalog__title-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
}

.catalog__title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
}

.catalog__source {
  padding: 0.125rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--color-muted);
  background: var(--color-surface);
}

.catalog__text {
  margin: 0;
  max-width: 52rem;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.catalog__body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(16rem, 22rem);
  align-items: start;
  gap: 1.25rem;
}

@media (width < 60rem) {
  .catalog__body {
    grid-template-columns: minmax(0, 1fr);
  }
}

.catalog__table {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  border-collapse: separate;
  border-spacing: 0;
  overflow: hidden;
  font-size: 0.875rem;
  background: var(--color-surface);
}

.catalog__table th,
.catalog__table td {
  padding: 0.625rem 1rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
  vertical-align: middle;
}

.catalog__table thead th {
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
  background: var(--color-sidebar);
}

.catalog__table tbody tr:last-child > * {
  border-bottom: 0;
}

.catalog__row--selected > * {
  background: var(--color-accent-soft);
}

.catalog__name {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.125rem;
  padding: 0;
  border: 0;
  font: inherit;
  font-weight: 600;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.catalog__name:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.catalog__muted {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--color-muted);
}

.catalog__mono {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.catalog__state {
  font-weight: 600;
  color: var(--color-success);
}

.detail {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1rem 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  font-size: 0.875rem;
  background: var(--color-surface);
}

.detail__kicker {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.detail__title {
  margin: 0;
  font-size: 1.125rem;
  font-weight: 600;
}

.detail__meta,
.detail__note {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.detail__text {
  margin: 0;
}

.detail__section {
  margin: 0.5rem 0 0;
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.detail__fields {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.25rem 0.875rem;
  margin: 0;
}

.detail__fields dt {
  color: var(--color-muted);
}

.detail__fields dd {
  margin: 0;
}

.detail__rules {
  margin: 0;
  padding-left: 1.125rem;
}
</style>
