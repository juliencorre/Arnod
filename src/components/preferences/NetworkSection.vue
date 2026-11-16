<script setup lang="ts">
/**
 * Préférences → Réseau et certificats (1.6-e) : proxy du système, aucun ou manuel, délai de
 * connexion ; autorités du système et autorités internes, ajoutées depuis un fichier après avoir
 * montré leur empreinte (R4). Tout s'enregistre seul. La vérification des certificats n'est pas
 * réglable : elle n'est jamais coupée (docs/plan-mvp-h-reseau-auth.md).
 */
import { computed, ref, useId, watch } from "vue";

import CertificateCard from "@/components/preferences/CertificateCard.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import type { ProxyMode } from "@/types/ipc";

const network = useNetworkStore();
const toasts = useToastStore();
const ids = {
  proxy: useId(),
  url: useId(),
  noProxy: useId(),
  user: useId(),
  password: useId(),
  timeout: useId(),
};

const view = computed(() => network.view);

/** Champs saisis, repris de la vue à chaque changement venu du cœur. */
const mode = ref<ProxyMode>("system");
const url = ref("");
const noProxy = ref("");
const username = ref("");
const password = ref("");
const timeout = ref(10);

watch(
  view,
  (current) => {
    if (!current) return;
    mode.value = current.proxy.mode;
    url.value = current.proxy.manual.url;
    noProxy.value = current.proxy.manual.noProxy;
    username.value = current.proxy.manual.username;
    timeout.value = Math.round(current.connectTimeoutMs / 1000);
  },
  { immediate: true },
);

/** Proxy du système détecté, en clair. */
const detected = computed(() => {
  const system = view.value?.system;
  if (!system) return "";
  const address = system.https ?? system.http;
  if (address) {
    const source =
      system.source === "windows" ? "réglage de Windows" : "variables HTTPS_PROXY / HTTP_PROXY";
    return `Détecté : ${address}, par ${source}.`;
  }
  if (system.autoConfig) {
    return `Configuration automatique (${system.autoConfig}) : non prise en charge, saisissez le proxy à la main.`;
  }
  return "Aucun proxy détecté : connexion directe.";
});

async function persist(withPassword = false): Promise<void> {
  if (!view.value) return;
  try {
    await network.save({
      proxy: {
        mode: mode.value,
        manual: { url: url.value.trim(), noProxy: noProxy.value, username: username.value.trim() },
      },
      proxyPassword: withPassword ? password.value : null,
      connectTimeoutMs: Math.max(1, timeout.value) * 1000,
    });
    if (withPassword) password.value = "";
  } catch (error) {
    toasts.showError(error);
  }
}

function choose(next: ProxyMode): void {
  mode.value = next;
  void persist();
}

const MODES: { id: ProxyMode; label: string }[] = [
  { id: "system", label: "Celui du système" },
  { id: "none", label: "Aucun" },
  { id: "manual", label: "Manuel" },
];

async function add(): Promise<void> {
  try {
    await network.pickAuthority();
  } catch (error) {
    toasts.showError(error);
  }
}

async function trust(): Promise<void> {
  const name = network.pendingAuthority?.subject ?? "";
  try {
    await network.trustPending();
    toasts.show(`« ${name} » est reconnue sur ce poste. Renvoyez la requête.`);
  } catch (error) {
    toasts.showError(error);
  }
}

async function remove(fingerprint: string, subject: string): Promise<void> {
  try {
    await network.removeAuthority(fingerprint);
    toasts.show(`« ${subject} » n'est plus reconnue.`);
  } catch (error) {
    toasts.showError(error);
  }
}
</script>

<template>
  <div v-if="view" class="network">
    <section class="network__block" aria-labelledby="network-proxy">
      <h3 id="network-proxy" class="network__title">Réseau</h3>
      <p class="network__hint">Réglages de ce poste ; rien n'est partagé avec l'équipe.</p>
      <fieldset class="network__group">
        <legend class="network__legend">Proxy</legend>
        <label v-for="m in MODES" :key="m.id" class="network__choice">
          <input
            type="radio"
            :name="ids.proxy"
            :value="m.id"
            :checked="mode === m.id"
            @change="choose(m.id)"
          />
          <span>
            <span class="network__choice-label">{{ m.label }}</span>
            <span v-if="m.id === 'system'" class="network__detail">{{ detected }}</span>
            <span v-else-if="m.id === 'none'" class="network__detail">Connexion directe.</span>
            <span v-else class="network__detail">Adresse, exceptions et identifiants.</span>
          </span>
        </label>
        <div v-if="mode === 'manual'" class="network__manual">
          <label :for="ids.url">Adresse</label>
          <input
            :id="ids.url"
            v-model="url"
            type="text"
            placeholder="proxy.entreprise.local:8080"
            autocomplete="off"
            @change="persist()"
          />
          <label :for="ids.noProxy">Exceptions</label>
          <input
            :id="ids.noProxy"
            v-model="noProxy"
            type="text"
            placeholder="<local>; .entreprise.local"
            autocomplete="off"
            @change="persist()"
          />
          <label :for="ids.user">Identifiant</label>
          <input
            :id="ids.user"
            v-model="username"
            type="text"
            autocomplete="off"
            @change="persist()"
          />
          <label :for="ids.password">Mot de passe</label>
          <input
            :id="ids.password"
            v-model="password"
            type="password"
            autocomplete="off"
            :placeholder="view.hasProxyPassword ? '•••••••• (au coffre du poste)' : ''"
            @change="persist(true)"
          />
        </div>
      </fieldset>
      <div class="network__group">
        <span class="network__legend">Délais</span>
        <p class="network__delay">
          <label :for="ids.timeout">Connexion</label>
          <input
            :id="ids.timeout"
            v-model.number="timeout"
            class="network__seconds"
            type="number"
            min="1"
            max="300"
            @change="persist()"
          />
          <span aria-hidden="true">s</span>
          <span class="network__detail">La réponse se règle par requête ou par dossier.</span>
        </p>
      </div>
    </section>

    <section class="network__block network__block--certs" aria-labelledby="network-certs">
      <h3 id="network-certs" class="network__title">Certificats</h3>
      <p class="network__hint">Autorités reconnues pour vérifier les serveurs.</p>
      <div class="network__group">
        <span class="network__legend">Autorités racines</span>
        <p class="network__line">
          <strong>Celles du système</strong>
          <span class="network__detail">
            {{ view.systemAuthorities }} autorités reconnues par le système.
          </span>
        </p>
        <ul
          v-if="view.authorities.length > 0"
          class="network__authorities"
          aria-label="Autorités ajoutées"
        >
          <li v-for="a in view.authorities" :key="a.fingerprint" class="network__authority">
            <span>
              <strong>{{ a.subject }}</strong>
              <span class="network__detail">
                Ajoutée sur ce poste · valide jusqu'au {{ a.notAfter }}
              </span>
            </span>
            <BaseButton
              variant="link"
              :aria-label="`Retirer ${a.subject}`"
              @click="remove(a.fingerprint, a.subject)"
            >
              Retirer
            </BaseButton>
          </li>
        </ul>
        <CertificateCard
          v-if="network.pendingAuthority"
          :certificate="network.pendingAuthority"
          @trust="trust"
          @cancel="network.cancelPending()"
        />
        <BaseButton v-else variant="link" class="network__add" @click="add">
          + Ajouter une autorité…
        </BaseButton>
      </div>
      <p class="network__verify">
        <span aria-hidden="true">✓</span>
        <span>
          <strong>Les certificats des serveurs sont toujours vérifiés.</strong>
          <span class="network__detail">
            Pour un serveur interne, ajoutez son autorité plutôt que de couper la vérification.
          </span>
        </span>
      </p>
    </section>
  </div>
  <p v-else class="network__hint">Chargement des préférences…</p>
</template>

<style scoped>
.network {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.network__block {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.network__block--certs {
  padding-top: 1.25rem;
  border-top: 1px solid var(--color-border);
}

.network__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.network__hint {
  margin: -0.5rem 0 0;
  color: var(--color-muted);
}

.network__group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0;
  padding: 0;
  border: 0;
}

.network__legend {
  margin-bottom: 0.25rem;
  padding: 0;
  font-weight: 600;
}

.network__choice {
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  cursor: pointer;
}

.network__choice input {
  margin-top: 0.15rem;
}

.network__choice-label,
.network__detail {
  display: block;
}

.network__detail {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.network__manual {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  align-items: center;
  gap: 0.5rem 0.75rem;
  margin-left: 1.5rem;
}

.network__manual input,
.network__seconds {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

.network__manual input:focus-visible,
.network__seconds:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.network__delay {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
}

.network__seconds {
  width: 4.5rem;
  font-family: var(--font-mono);
}

.network__line {
  margin: 0;
}

.network__authorities {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.network__authority {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.network__add {
  align-self: flex-start;
  padding: 0;
}

.network__verify {
  display: flex;
  gap: 0.625rem;
  margin: 0;
}

.network__verify > span:first-child {
  color: var(--color-success);
  font-weight: 600;
}
</style>
