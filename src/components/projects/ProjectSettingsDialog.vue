<script setup lang="ts">
/**
 * Réglages du projet → Authentification (3.4-a), ouverts par le menu ⋯ du projet : réglée une
 * fois, reprise par les requêtes qui l'héritent. OAuth 2 par identifiants client : le secret va
 * au coffre de l'environnement, le fichier n'en garde que la référence ; le jeton de
 * l'environnement est obtenu et renouvelé seul (lot H, docs/plan-mvp-h-reseau-auth.md).
 * Les réglages s'enregistrent seuls, comme une requête (ER2) : fermer enregistre d'abord ce qui
 * attend, et un échec garde la fenêtre ouverte.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import SaveState from "@/components/ui/SaveState.vue";
import AuthEditor from "@/components/workbench/AuthEditor.vue";
import { useAutosave } from "@/composables/useAutosave";
import { authLabel, formatClock } from "@/lib/auth";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { Auth, Inherited } from "@/types/ipc";

const network = useNetworkStore();
const catalog = useCatalogStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const secretId = useId();

const projectId = computed(() => network.projectSettings?.projectId ?? null);
const projectName = computed(
  () => catalog.projects.find((p) => p.id === projectId.value)?.name ?? "",
);
const environment = computed(() =>
  projectId.value === null ? null : workbench.activeEnvironment(projectId.value),
);
const view = computed(() => network.projectAuth);

/** Brouillon de l'authentification, repris de la vue renvoyée par le cœur. */
const draft = ref<Auth | null>(null);
const secret = ref("");
const busy = ref(false);

/** Enregistre le brouillon et le secret saisi (au coffre) : une copie, la saisie peut continuer. */
async function saveDraft(): Promise<void> {
  const id = projectId.value;
  const env = environment.value;
  if (id === null || env === null || !draft.value) return;
  const auth = JSON.parse(JSON.stringify(draft.value)) as Auth;
  await network.saveProjectAuth(id, env, auth, secret.value || null);
}

const autosave = useAutosave(
  () => (draft.value ? { auth: draft.value, secret: secret.value } : null),
  saveDraft,
);
const saveState = autosave.state;
const saveError = autosave.error;

// Le brouillon est repris de la vue à son chargement seulement : une réponse à un
// enregistrement, un jeton renouvelé ou oublié ne remplacent pas une saisie en cours.
watch(
  view,
  (current, previous) => {
    if (current !== null && previous !== null && previous !== undefined) return;
    draft.value = current ? (JSON.parse(JSON.stringify(current.auth)) as Auth) : null;
    secret.value = "";
    autosave.reset();
  },
  { immediate: true },
);

async function close(): Promise<void> {
  if (await autosave.flush()) network.closeProjectSettings();
}

const NO_INHERIT: Inherited = {
  auth: null,
  headers: [],
  settings: { timeoutMs: 30000, followRedirects: true, maxRedirects: 5, encodeUrl: true },
  variables: [],
};

const token = computed(() => view.value?.token ?? null);

function overrideText(o: { label: string; type: Auth["type"]; other: string; count: number }) {
  if (o.label === "") return `${countLabel(o.count, "requête", "requêtes")} sans authentification`;
  const kind = authLabel(o.type, o.other);
  return o.label.startsWith("dossier")
    ? `le ${o.label} (${kind}, ${countLabel(o.count, "requête", "requêtes")})`
    : `la ${o.label} (${kind})`;
}

async function run(action: () => Promise<void>): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    await action();
  } catch (error) {
    toasts.showError(error);
  } finally {
    busy.value = false;
  }
}

function renew(): Promise<void> {
  return run(async () => {
    if (projectId.value === null || environment.value === null) return;
    await network.renewToken(projectId.value, environment.value);
  });
}

function forget(): Promise<void> {
  return run(async () => {
    if (projectId.value === null || environment.value === null) return;
    await network.forgetToken(projectId.value, environment.value);
  });
}
</script>

<template>
  <BaseDialog
    :open="network.projectSettings !== null"
    :title="`${projectName} · Réglages du projet`"
    description="Valent pour tout le projet ; un dossier ou une requête peut les remplacer."
    wide
    @close="close"
  >
    <div class="settings">
      <div role="tablist" aria-label="Réglages du projet" class="settings__tabs">
        <button type="button" role="tab" aria-selected="true" class="settings__tab">
          Authentification
        </button>
      </div>
      <div v-if="draft && view" role="tabpanel" class="settings__body">
        <div class="settings__form">
          <AuthEditor v-model="draft" :inherited="NO_INHERIT" project>
            <template #secret>
              <div class="settings__secret">
                <label :for="secretId">Secret client</label>
                <input
                  :id="secretId"
                  v-model="secret"
                  type="password"
                  autocomplete="off"
                  :placeholder="
                    view.hasClientSecret ? '•••••••• au coffre' : 'Gardé au coffre du poste'
                  "
                />
              </div>
            </template>
          </AuthEditor>
          <p class="settings__hint">
            Les valeurs viennent de l'environnement du projet : chaque environnement a son URL, son
            identifiant et son secret. Le secret saisi va au coffre de ce poste, jamais dans le
            projet.
          </p>
        </div>
        <aside class="settings__side">
          <section
            v-if="draft.type === 'oauth2'"
            class="settings__card"
            aria-labelledby="token-title"
          >
            <h3 id="token-title" class="settings__card-title">Jeton · {{ environment }}</h3>
            <template v-if="token">
              <p class="settings__strong">
                Obtenu automatiquement à {{ formatClock(token.obtainedAt) }}
              </p>
              <p v-if="token.expiresAt" class="settings__muted">
                Expire à {{ formatClock(token.expiresAt) }} ; Arnod le renouvelle avant.
              </p>
            </template>
            <p v-else class="settings__muted">
              Aucun jeton pour l'instant : il sera demandé au premier envoi.
            </p>
            <p class="settings__actions">
              <BaseButton :disabled="busy || view.auth.type !== 'oauth2'" @click="renew">
                {{ token ? "Renouveler maintenant" : "Obtenir maintenant" }}
              </BaseButton>
              <BaseButton v-if="token" variant="link" :disabled="busy" @click="forget">
                Oublier le jeton
              </BaseButton>
            </p>
            <p class="settings__muted settings__small">
              Un jeton par environnement. Il reste sur ce poste et n'apparaît que masqué.
            </p>
          </section>
          <section class="settings__card" aria-labelledby="usage-title">
            <h3 id="usage-title" class="settings__card-title">Qui l'utilise</h3>
            <p class="settings__strong">
              {{ countLabel(view.usage.inheriting, "requête l'hérite", "requêtes l'héritent") }}
            </p>
            <p v-if="view.usage.overrides.length > 0" class="settings__muted">
              Remplacée par
              <template v-for="(o, index) in view.usage.overrides" :key="o.label + o.path">
                {{ index === 0 ? "" : index === view.usage.overrides.length - 1 ? " et " : ", "
                }}<span class="settings__override">{{ overrideText(o) }}</span> </template
              >.
            </p>
          </section>
        </aside>
      </div>
      <p v-else class="settings__muted">Chargement des réglages…</p>
    </div>
    <template #footer>
      <p v-if="saveError" class="settings__error" role="alert">{{ saveError }}</p>
      <SaveState :state="saveState" :error="saveError" @retry="autosave.flush()" />
      <BaseButton @click="close">Fermer</BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.settings__error {
  flex: 1;
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-danger);
}

.settings {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.settings__tabs {
  display: flex;
  border-bottom: 1px solid var(--color-border);
}

.settings__tab {
  padding: 0.375rem 0.75rem;
  border: 0;
  border-bottom: 2px solid var(--color-accent);
  font: inherit;
  font-weight: 600;
  color: var(--color-text);
  background: none;
}

.settings__tab:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.settings__body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 15rem;
  gap: 1.25rem;
}

.settings__form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
}

.settings__secret {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.settings__secret label {
  color: var(--color-muted);
}

.settings__secret input {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

.settings__hint,
.settings__muted {
  margin: 0;
  color: var(--color-muted);
}

.settings__small {
  font-size: var(--text-small);
}

.settings__side {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.settings__card {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.settings__card-title {
  margin: 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.settings__strong {
  margin: 0;
}

.settings__override {
  font-weight: 600;
  color: var(--color-text);
}

.settings__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  margin: 0.25rem 0 0;
}
</style>
