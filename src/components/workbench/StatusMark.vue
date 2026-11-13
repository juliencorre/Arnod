<script setup lang="ts">
/**
 * Résultat de la dernière exécution d'une requête (ST1) : ✓ vert ou ✕ rouge, jamais la couleur
 * seule, avec la date en titre et le résultat lu par les lecteurs d'écran.
 */
import { statusTitle } from "@/lib/status";
import type { RequestStatus } from "@/types/ipc";

defineProps<{ status: RequestStatus | null }>();
</script>

<template>
  <span
    v-if="status"
    class="status-mark"
    :class="status.passed ? 'status-mark--ok' : 'status-mark--ko'"
    :title="statusTitle(status)"
  >
    <span aria-hidden="true">{{ status.passed ? "✓" : "✕" }}</span>
    <span class="visually-hidden">, {{ status.passed ? "réussie" : "en échec" }}</span>
  </span>
</template>

<style scoped>
.status-mark {
  flex: none;
  width: 1rem;
  font-size: var(--text-small);
  font-weight: 700;
  text-align: center;
}

.status-mark--ok {
  color: var(--color-success);
}

.status-mark--ko {
  color: var(--color-danger);
}
</style>
