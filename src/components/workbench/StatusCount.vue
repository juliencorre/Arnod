<script setup lang="ts">
/**
 * Compteur d'un dossier ou d'une ressource (ST2, R6) : « réussies / exécutées », en rouge s'il y
 * a un échec ; sans aucune exécution, le nombre de requêtes (`fallback`).
 */
import { computed } from "vue";

import { countTitle, type StatusCount } from "@/lib/status";

const props = defineProps<{ count: StatusCount; fallback: number }>();

const failed = computed(() => props.count.executed > props.count.passed);
</script>

<template>
  <span
    v-if="count.executed > 0"
    class="status-count"
    :class="failed ? 'status-count--ko' : 'status-count--ok'"
    :title="countTitle(count)"
  >
    <span aria-hidden="true">{{ count.passed }}/{{ count.executed }}</span>
    <span class="visually-hidden">, {{ countTitle(count) }}</span>
  </span>
  <span v-else class="status-count">{{ fallback }}</span>
</template>

<style scoped>
.status-count {
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.status-count--ok {
  color: var(--color-success);
}

.status-count--ko {
  color: var(--color-danger);
}
</style>
