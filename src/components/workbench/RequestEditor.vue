<script setup lang="ts">
/**
 * Éditeur d'un endpoint (docs/plan-configuration-endpoint.md, EP1 à EP7) : tout se modifie sur
 * place et s'enregistre seul. En-tête (contrat lié, méthode, adresse, nom, description,
 * étiquettes), puis la requête par onglets, comme dans Bruno.
 *
 * Saisie (docs/plan-mvp-b-saisie-curl.md) : variables inconnues soulignées et signalées sous
 * l'adresse, un nombre ou un point par onglet rempli, en-têtes ajoutés à l'envoi listés,
 * `</>` et Ctrl+Maj+C pour copier la requête en cURL.
 */
import {
  computed,
  nextTick,
  onBeforeUnmount,
  provide,
  ref,
  useId,
  useTemplateRef,
  watch,
} from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import AssertionsEditor from "@/components/workbench/AssertionsEditor.vue";
import AuthEditor from "@/components/workbench/AuthEditor.vue";
import BodyEditor from "@/components/workbench/BodyEditor.vue";
import EnvironmentsDialog from "@/components/workbench/EnvironmentsDialog.vue";
import PairsEditor from "@/components/workbench/PairsEditor.vue";
import SettingsEditor from "@/components/workbench/SettingsEditor.vue";
import TagsEditor from "@/components/workbench/TagsEditor.vue";
import VariablesEditor from "@/components/workbench/VariablesEditor.vue";
import { pathParamNames } from "@/lib/url";
import { KNOWN_VARIABLES, vVariables } from "@/lib/variables";
import { useContractsStore } from "@/stores/contracts";
import { useInputStore } from "@/stores/input";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { CurlCopy, HttpMethod, Pair, RequestDoc, UnknownVariable } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const contracts = useContractsStore();
const inputs = useInputStore();
const toasts = useToastStore();
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
  unknown: useId(),
};

// --- Contrat lié (C7) ------------------------------------------------------

const contract = computed(() => {
  const link = request.value?.contract;
  if (!link || !editor.value) return null;
  const summary = contracts.lists[editor.value.projectId]?.find((c) => c.path === link.file);
  return {
    name: summary?.name ?? link.file,
    // Un contrat déduit des essais n'a pas de version à montrer (0.0.0).
    version: summary && !summary.deduced ? summary.version : "",
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

// --- Saisie : variables connues, inconnues, en-têtes ajoutés (5.1-b ter) ------------

const input = computed(() => inputs.inputs.get(props.editorKey) ?? null);
const known = computed<ReadonlySet<string> | null>(() =>
  input.value ? new Set(input.value.known) : null,
);
provide(KNOWN_VARIABLES, known);
const unknown = computed<UnknownVariable[]>(() => input.value?.unknown ?? []);
const added = computed(() => input.value?.added ?? []);
const activeEnvironment = computed(() =>
  editor.value ? workbench.activeEnvironment(editor.value.projectId) : null,
);

/** Attente après la dernière frappe avant de relire les variables. */
const INSPECT_MS = 250;
let inspectTimer: ReturnType<typeof setTimeout> | undefined;
watch(
  [
    request,
    activeEnvironment,
    () => editor.value && workbench.environments[editor.value.projectId],
  ],
  () => {
    clearTimeout(inspectTimer);
    inspectTimer = setTimeout(() => {
      void inputs.inspect(props.editorKey);
    }, INSPECT_MS);
  },
  { deep: true, immediate: true },
);
onBeforeUnmount(() => {
  clearTimeout(inspectTimer);
  inputs.forget(props.editorKey);
});

/** « définie dans Local (v2) », ou rien. */
function definedIn(variable: UnknownVariable): string {
  const places = variable.definedIn.map((source) =>
    source.value === null
      ? `${source.environment} (locale)`
      : source.value === ""
        ? source.environment
        : `${source.environment} (${source.value})`,
  );
  return places.length === 0 ? "" : ` ; définie dans ${places.join(", ")}`;
}

async function addVariable(variable: UnknownVariable): Promise<void> {
  if (!editor.value) return;
  const value = variable.definedIn.find((source) => source.value !== null)?.value ?? "";
  try {
    const env = await inputs.addVariable(editor.value.projectId, variable.name, value);
    toasts.show(`✓ Variable « ${variable.name} » ajoutée à ${env}.`);
  } catch (cause) {
    toasts.showError(cause);
  }
}

const managingEnvironments = ref(false);

// --- Copier en cURL (5.1-b quater) -------------------------------------------------

const curlOpen = ref(false);
const curl = ref<CurlCopy | null>(null);
const curlError = ref<string | null>(null);
const curlBox = useTemplateRef<HTMLElement>("curlBox");
const curlButton = useTemplateRef<HTMLButtonElement>("curlButton");

/** Morceaux de la commande : les variables du shell (`$TOKEN`) sont mises en évidence. */
const curlParts = computed(() =>
  (curl.value?.command ?? "")
    .split(/(\$\{?[A-Z_][A-Z0-9_]*\}?)/)
    .map((text) => ({ text, variable: /^\$\{?[A-Z_]/.test(text) })),
);
const curlVariables = computed(() =>
  curlParts.value.filter((p) => p.variable).map((p) => p.text.replace(/[{}]/g, "")),
);

/** Place de la fenêtre cURL : posée sous `</>`, hors de la zone (qui coupe ce qui déborde). */
const curlPlace = ref<{ top: string; left: string }>({ top: "0", left: "0" });

function placeCurl(): void {
  const rect = curlButton.value?.getBoundingClientRect();
  if (!rect) return;
  const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
  const width = Math.min(30 * rem, window.innerWidth - 2 * rem);
  const left = Math.max(rem, Math.min(rect.left - 0.75 * rem, window.innerWidth - width - rem));
  curlPlace.value = { top: `${String(rect.bottom + 6)}px`, left: `${String(left)}px` };
}

async function openCurl(copy: boolean): Promise<void> {
  curlError.value = null;
  placeCurl();
  curlOpen.value = true;
  try {
    curl.value = await inputs.curl(props.editorKey);
  } catch (cause) {
    curl.value = null;
    curlError.value = cause instanceof Error ? cause.message : String(cause);
    return;
  }
  if (copy) await copyCurl();
  await nextTick();
  curlBox.value?.focus();
}

async function copyCurl(): Promise<void> {
  if (!curl.value) return;
  try {
    await navigator.clipboard.writeText(curl.value.command);
    toasts.show("✓ Requête copiée en cURL.");
  } catch {
    toasts.show("Copie impossible : sélectionnez la commande, puis Ctrl+C.");
  }
}

function closeCurl(): void {
  curlOpen.value = false;
  void nextTick(() => curlButton.value?.focus());
}

function onCurlFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && (curlBox.value?.contains(next) || curlButton.value?.contains(next))) return;
  if (next) curlOpen.value = false;
}

/** Ctrl+Maj+C dans l'éditeur : copie la requête en cURL (code de la touche, pour l'AZERTY). */
function onKeydown(event: KeyboardEvent): void {
  if (event.ctrlKey && event.shiftKey && !event.altKey && event.code === "KeyC") {
    event.preventDefault();
    void openCurl(true);
  }
}

// --- Onglets ---------------------------------------------------------------------

type Tab =
  "params" | "body" | "headers" | "auth" | "docs" | "settings" | "variables" | "assertions";
const tab = ref<Tab>("params");
/** Un nombre pour une liste, un point pour un onglet rempli d'une seule valeur. */
const tabs = computed<{ id: Tab; label: string; count?: number; filled?: boolean }[]>(() => {
  const r = request.value;
  const activeQuery = r?.params.filter((p) => p.kind === "query" && p.enabled).length ?? 0;
  const settings = r ? Object.values(r.settings).some((value) => value !== null) : false;
  return [
    { id: "params", label: "Paramètres", count: activeQuery + pathNames.value.length },
    { id: "body", label: "Corps", filled: r !== null && r.body.kind !== "none" },
    { id: "headers", label: "En-têtes", count: r?.headers.filter((h) => h.enabled).length },
    {
      id: "auth",
      label: "Auth",
      filled: r !== null && r.auth.type !== "none" && r.auth.type !== "inherit",
    },
    { id: "docs", label: "Docs", filled: r !== null && r.docs.trim() !== "" },
    { id: "settings", label: "Réglages", filled: settings },
    { id: "variables", label: "Variables", count: r?.extractions.length },
    { id: "assertions", label: "Vérifications", count: r?.assertions.length },
  ];
});
// --- Lot E : modifié depuis l'envoi (écart 4.2-b), curseur dans l'adresse (4.1-b) ---------

const changedSinceSent = computed(() => workbench.isChangedSinceSent(props.editorKey));
const urlInput = useTemplateRef<HTMLInputElement>("urlInput");

watch(
  () => [workbench.focusUrl, urlInput.value] as const,
  async ([wanted, input]) => {
    if (wanted !== props.editorKey || !input) return;
    workbench.focusUrl = null;
    await nextTick();
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);
  },
  { immediate: true },
);
</script>

<template>
  <section v-if="request" class="editor" aria-label="Requête" @keydown="onKeydown">
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
    <!-- Lot E (écart 4.2-b) : à part de l'état d'enregistrement, qu'il ne change pas. -->
    <p v-if="changedSinceSent" class="editor__unsent" role="status">
      <span><b aria-hidden="true">M</b> modifié, pas encore envoyé</span>
      <span aria-hidden="true">·</span>
      <button type="button" class="link" @click="workbench.revertToSent(editorKey)">
        Revenir à la version envoyée
      </button>
    </p>
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
      <span class="editor__url-box">
        <input
          :id="ids.url"
          ref="urlInput"
          v-model="request.url"
          v-variables="known"
          class="editor__url"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :placeholder="URL_PLACEHOLDER"
          :aria-describedby="unknown.length > 0 ? ids.unknown : undefined"
        />
      </span>
      <span class="editor__curl">
        <button
          ref="curlButton"
          type="button"
          class="editor__curl-button"
          aria-label="Copier la requête en cURL"
          title="Copier en cURL (Ctrl+Maj+C)"
          aria-haspopup="dialog"
          :aria-expanded="curlOpen"
          @click="curlOpen ? closeCurl() : openCurl(false)"
        >
          &lt;/&gt;
        </button>
        <div
          v-if="curlOpen"
          ref="curlBox"
          class="curl"
          :style="curlPlace"
          role="dialog"
          aria-label="Copier la requête en cURL"
          tabindex="-1"
          @keydown.escape.stop.prevent="closeCurl"
          @focusout="onCurlFocusOut"
        >
          <div class="curl__head">
            <span class="curl__title">Copier la requête <span class="curl__tag">cURL</span></span>
            <span v-if="curl" class="curl__env">variables de {{ curl.environment }}</span>
          </div>
          <p v-if="curlError" class="curl__error" role="alert">{{ curlError }}</p>
          <pre
            v-else
            class="curl__command"
          ><template v-for="(part, i) in curlParts" :key="i"><b v-if="part.variable" class="curl__variable">{{ part.text }}</b><template v-else>{{ part.text }}</template></template></pre>
          <div class="curl__foot">
            <p class="curl__note">
              <template v-if="curlVariables.length === 1 && curlVariables[0] === '$TOKEN'">
                Le jeton est remplacé par $TOKEN : aucun secret ne quitte Arnod.
              </template>
              <template v-else-if="curlVariables.length > 0">
                Secrets remplacés par {{ curlVariables.join(", ") }} : aucun secret ne quitte Arnod.
              </template>
              <template v-else>Variables résolues, aucun secret dans la commande.</template>
            </p>
            <BaseButton :disabled="curl === null" @click="copyCurl">Copier</BaseButton>
          </div>
        </div>
      </span>
      <BaseButton v-if="editor?.sending" @click="workbench.cancelSend(editorKey)">
        Annuler
      </BaseButton>
      <BaseButton v-else variant="primary" type="submit" :disabled="request.url.trim() === ''">
        Envoyer
      </BaseButton>
    </form>
    <div v-if="unknown.length > 0" :id="ids.unknown" class="unknown" role="status">
      <span class="unknown__icon" aria-hidden="true">!</span>
      <ul class="unknown__list">
        <li v-for="variable in unknown" :key="variable.name" class="unknown__item">
          <span>
            <code class="unknown__name">{{ variable.name }}</code> est inconnue dans
            {{ input?.environment }}{{ definedIn(variable) }}.
          </span>
          <button type="button" class="link" @click="addVariable(variable)">
            Ajouter<span class="sr-only"> {{ variable.name }} à {{ input?.environment }}</span>
          </button>
        </li>
      </ul>
      <button type="button" class="link unknown__manage" @click="managingEnvironments = true">
        Environnements…
      </button>
    </div>

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
        <TagsEditor
          v-model="request.tags"
          :project-id="editor?.projectId ?? ''"
          :labelledby="ids.tags"
        />
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
        <span v-else-if="t.filled" class="editor__dot">
          <span aria-hidden="true">●</span><span class="sr-only">, rempli</span>
        </span>
      </button>
    </div>

    <div :id="`panel-${tab}`" role="tabpanel" :aria-labelledby="`tab-${tab}`" class="editor__panel">
      <div v-if="tab === 'params'" class="stack">
        <PairsEditor
          v-model="request.params"
          kind="query"
          label="Paramètres de requête"
          title="Paramètres de requête"
          text-format="query"
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
            v-variables="known"
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
        <PairsEditor
          v-model="request.headers"
          label="En-têtes de cette requête"
          title="De cette requête"
          text-format="header"
          add-label="+ En-tête"
        />
        <template v-if="request.inherited.headers.length > 0">
          <h3 class="caption">Hérités · {{ request.inherited.headers.length }}</h3>
          <ul class="sent">
            <li v-for="h in request.inherited.headers" :key="h.name" class="sent__row">
              <code class="sent__name">{{ h.name }}</code>
              <code class="sent__value">{{ h.value }}</code>
              <span class="sent__source">{{ h.source }}</span>
            </li>
          </ul>
        </template>
        <template v-if="added.length > 0">
          <h3 class="caption">
            Ajoutés à l'envoi · {{ added.length }} <span class="caption__by">par Arnod</span>
          </h3>
          <ul class="sent sent--added" aria-label="En-têtes ajoutés par Arnod à l'envoi">
            <li v-for="h in added" :key="h.name" class="sent__row">
              <code class="sent__name">{{ h.name }}</code>
              <code class="sent__value">{{ h.value }}</code>
              <span v-if="h.origin !== 'client'" class="sent__source">
                {{ h.origin === "auth" ? (h.source ?? "Auth") : "Corps" }}
              </span>
            </li>
          </ul>
          <p class="hint">
            Un en-tête du même nom saisi plus haut remplace celui d'Arnod, sauf celui de l'onglet
            Auth. « Envoyé » montre les valeurs réellement parties.
          </p>
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

      <VariablesEditor
        v-else-if="tab === 'variables' && editor"
        v-model="request.extractions"
        :project-id="editor.projectId"
        :path="editor.path"
        :extracted="editor.outcome?.sent?.extracted ?? null"
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
    <EnvironmentsDialog
      v-if="managingEnvironments && editor"
      :open="managingEnvironments"
      :project-id="editor.projectId"
      :start-new="false"
      @close="managingEnvironments = false"
    />
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
  font-size: var(--text-body);
  color: var(--color-muted);
}

.editor__folder {
  font-family: var(--font-mono);
  font-size: var(--text-body);
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
  border-radius: var(--radius-pill);
  font: 600 var(--text-small) var(--font-mono);
  color: var(--color-text);
  background: var(--color-chip);
}

.editor__saved {
  display: flex;
  gap: 0.375rem;
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.editor__saved--error {
  color: var(--color-danger);
}

.editor__unsent {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  justify-content: flex-end;
  margin: -0.5rem 0 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.editor__unsent b {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-link);
}

.editor__elsewhere {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: var(--text-body);
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
.path-param__input {
  box-sizing: border-box;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

/* La barre d'adresse est le champ principal : plus haute que les autres contrôles. */
.editor__method,
.editor__url {
  min-height: var(--control-lg);
}

.editor__method {
  padding-right: 1.75rem;
  font-weight: 600;
}

.editor__url-box {
  display: flex;
  flex: 1 1 12rem;
  min-width: 0;
}

.editor__url {
  flex: 1;
  width: 100%;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: var(--text-body-large);
}

/* `</>` : copier en cURL, et sa fenêtre posée sous le bouton. */
.editor__curl {
  position: relative;
  display: flex;
}

.editor__curl-button {
  box-sizing: border-box;
  width: var(--control-lg);
  min-height: var(--control-lg);
  padding: 0;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: 600 var(--text-body) var(--font-mono);
  color: var(--color-link);
  background-color: var(--color-surface);
  cursor: pointer;
}

.editor__curl-button[aria-expanded="true"] {
  border-color: var(--color-accent);
  background-color: var(--color-accent-soft);
}

.curl {
  position: fixed;
  z-index: 20;
  display: flex;
  flex-direction: column;
  width: min(30rem, calc(100vw - 2rem));
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background-color: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.curl:focus {
  outline: none;
}

.curl__head,
.curl__foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.625rem 0.875rem;
}

.curl__head {
  border-bottom: 1px solid var(--color-border);
}

.curl__foot {
  flex-wrap: nowrap;
  border-top: 1px solid var(--color-border);
}

.curl__title {
  font-size: var(--text-body);
  font-weight: 600;
}

.curl__tag {
  margin-left: 0.25rem;
  padding: 0.0625rem 0.375rem;
  border-radius: var(--radius-pill);
  font: 600 var(--text-small) var(--font-mono);
  background-color: var(--color-chip);
}

.curl__env,
.curl__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.curl__command {
  margin: 0;
  padding: 0.75rem 0.875rem;
  max-height: 16rem;
  overflow: auto;
  font: var(--text-body) / 1.6 var(--font-mono);
  white-space: pre;
  user-select: text;
}

.curl__variable {
  font-weight: 600;
  color: var(--color-link);
}

.curl__error {
  margin: 0;
  padding: 0.75rem 0.875rem;
  font-size: var(--text-body);
  color: var(--color-danger);
}

/* Variables inconnues de l'environnement actif (5.1-b ter). */
.unknown {
  display: flex;
  align-items: baseline;
  gap: 0.625rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-md);
  font-size: var(--text-body);
  background-color: var(--color-danger-soft);
}

.unknown__icon {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  align-self: flex-start;
  width: 1.125rem;
  height: 1.125rem;
  margin-top: 0.0625rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-surface);
  background-color: var(--color-danger);
}

.unknown__list {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  margin: 0;
  padding: 0;
  list-style: none;
}

.unknown__item {
  display: flex;
  align-items: baseline;
  gap: 0.625rem;
}

.unknown__item > span {
  flex: 1;
  min-width: 0;
}

.unknown__item > .link {
  flex: none;
}

.unknown__name {
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--color-danger);
}

.unknown__manage {
  flex: none;
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
  font-size: var(--text-body);
  color: var(--color-muted);
}

.meta__input {
  min-width: 0;
  font-size: var(--text-body);
}

/* Zone étroite : les onglets passent à la ligne, aucun n'est caché. */
.editor__tabs {
  display: flex;
  flex-wrap: wrap;
  border-bottom: 1px solid var(--color-border);
}

.editor__tab {
  flex: none;
  padding: 0.4375rem 0.375rem;
  white-space: nowrap;
  border: 0;
  border-bottom: 2px solid transparent;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  cursor: pointer;
}

.editor__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.editor__dot {
  margin-left: 0.25rem;
  font-size: var(--text-caption);
  color: var(--color-accent);
}

.editor__badge {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-caption);
  background: var(--color-chip);
}

.stack {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.section {
  margin: 0.25rem 0 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.hint {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.path-param {
  display: grid;
  grid-template-columns: minmax(6rem, auto) minmax(0, 1fr);
  gap: 0.25rem 0.5rem;
  align-items: center;
}

.path-param__name {
  font: 600 var(--text-body) var(--font-mono);
}

.path-param__input {
  min-width: 0;
  min-height: var(--control-sm);
  padding: 0.1875rem 0.5rem;
  font-family: var(--font-mono);
  font-size: var(--text-body);
}

.path-param__description {
  grid-column: 2;
  font-family: inherit;
  font-size: var(--text-small);
}

/* En-têtes hérités et ajoutés à l'envoi : lus, pas modifiables ici. */
.caption {
  margin: 0.5rem 0 0;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.caption__by {
  margin-left: 0.25rem;
  font-weight: 400;
  letter-spacing: 0;
  text-transform: none;
}

.sent {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--text-body);
}

.sent__row {
  display: grid;
  grid-template-columns: minmax(7rem, 2fr) minmax(0, 3fr) auto;
  gap: 0.5rem;
  align-items: baseline;
  padding: 0.375rem 0 0.375rem 1.5rem;
  border-bottom: 1px solid var(--color-border);
}

.sent__name,
.sent__value {
  min-width: 0;
  overflow-wrap: anywhere;
  font-family: var(--font-mono);
}

.sent--added .sent__name,
.sent--added .sent__value {
  color: var(--color-subtle);
}

.sent__source {
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  color: var(--color-muted);
  background-color: var(--color-chip);
}

.editor__docs {
  min-height: 12rem;
  font: var(--text-body) var(--font-mono);
  resize: vertical;
}

.editor__preserved {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: var(--text-body);
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
  .editor__tab,
  .editor__contract,
  .editor__curl-button,
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
