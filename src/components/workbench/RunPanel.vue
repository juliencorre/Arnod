<script setup lang="ts">
/** Run d'un dossier (X1 à X8) : avancement, résultat par requête, annulation, relance des échecs. */
import { computed } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { formatDuration } from "@/lib/format";
import { useWorkbenchStore } from "@/stores/workbench";
import type { AssertionResult, RunItemStatus } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
const run = computed(() => workbench.runs[props.projectId] ?? null);
const heading = computed(() => {
  const current = run.value;
  if (!current) return "";
  if (current.contract) return `Run du contrat « ${current.contract} »`;
  return `Run de ${current.folder === "" ? "tout le projet" : current.folder}`;
});

const LABELS: Record<RunItemStatus, string> = {
  pending: "En attente",
  running: "En cours",
  passed: "Réussie",
  failed: "Échec",
  error: "Erreur",
  cancelled: "Annulée",
};

const counts = computed(() => {
  const items = run.value?.items ?? [];
  const done = items.filter((i) => !["pending", "running"].includes(i.status)).length;
  const bad = items.filter((i) => i.status === "failed" || i.status === "error").length;
  return { total: items.length, done, bad };
});

/** Nombre de vérifications en échec et détail de la première (C8). */
function failureText(results: AssertionResult[]): string {
  const failed = results.filter((a) => !a.passed);
  const first = failed[0]?.message;
  const count = `${String(failed.length)} vérification(s) en échec`;
  return first ? `${count} : ${first}` : count;
}
</script>

<template>
  <section v-if="run" class="run" aria-label="Run">
    <header class="run__head">
      <h2 class="run__title">
        {{ heading }}
        <span class="run__env">· {{ run.environment }}</span>
      </h2>
      <span class="run__progress" role="status">
        {{ counts.done }}/{{ counts.total }}
        <template v-if="run.finished">
          ·
          {{
            run.cancelled
              ? "annulé"
              : counts.bad === 0
                ? "tout est réussi"
                : `${counts.bad} en échec`
          }}
        </template>
      </span>
      <span class="run__spacer" />
      <BaseButton v-if="!run.finished" @click="workbench.cancelRun(projectId)">Annuler</BaseButton>
      <BaseButton v-else-if="counts.bad > 0" @click="workbench.retryFailures(projectId)">
        Relancer les échecs
      </BaseButton>
    </header>
    <ul class="run__items">
      <li
        v-for="item in run.items"
        :key="item.path"
        class="run__item"
        :class="`run__item--${item.status}`"
      >
        <span class="run__method">{{ item.method }}</span>
        <button
          type="button"
          class="run__name"
          @click="workbench.openRequest(projectId, item.path)"
        >
          {{ item.name }}
        </button>
        <span class="run__status">{{ LABELS[item.status] }}</span>
        <span v-if="item.httpStatus" class="run__meta">{{ item.httpStatus }}</span>
        <span v-if="item.durationMs !== null" class="run__meta">
          {{ formatDuration(item.durationMs) }}
          <span v-if="item.slow" class="run__slow">lente</span>
        </span>
        <span v-if="item.error" class="run__error">{{ item.error }}</span>
        <span v-else-if="item.status === 'failed'" class="run__error">
          {{ failureText(item.assertions) }}
        </span>
      </li>
    </ul>
  </section>
  <p v-else class="run-none">
    Aucun run pour l'instant : ▶ sur la ligne du projet ou « Exécuter le dossier » dans la barre
    latérale.
  </p>
</template>

<style scoped>
.run-none {
  margin: 2rem 0;
  text-align: center;
  color: var(--color-muted);
}

.run {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
}

.run__head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.run__title {
  margin: 0;
  font-size: 1rem;
}

.run__env,
.run__progress,
.run__meta {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.run__spacer {
  flex: 1;
}

.run__items {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.run__item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.625rem;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-size: 0.875rem;
}

.run__item--passed {
  background: var(--color-success-soft);
}

.run__item--failed,
.run__item--error {
  background: var(--color-danger-soft);
}

.run__method {
  min-width: 3rem;
  font: 700 0.6875rem var(--font-mono);
}

.run__name {
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-text);
  cursor: pointer;
  text-align: left;
}

.run__name:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.run__status {
  font-size: 0.8125rem;
}

.run__slow {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  border-radius: 999px;
  background: var(--color-chip);
}

.run__error {
  color: var(--color-danger);
  font-size: 0.8125rem;
}
</style>
