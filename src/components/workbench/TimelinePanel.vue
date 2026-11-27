<script setup lang="ts">
/**
 * Chronologie d'un envoi (lot K, écran 5.1-c) : où le temps est passé, mesuré par le cœur
 * (`http::Timeline`). Chaque phase est une barre placée à la suite de la précédente ; une
 * connexion reprise du pool n'a ni DNS ni connexion, ce qui est dit plutôt que montré à zéro.
 */
import { computed } from "vue";

import { formatDuration } from "@/lib/format";
import type { Timeline } from "@/types/ipc";

const props = defineProps<{
  timeline: Timeline;
  /** Assertion de durée évaluée (« responseTime inférieur à 500 ms ») : elle porte sur le total. */
  durationCheck: string | null;
}>();

interface Row {
  label: string;
  ms: number;
  kind: "network" | "server" | "receive";
}

const rows = computed<Row[]>(() => {
  const t = props.timeline;
  const list: Row[] = [];
  if (t.dnsMs !== null) list.push({ label: "DNS", ms: t.dnsMs, kind: "network" });
  if (t.connectMs !== null) {
    list.push({
      label: t.secure ? "Connexion et TLS" : "Connexion",
      ms: t.connectMs,
      kind: "network",
    });
  }
  list.push({ label: "Attente du serveur", ms: t.serverMs, kind: "server" });
  list.push({ label: "Réception", ms: t.receiveMs, kind: "receive" });
  return list;
});

const reused = computed(() => props.timeline.connectMs === null);

/** Position et largeur de chaque barre, en pourcentage du total. */
function bar(index: number): Record<string, string> {
  const total = Math.max(props.timeline.totalMs, 1);
  const before = rows.value.slice(0, index).reduce((sum, r) => sum + r.ms, 0);
  const row = rows.value[index];
  const width = row ? Math.max((row.ms / total) * 100, 0.8) : 0;
  return {
    marginLeft: `${String(Math.min((before / total) * 100, 99))}%`,
    width: `${String(Math.min(width, 100))}%`,
  };
}
</script>

<template>
  <div class="timeline">
    <dl class="timeline__rows">
      <div v-for="(row, index) in rows" :key="row.label" class="timeline__row">
        <dt class="timeline__label">{{ row.label }}</dt>
        <dd class="timeline__track" aria-hidden="true">
          <span class="timeline__bar" :class="`timeline__bar--${row.kind}`" :style="bar(index)" />
        </dd>
        <dd class="timeline__ms">{{ formatDuration(row.ms) }}</dd>
      </div>
    </dl>
    <p class="timeline__total">
      <span>Total</span>
      <strong>{{ formatDuration(timeline.totalMs) }}</strong>
    </p>
    <p v-if="reused" class="timeline__note">
      Connexion réutilisée : ni DNS ni connexion pour cet envoi.
    </p>
    <p v-if="durationCheck" class="timeline__note">
      L'assertion « {{ durationCheck }} » porte sur ce total.
    </p>
  </div>
</template>

<style scoped>
.timeline {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem 0;
}

.timeline__rows {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0;
}

.timeline__row {
  display: grid;
  grid-template-columns: 9rem minmax(0, 1fr) 4.5rem;
  align-items: center;
  gap: 0.75rem;
}

.timeline__label,
.timeline__ms,
.timeline__track {
  margin: 0;
}

.timeline__ms {
  text-align: right;
  font: var(--text-small) / 1.4 var(--font-mono);
  color: var(--color-muted);
}

.timeline__track {
  display: flex;
  height: 0.625rem;
}

.timeline__bar {
  display: block;
  height: 100%;
  border-radius: var(--radius-sm);
}

.timeline__bar--network {
  background: var(--color-muted);
}

.timeline__bar--server {
  background: var(--color-accent);
}

.timeline__bar--receive {
  background: var(--color-success);
}

.timeline__total {
  display: flex;
  justify-content: space-between;
  margin: 0;
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-border);
}

.timeline__total strong {
  font: 600 var(--text-body) / 1.4 var(--font-mono);
}

.timeline__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
