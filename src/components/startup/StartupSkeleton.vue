<script setup lang="ts">
/**
 * Démarrage lent (lot G, 0.3-a) : squelette de l'écran attendu (barre latérale, zone, cartes),
 * jamais un écran « Chargement… ». La fenêtre, créée cachée, n'est montrée avec lui que si
 * l'interface n'est pas prête après 300 ms (`windows::show_later`).
 */
withDefaults(
  defineProps<{
    /** Fenêtre attendue : principale, de contenu (sans barre) ou barre latérale détachée. */
    kind?: "main" | "content" | "sidebar";
    /** « Ouverture de « Boutique »… », discret en bas de la zone. */
    caption?: string | null;
  }>(),
  { kind: "main", caption: null },
);
</script>

<template>
  <div class="startup-skeleton" aria-busy="true">
    <div v-if="kind !== 'content'" class="startup-skeleton__sidebar" aria-hidden="true">
      <div class="startup-skeleton__project">
        <span class="startup-skeleton__block startup-skeleton__block--dot"></span>
        <span class="startup-skeleton__lines">
          <span class="startup-skeleton__block startup-skeleton__block--w110"></span>
          <span
            class="startup-skeleton__block startup-skeleton__block--w80 startup-skeleton__block--thin"
          ></span>
        </span>
      </div>
      <span
        class="startup-skeleton__block startup-skeleton__block--w60 startup-skeleton__block--thin"
      ></span>
      <div v-for="width in ['w120', 'w90', 'w140']" :key="width" class="startup-skeleton__row">
        <span class="startup-skeleton__block startup-skeleton__block--icon"></span>
        <span class="startup-skeleton__block" :class="`startup-skeleton__block--${width}`"></span>
      </div>
    </div>
    <div v-if="kind !== 'sidebar'" class="startup-skeleton__main">
      <div class="startup-skeleton__heading" aria-hidden="true">
        <span
          class="startup-skeleton__block startup-skeleton__block--w70 startup-skeleton__block--thin"
        ></span>
        <span class="startup-skeleton__block startup-skeleton__block--title"></span>
        <span
          class="startup-skeleton__block startup-skeleton__block--w360 startup-skeleton__block--thin"
        ></span>
      </div>
      <div class="startup-skeleton__cards" aria-hidden="true">
        <div v-for="card in 2" :key="card" class="startup-skeleton__card">
          <span class="startup-skeleton__block startup-skeleton__block--icon-lg"></span>
          <span class="startup-skeleton__lines startup-skeleton__lines--grow">
            <span class="startup-skeleton__block startup-skeleton__block--w140"></span>
            <span
              class="startup-skeleton__block startup-skeleton__block--w190 startup-skeleton__block--thin"
            ></span>
          </span>
          <span class="startup-skeleton__block startup-skeleton__block--button"></span>
        </div>
      </div>
      <p v-if="caption" class="startup-skeleton__caption" role="status">{{ caption }}</p>
    </div>
  </div>
</template>

<style scoped>
.startup-skeleton {
  display: flex;
  height: 100vh;
  background: var(--color-bg);
}

.startup-skeleton__sidebar {
  display: flex;
  flex: none;
  flex-direction: column;
  gap: 1rem;
  width: 260px;
  padding: 1.5rem 0.75rem;
  border-right: 1px solid var(--color-border);
  background: var(--color-sidebar);
  box-sizing: border-box;
}

.startup-skeleton__project,
.startup-skeleton__row {
  display: flex;
  gap: 0.625rem;
  align-items: center;
}

.startup-skeleton__lines {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.startup-skeleton__lines--grow {
  flex: 1;
}

.startup-skeleton__block {
  display: block;
  height: 0.625rem;
  border-radius: var(--radius-pill);
  background: var(--color-chip);
}

.startup-skeleton__block--thin {
  height: 0.5rem;
}

.startup-skeleton__block--dot {
  width: 0.875rem;
  height: 0.875rem;
  border-radius: var(--radius-sm);
}

.startup-skeleton__block--icon {
  width: 1.125rem;
  height: 1.125rem;
  border-radius: var(--radius-sm);
}

.startup-skeleton__block--icon-lg {
  width: 1.625rem;
  height: 1.625rem;
  border-radius: var(--radius-md);
}

.startup-skeleton__block--title {
  width: 220px;
  height: 1.375rem;
  border-radius: var(--radius-md);
}

.startup-skeleton__block--button {
  width: 4rem;
  height: var(--control-md);
  border-radius: var(--radius-md);
}

.startup-skeleton__block--w60 {
  width: 60px;
}

.startup-skeleton__block--w70 {
  width: 70px;
}

.startup-skeleton__block--w80 {
  width: 80px;
}

.startup-skeleton__block--w90 {
  width: 90px;
}

.startup-skeleton__block--w120 {
  width: 120px;
}

.startup-skeleton__block--w110 {
  width: 110px;
}

.startup-skeleton__block--w140 {
  width: 140px;
}

.startup-skeleton__block--w190 {
  width: 190px;
}

.startup-skeleton__block--w360 {
  width: min(360px, 100%);
}

.startup-skeleton__main {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1.5rem;
  min-width: 0;
  padding: 2.25rem 2.25rem 1.5rem;
}

.startup-skeleton__heading {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.startup-skeleton__cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
  gap: 0.875rem;
}

.startup-skeleton__card {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 1.125rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.startup-skeleton__caption {
  margin: auto 0 0;
  text-align: center;
  color: var(--color-muted);
}
</style>
