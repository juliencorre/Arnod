<script setup lang="ts">
/**
 * Gestion des environnements du projet : variables partagées (versionnées),
 * variables locales (hors du dépôt) dont les secrets vont dans le coffre du système.
 * L'environnement s'enregistre seul, comme une requête (ER2) : changer d'environnement, fermer ou
 * Échap enregistre d'abord ce qui attend, et un échec garde la fenêtre ouverte.
 *
 * AM33 (3.3-a) : liste numérotée avec l'adresse de base, « Couleur ▾ » à droite du nom,
 * protection « Demander une confirmation avant POST, PUT, PATCH et DELETE », « Supprimer
 * l'environnement ». AM34 (3.3-b) : « + Nouvel environnement » ouvre sa fenêtre de création.
 */
import { computed, ref, useId, watch } from "vue";

import RowRemoveButton from "@/components/ui/RowRemoveButton.vue";
import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import SaveState from "@/components/ui/SaveState.vue";
import NewEnvironmentDialog from "@/components/workbench/NewEnvironmentDialog.vue";
import { useAutosave } from "@/composables/useAutosave";
import { ENVIRONMENT_COLORS, envHost, envSwatch } from "@/lib/environments";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { EnvironmentDoc, EnvironmentView } from "@/types/ipc";

const props = withDefaults(
  defineProps<{ open: boolean; projectId: string; startNew?: boolean }>(),
  { startNew: false },
);
const emit = defineEmits<{ close: [] }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const nameId = useId();
const protectionId = useId();

/** « Couleur ▾ » à droite du nom (AM33). */
const colorItems = computed<MenuItem[]>(() =>
  ENVIRONMENT_COLORS.map(([id, label]) => ({
    id,
    label,
    radio: true,
    checked: draft.value?.color === id,
    swatch: envSwatch(id),
  })),
);

function setColor(color: string): void {
  if (draft.value) draft.value.color = color;
}

/** Fenêtre « Nouvel environnement » ouverte (AM34). */
const creating = ref(false);

const editing = ref<string | null>(null);
const draft = ref<EnvironmentDoc | null>(null);
const newLocal = ref("");
const localValues = ref<Record<string, string>>({});
const error = ref<string | null>(null);

const view = computed(() => workbench.environments[props.projectId] ?? null);
const list = computed(() => view.value?.environments ?? []);
const current = computed<EnvironmentView | null>(
  () => list.value.find((e) => e.name === editing.value) ?? null,
);

/** Enregistre le brouillon (une copie : la saisie peut continuer pendant l'appel). */
async function saveDraft(): Promise<void> {
  const doc = draft.value;
  if (!doc) return;
  const sent = JSON.parse(JSON.stringify(doc)) as EnvironmentDoc;
  await workbench.saveEnvironment(props.projectId, sent, editing.value);
  editing.value = sent.name;
}

const autosave = useAutosave(() => draft.value, saveDraft);
const saveState = autosave.state;
const saveError = autosave.error;

function load(env: EnvironmentView | undefined): void {
  error.value = null;
  localValues.value = {};
  if (!env) {
    editing.value = null;
    draft.value = null;
    autosave.reset();
    return;
  }
  editing.value = env.name;
  draft.value = {
    name: env.name,
    color: env.color,
    protected: env.protected,
    variables: env.variables.map((v) => ({ ...v })),
    locals: [...env.locals],
  };
  autosave.reset();
}

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    load(list.value.find((e) => e.name === view.value?.active) ?? list.value[0]);
    // « + Nouvel environnement » du menu de la pastille : la gestion s'ouvre sur un nouveau.
    if (props.startNew) void create();
  },
  { immediate: true },
);

async function guard(work: () => Promise<void>): Promise<void> {
  error.value = null;
  try {
    await work();
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
}

/** Passe à l'environnement `env` après avoir enregistré celui qui est ouvert. */
async function select(env: EnvironmentView): Promise<void> {
  if (await autosave.flush()) load(env);
}

/**
 * Fermer (Échap, fond, « Fermer ») : ce qui attend est enregistré d'abord ; un échec, ou une
 * valeur locale saisie sans choisir où elle va, garde la fenêtre ouverte et le dit.
 */
async function close(): Promise<void> {
  if (!(await autosave.flush())) return;
  const typed = Object.entries(localValues.value).find(([, value]) => value !== "");
  if (typed) {
    error.value = `La valeur de « ${typed[0]} » n'est pas enregistrée : choisissez « Enregistrer » ou « Enregistrer comme secret ».`;
    return;
  }
  emit("close");
}

/** « + Nouvel environnement » : ce qui attend est enregistré, puis la fenêtre de création s'ouvre. */
async function create(): Promise<void> {
  if (await autosave.flush()) creating.value = true;
}

/** L'environnement créé est ouvert dans la gestion. */
function onCreated(name: string): void {
  creating.value = false;
  load(list.value.find((e) => e.name.toLowerCase() === name.toLowerCase()));
}

async function remove(): Promise<void> {
  if (!(await autosave.flush())) return;
  const name = editing.value;
  if (name === null) return;
  await guard(async () => {
    await workbench.deleteEnvironment(props.projectId, name);
    load(list.value[0]);
  });
}

async function move(delta: -1 | 1): Promise<void> {
  if (!(await autosave.flush())) return;
  const names = list.value.map((e) => e.name);
  const index = names.findIndex((n) => n === editing.value);
  const target = index + delta;
  const a = names[index];
  const b = names[target];
  if (a === undefined || b === undefined) return;
  names[index] = b;
  names[target] = a;
  await guard(() => workbench.reorderEnvironments(props.projectId, names));
}

function addVariable(): void {
  draft.value?.variables.push({ name: "", value: "" });
}

function removeVariable(index: number): void {
  draft.value?.variables.splice(index, 1);
}

function addLocal(): void {
  const name = newLocal.value.trim();
  if (name === "" || !draft.value || draft.value.locals.includes(name)) return;
  draft.value.locals.push(name);
  newLocal.value = "";
}

async function saveLocal(name: string, secret: boolean): Promise<void> {
  const env = editing.value;
  if (env === null) return;
  await guard(async () => {
    await workbench.setLocal(props.projectId, env, name, localValues.value[name] ?? "", secret);
    localValues.value[name] = "";
    toasts.show(
      secret
        ? `Secret « ${name} » enregistré dans le coffre du système.`
        : `Valeur de « ${name} » enregistrée.`,
    );
  });
}
</script>

<template>
  <BaseDialog
    :open="open"
    title="Environnements"
    wide
    description="Les variables partagées sont enregistrées dans le projet. Les variables locales restent sur ce poste."
    @close="close"
  >
    <div class="envs">
      <ul class="envs__list" aria-label="Environnements du projet">
        <li v-for="(env, index) in list" :key="env.name">
          <button
            type="button"
            class="envs__item"
            :aria-current="env.name === editing"
            @click="select(env)"
          >
            <span class="envs__number" aria-hidden="true">{{ index + 1 }}</span>
            <span class="envs__swatch" :style="{ background: envSwatch(env.color) }" />
            <span class="envs__item-text">
              <span class="envs__item-name">{{ env.name }}</span>
              <span v-if="envHost(env)" class="envs__item-host">{{ envHost(env) }}</span>
            </span>
          </button>
        </li>
        <li><BaseButton variant="link" @click="create">+ Nouvel environnement</BaseButton></li>
      </ul>

      <form v-if="draft" class="envs__form" novalidate @submit.prevent="autosave.flush()">
        <label :for="nameId" class="envs__label">Nom</label>
        <div class="envs__name-row">
          <input :id="nameId" v-model="draft.name" type="text" autocomplete="off" />
          <BaseMenu
            :items="colorItems"
            label="Couleur de l'environnement"
            align="end"
            @select="setColor"
          >
            <template #trigger="{ attrs }">
              <button v-bind="attrs" type="button" class="envs__color">
                <span class="envs__swatch" :style="{ background: envSwatch(draft.color) }" />
                Couleur ▾
              </button>
            </template>
          </BaseMenu>
        </div>

        <h3 class="envs__title">Variables partagées</h3>
        <div v-for="(variable, index) in draft.variables" :key="index" class="envs__var">
          <input
            v-model="variable.name"
            type="text"
            :aria-label="`Nom de la variable ${index + 1}`"
          />
          <input
            v-model="variable.value"
            type="text"
            :aria-label="`Valeur de la variable ${index + 1}`"
          />
          <RowRemoveButton
            :label="`Supprimer la variable ${index + 1}`"
            @click="removeVariable(index)"
          />
        </div>
        <BaseButton class="envs__add" @click="addVariable">+ Variable</BaseButton>

        <h3 class="envs__title">Variables locales</h3>
        <p class="envs__hint">
          Seuls leurs noms sont enregistrés dans le projet ; les valeurs restent sur ce poste.
        </p>
        <div v-for="name in draft.locals" :key="name" class="envs__local">
          <code>{{ name }}</code>
          <input
            v-model="localValues[name]"
            type="password"
            autocomplete="off"
            :placeholder="
              current?.localVars.find((v) => v.name === name)?.hasValue
                ? '•••• (définie)'
                : 'Valeur'
            "
            :aria-label="`Valeur de ${name}`"
          />
          <span class="envs__local-actions">
            <BaseButton @click="saveLocal(name, true)">Enregistrer comme secret</BaseButton>
            <BaseButton @click="saveLocal(name, false)">Enregistrer</BaseButton>
          </span>
        </div>
        <div class="envs__declare">
          <input
            v-model="newLocal"
            type="text"
            placeholder="Nom de la variable locale"
            aria-label="Nouvelle variable locale"
            @keydown.enter.prevent="addLocal"
          />
          <BaseButton @click="addLocal">Déclarer</BaseButton>
        </div>

        <h3 :id="protectionId" class="envs__title">Protection</h3>
        <label class="envs__check">
          <input v-model="draft.protected" type="checkbox" :aria-describedby="protectionId" />
          Demander une confirmation avant POST, PUT, PATCH et DELETE
        </label>
        <p class="envs__hint">
          Pour la pré-production ou tout environnement partagé. Les GET partent directement.
        </p>

        <p v-if="error ?? saveError" class="envs__error" role="alert">{{ error ?? saveError }}</p>
        <div class="envs__actions">
          <BaseButton variant="danger" @click="remove">Supprimer l'environnement</BaseButton>
          <BaseButton aria-label="Monter l'environnement" @click="move(-1)">↑</BaseButton>
          <BaseButton aria-label="Descendre l'environnement" @click="move(1)">↓</BaseButton>
          <span class="envs__spacer" />
          <SaveState :state="saveState" :error="saveError" @retry="autosave.flush()" />
          <BaseButton @click="close">Fermer</BaseButton>
        </div>
      </form>
    </div>
  </BaseDialog>
  <NewEnvironmentDialog
    v-if="creating"
    :open="creating"
    :project-id="projectId"
    @close="creating = false"
    @created="onCreated"
  />
</template>

<style scoped>
/* Le contenu suit la largeur de la fenêtre : aucune colonne n'impose de largeur minimale. */
.envs {
  display: grid;
  grid-template-columns: minmax(8rem, 12rem) minmax(0, 1fr);
  gap: 1rem;
}

/* Fenêtre étroite : la liste passe au-dessus du formulaire. */
@media (max-width: 40rem) {
  .envs {
    grid-template-columns: minmax(0, 1fr);
  }
}

.envs__list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.envs__item {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  width: 100%;
  padding: 0.375rem 0.5rem;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: var(--color-text);
  text-align: left;
  overflow-wrap: anywhere;
  cursor: pointer;
}

.envs__item[aria-current="true"] {
  font-weight: 600;
  background: var(--color-accent-soft);
}

.envs__form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.envs__form input[type="text"],
.envs__form input[type="password"],
.envs__form select {
  box-sizing: border-box;
  min-width: 0;
  min-height: var(--control-md);
  padding: 0.3125rem 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-surface);
}

.envs__form :focus-visible,
.envs__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.envs__label {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.envs__check {
  display: flex;
  gap: 0.5rem;
  font-size: var(--text-body);
}

.envs__title {
  margin: 0.5rem 0 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.envs__hint {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.envs__var {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.5fr) auto;
  gap: 0.375rem;
}

.envs__declare {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.375rem;
}

/* Nom, valeur, puis les deux boutons, qui passent ensemble à la ligne s'il manque de place. */
.envs__local {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.375rem;
}

.envs__local code {
  flex: 0 1 8rem;
  min-width: 0;
  overflow-wrap: anywhere;
}

.envs__local input {
  flex: 1 1 10rem;
}

.envs__local-actions {
  display: flex;
  flex: none;
  gap: 0.375rem;
  margin-left: auto;
}

.envs__add {
  align-self: flex-start;
}

.envs__error {
  margin: 0;
  color: var(--color-danger);
}

.envs__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.envs__spacer {
  flex: 1;
}

/* La flèche de la liste déroulante garde sa place (styles.css). */
.envs__form select {
  padding-right: 1.75rem;
}

.envs__name-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.envs__name-row input {
  flex: 1;
  min-width: 0;
}

.envs__color {
  display: inline-flex;
  flex: none;
  gap: 0.375rem;
  align-items: center;
  min-height: var(--control-md);
  padding: 0 0.625rem;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: var(--color-muted);
  cursor: pointer;
}

.envs__color:hover,
.envs__color[aria-expanded="true"] {
  border-color: var(--color-border);
}

.envs__color:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.envs__number {
  flex: none;
  width: 1rem;
  font: var(--text-small) var(--font-mono);
  color: var(--color-muted);
}

.envs__swatch {
  flex: none;
  width: 0.5rem;
  height: 0.5rem;
  border-radius: var(--radius-sm);
}

.envs__item-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.envs__item-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.envs__item-host {
  overflow: hidden;
  font: var(--text-small) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-muted);
}
</style>
