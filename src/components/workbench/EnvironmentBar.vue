<script setup lang="ts">
/** Environnement actif du projet (N1 à N6) : sélecteur et accès à la gestion. */
import { computed, ref, useId } from "vue";

import EnvironmentsDialog from "@/components/workbench/EnvironmentsDialog.vue";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
const view = computed(() => workbench.environments[props.projectId] ?? null);
const projectName = computed(() => useCatalogStore().projectById(props.projectId)?.name ?? "");
const toasts = useToastStore();
const selectId = useId();
const managing = ref(false);

const active = computed(
  () => view.value?.environments.find((e) => e.name === view.value?.active) ?? null,
);

async function choose(name: string): Promise<void> {
  try {
    await workbench.selectEnvironment(props.projectId, name);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}
</script>

<template>
  <div v-if="view" class="envbar">
    <label :for="selectId" class="visually-hidden">Environnement de {{ projectName }}</label>
    <span
      class="envbar__dot"
      :class="`envbar__dot--${active?.color ?? 'gray'}`"
      aria-hidden="true"
    />
    <select
      :id="selectId"
      class="envbar__select"
      :value="view.active"
      @change="choose(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="env in view.environments" :key="env.name" :value="env.name">
        {{ env.name }}{{ env.protected ? " 🔒" : "" }}
      </option>
    </select>
    <button
      type="button"
      class="envbar__manage"
      :aria-label="`Gérer les environnements de ${projectName}`"
      @click="managing = true"
    >
      Gérer
    </button>
    <EnvironmentsDialog :open="managing" :project-id="projectId" @close="managing = false" />
  </div>
</template>

<style scoped>
.envbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.envbar__manage {
  padding: 0;
  border: none;
  font: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.envbar__manage:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
  border-radius: 4px;
}

.envbar__select {
  min-width: 0;
  max-width: 11rem;
  padding: 0.125rem 0.375rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.envbar__select:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.envbar__dot {
  width: 0.625rem;
  height: 0.625rem;
  border-radius: 50%;
  background: var(--color-ws-gray);
}

.envbar__dot--blue {
  background: var(--color-ws-blue);
}

.envbar__dot--green {
  background: var(--color-ws-green);
}

.envbar__dot--orange {
  background: var(--color-ws-orange);
}

.envbar__dot--red {
  background: var(--color-ws-red);
}

.envbar__dot--purple {
  background: var(--color-ws-purple);
}
</style>
