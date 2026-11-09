<script setup lang="ts">
/**
 * Éditeur d'un endpoint (docs/plan-configuration-endpoint.md, EP1 à EP7) : tout se modifie sur
 * place et s'enregistre seul. En-tête (contrat lié, méthode, adresse, nom, description,
 * étiquettes), puis la requête par onglets, comme dans Bruno.
 */
import { computed, nextTick, ref, useId, useTemplateRef } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import AssertionsEditor from "@/components/workbench/AssertionsEditor.vue";
import AuthEditor from "@/components/workbench/AuthEditor.vue";
import BodyEditor from "@/components/workbench/BodyEditor.vue";
import PairsEditor from "@/components/workbench/PairsEditor.vue";
import SettingsEditor from "@/components/workbench/SettingsEditor.vue";
import { pathParamNames } from "@/lib/url";
import { useContractsStore } from "@/stores/contracts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { HttpMethod, Pair, RequestDoc } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const request = computed<RequestDoc | null>(() => editor.value?.draft ?? null);
const folder = computed(() => {
  const path = editor.value?.path ?? "";
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
});

const METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const URL_PLACEHOLDER = "{{baseUrl}}/chemin";
const ids = {
  method: useId(),
  url: useId(),
  name: useId(),
  desc: useId(),
  tags: useId(),
  docs: useId(),
};

// --- Contrat lié (C7) ------------------------------------------------------

const contract = computed(() => {
  const link = request.value?.contract;
  if (!link || !editor.value) return null;
  const summary = contracts.lists[editor.value.projectId]?.find((c) => c.path === link.file);
  return {
    name: summary?.name ?? link.file,
    version: summary?.version ?? "",
    operation: link.operation,
    file: link.file,
  };
});

function openContract(): void {
  const link = request.value?.contract;
  if (link && editor.value) void contracts.show(editor.value.projectId, link.file);
}

/** Délier (R8) : la requête n'est plus vérifiée contre le contrat. */
function unlinkContract(): void {
  if (request.value) delete request.value.contract;
}

// --- Enregistrement automatique (EP1) -------------------------------------------

const saveLabel = computed(() => {
  switch (editor.value?.saveState) {
    case "pending":
      return "Modifications en attente…";
    case "saving":
      return "Enregistrement…";
    case "error":
      return "Non enregistré";
    default:
      return "✓ Enregistré";
  }
});

function retrySave(): void {
  void workbench.save(props.editorKey).catch(() => undefined);
}

// --- Étiquettes (EP1) -------------------------------------------------------------

const addingTag = ref(false);
const newTag = ref("");
const tagInput = useTemplateRef<HTMLInputElement>("tagInput");

async function startTag(): Promise<void> {
  addingTag.value = true;
  newTag.value = "";
  await nextTick();
  tagInput.value?.focus();
}

function commitTag(): void {
  const tag = newTag.value.trim();
  const tags = request.value?.tags;
  if (tags && tag !== "" && !tags.some((t) => t.toLowerCase() === tag.toLowerCase())) {
    tags.push(tag);
  }
  addingTag.value = false;
  newTag.value = "";
}

function removeTag(index: number): void {
  request.value?.tags.splice(index, 1);
}

// --- Paramètres de chemin (EP2, R1) ---------------------------------------------

const pathNames = computed(() => pathParamNames(request.value?.url ?? ""));

function pathParam(name: string): Pair | undefined {
  return request.value?.params.find((p) => p.kind === "path" && p.name === name);
}

function setPathParam(name: string, field: "value" | "description", text: string): void {
  const params = request.value?.params;
  if (!params) return;
  let pair = pathParam(name);
  if (!pair) {
    pair = { name, value: "", enabled: true, kind: "path", description: "" };
    params.unshift(pair);
  }
  pair[field] = text;
}

// --- Onglets ---------------------------------------------------------------------

type Tab = "params" | "body" | "headers" | "auth" | "docs" | "settings" | "assertions";
const tab = ref<Tab>("params");
const tabs = computed<{ id: Tab; label: string; count?: number }[]>(() => {
  const r = request.value;
  const activeQuery = r?.params.filter((p) => p.kind === "query" && p.enabled).length ?? 0;
  return [
    { id: "params", label: "Paramètres", count: activeQuery + pathNames.value.length },
    { id: "body", label: "Corps" },
    { id: "headers", label: "En-têtes", count: r?.headers.filter((h) => h.enabled).length },
    { id: "auth", label: "Auth" },
    { id: "docs", label: "Docs" },
    { id: "settings", label: "Réglages" },
    { id: "assertions", label: "Vérifications", count: r?.assertions.length },
  ];
});
</script>

<template>
  <section v-if="request" class="editor" aria-label="Requête">
    <div class="editor__top">
      <p v-if="contract" class="editor__crumbs">
        <span>Contrats de service</span>
        <span aria-hidden="true">/</span>
        <button
          type="button"
          class="editor__contract"
          :title="`Ouvrir le contrat (${contract.file})`"
          @click="openContract"
        >
          {{ contract.name }} · {{ contract.operation }}
        </button>
        <span v-if="contract.version" class="editor__version">Contrat v{{ contract.version }}</span>
        <button
          type="button"
          class="link"
          title="La requête ne sera plus vérifiée contre ce contrat"
          @click="unlinkContract"
        >
          Délier
        </button>
      </p>
      <p v-else-if="folder" class="editor__crumbs editor__folder">{{ folder }}</p>
      <p v-else class="editor__crumbs" />
      <p
        class="editor__saved"
        :class="`editor__saved--${editor?.saveState ?? 'saved'}`"
        role="status"
        :title="editor?.saveError ?? undefined"
      >
        {{ saveLabel }}
        <button v-if="editor?.saveState === 'error'" type="button" class="link" @click="retrySave">
          Réessayer
        </button>
      </p>
    </div>
    <p v-if="editor?.changedElsewhere" class="editor__elsewhere" role="status">
      Modifiée dans une autre fenêtre.
      <button type="button" class="link" @click="workbench.reload(editorKey)">Recharger</button>
    </p>

    <form class="editor__line" @submit.prevent="workbench.send(editorKey)">
      <label :for="ids.method" class="sr-only">Méthode</label>
      <select :id="ids.method" v-model="request.method" class="editor__method">
        <option v-for="m in METHODS" :key="m" :value="m">{{ m }}</option>
      </select>
      <label :for="ids.url" class="sr-only">Adresse</label>
      <input
        :id="ids.url"
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
    </form>

    <div class="editor__meta">
      <div class="meta">
        <label :for="ids.name">Nom</label>
        <input :id="ids.name" v-model="request.name" class="meta__input" type="text" />
      </div>
      <div class="meta">
        <label :for="ids.desc">Description</label>
        <input :id="ids.desc" v-model="request.description" class="meta__input" type="text" />
      </div>
      <div class="meta">
        <span :id="ids.tags">Étiquettes</span>
        <ul class="tags" :aria-labelledby="ids.tags">
          <li v-for="(tag, index) in request.tags" :key="tag" class="tag">
            {{ tag }}
            <button
              type="button"
              class="tag__remove"
              :aria-label="`Retirer l'étiquette ${tag}`"
              @click="removeTag(index)"
            >
              ×
            </button>
          </li>
          <li>
            <input
              v-if="addingTag"
              ref="tagInput"
              v-model="newTag"
              class="tag__input"
              type="text"
              aria-label="Nouvelle étiquette"
              @keydown.enter.prevent="commitTag"
              @keydown.escape.prevent="addingTag = false"
              @blur="commitTag"
            />
            <button v-else type="button" class="tag__add" @click="startTag">+ Étiquette</button>
          </li>
        </ul>
      </div>
    </div>

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
      <div v-if="tab === 'params'" class="stack">
        <h3 class="section">Paramètres de requête</h3>
        <PairsEditor
          v-model="request.params"
          kind="query"
          label="Paramètres de requête"
          add-label="+ Paramètre"
          value-title="Valeur d'exemple"
        />
        <h3 class="section">Paramètres de chemin</h3>
        <p v-if="pathNames.length === 0" class="hint">
          Ils viennent du chemin : écrivez <code>/orders/:id</code> et « id » apparaît ici.
        </p>
        <div v-for="name in pathNames" :key="name" class="path-param">
          <span class="path-param__name"><span aria-hidden="true">{ }</span> {{ name }}</span>
          <input
            class="path-param__input"
            type="text"
            spellcheck="false"
            :value="pathParam(name)?.value ?? ''"
            :aria-label="`Valeur de ${name}`"
            placeholder="Valeur d'exemple"
            @input="setPathParam(name, 'value', ($event.target as HTMLInputElement).value)"
          />
          <input
            class="path-param__input path-param__description"
            type="text"
            :value="pathParam(name)?.description ?? ''"
            :aria-label="`Description de ${name}`"
            placeholder="Description"
            @input="setPathParam(name, 'description', ($event.target as HTMLInputElement).value)"
          />
        </div>
      </div>

      <BodyEditor v-else-if="tab === 'body'" v-model="request.body" />

      <div v-else-if="tab === 'headers'" class="stack">
        <PairsEditor v-model="request.headers" label="En-têtes" add-label="+ En-tête" />
        <template v-if="request.inherited.headers.length > 0">
          <h3 class="section">Hérités</h3>
          <ul class="inherited">
            <li v-for="h in request.inherited.headers" :key="h.name">
              <code>{{ h.name }}: {{ h.value }}</code>
              <span class="inherited__source">{{ h.source }}</span>
            </li>
          </ul>
        </template>
      </div>

      <AuthEditor
        v-else-if="tab === 'auth'"
        v-model="request.auth"
        :inherited="request.inherited"
      />

      <div v-else-if="tab === 'docs'" class="stack">
        <label class="section" :for="ids.docs">Documentation, en Markdown</label>
        <textarea :id="ids.docs" v-model="request.docs" class="editor__docs" />
        <p class="hint">Enregistrée avec l'endpoint, dans son fichier OpenCollection.</p>
      </div>

      <SettingsEditor
        v-else-if="tab === 'settings'"
        v-model="request.settings"
        :inherited="request.inherited.settings"
      />

      <AssertionsEditor
        v-else
        v-model="request.assertions"
        :linked="request.contract !== undefined"
        :results="editor?.outcome?.assertions ?? null"
      />
    </div>

    <p v-if="request.preserved.length > 0" class="editor__preserved" role="note">
      Conservé dans le fichier sans être modifié par Arnod : {{ request.preserved.join(", ") }}.
    </p>
  </section>
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
}

.editor__top {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.5rem;
}

.editor__crumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
  margin: 0;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.editor__folder {
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.editor__contract {
  padding: 0;
  border: 0;
  font: inherit;
  color: var(--color-link);
  background: none;
  cursor: pointer;
  overflow-wrap: anywhere;
}

.editor__version {
  padding: 0.0625rem 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 999px;
  font: 600 0.75rem var(--font-mono);
  color: var(--color-text);
  background: var(--color-chip);
}

.editor__saved {
  display: flex;
  gap: 0.375rem;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.editor__saved--error {
  color: var(--color-danger);
}

.editor__elsewhere {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-radius: 8px;
  font-size: 0.8125rem;
  background: var(--color-accent-soft);
}

/* Zone étroite (jusqu'à 280 px, Z4) : le bouton passe sous l'adresse. */
.editor__line {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.editor__method,
.editor__url,
.meta__input,
.editor__docs,
.path-param__input,
.tag__input {
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
  font-size: 0.9375rem;
}

.editor__meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
}

.meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.meta__input {
  min-width: 0;
  font-size: 0.875rem;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border-radius: 999px;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-chip);
}

.tag__remove,
.tag__add {
  border: 0;
  background: none;
  font: inherit;
  cursor: pointer;
}

.tag__remove {
  padding: 0 0.125rem;
  color: var(--color-muted);
}

.tag__add {
  padding: 0.25rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  font-size: 0.8125rem;
  color: var(--color-text);
  background: var(--color-surface);
}

.tag__input {
  width: 9rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.8125rem;
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
  padding: 0.5rem 0.625rem;
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

.section {
  margin: 0.25rem 0 0;
  font-size: 0.8125rem;
  font-weight: 600;
}

.hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.path-param {
  display: grid;
  grid-template-columns: minmax(6rem, auto) minmax(0, 1fr);
  gap: 0.25rem 0.5rem;
  align-items: center;
}

.path-param__name {
  font: 600 0.8125rem var(--font-mono);
}

.path-param__input {
  min-width: 0;
  padding: 0.375rem 0.5rem;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.path-param__description {
  grid-column: 2;
  font-family: inherit;
  font-size: 0.75rem;
}

.inherited {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: 0.8125rem;
}

.inherited__source {
  margin-left: 0.5rem;
  color: var(--color-muted);
}

.editor__docs {
  min-height: 12rem;
  font: 0.8125rem var(--font-mono);
  resize: vertical;
}

.editor__preserved {
  margin: 0;
  font-size: 0.75rem;
  color: var(--color-muted);
}

.link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

:is(
  .editor__method,
  .editor__url,
  .meta__input,
  .editor__docs,
  .path-param__input,
  .tag__input,
  .tag__remove,
  .tag__add,
  .editor__tab,
  .editor__contract,
  .link
):focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
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
