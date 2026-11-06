<script setup lang="ts">
/**
 * Barre latérale. Au premier lancement (1a) : la marque et le workspace par
 * défaut créé automatiquement. Ensuite : le workspace courant (2a), qui
 * deviendra le sélecteur de workspaces (2b, T20), et ses projets (T18).
 */
import { computed, useId } from "vue";

import ColorDot from "@/components/ui/ColorDot.vue";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";

const catalog = useCatalogStore();
const navigation = useNavigationStore();
const labelId = useId();

const isFirstRun = computed(() => navigation.view.kind === "firstRun");
const workspace = computed(() =>
  navigation.currentWorkspaceId === null
    ? null
    : catalog.workspaceById(navigation.currentWorkspaceId),
);
const projectCount = computed(() => workspace.value?.projectIds.length ?? 0);
</script>

<template>
  <aside class="sidebar" aria-label="Barre latérale">
    <template v-if="isFirstRun">
      <div class="sidebar__brand">
        <span class="sidebar__logo" aria-hidden="true" />
        Arnod
      </div>
      <section v-if="workspace" class="sidebar__section" :aria-labelledby="labelId">
        <h2 :id="labelId" class="sidebar__label">Workspace</h2>
        <div class="sidebar__workspace">
          <ColorDot :color="workspace.color" size="sm" />
          <span class="sidebar__name">{{ workspace.name }}</span>
          <span v-if="workspace.isDefault" class="sidebar__badge">par défaut</span>
          <span class="sidebar__count">
            <span class="visually-hidden">Projets : </span>{{ projectCount }}
          </span>
        </div>
        <p class="sidebar__note">Créé automatiquement. Votre premier projet y sera rangé.</p>
      </section>
    </template>

    <header v-else-if="workspace" class="sidebar__current">
      <ColorDot :color="workspace.color" />
      <div class="sidebar__current-text">
        <div class="sidebar__current-name">
          <span class="sidebar__name">{{ workspace.name }}</span>
          <span v-if="workspace.isDefault" class="sidebar__badge">par défaut</span>
        </div>
        <div class="sidebar__meta">
          Workspace · {{ countLabel(projectCount, "projet", "projets") }}
        </div>
      </div>
    </header>
  </aside>
</template>

<style scoped>
.sidebar {
  box-sizing: border-box;
  display: flex;
  flex: none;
  flex-direction: column;
  gap: 1.25rem;
  width: 260px;
  padding: 1rem 0.75rem;
  border-right: 1px solid var(--color-border);
  background: var(--color-sidebar);
  overflow-y: auto;
}

.sidebar__brand {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0 0.375rem;
  font-size: 0.9375rem;
  font-weight: 700;
}

.sidebar__logo {
  width: 1.375rem;
  height: 1.375rem;
  border-radius: 6px;
  background: var(--color-accent);
}

.sidebar__section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.sidebar__label {
  margin: 0;
  padding: 0 0.5rem;
  font: 600 0.65625rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.sidebar__workspace {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font-size: 0.8125rem;
  background: var(--color-surface);
}

.sidebar__name {
  font-weight: 600;
}

.sidebar__badge {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font-size: 0.65625rem;
  color: var(--color-muted);
  background: var(--color-chip);
}

.sidebar__count {
  margin-left: auto;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.sidebar__note {
  margin: 0;
  padding: 0 0.5rem;
  font-size: 0.75rem;
  line-height: 1.5;
  color: var(--color-muted);
}

.sidebar__current {
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  padding: 0.5rem 0.5rem 0.25rem;
}

.sidebar__current .dot {
  margin-top: 0.25rem;
}

.sidebar__current-text {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.sidebar__current-name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.375rem;
}

.sidebar__meta {
  font-size: 0.75rem;
  color: var(--color-muted);
}
</style>
