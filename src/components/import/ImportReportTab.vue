<script setup lang="ts">
/**
 * Rapport d'import (2.3-f, docs/plan-mvp-c-import-bruno.md) : ce qui a été repris (comptes),
 * ce qui est signalé avec son fichier d'origine (« À reprendre »), des exemples convertis sans
 * code, et le détail de l'élément choisi : avant (Bruno), après (dans Arnod). Le rapport est
 * calculé par le cœur et gardé sur le poste.
 */
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { countLabel, formatDayTime } from "@/lib/format";
import {
  FORMAT_LABELS,
  KIND_TAGS,
  STATUS_HEADLINES,
  STATUS_LABELS,
  originalBlock,
} from "@/lib/imports";
import { useCatalogStore } from "@/stores/catalog";
import { useImportStore } from "@/stores/imports";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";

const props = defineProps<{ projectId: string }>();

/** Lignes montrées avant « … N autres ». */
const SHOWN = 5;

const imports = useImportStore();
const catalog = useCatalogStore();
const windows = useWindowsStore();
const toasts = useToastStore();
const list = useTemplateRef<HTMLElement>("list");

const report = computed(() => imports.reports[props.projectId] ?? null);
const loaded = ref(false);
const selected = ref(0);
const expanded = ref(false);

async function load(): Promise<void> {
  loaded.value = false;
  try {
    if (imports.reports[props.projectId] === undefined) await imports.load(props.projectId);
  } catch (error) {
    toasts.showError(error);
  } finally {
    loaded.value = true;
  }
}

onMounted(load);
watch(() => props.projectId, load);

const projectName = computed(
  () => catalog.projectById(props.projectId)?.name ?? report.value?.projectName ?? "projet",
);
const items = computed(() => report.value?.items ?? []);
const visible = computed(() => (expanded.value ? items.value : items.value.slice(0, SHOWN)));
const current = computed(() => items.value[selected.value] ?? null);

const subtitle = computed(() => {
  const r = report.value;
  if (!r) return "";
  const parts = [FORMAT_LABELS[r.format]];
  if (r.format === "bruno") parts.push(countLabel(r.sourceFiles, "fichier .bru", "fichiers .bru"));
  parts.push(
    r.folder ? `dans ${projectName.value}, dossier « ${r.folder} »` : `dans ${projectName.value}`,
  );
  parts.push(formatDayTime(r.importedAt));
  return parts.join(" · ");
});

const stats = computed(() => {
  const c = report.value?.counts;
  if (!c) return [];
  return [
    { value: c.folders, label: c.folders > 1 ? "dossiers" : "dossier" },
    { value: c.requests, label: c.requests > 1 ? "requêtes" : "requête" },
    { value: c.environments, label: c.environments > 1 ? "environnements" : "environnement" },
    {
      value: c.assertions,
      label: c.assertions > 1 ? "vérifications sans code" : "vérification sans code",
    },
    { value: c.extractions, label: c.extractions > 1 ? "extractions" : "extraction" },
    {
      value: c.secrets,
      label: c.secrets > 1 ? "secrets en variables locales" : "secret en variable locale",
    },
  ];
});

function select(index: number): void {
  selected.value = index;
}

async function focusRow(index: number): Promise<void> {
  await nextTick();
  list.value?.querySelectorAll<HTMLElement>("[data-row]")[index]?.focus();
}

/** Flèches haut et bas, Début et Fin : la ligne suivante est choisie et prend le focus. */
function onKeydown(event: KeyboardEvent, index: number): void {
  const last = visible.value.length - 1;
  const target =
    event.key === "ArrowDown"
      ? Math.min(index + 1, last)
      : event.key === "ArrowUp"
        ? Math.max(index - 1, 0)
        : event.key === "Home"
          ? 0
          : event.key === "End"
            ? last
            : null;
  if (target === null) return;
  event.preventDefault();
  select(target);
  void focusRow(target);
}

async function openRequest(path: string): Promise<void> {
  await windows.openSelection({ kind: "request", projectId: props.projectId, path });
}

const before = computed(() => (current.value ? originalBlock(current.value) : null));
</script>

<template>
  <section class="report" aria-labelledby="report-title">
    <nav class="report__crumbs" aria-label="Emplacement">
      <span>{{ projectName }}</span>
      <span class="report__sep" aria-hidden="true">/</span>
      <span class="report__crumb-current">Rapport d'import</span>
    </nav>

    <p v-if="loaded && !report" class="report__empty">
      Aucun rapport d'import pour ce projet sur ce poste.
    </p>

    <template v-if="report">
      <header class="report__head">
        <h1 id="report-title" class="report__title">
          {{ report.collection }} {{ report.format === "bruno" ? "importée" : "importé" }}
          <span class="report__subtitle">{{ subtitle }}</span>
        </h1>
        <p class="report__note">
          Les fichiers .bru d'origine ne sont pas modifiés : Arnod écrit le projet au format
          OpenCollection.
        </p>
      </header>

      <ul class="report__stats" aria-label="Repris">
        <li v-for="stat in stats" :key="stat.label" class="report__stat">
          <strong class="report__stat-value">{{ stat.value }}</strong>
          <span class="report__stat-label">{{ stat.label }}</span>
        </li>
      </ul>

      <div class="report__columns">
        <div class="report__main">
          <h2 class="report__heading">
            À reprendre <span class="report__count">· {{ items.length }}</span>
          </h2>
          <p v-if="items.length === 0" class="report__text">
            Rien à reprendre : tout a été converti sans code.
          </p>
          <ul v-else ref="list" class="report__items">
            <li v-for="(item, index) in visible" :key="index">
              <button
                type="button"
                data-row
                class="report__item"
                :aria-pressed="index === selected"
                :tabindex="index === selected ? 0 : -1"
                @click="select(index)"
                @keydown="onKeydown($event, index)"
              >
                <span class="report__tag">{{ KIND_TAGS[item.kind] }}</span>
                <span class="report__item-text">
                  <span class="report__item-name">{{ item.name }}</span>
                  <span class="report__item-place">{{ item.place || item.file }}</span>
                </span>
                <span class="report__status" :class="`report__status--${item.status}`">{{
                  STATUS_LABELS[item.status]
                }}</span>
              </button>
            </li>
          </ul>
          <button
            v-if="items.length > SHOWN"
            type="button"
            class="report__more"
            :aria-expanded="expanded"
            @click="expanded = !expanded"
          >
            {{
              expanded
                ? "Montrer moins"
                : `… ${countLabel(items.length - SHOWN, "autre", "autres")}`
            }}
          </button>

          <p v-if="report.format === 'bruno'" class="report__text">
            bruno.json devient le projet, collection.bru ses réglages (authentification, en-têtes),
            folder.bru ceux de chaque dossier ; l'ordre seq est gardé. Les environnements Bruno
            deviennent ceux du projet ; vars:secret et .env vont en variables locales, au coffre du
            système, jamais dans Git.
          </p>

          <template v-if="report.examples.length > 0">
            <h3 class="report__label">Converti sans code · exemples</h3>
            <ul class="report__examples">
              <li v-for="example in report.examples" :key="example.before" class="report__example">
                <code class="report__before">{{ example.before }}</code>
                <code class="report__after">→ {{ example.after }}</code>
              </li>
            </ul>
          </template>
        </div>

        <aside v-if="current" class="report__detail" aria-label="Élément choisi">
          <div class="report__detail-head">
            <h2 class="report__heading">
              {{ current.place ? `${current.place} · ${current.name}` : current.name }}
            </h2>
            <span v-if="current.block" class="report__block">bloc {{ current.block }}</span>
            <BaseButton
              v-if="current.request"
              variant="link"
              class="report__open"
              @click="openRequest(current.request)"
            >
              Ouvrir la requête
            </BaseButton>
          </div>
          <p class="report__file">{{ current.file }}</p>
          <template v-if="before">
            <h3 class="report__label">Avant · Bruno</h3>
            <pre class="report__code">{{ before }}</pre>
          </template>
          <h3 class="report__label">Après · dans Arnod</h3>
          <div class="report__after-box">
            <p class="report__text">
              <strong>{{ STATUS_HEADLINES[current.status] }}</strong> {{ current.detail }}
            </p>
            <p v-if="current.status === 'notRun'" class="report__hint">
              Les vérifications et extractions de la requête sont reprises sans code : elle
              s'exécute sans ce script.
            </p>
          </div>
        </aside>
      </div>
    </template>
  </section>
</template>

<style scoped>
.report {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
  min-height: 100%;
  padding: 1.25rem 2rem 2rem;
  overflow-y: auto;
}

.report__crumbs {
  display: flex;
  gap: 0.5rem;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.report__crumb-current {
  font-weight: 600;
  color: var(--color-text);
}

.report__empty {
  margin: 2rem auto;
  color: var(--color-muted);
}

.report__head {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.report__title {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.25rem 0.75rem;
  margin: 0;
  font-size: var(--text-display);
  font-weight: 600;
}

.report__subtitle {
  font-size: var(--text-body);
  font-weight: 400;
  color: var(--color-muted);
}

.report__note {
  margin: 0;
  font-size: var(--text-body);
}

.report__stats {
  display: flex;
  flex-wrap: wrap;
  gap: 0.625rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.report__stat {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding: 0.5rem 0.875rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.report__stat-value {
  font-size: var(--text-title);
  font-weight: 600;
}

.report__stat-label {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.report__columns {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.55fr);
  gap: 1.25rem;
  align-items: start;
}

@media (width < 60rem) {
  .report__columns {
    grid-template-columns: minmax(0, 1fr);
  }
}

.report__main,
.report__detail {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  min-width: 0;
}

.report__heading {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.report__count {
  font-weight: 600;
}

.report__items {
  margin: 0;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  list-style: none;
  background: var(--color-surface);
}

.report__items li + li {
  border-top: 1px solid var(--color-border);
}

.report__item {
  display: grid;
  grid-template-columns: 4.75rem minmax(0, 1fr) auto;
  gap: 0.625rem;
  align-items: center;
  width: 100%;
  padding: 0.375rem 0.75rem;
  border: 0;
  border-left: 3px solid transparent;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: transparent;
  cursor: pointer;
}

.report__item:hover {
  background-color: var(--color-sidebar);
}

.report__item[aria-pressed="true"] {
  border-left-color: var(--color-text);
  background-color: var(--color-accent-soft);
}

.report__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.report__tag {
  font: 600 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.report__item-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.report__item-name,
.report__item-place {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.report__item-place {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.report__status {
  font-size: var(--text-body);
  color: var(--color-link);
}

.report__status--stored {
  color: var(--color-success);
}

.report__more {
  align-self: flex-start;
  padding: 0;
  border: 0;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.report__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.report__text {
  margin: 0;
  font-size: var(--text-body);
  line-height: 1.55;
}

.report__label {
  margin: 0.25rem 0 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.report__examples {
  margin: 0;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  list-style: none;
  background: var(--color-surface);
}

.report__example {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding: 0.375rem 0.75rem;
  font: var(--text-small) var(--font-mono);
}

.report__example + .report__example {
  border-top: 1px solid var(--color-border);
}

.report__before {
  color: var(--color-muted);
  overflow-wrap: anywhere;
}

.report__after {
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.report__detail-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.25rem 0.75rem;
}

.report__block {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.report__open {
  margin-left: auto;
}

.report__file {
  margin: -0.375rem 0 0;
  font: var(--text-small) var(--font-mono);
  color: var(--color-muted);
}

.report__code {
  margin: 0;
  padding: 0.75rem 0.875rem;
  overflow-x: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: var(--text-body) / 1.5 var(--font-mono);
  background: var(--color-sidebar);
}

.report__after-box {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem 0.875rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.report__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
