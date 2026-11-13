<script setup lang="ts">
/**
 * Autorité à confirmer (R4) : sujet, émetteur, validité et empreinte SHA-256, montrés avant
 * « Faire confiance ». Rien n'est ajouté sans ce clic (lot H, docs/plan-mvp-h-reseau-auth.md).
 */
import BaseButton from "@/components/ui/BaseButton.vue";
import type { CertificateInfo } from "@/types/ipc";

defineProps<{ certificate: CertificateInfo }>();
const emit = defineEmits<{ trust: []; cancel: [] }>();
</script>

<template>
  <div class="cert-card" role="group" :aria-label="`Autorité ${certificate.subject}`">
    <p class="cert-card__title">Faire confiance à « {{ certificate.subject }} » ?</p>
    <dl class="cert-card__facts">
      <dt>Sujet</dt>
      <dd>{{ certificate.subject }}</dd>
      <dt>Délivré par</dt>
      <dd>{{ certificate.issuer }}</dd>
      <dt>Valide</dt>
      <dd>du {{ certificate.notBefore }} au {{ certificate.notAfter }}</dd>
      <dt>Empreinte SHA-256</dt>
      <dd class="cert-card__fingerprint">{{ certificate.fingerprint }}</dd>
    </dl>
    <p class="cert-card__hint">
      Vérifiez l'empreinte avec votre équipe. L'autorité vaudra pour tout le poste et se retire ici.
    </p>
    <p class="cert-card__actions">
      <BaseButton variant="primary" @click="emit('trust')">Faire confiance</BaseButton>
      <BaseButton @click="emit('cancel')">Annuler</BaseButton>
    </p>
  </div>
</template>

<style scoped>
.cert-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-lg);
  background: var(--color-bg);
}

.cert-card__title {
  margin: 0;
  font-weight: 600;
}

.cert-card__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: 0.25rem 1rem;
  margin: 0;
  font: var(--text-small) var(--font-mono);
}

.cert-card__facts dt {
  color: var(--color-muted);
}

.cert-card__facts dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.cert-card__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.cert-card__actions {
  display: flex;
  gap: 0.5rem;
  margin: 0;
}
</style>
