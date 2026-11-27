<script setup lang="ts">
/**
 * Confirmation avant d'envoyer une requête qui modifie des données dans un environnement protégé
 * (R8). AM31 (3.3-c) : « Envoyer vers <env> ? » avec le cadenas, la requête et l'hôte visés, et
 * le bouton plein « Envoyer vers <env> ».
 */
import { computed } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import LockIcon from "@/components/ui/LockIcon.vue";
import { useGuard } from "@/composables/useGuard";
import { useWorkbenchStore } from "@/stores/workbench";

const workbench = useWorkbenchStore();
const guard = useGuard();

const environment = computed(() => workbench.pendingEnvironment());
const target = computed(() => workbench.pendingTarget());
</script>

<template>
  <BaseDialog
    :open="workbench.pending !== null"
    :title="`Envoyer vers ${environment} ?`"
    @close="workbench.dismissPending()"
  >
    <p class="confirm__text">
      <LockIcon class="confirm__lock" />
      <span>
        Cet environnement est protégé.
        <template v-if="target.method && target.path">
          <code class="confirm__method">{{ target.method }}</code>{{ " "
          }}<code>{{ target.path }}</code> peut modifier des données</template
        ><template v-else>Cette exécution peut modifier des données</template
        ><template v-if="target.host">
          sur <strong>{{ target.host }}</strong></template
        >.
      </span>
    </p>
    <div class="confirm__actions">
      <BaseButton @click="workbench.dismissPending()">Annuler</BaseButton>
      <BaseButton variant="primary" @click="guard(() => workbench.confirmPending())"
        >Envoyer vers {{ environment }}</BaseButton
      >
    </div>
  </BaseDialog>
</template>

<style scoped>
.confirm__text {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  margin: 0 0 1rem;
  font-size: var(--text-body);
}

.confirm__lock {
  align-self: center;
  color: var(--color-muted);
}

.confirm__method {
  font-weight: 600;
}

.confirm__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
