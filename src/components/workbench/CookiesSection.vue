<script setup lang="ts">
/**
 * Cookies gardés de l'environnement pour l'hôte de la requête (3.4-g), sous ses en-têtes : nom,
 * valeur (masquée si HttpOnly, abrégée sinon), expiration et requête d'origine ; « Vider » et
 * « Renvoyer les cookies reçus de ce domaine ». Relus après chaque envoi (lot H,
 * docs/plan-mvp-h-reseau-auth.md, CK1 à CK4).
 */
import { computed, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ editorKey: string }>();

const network = useNetworkStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();

const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const projectId = computed(() => editor.value?.projectId ?? null);
const environment = computed(() =>
  projectId.value === null ? null : workbench.activeEnvironment(projectId.value),
);
const url = computed(() => editor.value?.draft?.url ?? null);
const jar = computed(() =>
  projectId.value === null || environment.value === null
    ? null
    : (network.cookies[network.cookieKey(projectId.value, environment.value, url.value)] ?? null),
);

function reload(): void {
  if (projectId.value === null || environment.value === null) return;
  network.loadCookies(projectId.value, environment.value, url.value).catch(() => undefined);
}

// Relu à l'ouverture, quand l'adresse ou l'environnement change, et après chaque réponse.
watch([projectId, environment, url, () => editor.value?.receivedAt], reload, { immediate: true });

/** Expiration lisible : « dans 28 min », « fin de session ». */
function expires(at: number | null): string {
  if (at === null) return "fin de session";
  const minutes = Math.round((at - Date.now()) / 60000);
  if (minutes < 60) return `dans ${Math.max(minutes, 1)} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `dans ${hours} h`;
  return `dans ${Math.round(hours / 24)} j`;
}

async function clear(domain: string): Promise<void> {
  if (projectId.value === null || environment.value === null) return;
  try {
    await network.clearCookies(projectId.value, environment.value, domain, url.value);
    toasts.show(`Cookies de ${domain} oubliés pour ${environment.value}.`);
  } catch (error) {
    toasts.showError(error);
  }
}

async function toggle(domain: string, enabled: boolean): Promise<void> {
  if (projectId.value === null || environment.value === null) return;
  try {
    await network.setCookieSending(projectId.value, environment.value, domain, enabled, url.value);
  } catch (error) {
    toasts.showError(error);
  }
}
</script>

<template>
  <template v-if="jar && jar.domains.length > 0">
    <section
      v-for="domain in jar.domains"
      :key="domain.domain"
      class="cookies"
      :aria-label="`Cookies gardés de ${domain.domain}`"
    >
      <header class="cookies__head">
        <h3 class="cookies__title">Cookies gardés · {{ domain.domain }}</h3>
        <BaseButton
          v-if="domain.cookies.length > 0"
          variant="link"
          :aria-label="`Vider les cookies de ${domain.domain}`"
          @click="clear(domain.domain)"
        >
          Vider
        </BaseButton>
      </header>
      <table v-if="domain.cookies.length > 0" class="cookies__table">
        <thead>
          <tr>
            <th scope="col">Nom</th>
            <th scope="col">Valeur</th>
            <th scope="col">Expire</th>
            <th scope="col">Reçu de</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in domain.cookies" :key="c.name + c.path">
            <td class="cookies__name">{{ c.name }}</td>
            <td class="cookies__value">{{ c.value }}</td>
            <td>{{ expires(c.expiresAt) }}</td>
            <td class="cookies__origin">{{ c.receivedFrom }}</td>
          </tr>
        </tbody>
      </table>
      <label class="cookies__send">
        <input
          type="checkbox"
          :checked="domain.sending"
          @change="toggle(domain.domain, ($event.target as HTMLInputElement).checked)"
        />
        Renvoyer les cookies reçus de ce domaine
      </label>
    </section>
    <p class="cookies__note">
      Les cookies sont gardés par environnement, sur ce poste : jamais dans Git. Les cookies
      HttpOnly restent masqués, comme les secrets.
    </p>
  </template>
</template>

<style scoped>
.cookies {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.cookies__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.cookies__title {
  margin: 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.cookies__table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0 0.25rem;
  font-size: var(--text-body);
}

.cookies__table th {
  padding: 0 0.75rem;
  font: 600 var(--text-caption) var(--font-mono);
  text-align: left;
  text-transform: uppercase;
  color: var(--color-muted);
}

.cookies__table td {
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
}

.cookies__table td:first-child {
  border-left: 1px solid var(--color-border);
  border-radius: var(--radius-md) 0 0 var(--radius-md);
}

.cookies__table td:last-child {
  border-right: 1px solid var(--color-border);
  border-radius: 0 var(--radius-md) var(--radius-md) 0;
}

.cookies__name {
  font-weight: 600;
}

.cookies__value,
.cookies__origin {
  font-family: var(--font-mono);
}

.cookies__origin {
  font-size: var(--text-small);
}

.cookies__send {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.cookies__note {
  margin: 0;
  padding: 0.625rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  color: var(--color-muted);
  background: var(--color-chip);
}
</style>
