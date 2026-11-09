<script setup lang="ts">
/** Confirmation avant d'envoyer une requête qui modifie des données dans un environnement protégé (R8). */
import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useWorkbenchStore } from "@/stores/workbench";

const workbench = useWorkbenchStore();
</script>

<template>
  <BaseDialog
    :open="workbench.pending !== null"
    title="Environnement protégé"
    :description="`L'environnement « ${workbench.pendingEnvironment()} » est protégé. Cette exécution peut modifier des données.`"
    @close="workbench.dismissPending()"
  >
    <div class="confirm__actions">
      <BaseButton @click="workbench.dismissPending()">Annuler</BaseButton>
      <BaseButton variant="danger" @click="workbench.confirmPending()"
        >Envoyer quand même</BaseButton
      >
    </div>
  </BaseDialog>
</template>

<style scoped>
.confirm__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
