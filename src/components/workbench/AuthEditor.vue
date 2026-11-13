<script setup lang="ts">
/**
 * Authentification d'un endpoint (EP5) : héritée du dossier ou de la collection (R4), aucune,
 * Basic, Bearer ou clé d'API. Une note dit ce qui part.
 */
import { computed, useId } from "vue";

import { useKnownVariables, vVariables } from "@/lib/variables";
import type { Auth, AuthKind, Inherited } from "@/types/ipc";

const props = defineProps<{ inherited: Inherited }>();
const auth = defineModel<Auth>({ required: true });

const MODES: { id: AuthKind; label: string }[] = [
  { id: "inherit", label: "Héritée" },
  { id: "none", label: "Aucune" },
  { id: "basic", label: "Basic" },
  { id: "bearer", label: "Bearer" },
  { id: "apiKey", label: "Clé d'API" },
];

const TOKEN_EXAMPLE = "{{token}}";
const ids = { mode: useId(), a: useId(), b: useId(), c: useId() };
const known = useKnownVariables();

/** Résumé d'une authentification héritée. */
function summary(auth: Auth): string {
  switch (auth.type) {
    case "basic":
      return `Basic, identifiant ${auth.username || "vide"}`;
    case "bearer":
      return `Bearer, jeton ${auth.token || "vide"}`;
    case "apiKey":
      return `clé d'API ${auth.name} dans ${auth.location === "query" ? "un paramètre de requête" : "un en-tête"}`;
    case "other":
      return `${auth.other}, que Arnod n'envoie pas`;
    default:
      return "aucune";
  }
}

const note = computed(() => {
  const current = auth.value;
  switch (current.type) {
    case "inherit": {
      const inherited = props.inherited.auth;
      return inherited
        ? `Reprend l'authentification du ${inherited.source} : ${summary(inherited.auth)}.`
        : "Aucune authentification à reprendre d'un dossier ou de la collection : la requête part sans authentification.";
    }
    case "none":
      return "Aucune authentification : la requête part sans en-tête Authorization.";
    case "basic":
      return "Identifiant et mot de passe, encodés en Base64 dans l'en-tête Authorization.";
    case "bearer":
      return "Jeton envoyé dans l'en-tête Authorization: Bearer … Mettez une variable plutôt qu'une valeur en clair.";
    case "apiKey":
      return "Une clé envoyée dans un en-tête ou un paramètre de requête.";
    default:
      return `Authentification « ${current.other} » conservée dans le fichier ; Arnod ne l'envoie pas.`;
  }
});
</script>

<template>
  <div class="auth">
    <div class="field">
      <label :for="ids.mode">Mode</label>
      <select :id="ids.mode" v-model="auth.type">
        <option v-for="m in MODES" :key="m.id" :value="m.id">{{ m.label }}</option>
        <option v-if="auth.type === 'other'" value="other">Autre ({{ auth.other }})</option>
      </select>
    </div>
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
    <p v-if="['basic', 'bearer', 'apiKey'].includes(auth.type)" class="hint">
      Utilisez des variables, par exemple <code>{{ TOKEN_EXAMPLE }}</code
      >, plutôt que des valeurs en clair : les secrets restent sur votre poste.
    </p>
  </div>
</template>

<style scoped>
.auth {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field {
  display: grid;
  grid-template-columns: 8rem minmax(0, 1fr);
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-body);
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
</style>
