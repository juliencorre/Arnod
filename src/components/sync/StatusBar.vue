<script setup lang="ts">
/**
 * Barre d'état de la fenêtre principale (`docs/plan-v2.md`, lot E, SY4) : l'état de la
 * synchronisation du projet courant (« Pas encore partagé », « À jour avec l'équipe »,
 * « 3 à récupérer · 2 à envoyer », « 1 conflit à résoudre ») ; un clic ouvre le panneau, le
 * dialogue « Relier » ou le conflit. Les entrées « mocks » et « catalogue » viennent en V5 et V6.
 */
import { computed, onBeforeUnmount, onMounted, watch } from "vue";

import { statusLabel } from "@/lib/sync";
import { useSyncStore } from "@/stores/sync";

const sync = useSyncStore();

const label = computed(() => statusLabel(sync.current));
const tone = computed(() => {
  const view = sync.current;
  if (!view?.remote) return "idle";
  if (view.conflicts > 0) return "conflict";
  return view.incoming.length + view.outgoing.length > 0 ? "changes" : "idle";
});
const title = computed(() =>
  sync.current?.error ? `Dernière vérification impossible : ${sync.current.error}` : undefined,
);

watch(
  () => sync.projectId,
  (id) => {
    if (id) void sync.check(id);
  },
  { immediate: true },
);

onMounted(() => void sync.start());
onBeforeUnmount(() => {
  sync.stop();
});
</script>

<template>
  <footer v-if="sync.projectId" class="statusbar">
    <button
      type="button"
      class="statusbar__sync"
      :class="`statusbar__sync--${tone}`"
      :title="title"
      aria-label="Synchronisation"
      @click="sync.openPanel(sync.projectId)"
    >
      <span class="statusbar__icon" aria-hidden="true">⟳</span>
      <span role="status" class="statusbar__label">{{ label }}</span>
    </button>
  </footer>
</template>

<style scoped>
.statusbar {
  display: flex;
  flex: none;
  align-items: center;
  height: var(--control-sm);
  padding: 0 0.5rem;
  border-top: 1px solid var(--color-border);
  font-size: var(--text-small);
  color: var(--color-muted);
  background: var(--color-sidebar);
}

.statusbar__sync {
  display: inline-flex;
  gap: 0.375rem;
  align-items: center;
  min-height: 1.5rem;
  padding: 0 0.5rem;
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  color: inherit;
  background: transparent;
  cursor: pointer;
}

.statusbar__sync:hover {
  color: var(--color-text);
  background: var(--color-chip);
}

.statusbar__sync--changes {
  color: var(--color-link);
}

.statusbar__sync--conflict {
  color: var(--color-danger);
}

.statusbar__icon {
  font-size: var(--text-body);
}
</style>
