<script setup lang="ts">
/**
 * Run d'un dossier ou d'un contrat (maquette 1f, `docs/plan-fin-premier-usage.md`, RU1 à RU6) :
 * résumé, barre, onglets « Échecs », « Tous », « Lents », détail des vérifications en échec
 * (ignorables pour ce run), résultat par dossier qui filtre la liste. Le cœur calcule tout
 * (statuts, groupes) ; ce composant ne fait que filtrer et compter pour l'affichage.
 */
import { computed, nextTick, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { formatDuration } from "@/lib/format";
import { useWorkbenchStore } from "@/stores/workbench";
import type { AssertionResult, RunItem, RunItemStatus } from "@/types/ipc";

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

const MARKS: Record<RunItemStatus, string> = {
  pending: "·",
  running: "…",
  passed: "✓",
  failed: "✕",
  error: "✕",
  cancelled: "–",
};

/** Identifiant du résultat de conformité au contrat lié (`CONTRACT_CHECK`). */
const CONTRACT_CHECK = "contrat";

const isBad = (item: RunItem): boolean => item.status === "failed" || item.status === "error";

const counts = computed(() => {
  const items = run.value?.items ?? [];
  return {
    total: items.length,
    done: items.filter((i) => !["pending", "running"].includes(i.status)).length,
    passed: items.filter((i) => i.status === "passed").length,
    bad: items.filter(isBad).length,
    slow: items.filter((i) => i.slow).length,
  };
});

const plural = (count: number, word: string): string =>
  `${String(count)} ${word}${count > 1 ? "s" : ""}`;

/** Résumé du run (RU1) : avancement, puis réussis et échecs. */
const summary = computed(() => {
  const current = run.value;
  const { done, total, passed, bad } = counts.value;
  if (!current) return "";
  if (!current.finished) return `En cours · ${String(done)}/${String(total)}`;
  if (current.cancelled) return `Annulé · ${String(done)}/${String(total)}`;
  if (bad === 0) return `${plural(passed, "réussi")} · tout est réussi`;
  return `${plural(passed, "réussi")}, ${String(bad)} en échec`;
});

/** Ligne `run #3 · Recette · 8 en même temps · 4,1 s` (RU1). */
const meta = computed(() => {
  const current = run.value;
  if (!current) return "";
  const parts = [
    current.number > 0 ? `run #${String(current.number)}` : "run",
    current.environment,
    `${String(current.concurrency)} en même temps`,
  ];
  if (current.finishedAt !== null) {
    parts.push(formatDuration(Math.max(0, current.finishedAt - current.startedAt)));
  }
  return parts.join(" · ");
});

/** Parts de la barre, en pourcentage du total. */
const bar = computed(() => {
  const { total, passed, bad } = counts.value;
  if (total === 0) return { passed: 0, bad: 0 };
  return { passed: (passed / total) * 100, bad: (bad / total) * 100 };
});

// --- Onglets et filtres (RU2, RU4) ------------------------------------------------

type View = "failures" | "all" | "slow";
const chosen = ref<View | null>(null);
const group = ref<string | null>(null);
const layout = ref<"groups" | "flat">("groups");

// Un autre run remet les filtres à zéro.
watch(
  () => run.value?.id,
  () => {
    chosen.value = null;
    group.value = null;
  },
);

/** Onglet affiché : « Échecs » par défaut quand le run terminé en a, sinon « Tous ». */
const view = computed<View>(
  () => chosen.value ?? (run.value?.finished && counts.value.bad > 0 ? "failures" : "all"),
);

const tabs = computed<{ id: View; label: string; count: number }[]>(() => [
  { id: "failures", label: "Échecs", count: counts.value.bad },
  { id: "all", label: "Tous", count: counts.value.total },
  { id: "slow", label: "Lents", count: counts.value.slow },
]);

const tabButtons = ref<HTMLButtonElement[]>([]);

/** Flèches gauche et droite, Début et Fin entre les onglets (motif WAI-ARIA). */
async function onTabKey(event: KeyboardEvent, index: number): Promise<void> {
  const last = tabs.value.length - 1;
  const next =
    event.key === "ArrowRight"
      ? index === last
        ? 0
        : index + 1
      : event.key === "ArrowLeft"
        ? index === 0
          ? last
          : index - 1
        : event.key === "Home"
          ? 0
          : event.key === "End"
            ? last
            : null;
  if (next === null) return;
  event.preventDefault();
  const tab = tabs.value[next];
  if (!tab) return;
  chosen.value = tab.id;
  await nextTick();
  tabButtons.value[next]?.focus();
}

/**
 * Groupe d'une requête (R2, calculé par le cœur) : le plus long chemin de groupe qui la contient,
 * puisque le groupe du dossier exécuté contient aussi ceux de ses sous-dossiers.
 */
function groupOf(item: RunItem): string {
  let best: string | null = null;
  for (const g of run.value?.groups ?? []) {
    const holds = g.path === "" || item.path.startsWith(`${g.path}/`);
    if (holds && (best === null || g.path.length > best.length)) best = g.path;
  }
  return best ?? "";
}

const shown = computed(() =>
  (run.value?.items ?? []).filter((item) => {
    if (view.value === "failures" && !isBad(item)) return false;
    if (view.value === "slow" && !item.slow) return false;
    return group.value === null || groupOf(item) === group.value;
  }),
);

/** Lignes regroupées par dossier, dans l'ordre des groupes (RU4). */
const sections = computed(() => {
  if (layout.value === "flat") return [{ path: null, name: "", items: shown.value }];
  return (run.value?.groups ?? [])
    .map((g) => ({
      path: g.path,
      name: g.name,
      items: shown.value.filter((item) => groupOf(item) === g.path),
    }))
    .filter((section) => section.items.length > 0);
});

function toggleGroup(path: string): void {
  group.value = group.value === path ? null : path;
}

// --- Détail des échecs (RU3, VT2) ----------------------------------------------

const label = (check: AssertionResult): string =>
  check.target === CONTRACT_CHECK ? "conformité au contrat" : check.target || check.id;

const failures = (item: RunItem): AssertionResult[] =>
  item.assertions.filter((a) => !a.passed && !a.ignored);

const ignored = (item: RunItem): AssertionResult[] =>
  item.assertions.filter((a) => !a.passed && a.ignored);

/** « — body.amount » : la première vérification en échec, à côté du nom. */
function firstFailure(item: RunItem): string | null {
  const first = failures(item)[0];
  return item.status === "failed" && first ? label(first) : null;
}

const EMPTY: Record<View, string> = {
  failures: "Aucun échec.",
  all: "Aucune requête.",
  slow: "Aucune requête lente.",
};
</script>

<template>
  <section v-if="run" class="run" aria-label="Run">
    <header class="run__head">
      <div class="run__intro">
        <p class="run__meta">{{ meta }}</p>
        <h2 class="run__title">{{ heading }}</h2>
        <p class="run__progress" role="status">
          <template v-if="run.finished && !run.cancelled && counts.bad > 0">
            {{ plural(counts.passed, "réussi") }},
            <span class="run__bad">{{ counts.bad }} en échec</span>
          </template>
          <template v-else>{{ summary }}</template>
        </p>
      </div>
      <span class="run__spacer" />
      <BaseButton v-if="!run.finished" @click="workbench.cancelRun(projectId)">Annuler</BaseButton>
      <BaseButton v-else-if="counts.bad > 0" @click="workbench.retryFailures(projectId)">
        Relancer les échecs
      </BaseButton>
    </header>

    <div v-if="counts.total > 0" class="run__bar" aria-hidden="true">
      <span class="run__bar-passed" :style="{ width: `${bar.passed}%` }" />
      <span class="run__bar-bad" :style="{ width: `${bar.bad}%` }" />
    </div>

    <p v-if="counts.total === 0" class="run__empty">Aucune requête à exécuter dans ce dossier.</p>

    <div v-else class="run__body">
      <div class="run__card">
        <div class="run__toolbar">
          <div role="tablist" aria-label="Requêtes du run" class="run__tabs">
            <button
              v-for="(tab, index) in tabs"
              :key="tab.id"
              ref="tabButtons"
              type="button"
              role="tab"
              class="run__tab"
              :aria-selected="view === tab.id"
              :tabindex="view === tab.id ? 0 : -1"
              @click="chosen = tab.id"
              @keydown="onTabKey($event, index)"
            >
              {{ tab.label }} <span class="run__tab-count">{{ tab.count }}</span>
            </button>
          </div>
          <span class="run__spacer" />
          <label class="run__layout">
            <span class="visually-hidden">Présentation</span>
            <select v-model="layout">
              <option value="groups">Grouper par dossier</option>
              <option value="flat">À plat</option>
            </select>
          </label>
        </div>
        <div
          role="tabpanel"
          class="run__panel"
          :aria-label="tabs.find((t) => t.id === view)?.label"
        >
          <p v-if="shown.length === 0" class="run__none">{{ EMPTY[view] }}</p>
          <template v-for="section in sections" :key="section.path ?? 'flat'">
            <h3 v-if="section.path !== null" class="run__section">{{ section.name }}</h3>
            <ul class="run__items">
              <li
                v-for="item in section.items"
                :key="item.path"
                class="run__item"
                :class="`run__item--${item.status}`"
              >
                <div class="run__line">
                  <span class="run__mark" aria-hidden="true">{{ MARKS[item.status] }}</span>
                  <span class="run__method" :class="`run__method--${item.method.toLowerCase()}`">
                    {{ item.method }}
                  </span>
                  <button
                    type="button"
                    class="run__name"
                    @click="workbench.openRequest(projectId, item.path)"
                  >
                    {{ item.name }}
                  </button>
                  <span v-if="firstFailure(item)" class="run__check">
                    — {{ firstFailure(item) }}
                  </span>
                  <span class="run__status">{{ LABELS[item.status] }}</span>
                  <span class="run__spacer" />
                  <span v-if="item.slow" class="run__slow">lente</span>
                  <span v-if="item.httpStatus" class="run__info">{{ item.httpStatus }}</span>
                  <span v-if="item.durationMs !== null" class="run__info">
                    {{ formatDuration(item.durationMs) }}
                  </span>
                </div>
                <p v-if="item.error" class="run__error">{{ item.error }}</p>
                <ul v-if="failures(item).length + ignored(item).length > 0" class="run__checks">
                  <li v-for="check in failures(item)" :key="check.id" class="run__failure">
                    <span class="run__error">
                      <span class="run__target">{{ label(check) }}</span> :
                      {{ check.message ?? "en échec" }}
                    </span>
                    <button
                      type="button"
                      class="run__link"
                      :aria-label="`Ignorer ${label(check)} pour ce run`"
                      @click="workbench.ignoreInRun(projectId, item.path, check.id)"
                    >
                      Ignorer pour ce run
                    </button>
                  </li>
                  <li v-for="check in ignored(item)" :key="check.id" class="run__ignored">
                    {{ label(check) }} : ignorée pour ce run
                  </li>
                </ul>
              </li>
            </ul>
          </template>
        </div>
      </div>

      <aside class="run__groups" aria-labelledby="run-groups-title">
        <h3 id="run-groups-title" class="run__groups-title">Résultat par dossier</h3>
        <ul class="run__group-list">
          <li v-for="g in run.groups" :key="g.path">
            <button
              type="button"
              class="run__group"
              :aria-pressed="group === g.path"
              :aria-label="`${g.name} : ${g.passed} réussies sur ${g.total}${g.failed > 0 ? `, ${g.failed} en échec` : ''}`"
              @click="toggleGroup(g.path)"
            >
              <span class="run__group-name">{{ g.name }}</span>
              <span class="run__group-bar" aria-hidden="true">
                <span
                  class="run__bar-passed"
                  :style="{ width: `${g.total ? (g.passed / g.total) * 100 : 0}%` }"
                />
                <span
                  class="run__bar-bad"
                  :style="{ width: `${g.total ? (g.failed / g.total) * 100 : 0}%` }"
                />
              </span>
              <span class="run__group-count" :class="{ 'run__group-count--bad': g.failed > 0 }">
                {{ g.passed }}/{{ g.total }}
              </span>
            </button>
          </li>
        </ul>
      </aside>
    </div>
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
  gap: 0.75rem;
  padding: 0.25rem 0;
  container-type: inline-size;
}

.run__head {
  display: flex;
  align-items: flex-end;
  gap: 0.75rem;
}

.run__intro {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.run__meta {
  margin: 0;
  font: var(--text-body) var(--font-mono);
  color: var(--color-muted);
}

.run__title {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
  color: var(--color-muted);
}

.run__progress {
  margin: 0;
  font-size: var(--text-display);
  font-weight: 700;
  color: var(--color-text);
}

.run__bad {
  color: var(--color-danger);
}

.run__spacer {
  flex: 1;
}

.run__bar,
.run__group-bar {
  display: flex;
  overflow: hidden;
  border-radius: var(--radius-pill);
  background: var(--color-border);
}

.run__bar {
  height: 0.5rem;
}

.run__bar-passed {
  background: var(--color-success);
}

.run__bar-bad {
  background: var(--color-danger);
}

.run__empty,
.run__none {
  margin: 0.5rem 0;
  color: var(--color-muted);
}

.run__body {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(12rem, 1fr);
  gap: 0.75rem;
  align-items: start;
}

@container (max-width: 40rem) {
  .run__body {
    grid-template-columns: minmax(0, 1fr);
  }
}

.run__card,
.run__groups {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.run__toolbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.run__tabs {
  display: flex;
  gap: 0.25rem;
}

.run__tab {
  padding: 0.25rem 0.5rem;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  cursor: pointer;
}

.run__tab[aria-selected="true"] {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-accent-soft);
}

.run__layout select {
  min-height: var(--control-sm);
  padding: 0 1.75rem 0 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  background-color: var(--color-surface);
}

.run__panel {
  padding: 0.25rem 0;
}

.run__panel > .run__none {
  padding: 0 0.75rem;
}

.run__section {
  margin: 0.5rem 0 0;
  padding: 0 0.75rem;
  font-size: var(--text-small);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-muted);
}

.run__items,
.run__checks,
.run__group-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.run__item {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--color-border);
  font-size: var(--text-body);
}

.run__item:last-child {
  border-bottom: 0;
}

.run__item--failed,
.run__item--error {
  background: var(--color-danger-soft);
}

.run__line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.run__mark {
  width: 1rem;
  font-weight: 700;
  text-align: center;
  color: var(--color-muted);
}

.run__item--passed .run__mark {
  color: var(--color-success);
}

.run__item--failed .run__mark,
.run__item--error .run__mark {
  color: var(--color-danger);
}

.run__method {
  min-width: 3rem;
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.run__method--get {
  color: var(--color-success);
}

.run__method--post,
.run__method--put,
.run__method--patch {
  color: var(--color-link);
}

.run__method--delete {
  color: var(--color-danger);
}

.run__name {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-text);
  cursor: pointer;
  text-align: left;
}

.run__check,
.run__info,
.run__status {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.run__status {
  font-size: var(--text-small);
}

.run__slow {
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  background: var(--color-chip);
}

.run__checks {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0.25rem 0 0 1.5rem;
}

.run__failure {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
}

.run__error {
  margin: 0.25rem 0 0 1.5rem;
  font: var(--text-body) var(--font-mono);
  color: var(--color-danger);
}

.run__failure .run__error {
  margin: 0;
}

.run__target {
  font-weight: 700;
}

.run__ignored {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.run__link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  text-decoration: underline;
  color: var(--color-link);
  cursor: pointer;
}

.run__groups {
  padding: 0.5rem 0;
}

.run__groups-title {
  margin: 0 0 0.25rem;
  padding: 0 0.75rem;
  font-size: var(--text-body);
}

.run__group {
  display: grid;
  grid-template-columns: minmax(5rem, 1fr) minmax(3rem, 2fr) auto;
  gap: 0.75rem;
  align-items: center;
  width: 100%;
  padding: 0.375rem 0.75rem;
  border: 0;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.run__group[aria-pressed="true"] {
  background: var(--color-accent-soft);
}

.run__group-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.run__group-bar {
  height: 0.375rem;
}

.run__group-count {
  font: var(--text-body) var(--font-mono);
}

.run__group-count--bad {
  color: var(--color-danger);
}

:is(.run__tab, .run__name, .run__link, .run__group, .run__layout select):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
