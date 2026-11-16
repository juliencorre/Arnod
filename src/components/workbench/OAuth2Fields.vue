<script setup lang="ts">
/**
 * Champs OAuth 2 par identifiants client (3.4-a), partagés par les réglages du projet et l'onglet
 * Auth d'une requête : URL du jeton, identifiant, secret (si `withSecret`), portée, présentation
 * des identifiants et envoi du jeton (lot H, docs/plan-mvp-h-reseau-auth.md, R8, R11).
 */
import { computed, useId } from "vue";

import { useKnownVariables, vVariables } from "@/lib/variables";
import type { OAuth2 } from "@/types/ipc";

withDefaults(defineProps<{ withSecret?: boolean }>(), { withSecret: true });
const oauth2 = defineModel<OAuth2>({ required: true });

const ids = {
  url: useId(),
  client: useId(),
  secret: useId(),
  scope: useId(),
  credentials: useId(),
  placement: useId(),
  name: useId(),
};
const known = useKnownVariables();

type Sending = "authorization" | "header" | "query";

/** Envoi du jeton : en-tête Authorization (Bearer), autre en-tête, ou paramètre de requête. */
const sending = computed<Sending>({
  get: () => {
    if (oauth2.value.tokenPlacement === "query") return "query";
    const name = oauth2.value.tokenName.trim();
    return name === "" || name.toLowerCase() === "authorization" ? "authorization" : "header";
  },
  set: (next) => {
    if (next === "authorization") {
      oauth2.value.tokenPlacement = "header";
      oauth2.value.tokenName = "Authorization";
    } else if (next === "header") {
      oauth2.value.tokenPlacement = "header";
      oauth2.value.tokenName = "X-Access-Token";
    } else {
      oauth2.value.tokenPlacement = "query";
      oauth2.value.tokenName = "access_token";
    }
  },
});
</script>

<template>
  <div class="oauth2">
    <div class="oauth2__field oauth2__field--wide">
      <label :for="ids.url">URL du jeton</label>
      <input
        :id="ids.url"
        v-model="oauth2.accessTokenUrl"
        v-variables="known"
        type="text"
        autocomplete="off"
        placeholder="{{authUrl}}/oauth/token"
      />
    </div>
    <div class="oauth2__field">
      <label :for="ids.client">Identifiant client</label>
      <input
        :id="ids.client"
        v-model="oauth2.clientId"
        v-variables="known"
        type="text"
        autocomplete="off"
      />
    </div>
    <div v-if="withSecret" class="oauth2__field">
      <label :for="ids.secret">Secret client</label>
      <input
        :id="ids.secret"
        v-model="oauth2.clientSecret"
        v-variables="known"
        type="text"
        autocomplete="off"
        placeholder="{{oauth2ClientSecret}}"
      />
    </div>
    <slot name="secret" />
    <div class="oauth2__field oauth2__field--wide">
      <label :for="ids.scope">Portée</label>
      <input
        :id="ids.scope"
        v-model="oauth2.scope"
        v-variables="known"
        type="text"
        autocomplete="off"
        placeholder="orders:read orders:write"
      />
    </div>
    <div class="oauth2__field">
      <label :for="ids.placement">Envoi du jeton</label>
      <select :id="ids.placement" v-model="sending">
        <option value="authorization">Authorization · Bearer</option>
        <option value="header">Autre en-tête</option>
        <option value="query">Paramètre de requête</option>
      </select>
    </div>
    <div v-if="sending !== 'authorization'" class="oauth2__field">
      <label :for="ids.name">{{ sending === "query" ? "Paramètre" : "En-tête" }}</label>
      <input :id="ids.name" v-model="oauth2.tokenName" type="text" autocomplete="off" />
    </div>
    <div class="oauth2__field">
      <label :for="ids.credentials">Identifiants envoyés</label>
      <select :id="ids.credentials" v-model="oauth2.credentials">
        <option value="basicAuthHeader">Authorization · Basic</option>
        <option value="body">Dans le corps</option>
      </select>
    </div>
  </div>
</template>

<style scoped>
.oauth2 {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem 1rem;
}

.oauth2__field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  font-size: var(--text-body);
}

.oauth2__field--wide {
  grid-column: 1 / -1;
}

.oauth2__field label {
  color: var(--color-muted);
}

.oauth2__field input,
.oauth2__field select {
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

.oauth2__field input {
  font-family: var(--font-mono);
}

.oauth2__field select {
  padding-right: 1.75rem;
}

.oauth2__field input:focus-visible,
.oauth2__field select:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
