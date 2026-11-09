<script setup lang="ts">
/** Environnement actif du projet (N1 à N6) : sélecteur et accès à la gestion. */
import { computed, ref, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import EnvironmentsDialog from "@/components/workbench/EnvironmentsDialog.vue";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const selectId = useId();
const managing = ref(false);

const active = computed(() => workbench.activeEnvironment);

async function choose(name: string): Promise<void> {
  try {
    await workbench.selectEnvironment(name);
  } catch (caught) {
    toasts.show(caught instanceof Error ? caught.message : String(caught), { tone: "error" });
  }
}
</script>

<template>
  <div v-if="workbench.environments" class="envbar">
    <label :for="selectId" class="envbar__label">Environnement</label>
    <span
      class="envbar__dot"
      :class="`envbar__dot--${active?.color ?? 'gray'}`"
      aria-hidden="true"
    />
    <select
      :id="selectId"
      class="envbar__select"
      :value="workbench.environments.active"
      @change="choose(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="env in workbench.environments.environments" :key="env.name" :value="env.name">
        {{ env.name }}{{ env.protected ? " 🔒" : "" }}
      </option>
    </select>
    <BaseButton variant="link" @click="managing = true">Gérer</BaseButton>
    <EnvironmentsDialog :open="managing" @close="managing = false" />
  </div>
</template>

<style scoped>
.envbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.envbar__label {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.envbar__select {
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
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
