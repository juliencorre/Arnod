<script setup lang="ts">
/**
 * Onglet de résultat (docs/plan-resultat-detachable.md, D1 à D4) : le résultat de la requête
 * active, celle de la dernière fenêtre utilisée. Il suit les changements d'onglet et de
 * fenêtre, et chaque envoi, d'où qu'il parte.
 */
import { computed, watch } from "vue";

import ResultSide from "@/components/zones/ResultSide.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useWindowsStore } from "@/stores/windows";
import { findRequest, requestKey, useWorkbenchStore } from "@/stores/workbench";

const catalog = useCatalogStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();

const active = computed(() => windows.activeRequest);
const key = computed(() =>
  active.value ? requestKey(active.value.projectId, active.value.path) : null,
);
const editor = computed(() => (key.value ? (workbench.editors[key.value] ?? null) : null));

/** Méthode, nom et emplacement de la requête active. */
const heading = computed(() => {
  const current = active.value;
  if (!current) return null;
  const draft = editor.value?.draft;
  const node = findRequest(workbench.trees[current.projectId]?.nodes ?? [], current.path);
  const fallback =
    current.path
      .split("/")
      .at(-1)
      ?.replace(/\.yml$/, "") ?? current.path;
  const project = catalog.projectById(current.projectId)?.name ?? "projet";
  return {
    method: draft?.method ?? (node?.kind === "request" ? node.method : "GET"),
    name: draft?.name ?? node?.name ?? fallback,
    where: [project, ...current.path.split("/").slice(0, -1)].join(" › "),
  };
});

// La requête active précédente est oubliée si rien d'autre ne l'affiche ici.
watch(
  key,
  (next, previous) => {
    if (active.value) void workbench.openEditor(active.value.projectId, active.value.path);
    if (previous && previous !== next && !windows.zones.some((z) => hasRequest(z, previous))) {
      workbench.release(previous);
    }
  },
  { immediate: true },
);

function hasRequest(zone: (typeof windows.zones)[number], wanted: string): boolean {
  return zone.tabs.some(
    (t) =>
      t.content.kind === "request" && requestKey(t.content.projectId, t.content.path) === wanted,
  );
}
</script>

<template>
  <section class="result-tab" aria-label="Résultat de la requête active">
    <template v-if="heading && key">
      <header class="result-tab__head">
        <span
          class="result-tab__method"
          :class="`result-tab__method--${heading.method.toLowerCase()}`"
        >
          {{ heading.method }}
        </span>
        <span class="result-tab__name">{{ heading.name }}</span>
        <span class="result-tab__where">{{ heading.where }}</span>
      </header>
      <p v-if="editor?.loadError" class="result-tab__empty" role="alert">{{ editor.loadError }}</p>
      <p v-else-if="!editor?.draft" class="result-tab__empty">Chargement…</p>
      <ResultSide v-else :editor-key="key" />
    </template>
    <p v-else class="result-tab__empty">
      Ouvrez une requête : son résultat s'affiche ici, à chaque envoi.
    </p>
  </section>
</template>

<style scoped>
.result-tab {
  flex: 1;
  min-width: 0;
  padding: 1rem 1.25rem;
}

.result-tab__head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.result-tab__method {
  font-family: var(--font-mono);
  font-size: var(--text-small);
  font-weight: 700;
  color: var(--color-success);
}

.result-tab__method--post,
.result-tab__method--put,
.result-tab__method--patch {
  color: var(--color-link);
}

.result-tab__method--delete {
  color: var(--color-danger);
}

.result-tab__name {
  font-weight: 600;
}

.result-tab__where {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.result-tab__empty {
  margin: 2rem auto;
  text-align: center;
  color: var(--color-muted);
}
</style>
