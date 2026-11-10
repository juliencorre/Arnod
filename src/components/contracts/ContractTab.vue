<script setup lang="ts">
/**
 * Onglet d'un contrat (C5 à C10, C12) : en-tête, puis deux vues.
 * « Opérations » : opérations groupées par tag, cases à cocher pour créer les
 * requêtes manquantes, lien vers les requêtes liées, exécution du contrat et
 * requêtes hors contrat. « Source » : texte du contrat, enregistré s'il reste
 * lisible (Ctrl+S), ou remplacé par un fichier.
 */
import { computed, onMounted, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { contractKey, useContractsStore } from "@/stores/contracts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { OperationView } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const contracts = useContractsStore();
const workbench = useWorkbenchStore();
const key = computed(() => contractKey(props.projectId, props.path));
const entry = computed(() => contracts.open[key.value] ?? null);
const view = computed(() => entry.value?.view ?? null);

onMounted(() => {
  void contracts.openTab(props.projectId, props.path);
});
watch(key, () => {
  void contracts.openTab(props.projectId, props.path);
});

const tab = computed({
  get: () => entry.value?.tab ?? "operations",
  set: (value) => {
    if (entry.value) entry.value.tab = value;
  },
});

const kindLabel = computed(() => {
  const current = view.value;
  if (!current?.kind) return "";
  return current.kind === "swagger2"
    ? `Swagger ${current.specVersion}`
    : `OpenAPI ${current.specVersion}`;
});

/** Opérations groupées par tag, dans l'ordre du contrat. */
const groups = computed(() => {
  const list: { tag: string | null; operations: OperationView[] }[] = [];
  for (const operation of view.value?.operations ?? []) {
    let group = list.find((g) => g.tag === operation.tag);
    if (!group) {
      group = { tag: operation.tag, operations: [] };
      list.push(group);
    }
    group.operations.push(operation);
  }
  return list;
});

const total = computed(() => view.value?.operations.length ?? 0);
const linked = computed(() => view.value?.operations.filter((o) => o.request !== null).length ?? 0);
const selected = computed(() => entry.value?.selected ?? []);
const dirty = computed(() => contracts.isDirty(key.value));

function creatable(operation: OperationView): boolean {
  return operation.supported && operation.request === null;
}

function groupCreatable(operations: OperationView[]): string[] {
  return operations.filter(creatable).map((o) => o.key);
}

function groupChecked(operations: OperationView[]): boolean {
  const keys = groupCreatable(operations);
  return keys.length > 0 && keys.every((k) => selected.value.includes(k));
}

function toggleGroup(operations: OperationView[]): void {
  const keys = groupCreatable(operations);
  const next = groupChecked(operations)
    ? selected.value.filter((k) => !keys.includes(k))
    : [...new Set([...selected.value, ...keys])];
  contracts.setSelection(key.value, next);
}

function methodClass(method: string): string {
  return `op__method--${method.toLowerCase()}`;
}

function openRequest(path: string): void {
  void workbench.openRequest(props.projectId, path);
}

function onSourceKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void contracts.save(key.value);
  }
}
</script>

<template>
  <section class="contract" aria-label="Contrat">
    <p v-if="entry?.loadError" class="contract__error" role="alert">{{ entry.loadError }}</p>
    <template v-else-if="view && entry">
      <header class="contract__head">
        <div class="contract__title">
          <h2 class="contract__name">{{ view.name }}</h2>
          <span v-if="view.version && !view.deduced" class="contract__version"
            >version {{ view.version }}</span
          >
          <span v-if="kindLabel" class="contract__kind">{{ kindLabel }}</span>
          <code class="contract__file">{{ view.path }}</code>
        </div>
        <p v-if="view.description" class="contract__description">{{ view.description }}</p>
        <p v-if="view.servers.length > 0" class="contract__servers">
          {{ view.servers.length > 1 ? "Serveurs" : "Serveur" }} :
          <code v-for="server in view.servers" :key="server">{{ server }}</code>
        </p>
        <p v-if="view.error" class="contract__error" role="alert">
          Contrat illisible : {{ view.error }} Corrigez la source.
        </p>
      </header>

      <div role="tablist" aria-label="Vues du contrat" class="contract__tabs">
        <button
          type="button"
          role="tab"
          class="contract__tab"
          :aria-selected="tab === 'operations'"
          @click="tab = 'operations'"
        >
          Opérations <span class="contract__badge">{{ total }}</span>
        </button>
        <button
          type="button"
          role="tab"
          class="contract__tab"
          :aria-selected="tab === 'source'"
          @click="tab = 'source'"
        >
          Source <span v-if="dirty" class="contract__dirty">●</span>
        </button>
      </div>

      <div
        v-if="tab === 'operations'"
        role="tabpanel"
        aria-label="Opérations"
        class="contract__panel"
      >
        <div class="contract__actions">
          <p class="contract__coverage">
            {{ total }} opération{{ total > 1 ? "s" : "" }} · {{ linked }} avec requête
          </p>
          <BaseButton
            variant="primary"
            :disabled="selected.length === 0 || entry.busy"
            @click="contracts.generate(key)"
          >
            Créer les requêtes ({{ selected.length }})
          </BaseButton>
          <BaseButton :disabled="linked === 0" @click="contracts.run(key)">
            ▶ Exécuter ({{ linked }})
          </BaseButton>
        </div>
        <p v-if="total === 0 && !view.error" class="contract__hint">
          Ce contrat ne décrit aucune opération : complétez-le dans « Source ».
        </p>

        <section
          v-for="group in groups"
          :key="group.tag ?? ''"
          class="group"
          :aria-label="group.tag ?? 'Sans tag'"
        >
          <div class="group__head">
            <h3 class="group__title">{{ group.tag ?? "Sans tag" }}</h3>
            <label v-if="groupCreatable(group.operations).length > 0" class="group__all">
              <input
                type="checkbox"
                :checked="groupChecked(group.operations)"
                @change="toggleGroup(group.operations)"
              />
              Tout cocher
            </label>
          </div>
          <ul class="group__list">
            <li v-for="operation in group.operations" :key="operation.key" class="op">
              <input
                v-if="creatable(operation)"
                type="checkbox"
                class="op__check"
                :checked="selected.includes(operation.key)"
                :aria-label="`Créer la requête de ${operation.key}`"
                @change="contracts.toggle(key, operation.key)"
              />
              <span
                v-else
                class="op__check op__check--done"
                :title="operation.request ? 'Requête créée' : 'Méthode non envoyable'"
                aria-hidden="true"
                >{{ operation.request ? "✓" : "–" }}</span
              >
              <span class="op__method" :class="methodClass(operation.method)">{{
                operation.method
              }}</span>
              <code class="op__path">{{ operation.path }}</code>
              <span class="op__summary">{{
                operation.summary ?? operation.operationId ?? ""
              }}</span>
              <span v-if="operation.deprecated" class="op__flag">obsolète</span>
              <span
                v-if="operation.externalRefs"
                class="op__flag op__flag--warn"
                title="Référence vers un autre fichier : non résolue"
                >réf. externe</span
              >
              <span v-if="!operation.supported" class="op__flag">non envoyable</span>
              <button
                v-if="operation.request"
                type="button"
                class="op__open"
                :aria-label="`Ouvrir la requête de ${operation.key}`"
                @click="openRequest(operation.request)"
              >
                Requête ›
              </button>
            </li>
          </ul>
        </section>

        <section
          v-if="view.orphans.length > 0"
          class="group group--orphans"
          aria-label="Hors contrat"
        >
          <div class="group__head">
            <h3 class="group__title">Hors contrat</h3>
          </div>
          <p class="contract__hint">
            Ces requêtes sont liées à une opération qui n'est plus dans le contrat.
          </p>
          <ul class="group__list">
            <li v-for="orphan in view.orphans" :key="orphan.path" class="op">
              <span class="op__check op__check--warn" aria-hidden="true">⚠</span>
              <span class="op__summary">{{ orphan.name }}</span>
              <code class="op__path">{{ orphan.operation }}</code>
              <button
                type="button"
                class="op__open"
                :aria-label="`Ouvrir la requête ${orphan.name}`"
                @click="openRequest(orphan.path)"
              >
                Requête ›
              </button>
            </li>
          </ul>
        </section>
      </div>

      <div
        v-else
        role="tabpanel"
        aria-label="Source"
        class="contract__panel contract__panel--source"
      >
        <div class="contract__actions">
          <BaseButton variant="primary" :disabled="!dirty" @click="contracts.save(key)">
            Enregistrer
          </BaseButton>
          <BaseButton @click="contracts.replaceFromFile(key)">Remplacer par un fichier…</BaseButton>
          <span v-if="dirty" class="contract__state">● Modifiée</span>
        </div>
        <p v-if="entry.saveError" class="contract__error" role="alert">{{ entry.saveError }}</p>
        <textarea
          v-model="entry.draft"
          class="contract__source"
          spellcheck="false"
          aria-label="Source du contrat"
          @keydown="onSourceKeydown"
        />
      </div>
    </template>
    <p v-else class="contract__hint">Lecture du contrat…</p>
  </section>
</template>

<style scoped>
.contract {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
  min-height: 100%;
  padding: 1rem 1.25rem;
}

.contract__head {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.contract__title {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.375rem 0.75rem;
}

.contract__name {
  margin: 0;
  font-size: 1.125rem;
  font-weight: 650;
}

.contract__version {
  padding: 0.0625rem 0.5rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.contract__kind {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.contract__file {
  font: 0.75rem var(--font-mono);
  color: var(--color-muted);
}

.contract__description,
.contract__servers,
.contract__hint,
.contract__coverage {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.contract__servers {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem 0.5rem;
}

.contract__servers code {
  font: 0.75rem var(--font-mono);
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.contract__error {
  margin: 0;
  color: var(--color-danger);
  overflow-wrap: anywhere;
}

.contract__tabs {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--color-border);
}

.contract__tab {
  flex: none;
  padding: 0.5rem 0.75rem;
  border: none;
  border-bottom: 2px solid transparent;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
  white-space: nowrap;
}

.contract__tab[aria-selected="true"] {
  font-weight: 600;
  color: var(--color-text);
  border-bottom-color: var(--color-accent);
}

.contract__tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: 6px;
}

.contract__badge {
  margin-left: 0.25rem;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.contract__dirty,
.contract__state {
  font-size: 0.75rem;
  color: var(--color-link);
}

.contract__panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
}

.contract__panel--source {
  flex: 1;
}

.contract__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.contract__coverage {
  margin-right: auto;
}

.contract__source {
  flex: 1;
  min-height: 20rem;
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: 0.8125rem/1.5 var(--font-mono);
  color: var(--color-text);
  background: var(--color-surface);
  resize: vertical;
  tab-size: 2;
}

.contract__source:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.group__head {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.group__title {
  margin: 0;
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.group__all {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.group__list {
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.op {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.25rem 0.625rem;
  padding: 0.4375rem 0.625rem;
  font-size: 0.875rem;
}

.op + .op {
  border-top: 1px solid var(--color-border);
}

.op__check {
  flex: none;
  width: 1rem;
  text-align: center;
}

.op__check--done {
  color: var(--color-success);
}

.op__check--warn {
  color: var(--color-danger);
}

.op__method {
  min-width: 3.5rem;
  font: 700 0.6875rem var(--font-mono);
  color: var(--color-muted);
}

.op__method--get {
  color: var(--color-success);
}

.op__method--post,
.op__method--put,
.op__method--patch {
  color: var(--color-link);
}

.op__method--delete {
  color: var(--color-danger);
}

.op__path {
  font: 0.8125rem var(--font-mono);
  overflow-wrap: anywhere;
}

.op__summary {
  color: var(--color-muted);
}

.op__flag {
  padding: 0 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: 999px;
  font-size: 0.6875rem;
  color: var(--color-muted);
}

.op__flag--warn {
  color: var(--color-danger);
}

.op__open {
  margin-left: auto;
  padding: 0.125rem 0.25rem;
  border: none;
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.op__open:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
  border-radius: 4px;
}
</style>
