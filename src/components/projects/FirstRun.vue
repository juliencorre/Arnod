<script setup lang="ts">
/** Premier lancement (1a) : aucun projet encore. Le formulaire de création arrive avec T15. */
import { useId } from "vue";

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
</style>
