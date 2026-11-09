<script setup lang="ts">
/** Éditeur de requête (B2 à B8) : méthode, adresse, paramètres, en-têtes, authentification, corps, vérifications. */
import { computed, ref, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import PairsEditor from "@/components/workbench/PairsEditor.vue";
import { useWorkbenchStore } from "@/stores/workbench";
import type {
  AssertionOperator,
  AuthKind,
  BodyKind,
  HttpMethod,
  RequestDoc,
  Scalar,
} from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const request = computed<RequestDoc | null>(() => editor.value?.draft ?? null);
const folder = computed(() => {
  const path = editor.value?.path ?? "";
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
});

const METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const AUTH_KINDS: { id: AuthKind; label: string }[] = [
  { id: "none", label: "Aucune" },
  { id: "basic", label: "Basic" },
  { id: "bearer", label: "Bearer" },
  { id: "apiKey", label: "Clé d'API" },
];
const BODY_KINDS: { id: BodyKind; label: string }[] = [
  { id: "none", label: "Aucun" },
  { id: "json", label: "JSON" },
  { id: "text", label: "Texte" },
  { id: "form", label: "Formulaire" },
];
const OPERATORS: { id: AssertionOperator; label: string }[] = [
  { id: "equals", label: "est égal à" },
  { id: "notEquals", label: "est différent de" },
  { id: "exists", label: "existe" },
  { id: "contains", label: "contient" },
  { id: "lessThan", label: "est inférieur à" },
  { id: "greaterThan", label: "est supérieur à" },
  { id: "isNumber", label: "est un nombre" },
  { id: "isText", label: "est un texte" },
];
const NO_VALUE: AssertionOperator[] = ["exists", "isNumber", "isText"];

type Tab = "params" | "headers" | "auth" | "body" | "assertions";
const tab = ref<Tab>("params");
const tabs = computed<{ id: Tab; label: string; count?: number }[]>(() => [
  { id: "params", label: "Paramètres", count: request.value?.params.length },
  { id: "headers", label: "En-têtes", count: request.value?.headers.length },
  { id: "auth", label: "Authentification" },
  { id: "body", label: "Corps" },
  { id: "assertions", label: "Vérifications", count: request.value?.assertions.length },
]);

const TOKEN_EXAMPLE = "{{token}}";
const URL_PLACEHOLDER = "{{baseUrl}}/chemin";
const urlId = useId();
const methodId = useId();

function scalarText(value: Scalar | null | undefined): string {
  return value === null || value === undefined ? "" : String(value);
}

/** Convertit la saisie : nombre ou booléen si le texte en a la forme, sinon texte. */
function parseScalar(text: string): Scalar {
  if (text === "true") return true;
  if (text === "false") return false;
  const number = Number(text);
  return text.trim() !== "" && Number.isFinite(number) ? number : text;
}

function setAssertionValue(index: number, text: string): void {
  const assertion = request.value?.assertions[index];
  if (assertion) {
    assertion.value = parseScalar(text);
    assertion.origin = "manual";
  }
}

function addAssertion(): void {
  request.value?.assertions.push({
    id: `manuelle-${crypto.randomUUID().slice(0, 8)}`,
    target: "status",
    operator: "equals",
    value: 200,
    origin: "manual",
  });
}

function removeAssertion(index: number): void {
  request.value?.assertions.splice(index, 1);
}

function resultFor(id: string): boolean | null {
  const found = editor.value?.outcome?.assertions.find((a) => a.id === id);
  return found ? found.passed : null;
}
</script>

<template>
  <section v-if="request" class="editor" aria-label="Requête">
    <div class="editor__title">
      <h2 class="editor__name">{{ request.name }}</h2>
      <span v-if="folder" class="editor__folder">{{ folder }}</span>
      <span v-if="workbench.isDirty(editorKey)" class="editor__state">● Modifiée</span>
    </div>
    <p v-if="editor?.changedElsewhere" class="editor__elsewhere" role="status">
      Modifiée dans une autre fenêtre.
      <button type="button" class="link" @click="workbench.reload(editorKey)">Recharger</button>
    </p>
    <form class="editor__line" @submit.prevent="workbench.send(editorKey)">
      <label :for="methodId" class="sr-only">Méthode</label>
      <select :id="methodId" v-model="request.method" class="editor__method">
        <option v-for="m in METHODS" :key="m" :value="m">{{ m }}</option>
      </select>
      <label :for="urlId" class="sr-only">Adresse</label>
      <input
        :id="urlId"
        v-model="request.url"
        class="editor__url"
        type="text"
        spellcheck="false"
        autocomplete="off"
        :placeholder="URL_PLACEHOLDER"
      />
      <BaseButton v-if="editor?.sending" @click="workbench.cancelSend(editorKey)">
        Annuler
      </BaseButton>
      <BaseButton v-else variant="primary" type="submit" :disabled="request.url.trim() === ''">
        Envoyer
      </BaseButton>
      <BaseButton :disabled="!workbench.isDirty(editorKey)" @click="workbench.save(editorKey)">
        Enregistrer
      </BaseButton>
    </form>

    <div role="tablist" aria-label="Sections de la requête" class="editor__tabs">
      <button
        v-for="t in tabs"
        :id="`tab-${t.id}`"
        :key="t.id"
        type="button"
        role="tab"
        class="editor__tab"
        :aria-selected="tab === t.id"
        :aria-controls="`panel-${t.id}`"
        @click="tab = t.id"
      >
        {{ t.label }}
        <span v-if="t.count" class="editor__badge">{{ t.count }}</span>
      </button>
    </div>

    <div :id="`panel-${tab}`" role="tabpanel" :aria-labelledby="`tab-${tab}`" class="editor__panel">
      <PairsEditor v-if="tab === 'params'" v-model="request.params" label="Paramètres d'adresse" />
      <PairsEditor v-else-if="tab === 'headers'" v-model="request.headers" label="En-têtes" />

      <div v-else-if="tab === 'auth'" class="stack">
        <label class="field">
          <span>Type</span>
          <select v-model="request.auth.type">
            <option v-for="k in AUTH_KINDS" :key="k.id" :value="k.id">{{ k.label }}</option>
          </select>
        </label>
        <template v-if="request.auth.type === 'basic'">
          <label class="field">
            <span>Utilisateur</span>
            <input v-model="request.auth.username" type="text" autocomplete="off" />
          </label>
          <label class="field">
            <span>Mot de passe</span>
            <input v-model="request.auth.password" type="text" autocomplete="off" />
          </label>
        </template>
        <label v-else-if="request.auth.type === 'bearer'" class="field">
          <span>Jeton</span>
          <input v-model="request.auth.token" type="text" autocomplete="off" />
        </label>
        <template v-else-if="request.auth.type === 'apiKey'">
          <label class="field">
            <span>Nom</span>
            <input v-model="request.auth.name" type="text" autocomplete="off" />
          </label>
          <label class="field">
            <span>Valeur</span>
            <input v-model="request.auth.value" type="text" autocomplete="off" />
          </label>
          <label class="field">
            <span>Emplacement</span>
            <select v-model="request.auth.location">
              <option value="header">En-tête</option>
              <option value="query">Paramètre d'adresse</option>
            </select>
          </label>
        </template>
        <p class="hint">
          Utilisez des variables, par exemple <code>{{ TOKEN_EXAMPLE }}</code
          >, plutôt que des valeurs en clair : les secrets restent sur votre poste.
        </p>
      </div>

      <div v-else-if="tab === 'body'" class="stack">
        <label class="field">
          <span>Type</span>
          <select v-model="request.body.kind">
            <option v-for="k in BODY_KINDS" :key="k.id" :value="k.id">{{ k.label }}</option>
          </select>
        </label>
        <textarea
          v-if="request.body.kind === 'json' || request.body.kind === 'text'"
          v-model="request.body.content"
          class="editor__body"
          spellcheck="false"
          aria-label="Contenu du corps"
        />
        <PairsEditor
          v-else-if="request.body.kind === 'form'"
          v-model="request.body.form"
          label="Champs du formulaire"
        />
      </div>

      <div v-else class="stack">
        <p v-if="request.assertions.length === 0" class="hint">
          Sans vérification, la requête réussit si le statut est inférieur à 400.
        </p>
        <ul class="assertions">
          <li
            v-for="(assertion, index) in request.assertions"
            :key="assertion.id"
            class="assertion"
            :class="{
              'assertion--ok': resultFor(assertion.id) === true,
              'assertion--ko': resultFor(assertion.id) === false,
            }"
          >
            <input
              v-model="assertion.target"
              type="text"
              spellcheck="false"
              :aria-label="`Cible de la vérification ${index + 1}`"
            />
            <select
              v-model="assertion.operator"
              :aria-label="`Opérateur de la vérification ${index + 1}`"
            >
              <option v-for="o in OPERATORS" :key="o.id" :value="o.id">{{ o.label }}</option>
            </select>
            <input
              v-if="!NO_VALUE.includes(assertion.operator)"
              :value="scalarText(assertion.value)"
              type="text"
              :aria-label="`Valeur de la vérification ${index + 1}`"
              @input="setAssertionValue(index, ($event.target as HTMLInputElement).value)"
            />
            <span v-else />
            <span class="assertion__origin">{{
              assertion.origin === "proposed"
                ? "proposée"
                : assertion.origin === "default"
                  ? "par défaut"
                  : "manuelle"
            }}</span>
            <button
              type="button"
              class="assertion__remove"
              :aria-label="`Supprimer la vérification ${index + 1}`"
              @click="removeAssertion(index)"
            >
              ×
            </button>
          </li>
        </ul>
        <div class="editor__row">
          <button type="button" class="link" @click="addAssertion">
            + Ajouter une vérification
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
}

.editor__title {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.625rem;
}

.editor__name {
  margin: 0;
  font-size: 1.0625rem;
  font-weight: 650;
}

.editor__folder {
  font: 0.8125rem var(--font-mono);
  color: var(--color-muted);
}

.editor__state {
  font-size: 0.75rem;
  color: var(--color-accent-text);
}

.editor__elsewhere {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-radius: 8px;
  font-size: 0.8125rem;
  background: var(--color-accent-soft);
}

/* Zone étroite (jusqu'à 280 px, Z4) : les boutons passent sous l'adresse. */
.editor__line {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.editor__method,
.editor__url,
.field input,
.field select,
.assertion input,
.assertion select,
.editor__body {
  padding: 0.5rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.editor__method {
  font-weight: 700;
}

.editor__url {
  flex: 1 1 12rem;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 0.875rem;
}

:is(
  .editor__method,
  .editor__url,
  .field input,
  .field select,
  .assertion input,
  .assertion select,
  .editor__body,
  .editor__tab,
  .link,
  .assertion__remove
):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.editor__tabs {
  display: flex;
  gap: 0.25rem;
  overflow-x: auto;
  scrollbar-width: none;
  border-bottom: 1px solid var(--color-border);
}

.editor__tab {
  flex: none;
  padding: 0.5rem 0.75rem;
  white-space: nowrap;
  border: 0;
  border-bottom: 2px solid transparent;
  background: none;
  font: inherit;
  font-size: 0.875rem;
  color: var(--color-muted);
  cursor: pointer;
}

.editor__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.editor__badge {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  border-radius: 999px;
  font-size: 0.6875rem;
  background: var(--color-chip);
}

.stack {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field {
  display: grid;
  grid-template-columns: 9rem 1fr;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.editor__body {
  min-height: 9rem;
  font: 0.8125rem var(--font-mono);
  resize: vertical;
}

.hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.assertions {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.assertion {
  display: grid;
  grid-template-columns: 1.4fr 1.2fr 1fr auto auto;
  gap: 0.375rem;
  align-items: center;
  padding: 0.25rem;
  border-radius: 8px;
}

.assertion--ok {
  background: var(--color-success-soft);
}

.assertion--ko {
  background: var(--color-danger-soft);
}

.assertion input,
.assertion select {
  min-width: 0;
  padding: 0.3125rem 0.5rem;
  font-size: 0.8125rem;
}

.assertion__origin {
  font-size: 0.75rem;
  color: var(--color-muted);
}

.assertion__remove,
.link {
  border: 0;
  background: none;
  font: inherit;
  cursor: pointer;
}

.assertion__remove {
  font-size: 1.125rem;
  color: var(--color-muted);
}

.link {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
</style>
