<script setup lang="ts">
/**
 * Échec réseau expliqué (5.1-i) : la cause en clair, ce qui n'est pas parti, les faits utiles
 * (serveur, autorité, empreinte, proxy, délai) et l'action qui règle le poste. La vérification
 * TLS n'est jamais coupée : « Faire confiance… » ouvre les préférences sur l'autorité à confirmer
 * (lot H, docs/plan-mvp-h-reseau-auth.md, R5 à R7).
 */
import { computed } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { formatDuration } from "@/lib/format";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import type { FailureCause, NetworkFailure } from "@/types/ipc";

const props = defineProps<{ failure: NetworkFailure; environment: string | null }>();

const network = useNetworkStore();
const toasts = useToastStore();

const CAUSES: Record<FailureCause, string> = {
  certificate: "certificat",
  proxy: "proxy",
  proxyAuth: "proxy",
  dns: "nom introuvable",
  unreachable: "serveur injoignable",
  timeout: "délai dépassé",
  tls: "connexion sécurisée",
  other: "réseau",
};

/** Empreinte abrégée comme la maquette : 4 premiers octets, puis les 2 derniers. */
function short(fingerprint: string): string {
  const parts = fingerprint.split(":");
  return parts.length <= 6
    ? fingerprint
    : `${parts.slice(0, 4).join(":")}:…:${parts.slice(-2).join(":")}`;
}

const facts = computed<[string, string][]>(() => {
  const f = props.failure;
  const rows: [string, string][] = [["Serveur", f.port ? `${f.host}:${f.port}` : f.host]];
  const cert = f.certificate;
  if (cert) {
    rows.push(["Délivré par", cert.server.issuer]);
    rows.push(["Valide", `du ${cert.server.notBefore} au ${cert.server.notAfter}`]);
    if (cert.problem === "wrongName" && cert.server.names.length > 0) {
      rows.push(["Noms couverts", cert.server.names.join(", ")]);
    }
    rows.push(["Empreinte SHA-256", short(cert.server.fingerprint)]);
  }
  if (f.proxy) rows.push(["Proxy", f.proxy]);
  else if (f.cause === "dns") rows.push(["Proxy", "aucun (connexion directe)"]);
  if (f.cause === "unreachable") {
    rows.push(["Délai de connexion", formatDuration(f.connectTimeoutMs)]);
  }
  return rows;
});

const trustable = computed(() => props.failure.certificate?.authority ?? null);
const proxyCause = computed(() => ["proxy", "proxyAuth"].includes(props.failure.cause));

async function trust(): Promise<void> {
  const authority = trustable.value;
  if (!authority) return;
  try {
    network.proposeAuthority(authority);
    await network.openPreferences();
  } catch (error) {
    toasts.showError(error);
  }
}

function openPreferences(): void {
  network.openPreferences().catch(toasts.showError);
}
</script>

<template>
  <div class="net-failure" role="alert">
    <div class="net-failure__summary">
      <span class="net-failure__badge">Échec · {{ CAUSES[failure.cause] }}</span>
      <span>{{ formatDuration(failure.elapsedMs) }}</span>
      <span>0 o</span>
      <span v-if="environment" class="net-failure__env">{{ environment }}</span>
    </div>
    <p class="net-failure__title">{{ failure.title }}</p>
    <p v-if="failure.explanation" class="net-failure__explanation">{{ failure.explanation }}</p>
    <dl class="net-failure__facts">
      <template v-for="[label, value] in facts" :key="label">
        <dt>{{ label }}</dt>
        <dd>{{ value }}</dd>
      </template>
    </dl>
    <p class="net-failure__ask">Que voulez-vous faire ?</p>
    <div v-if="trustable" class="net-failure__action net-failure__action--main">
      <div>
        <p class="net-failure__action-title">
          Faire confiance à « {{ trustable.subject }} »
          <span class="net-failure__tag">Recommandé</span>
        </p>
        <p class="net-failure__action-hint">
          Vérifiez l'empreinte avec votre équipe, puis renvoyez la requête.
        </p>
      </div>
      <BaseButton @click="trust">Faire confiance…</BaseButton>
    </div>
    <div class="net-failure__action">
      <div>
        <p class="net-failure__action-title">
          {{ proxyCause ? "Vérifier le proxy" : "Ouvrir Réseau et certificats" }}
        </p>
        <p class="net-failure__action-hint">Proxy, autorités et délais de ce poste.</p>
      </div>
      <BaseButton @click="openPreferences">
        {{ proxyCause ? "Vérifier le proxy…" : "Ouvrir" }}
      </BaseButton>
    </div>
    <p class="net-failure__note">
      La vérification n'est jamais coupée d'un clic. Une autorité ajoutée vaut pour tout le poste et
      se retire dans Préférences → Réseau et certificats.
    </p>
  </div>
</template>

<style scoped>
.net-failure {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  min-width: 0;
}

.net-failure__summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  color: var(--color-muted);
}

.net-failure__badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-md);
  font: 600 var(--text-body) var(--font-mono);
  color: var(--color-danger);
  background: var(--color-danger-soft);
}

.net-failure__env {
  margin-left: auto;
}

.net-failure__title {
  margin: 0;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.net-failure__explanation {
  margin: -0.25rem 0 0;
  color: var(--color-muted);
}

.net-failure__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: 0.375rem 1.5rem;
  margin: 0;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: var(--text-small) var(--font-mono);
}

.net-failure__facts dt {
  color: var(--color-muted);
}

.net-failure__facts dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.net-failure__ask {
  margin: 0.25rem 0 0;
  font-weight: 600;
}

.net-failure__action {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.net-failure__action > :last-child {
  flex-shrink: 0;
  white-space: nowrap;
}

.net-failure__action--main {
  border-color: var(--color-text);
}

.net-failure__action-title {
  margin: 0;
  font-weight: 600;
}

.net-failure__action-hint {
  margin: 0.125rem 0 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.net-failure__tag {
  margin-left: 0.375rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-caption);
  font-weight: 600;
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.net-failure__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
