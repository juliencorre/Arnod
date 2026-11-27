<script setup lang="ts">
/**
 * Authentification d'un endpoint (EP5) : héritée du dossier ou de la collection (R4), aucune,
 * Basic, Bearer ou clé d'API. Une note dit ce qui part.
 *
 * AM35 (3.4-b) : « Type », dont « Hériter » ; héritée, une carte dit sa nature, d'où elle vient
 * (« Ouvrir » les réglages du projet), son jeton OAuth 2, où elle part et les valeurs lues.
 */
import { computed, ref, useId, watch } from "vue";

import OAuth2Fields from "@/components/workbench/OAuth2Fields.vue";
import { authKindLabel, authSentIn, authVariables, tokenLine } from "@/lib/auth";
import { useKnownVariables, vVariables } from "@/lib/variables";
import { useNetworkStore } from "@/stores/network";
import { useWorkbenchStore } from "@/stores/workbench";
import type { Auth, AuthKind, Inherited, TokenStatus } from "@/types/ipc";

const props = defineProps<{
  inherited: Inherited;
  /** Lot H : authentification du projet (3.4-a) : rien dont hériter, secret saisi à part. */
  project?: boolean;
  /** Projet de la requête : jeton OAuth 2 hérité et « Ouvrir » les réglages (AM35). */
  projectId?: string;
}>();
const auth = defineModel<Auth>({ required: true });

const MODES: { id: AuthKind; label: string }[] = [
  { id: "inherit", label: "Hériter" },
  { id: "none", label: "Aucune" },
  { id: "basic", label: "Basic" },
  { id: "bearer", label: "Bearer" },
  { id: "apiKey", label: "Clé d'API" },
  // Lot H : OAuth 2 par identifiants client (3.4-a).
  { id: "oauth2", label: "OAuth 2 (identifiants client)" },
];

/** Modes proposés : le projet n'hérite de rien. */
const modes = computed(() => (props.project ? MODES.filter((m) => m.id !== "inherit") : MODES));

const TOKEN_EXAMPLE = "{{token}}";
const ids = { mode: useId(), a: useId(), b: useId(), c: useId() };
const known = useKnownVariables();

// --- Authentification héritée (AM35) -------------------------------------------

const network = useNetworkStore();
const workbench = useWorkbenchStore();
const source = computed(() => (auth.value.type === "inherit" ? props.inherited.auth : null));
/** Réglée sur le projet : « Ouvrir » mène aux réglages du projet (3.4-a). */
const fromProject = computed(() => source.value?.source.startsWith("projet") === true);
const environment = computed(() =>
  props.projectId ? workbench.activeEnvironment(props.projectId) : null,
);
const token = ref<TokenStatus | null>(null);

watch(
  [() => source.value?.auth.type, () => props.projectId, environment],
  async ([type, projectId, env]) => {
    token.value = null;
    if (type !== "oauth2" || !projectId || env === null) return;
    try {
      token.value = await network.tokenStatus(projectId, env);
    } catch {
      // Projet ou environnement disparu : le jeton reste « à demander ».
    }
  },
  { immediate: true },
);

function openSettings(): void {
  if (props.projectId && environment.value !== null) {
    void network.openProjectSettings(props.projectId, environment.value);
  }
}

const note = computed(() => {
  const current = auth.value;
  switch (current.type) {
    case "inherit": {
      return props.inherited.auth
        ? "Pour cette requête seulement, choisir un autre type remplace l'authentification héritée : Aucune, Basic, Bearer, clé d'API ou OAuth 2."
        : "Aucune authentification à reprendre d'un dossier ou du projet : la requête part sans authentification.";
    }
    case "none":
      return "Aucune authentification : la requête part sans en-tête Authorization.";
    case "basic":
      return "Identifiant et mot de passe, encodés en Base64 dans l'en-tête Authorization.";
    case "bearer":
      return "Jeton envoyé dans l'en-tête Authorization: Bearer … Mettez une variable plutôt qu'une valeur en clair.";
    case "apiKey":
      return "Une clé envoyée dans un en-tête ou un paramètre de requête.";
    case "oauth2":
      return props.project
        ? "Le jeton est demandé au premier envoi, gardé en mémoire et renouvelé avant son expiration."
        : "Le jeton est demandé au premier envoi, gardé en mémoire et renouvelé avant son expiration. Mettez le secret dans une variable locale secrète.";
    default:
      return `Authentification « ${current.other} » conservée dans le fichier ; Arnod ne l'envoie pas.`;
  }
});
</script>

<template>
  <div class="auth">
    <div class="field">
      <label :for="ids.mode">Type</label>
      <select :id="ids.mode" v-model="auth.type">
        <option v-for="m in modes" :key="m.id" :value="m.id">{{ m.label }}</option>
        <option v-if="auth.type === 'other'" value="other">Autre ({{ auth.other }})</option>
      </select>
    </div>
    <section v-if="source" class="inherited" aria-label="Authentification héritée">
      <p class="inherited__kind">{{ authKindLabel(source.auth) }}</p>
      <p class="inherited__source">
        réglée sur le {{ source.source }}
        <button
          v-if="fromProject && projectId"
          type="button"
          class="inherited__open"
          @click="openSettings"
        >
          Ouvrir
        </button>
      </p>
      <dl class="inherited__facts">
        <template v-if="source.auth.type === 'oauth2'">
          <dt>Jeton</dt>
          <dd>{{ tokenLine(token) }}</dd>
        </template>
        <template v-if="authSentIn(source.auth)">
          <dt>Envoyé dans</dt>
          <dd>
            <code>{{ authSentIn(source.auth) }}</code>
          </dd>
        </template>
        <template v-if="authVariables(source.auth).length > 0">
          <dt>Valeurs</dt>
          <dd>{{ authVariables(source.auth).join(", ") }}</dd>
        </template>
      </dl>
    </section>
    <p class="note">{{ note }}</p>
    <template v-if="auth.type === 'basic'">
      <div class="field">
        <label :for="ids.a">Identifiant</label>
        <input
          :id="ids.a"
          v-model="auth.username"
          v-variables="known"
          type="text"
          autocomplete="off"
        />
      </div>
      <div class="field">
        <label :for="ids.b">Mot de passe</label>
        <input
          :id="ids.b"
          v-model="auth.password"
          v-variables="known"
          type="text"
          autocomplete="off"
        />
      </div>
    </template>
    <div v-else-if="auth.type === 'bearer'" class="field">
      <label :for="ids.a">Jeton</label>
      <input :id="ids.a" v-model="auth.token" v-variables="known" type="text" autocomplete="off" />
    </div>
    <template v-else-if="auth.type === 'apiKey'">
      <div class="field">
        <label :for="ids.a">Clé</label>
        <input :id="ids.a" v-model="auth.name" v-variables="known" type="text" autocomplete="off" />
      </div>
      <div class="field">
        <label :for="ids.b">Valeur</label>
        <input
          :id="ids.b"
          v-model="auth.value"
          v-variables="known"
          type="text"
          autocomplete="off"
        />
      </div>
      <div class="field">
        <label :for="ids.c">Envoyée dans</label>
        <select :id="ids.c" v-model="auth.location">
          <option value="header">un en-tête</option>
          <option value="query">un paramètre de requête</option>
        </select>
      </div>
    </template>
    <OAuth2Fields v-else-if="auth.type === 'oauth2'" v-model="auth.oauth2" :with-secret="!project">
      <template #secret><slot name="secret" /></template>
    </OAuth2Fields>
    <p v-if="['basic', 'bearer', 'apiKey'].includes(auth.type)" class="hint">
      Utilisez des variables, par exemple <code>{{ TOKEN_EXAMPLE }}</code>, plutôt que des valeurs
      en clair : les secrets restent sur votre poste.
    </p>
  </div>
</template>

<style scoped>
.auth {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* Libellé au-dessus de son champ, comme dans la maquette (3.4-a, 3.4-b). */
.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  max-width: 28rem;
  font-size: var(--text-body);
}

.field > label {
  color: var(--color-muted);
}

.field input,
.field select {
  min-width: 0;
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

.field input {
  font-family: var(--font-mono);
  font-size: var(--text-body);
}

.field input:focus-visible,
.field select:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.note {
  margin: 0;
  padding: 0.5rem 0.625rem;
  border-radius: var(--radius-md);
  font-size: var(--text-body);
  background: var(--color-chip);
}

.hint {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

/* La flèche de la liste déroulante garde sa place (styles.css). */
.field select {
  padding-right: 1.75rem;
}

/* Carte de l'authentification héritée (3.4-b, AM35). */
.inherited {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  max-width: 32rem;
  padding: 0.75rem 0.875rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  font-size: var(--text-body);
}

.inherited p {
  margin: 0;
}

.inherited__kind {
  font-weight: 600;
}

.inherited__source {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: baseline;
  color: var(--color-muted);
}

.inherited__open {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.inherited__open:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.inherited__facts {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.25rem 0.75rem;
  margin: 0.25rem 0 0;
}

.inherited__facts dt {
  color: var(--color-muted);
}

.inherited__facts dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
}
</style>
