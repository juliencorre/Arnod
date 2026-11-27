<script setup lang="ts">
/**
 * Après une récupération (`docs/plan-v2.md`, lot E, 7.1-e, SY8, R15) : les secrets ne voyagent
 * pas ; ceux que les environnements déclarent et qui manquent sur ce poste sont nommés.
 * « Renseigner » ouvre la page des environnements, « Plus tard » ferme jusqu'à un nouveau manque.
 */
import { computed } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel } from "@/lib/format";
import { useSyncStore } from "@/stores/sync";
import { useWorkbenchStore } from "@/stores/workbench";

const sync = useSyncStore();
const workbench = useWorkbenchStore();
const guard = useGuard();

const first = computed(() => sync.missing?.secrets[0] ?? null);

async function fill(): Promise<void> {
  const shown = sync.missing;
  sync.missing = null;
  if (shown) await workbench.openEnvironments(shown.projectId);
}
</script>

<template>
  <div v-if="first" class="missing" role="alert">
    <p class="missing__text">
      {{ countLabel(first.names.length, "secret", "secrets") }} à renseigner pour
      <strong>{{ first.environment }}</strong> :
      <template v-for="(name, index) in first.names" :key="name"
        ><code>{{ name }}</code
        >{{ index < first.names.length - 1 ? ", " : "" }}</template
      >. Ils sont rangés dans le coffre du système et ne sont jamais envoyés.
    </p>
    <BaseButton @click="guard(fill)">Renseigner</BaseButton>
    <BaseButton @click="guard(() => sync.later())">Plus tard</BaseButton>
  </div>
</template>

<style scoped>
.missing {
  display: flex;
  flex: none;
  gap: 0.75rem;
  align-items: center;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-accent-soft);
}

.missing__text {
  flex: 1;
  margin: 0;
}

.missing__text code {
  font-family: var(--editor-font);
}
</style>
