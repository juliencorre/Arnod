<script setup lang="ts">
/**
 * Pastille d'environnement d'un projet (docs/plan-barre-laterale-projet.md, B2, B3) : un
 * bouton qui ouvre le menu des environnements (couleur, nom, adresse, protégé), puis
 * « Gérer les environnements… » et « + Nouvel environnement ». `compact` : la version d'un
 * projet replié, à droite de son nom.
 */
import { computed, ref } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import EnvironmentsDialog from "@/components/workbench/EnvironmentsDialog.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { EnvironmentView } from "@/types/ipc";

const props = withDefaults(defineProps<{ projectId: string; compact?: boolean }>(), {
  compact: false,
});

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const view = computed(() => workbench.environments[props.projectId] ?? null);
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const active = computed(
  () => view.value?.environments.find((e) => e.name === view.value?.active) ?? null,
);

const managing = ref(false);
const creating = ref(false);

const MANAGE = "::gerer";
const CREATE = "::nouveau";

/** Couleur d'un environnement, en jeton du thème. */
function swatch(env: EnvironmentView | null): string {
  return `var(--color-ws-${env?.color ?? "gray"})`;
}

/** Adresse de base, sans le schéma : de quoi reconnaître l'environnement. */
function host(env: EnvironmentView): string | undefined {
  const base = env.variables.find((v) => v.name === "baseUrl")?.value.trim() ?? "";
  return base === "" ? undefined : base.replace(/^[a-z]+:\/\//i, "").replace(/\/$/, "");
}

const items = computed<MenuItem[]>(() => [
  ...(view.value?.environments ?? []).map((env) => ({
    id: env.name,
    label: env.name,
    checked: env.name === view.value?.active,
    radio: true,
    swatch: swatch(env),
    tag: env.protected === true ? "protégé" : undefined,
    detail: host(env),
  })),
  { id: MANAGE, label: "Gérer les environnements…", shortcut: "Ctrl+Maj+E", separatorBefore: true },
  { id: CREATE, label: "+ Nouvel environnement" },
]);

/** Ouvre la gestion des environnements, ou la création d'un nouveau. */
function manage(create = false): void {
  creating.value = create;
  managing.value = true;
}

async function onSelect(id: string): Promise<void> {
  if (id === MANAGE || id === CREATE) {
    manage(id === CREATE);
    return;
  }
  try {
    await workbench.selectEnvironment(props.projectId, id);
  } catch (caught) {
    toasts.showError(caught);
  }
}

defineExpose({ manage });
</script>

<template>
  <BaseMenu
    v-if="view && active"
    :items="items"
    :label="`Environnements de ${projectName}`"
    :heading="`Environnement · ${projectName}`"
    :align="compact ? 'end' : 'start'"
    @select="onSelect"
  >
    <template #trigger="{ attrs }">
      <button
        v-bind="attrs"
        type="button"
        class="chip"
        :class="{ 'chip--compact': compact }"
        :aria-label="`Environnement de ${projectName} : ${active.name}`"
        :title="`Environnement du projet : ${active.name}`"
      >
        <span class="chip__swatch" :style="{ background: swatch(active) }" aria-hidden="true" />
        <span class="chip__name">{{ active.name }}</span>
        <span class="chip__caret" aria-hidden="true">▾</span>
      </button>
    </template>
  </BaseMenu>
  <EnvironmentsDialog
    v-if="managing"
    :open="managing"
    :project-id="projectId"
    :start-new="creating"
    @close="managing = false"
  />
</template>

<style scoped>
.chip {
  box-sizing: border-box;
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  max-width: 100%;
  min-width: 0;
  padding: 0.1875rem 0.5rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-chip);
  cursor: pointer;
}

.chip:hover,
.chip[aria-expanded="true"] {
  background: var(--color-surface);
  box-shadow: inset 0 0 0 1px var(--color-border-strong);
}

.chip:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.chip--compact {
  padding: 0.125rem 0.375rem;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--color-muted);
}

.chip__swatch {
  flex: none;
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 2px;
}

.chip__name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip__caret {
  flex: none;
  font-size: 0.625rem;
  color: var(--color-muted);
}
</style>
