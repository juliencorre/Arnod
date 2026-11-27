<script setup lang="ts">
/**
 * Onglet « Historique » de la réponse (écran 5.1-c bis, version 1, `docs/plan-v1.md`) : les
 * derniers envois de la requête, gardés sur ce poste. Chacun se déplie sur place (corps,
 * en-têtes, envoyé, assertions) et se compare au précédent : le cœur résume ce qui a changé et
 * donne les différences ligne à ligne (« Voir les différences »). « Effacer » se rattrape par
 * « Annuler ».
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel, formatBytes, formatDuration, formatSentAt } from "@/lib/format";
import { onTablistKeydown } from "@/lib/tablist";
import { useHistoryStore } from "@/stores/history";
import { useToastStore } from "@/stores/toasts";
import type { HistoryItem } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const history = useHistoryStore();
const toasts = useToastStore();
const guard = useGuard();

const view = computed(() => history.viewOf(props.projectId, props.path));
const items = computed(() => view.value?.items ?? []);

/** Envoi déplié : le plus récent par défaut. */
const open = ref<string | null>(null);
watch(
  () => items.value[0]?.entry.id,
  (first) => {
    open.value = first ?? null;
  },
  { immediate: true },
);

type Part = "body" | "headers" | "sent" | "checks";
const part = ref<Part>("body");
/** Envois dont les différences sont montrées. */
const shownDiffs = ref(new Set<string>());

function toggle(id: string): void {
  open.value = open.value === id ? null : id;
  part.value = "body";
}

function passedOf(item: HistoryItem): string {
  const checks = item.entry.assertions;
  return `${String(checks.filter((c) => c.passed).length)}/${String(checks.length)}`;
}

function bodyOf(item: HistoryItem): string {
  try {
    return JSON.stringify(JSON.parse(item.entry.body), null, 2);
  } catch {
    return item.entry.body;
  }
}

/** « Comparé à hier 17:20 : l'en-tête Location a disparu ; 4 ms de plus. » */
function comparison(item: HistoryItem): string | null {
  if (item.previousAt === null) return null;
  const changes =
    item.changes.length === 0 ? "rien de notable n'a changé" : item.changes.join(" ; ");
  return `Comparé à ${formatSentAt(item.previousAt)} : ${changes}.`;
}

async function showDiff(id: string): Promise<void> {
  if (shownDiffs.value.has(id)) {
    shownDiffs.value.delete(id);
    return;
  }
  await history.diff(props.projectId, props.path, id);
  shownDiffs.value.add(id);
}

async function clear(): Promise<void> {
  const { projectId, path } = props;
  const cleared = await history.clear(projectId, path);
  toasts.show(`${countLabel(cleared.length, "envoi effacé", "envois effacés")} de l'historique.`, {
    action: { label: "Annuler", run: () => history.restore(projectId, path, cleared) },
  });
}

const PARTS: [Part, string][] = [
  ["body", "Corps"],
  ["headers", "En-têtes"],
  ["sent", "Envoyé"],
  ["checks", "Assertions"],
];
</script>

<template>
  <div class="history">
    <div class="history__head">
      <p class="history__count">
        {{ countLabel(items.length, "envoi", "envois") }} sur ce poste, hors Git
      </p>
      <BaseButton v-if="items.length > 0" variant="link" @click="guard(clear)">Effacer</BaseButton>
    </div>

    <p v-if="items.length === 0" class="history__empty">
      Aucun envoi gardé : chaque réponse reçue rejoint cet historique.
    </p>
    <ul v-else class="history__list" aria-label="Envois de la requête">
      <li
        v-for="item in items"
        :key="item.entry.id"
        class="history__item"
        :class="{ 'history__item--open': open === item.entry.id }"
      >
        <button
          type="button"
          class="history__row"
          :aria-expanded="open === item.entry.id"
          @click="toggle(item.entry.id)"
        >
          <span class="history__when">{{ formatSentAt(item.entry.at) }}</span>
          <span
            class="history__status"
            :class="item.entry.status < 400 ? 'history__status--ok' : 'history__status--ko'"
            >{{ item.entry.status }}</span
          >
          <span class="history__time">{{ formatDuration(item.entry.totalMs) }}</span>
          <span
            v-if="item.entry.assertions.length > 0"
            class="history__checks"
            :class="item.entry.passed ? 'history__checks--ok' : 'history__checks--ko'"
            :title="item.entry.passed ? 'Réussie' : 'En échec'"
          >
            <span aria-hidden="true">{{ item.entry.passed ? "✓" : "✕" }}</span>
            {{ passedOf(item) }}
          </span>
          <span v-if="item.entry.environment" class="history__env">{{
            item.entry.environment
          }}</span>
        </button>

        <div v-if="open === item.entry.id" class="history__detail">
          <div
            role="tablist"
            aria-label="Contenu de l'envoi"
            class="history__parts"
            @keydown="onTablistKeydown"
          >
            <button
              v-for="[id, label] in PARTS"
              :key="id"
              type="button"
              role="tab"
              class="history__part"
              :aria-selected="part === id"
              :tabindex="part === id ? 0 : -1"
              @click="part = id"
            >
              {{ label }}
            </button>
          </div>
          <template v-if="part === 'body'">
            <pre class="history__code">{{ bodyOf(item) }}</pre>
            <p v-if="item.entry.truncated" class="history__note">
              Corps tronqué : seuls les {{ formatBytes(item.entry.body.length) }} premiers sont
              gardés ({{ formatBytes(item.entry.sizeBytes) }} reçus).
            </p>
          </template>
          <dl v-else-if="part === 'headers'" class="history__pairs">
            <template v-for="[name, value] in item.entry.headers" :key="name + value">
              <dt>{{ name }}</dt>
              <dd>{{ value }}</dd>
            </template>
          </dl>
          <div v-else-if="part === 'sent'" class="history__sent">
            <template v-if="item.entry.sent">
              <p class="history__line">
                <b>{{ item.entry.sent.method }}</b> {{ item.entry.sent.url }}
              </p>
              <dl class="history__pairs">
                <template v-for="[name, value] in item.entry.sent.headers" :key="name + value">
                  <dt>{{ name }}</dt>
                  <dd>{{ value }}</dd>
                </template>
              </dl>
              <pre v-if="item.entry.sent.body" class="history__code">{{
                item.entry.sent.body
              }}</pre>
            </template>
            <p v-else class="history__note">Ce qui est parti n'a pas été gardé.</p>
          </div>
          <ul v-else class="history__assertions">
            <li
              v-for="check in item.entry.assertions"
              :key="check.id"
              :class="check.passed ? 'history__ok' : 'history__ko'"
            >
              <span aria-hidden="true">{{ check.passed ? "✓" : "✕" }}</span>
              {{ check.target || check.id }}
              <span v-if="check.message" class="history__note">{{ check.message }}</span>
            </li>
            <li v-if="item.entry.assertions.length === 0" class="history__note">
              Aucune assertion.
            </li>
          </ul>

          <p v-if="comparison(item)" class="history__compare">
            {{ comparison(item) }}
            <button
              v-if="item.changes.length > 0"
              type="button"
              class="history__link"
              :aria-expanded="shownDiffs.has(item.entry.id)"
              @click="guard(() => showDiff(item.entry.id))"
            >
              {{
                shownDiffs.has(item.entry.id) ? "Masquer les différences" : "Voir les différences"
              }}
            </button>
          </p>
          <ul
            v-if="shownDiffs.has(item.entry.id) && history.diffs[item.entry.id]"
            class="history__diff"
            aria-label="Différences avec l'envoi précédent"
          >
            <li
              v-for="(line, index) in history.diffs[item.entry.id]?.lines"
              :key="index"
              :class="line.added ? 'history__added' : 'history__removed'"
            >
              <span class="history__sign" aria-hidden="true">{{ line.added ? "+" : "−" }}</span>
              <span class="visually-hidden">{{ line.added ? "Ajouté" : "Retiré" }} :</span>
              {{ line.text }}
            </li>
            <li v-if="history.diffs[item.entry.id]?.truncated" class="history__note">
              Différences coupées : seules les premières lignes sont montrées.
            </li>
          </ul>
        </div>
      </li>
    </ul>

    <p class="history__footer">
      Les {{ view?.max ?? 20 }} derniers envois de chaque requête restent sur ce poste ; rien n'est
      partagé avec l'équipe. L'horloge de l'en-tête ouvre cet onglet ; un envoi déplié se compare au
      précédent.
    </p>
  </div>
</template>

<style scoped>
.history {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.history__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.history__count,
.history__empty,
.history__footer {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.history__list {
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.history__item + .history__item {
  border-top: 1px solid var(--color-border);
}

.history__item--open {
  background-color: var(--color-surface);
}

.history__row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  min-height: var(--control-md);
  padding: 0 0.625rem;
  border: 0;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.history__row:hover {
  background-color: var(--color-sidebar);
}

.history__row:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.history__when {
  min-width: 6.5rem;
  font-variant-numeric: tabular-nums;
}

.history__status {
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font: 600 var(--text-small) var(--font-mono);
}

.history__status--ok {
  color: var(--color-success);
  background-color: var(--color-success-soft);
}

.history__status--ko {
  color: var(--color-danger);
  background-color: var(--color-danger-soft);
}

.history__time {
  min-width: 3.5rem;
  color: var(--color-muted);
  font-variant-numeric: tabular-nums;
}

.history__checks {
  font-size: var(--text-small);
}

.history__checks--ok,
.history__ok {
  color: var(--color-success);
}

.history__checks--ko,
.history__ko {
  color: var(--color-danger);
}

.history__env {
  margin-left: auto;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.history__detail {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0 0.625rem 0.625rem;
}

.history__parts {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--color-border);
}

.history__part {
  padding: 0.25rem 0.375rem;
  border: 0;
  border-bottom: 2px solid transparent;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.history__part[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.history__part:focus-visible,
.history__link:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.history__code {
  max-height: 16rem;
  margin: 0;
  overflow: auto;
  font: var(--editor-font-size) / 1.5 var(--editor-font);
  white-space: pre-wrap;
}

.history__pairs {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.125rem 1rem;
  margin: 0;
  font: var(--text-small) var(--font-mono);
}

.history__pairs dt {
  font-weight: 600;
}

.history__pairs dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.history__line {
  margin: 0;
  font: var(--text-small) var(--font-mono);
  overflow-wrap: anywhere;
}

.history__sent {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.history__assertions {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.history__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.history__compare {
  margin: 0;
  padding: 0.375rem 0.625rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  background-color: var(--color-chip);
}

.history__link {
  min-height: 1.5rem;
  padding: 0;
  border: 0;
  font: inherit;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.history__diff {
  margin: 0;
  padding: 0.375rem 0;
  list-style: none;
  font: var(--text-small) / 1.5 var(--font-mono);
}

.history__diff li {
  padding: 0 0.5rem;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.history__added {
  color: var(--color-success);
  background-color: var(--color-success-soft);
}

.history__removed {
  color: var(--color-danger);
  background-color: var(--color-danger-soft);
}

.history__sign {
  display: inline-block;
  width: 1rem;
}
</style>
