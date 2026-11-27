<script setup lang="ts">
/**
 * Page des environnements d'un projet (3.3-a, AM43 ; décisions Q7, Q8 et Q10 du plan
 * d'alignement), dans un onglet : fil d'Ariane, « Environnements · N · propres à ce projet »,
 * « Copier depuis un autre projet… », « + Nouvel environnement » ; la liste réordonnée par ⋮⋮
 * (glisser, ↑ ↓ sur la poignée) ; l'environnement choisi : nom et couleur, un seul tableau de
 * variables avec leur portée (partagée : dans le projet ; locale : sur ce poste, au coffre),
 * « Utilisation » (dernier run), « Protection » et « Supprimer l'environnement ».
 *
 * L'environnement s'enregistre seul (ER2) : changer d'environnement ou fermer l'onglet
 * enregistre d'abord ce qui attend. Une valeur locale s'enregistre quand on quitte son champ.
 */
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  useId,
  useTemplateRef,
  watch,
} from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import RowRemoveButton from "@/components/ui/RowRemoveButton.vue";
import SaveState from "@/components/ui/SaveState.vue";
import CopyEnvironmentsDialog from "@/components/workbench/CopyEnvironmentsDialog.vue";
import NewEnvironmentDialog from "@/components/workbench/NewEnvironmentDialog.vue";
import { useAutosave } from "@/composables/useAutosave";
import { ENVIRONMENT_COLORS, envHost, envSwatch } from "@/lib/environments";
import { formatFinished } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { EnvironmentDoc, EnvironmentUsage, EnvironmentView } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const workbench = useWorkbenchStore();
const catalog = useCatalogStore();
const toasts = useToastStore();
const ids = {
  title: useId(),
  name: useId(),
  variables: useId(),
  usage: useId(),
  protection: useId(),
  handleHint: useId(),
};
const listRef = useTemplateRef<HTMLElement>("list");

const projectName = computed(() => catalog.projectById(props.projectId)?.name ?? "projet");
const view = computed(() => workbench.environments[props.projectId] ?? null);
const list = computed(() => view.value?.environments ?? []);

const editing = ref<string | null>(null);
const draft = ref<EnvironmentDoc | null>(null);
/** Valeurs locales saisies, enregistrées quand on quitte le champ. */
const localValues = ref<Record<string, string>>({});
const error = ref<string | null>(null);
const creating = ref(false);
const copying = ref(false);
const usage = ref<EnvironmentUsage[]>([]);

const current = computed<EnvironmentView | null>(
  () => list.value.find((e) => e.name === editing.value) ?? null,
);

// --- Enregistrement ----------------------------------------------------------------------

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

/** Valeur locale lisible : déjà en clair sur ce poste (une valeur extraite, Q10). */
function clearValue(env: EnvironmentView | null, name: string): string | null {
  const local = env?.localVars.find((v) => v.name === name);
  return local && !local.secret ? local.value : null;
}

function load(env: EnvironmentView | undefined): void {
  error.value = null;
  if (!env) {
    editing.value = null;
    draft.value = null;
    localValues.value = {};
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
  localValues.value = Object.fromEntries(env.locals.map((n) => [n, clearValue(env, n) ?? ""]));
  autosave.reset();
}

async function loadUsage(): Promise<void> {
  usage.value = await workbench.environmentUsage(props.projectId).catch(() => []);
}

onMounted(async () => {
  if (!view.value) await workbench.refreshEnvironments(props.projectId).catch(toasts.showError);
  const focus = workbench.environmentFocus;
  const wanted =
    focus?.projectId === props.projectId ? focus.name : (view.value?.active ?? undefined);
  if (focus?.projectId === props.projectId) workbench.environmentFocus = null;
  load(list.value.find((e) => e.name === wanted) ?? list.value[0]);
  await loadUsage();
});

/** Ouverte une seconde fois sur un environnement (après sa création ailleurs) : il est choisi. */
watch(
  () => workbench.environmentFocus,
  async (focus) => {
    if (focus?.projectId !== props.projectId) return;
    workbench.environmentFocus = null;
    const env = list.value.find((e) => e.name === focus.name);
    if (env && (await autosave.flush())) load(env);
  },
);

/** Un environnement retiré ailleurs (autre fenêtre, « Annuler ») : la page suit la liste. */
watch(list, (all) => {
  if (
    editing.value !== null &&
    !all.some((e) => e.name === editing.value) &&
    saveState.value === "saved"
  )
    load(all[0]);
});

onBeforeUnmount(() => {
  void autosave.flush();
});

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

// --- En-tête et liste ----------------------------------------------------------------------

/** « + Nouvel environnement » : ce qui attend est enregistré, puis la fenêtre de création. */
async function create(): Promise<void> {
  if (await autosave.flush()) creating.value = true;
}

function onCreated(name: string): void {
  creating.value = false;
  load(list.value.find((e) => e.name.toLowerCase() === name.toLowerCase()));
}

async function copy(): Promise<void> {
  if (await autosave.flush()) copying.value = true;
}

function onCopied(name: string): void {
  copying.value = false;
  load(list.value.find((e) => e.name === name));
}

/** Ordre de la chaîne : `name` retiré puis remis à la place `index`. */
function moved(name: string, index: number): string[] {
  const names = list.value.map((e) => e.name).filter((n) => n !== name);
  names.splice(Math.max(0, Math.min(index, names.length)), 0, name);
  return names;
}

async function reorder(names: string[], focus: string | null): Promise<void> {
  if (!(await autosave.flush())) return;
  await guard(() => workbench.reorderEnvironments(props.projectId, names));
  if (focus === null) return;
  await nextTick();
  listRef.value
    ?.querySelector<HTMLElement>(`.envs__handle[data-name="${CSS.escape(focus)}"]`)
    ?.focus();
}

/** ↑ ↓ sur la poignée : l'environnement monte ou descend d'un rang (équivalent du glisser). */
function onHandleKey(event: KeyboardEvent, env: EnvironmentView, index: number): void {
  const delta = event.key === "ArrowUp" ? -1 : event.key === "ArrowDown" ? 1 : 0;
  if (delta === 0) return;
  event.preventDefault();
  const target = index + delta;
  if (target < 0 || target >= list.value.length) return;
  void reorder(moved(env.name, target), env.name);
}

let dragged: string | null = null;
const dropping = ref<{ name: string; where: "before" | "after" } | null>(null);

function onDragStart(event: DragEvent, env: EnvironmentView): void {
  dragged = env.name;
  event.dataTransfer?.setData("text/plain", env.name);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}

function onDragOver(event: DragEvent, env: EnvironmentView): void {
  if (dragged === null || dragged === env.name) return;
  event.preventDefault();
  const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const ratio = box.height > 0 ? (event.clientY - box.top) / box.height : 0.5;
  dropping.value = { name: env.name, where: ratio < 0.5 ? "before" : "after" };
}

function onDrop(event: DragEvent, index: number): void {
  const source = dragged;
  const where = dropping.value?.where ?? "before";
  dragged = null;
  dropping.value = null;
  if (source === null) return;
  event.preventDefault();
  const from = list.value.findIndex((e) => e.name === source);
  const at = (where === "before" ? index : index + 1) - (from >= 0 && from < index ? 1 : 0);
  void reorder(moved(source, at), null);
}

function onDragEnd(): void {
  dragged = null;
  dropping.value = null;
}

// --- Environnement choisi ------------------------------------------------------------------

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

const variableCount = computed(
  () => (draft.value?.variables.length ?? 0) + (draft.value?.locals.length ?? 0),
);

function rowName(name: string, position: number): string {
  return name.trim() === "" ? `la variable ${String(position + 1)}` : name;
}

function addVariable(): void {
  draft.value?.variables.push({ name: "", value: "" });
}

function removeVariable(index: number): void {
  draft.value?.variables.splice(index, 1);
}

function removeLocal(index: number): void {
  draft.value?.locals.splice(index, 1);
}

/** Valeur d'une variable locale : au coffre, sauf une valeur déjà en clair sur ce poste (Q10). */
async function saveLocal(name: string): Promise<void> {
  const env = editing.value;
  const value = localValues.value[name] ?? "";
  const clear = clearValue(current.value, name);
  if (env === null || value === (clear ?? "")) return;
  if (!(await autosave.flush())) return;
  const secret = clear === null;
  await guard(async () => {
    await workbench.setLocal(props.projectId, env, name, value, secret);
    if (secret) {
      localValues.value[name] = "";
      toasts.show(`Valeur de « ${name} » rangée dans le coffre du système.`);
    }
  });
}

/** Portée « locale » : le nom reste déclaré dans le projet, la valeur va au coffre (Q10). */
async function toLocal(index: number): Promise<void> {
  const doc = draft.value;
  const variable = doc?.variables[index];
  if (!doc || !variable) return;
  const name = variable.name.trim();
  if (name === "" || doc.locals.includes(name)) return;
  doc.variables.splice(index, 1);
  doc.locals.push(name);
  localValues.value[name] = variable.value;
  if (variable.value !== "") await saveLocal(name);
  else await autosave.flush();
}

/** Portée « partagée » : la variable revient dans le projet ; une valeur au coffre est à saisir. */
async function toShared(index: number): Promise<void> {
  const doc = draft.value;
  const name = doc?.locals[index];
  if (!doc || name === undefined) return;
  const clear = clearValue(current.value, name);
  doc.locals.splice(index, 1);
  doc.variables.push({ name, value: clear ?? "" });
  if (clear === null && current.value?.localVars.some((v) => v.name === name && v.hasValue)) {
    toasts.show(`La valeur de « ${name} » reste au coffre : saisissez-la pour la partager.`);
  }
  await autosave.flush();
}

function onScope(event: Event, index: number, from: "shared" | "local"): void {
  const to = (event.target as HTMLSelectElement).value;
  if (to === from) return;
  void (from === "shared" ? toLocal(index) : toShared(index));
}

function localPlaceholder(name: string): string {
  return current.value?.localVars.find((v) => v.name === name)?.hasValue
    ? "•••• (au coffre)"
    : "Valeur à saisir";
}

/** « Utilisation » (Q8) : dernier run sur l'environnement que le cœur garde. */
const usageText = computed(() => {
  const found = usage.value.find((u) => u.environment === editing.value);
  if (!found) return "Aucun run sur cet environnement pour l'instant.";
  const what =
    found.contract !== null
      ? `du contrat « ${found.contract} »`
      : found.folder === ""
        ? "du projet"
        : `du dossier « ${found.folder} »`;
  const number = found.number > 0 ? ` n° ${String(found.number)}` : "";
  return `Dernier run${number} ${what}, ${formatFinished(found.startedAt)}.`;
});

async function remove(): Promise<void> {
  if (!(await autosave.flush())) return;
  const name = editing.value;
  if (name === null) return;
  await guard(async () => {
    await workbench.deleteEnvironment(props.projectId, name);
    load(list.value[0]);
  });
}
</script>

<template>
  <section class="envs" :aria-labelledby="ids.title">
    <nav class="envs__crumbs" aria-label="Emplacement">
      <span>{{ projectName }}</span>
      <span aria-hidden="true">/</span>
      <span class="envs__crumb-current">environnements</span>
    </nav>
    <header class="envs__head">
      <h1 :id="ids.title" class="envs__page-title">Environnements</h1>
      <span class="envs__count">{{ list.length }} · propres à ce projet</span>
      <SaveState :state="saveState" :error="saveError" @retry="autosave.flush()" />
      <span class="envs__head-actions">
        <BaseButton @click="copy">Copier depuis un autre projet…</BaseButton>
        <BaseButton variant="primary" @click="create">+ Nouvel environnement</BaseButton>
      </span>
    </header>
    <p class="envs__intro">
      Leur nombre et leurs noms suivent l'organisation de votre entreprise. L'ordre de la liste est
      votre chaîne de validation.
    </p>

    <div class="envs__columns">
      <div class="envs__side">
        <p :id="ids.handleHint" class="visually-hidden">
          Flèches haut et bas pour déplacer l'environnement dans la chaîne ; ou glisser.
        </p>
        <ol ref="list" class="envs__list" aria-label="Environnements du projet">
          <li
            v-for="(env, index) in list"
            :key="env.name"
            class="envs__entry"
            :class="dropping?.name === env.name && `envs__entry--${dropping.where}`"
            @dragover="onDragOver($event, env)"
            @dragleave="dropping = null"
            @drop="onDrop($event, index)"
          >
            <button
              type="button"
              class="envs__handle"
              draggable="true"
              :data-name="env.name"
              :aria-label="`Déplacer ${env.name}`"
              :aria-describedby="ids.handleHint"
              @dragstart="onDragStart($event, env)"
              @dragend="onDragEnd"
              @keydown="onHandleKey($event, env, index)"
            >
              ⋮⋮
            </button>
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
              <span v-if="env.protected" class="envs__item-meta">protégé</span>
            </button>
          </li>
        </ol>
        <BaseButton variant="link" class="envs__new" @click="create">
          + Nouvel environnement
        </BaseButton>
        <p class="envs__side-note">Glissez ⋮⋮ pour réordonner.</p>
      </div>

      <form v-if="draft" class="envs__form" novalidate @submit.prevent="autosave.flush()">
        <label :for="ids.name" class="envs__label">Nom</label>
        <div class="envs__name-row">
          <span class="envs__name-field">
            <span class="envs__swatch" :style="{ background: envSwatch(draft.color) }" />
            <input :id="ids.name" v-model="draft.name" type="text" autocomplete="off" />
          </span>
          <BaseMenu
            :items="colorItems"
            label="Couleur de l'environnement"
            align="end"
            @select="setColor"
          >
            <template #trigger="{ attrs }">
              <button v-bind="attrs" type="button" class="envs__color">Couleur ▾</button>
            </template>
          </BaseMenu>
        </div>

        <h2 :id="ids.variables" class="envs__title">
          Variables · {{ variableCount }}
          <span class="envs__title-hint"
            >utilisables partout avec <code v-pre>{{ nom }}</code></span
          >
        </h2>
        <table class="envs__table" :aria-labelledby="ids.variables">
          <thead>
            <tr>
              <th scope="col">Variable</th>
              <th scope="col">Valeur</th>
              <th scope="col">Portée</th>
              <th scope="col"><span class="visually-hidden">Retirer</span></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(variable, index) in draft.variables" :key="`p${index}`">
              <td>
                <input
                  v-model="variable.name"
                  type="text"
                  spellcheck="false"
                  :aria-label="`Nom de la variable ${index + 1}`"
                />
              </td>
              <td>
                <input
                  v-model="variable.value"
                  type="text"
                  spellcheck="false"
                  :aria-label="`Valeur de ${rowName(variable.name, index)}`"
                />
              </td>
              <td>
                <select
                  class="envs__scope envs__scope--shared"
                  value="shared"
                  :aria-label="`Portée de ${rowName(variable.name, index)}`"
                  @change="onScope($event, index, 'shared')"
                >
                  <option value="shared">partagée</option>
                  <option value="local">locale</option>
                </select>
              </td>
              <td>
                <RowRemoveButton
                  :label="`Supprimer ${rowName(variable.name, index)}`"
                  @click="removeVariable(index)"
                />
              </td>
            </tr>
            <tr v-for="(name, index) in draft.locals" :key="`l${name}`">
              <td>
                <code class="envs__local-name">{{ name }}</code>
              </td>
              <td>
                <input
                  v-model="localValues[name]"
                  :type="clearValue(current, name) === null ? 'password' : 'text'"
                  autocomplete="off"
                  :placeholder="localPlaceholder(name)"
                  :aria-label="`Valeur de ${name}`"
                  @change="saveLocal(name)"
                  @keydown.enter.prevent="saveLocal(name)"
                />
              </td>
              <td>
                <select
                  class="envs__scope envs__scope--local"
                  value="local"
                  :aria-label="`Portée de ${name}`"
                  @change="onScope($event, index, 'local')"
                >
                  <option value="shared">partagée</option>
                  <option value="local">locale</option>
                </select>
              </td>
              <td>
                <RowRemoveButton :label="`Supprimer ${name}`" @click="removeLocal(index)" />
              </td>
            </tr>
          </tbody>
        </table>
        <div class="envs__add-row">
          <BaseButton @click="addVariable">+ Variable</BaseButton>
          <p class="envs__hint">
            Partagée : enregistrée dans le projet, envoyée à l'équipe avec lui. Locale : rangée dans
            le coffre du système de ce poste, jamais envoyée.
          </p>
        </div>

        <h2 :id="ids.usage" class="envs__title">Utilisation</h2>
        <p class="envs__usage" :aria-labelledby="ids.usage">{{ usageText }}</p>

        <h2 :id="ids.protection" class="envs__title">Protection</h2>
        <label class="envs__check">
          <input v-model="draft.protected" type="checkbox" :aria-describedby="ids.protection" />
          <span>
            Demander une confirmation avant POST, PUT, PATCH et DELETE
            <span class="envs__hint envs__hint--block"
              >Pour la pré-production ou tout environnement partagé. Les GET partent
              directement.</span
            >
          </span>
        </label>

        <p v-if="error ?? saveError" class="envs__error" role="alert">{{ error ?? saveError }}</p>
        <div class="envs__footer">
          <BaseButton variant="danger" @click="remove">Supprimer l'environnement</BaseButton>
          <p class="envs__hint">
            Une suppression se rattrape par « Annuler » dans la notification.
          </p>
        </div>
      </form>
    </div>
  </section>
  <NewEnvironmentDialog
    v-if="creating"
    :open="creating"
    :project-id="projectId"
    @close="creating = false"
    @created="onCreated"
  />
  <CopyEnvironmentsDialog
    v-if="copying"
    :open="copying"
    :project-id="projectId"
    @close="copying = false"
    @copied="onCopied"
  />
</template>

<style scoped>
.envs {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
  min-height: 100%;
  padding: 1.25rem 2rem 2rem;
  overflow-y: auto;
}

.envs__crumbs {
  display: flex;
  gap: 0.5rem;
  font-size: var(--text-body);
  color: var(--color-muted);
}

.envs__crumb-current {
  color: var(--color-link);
}

.envs__head {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 0.875rem;
  align-items: baseline;
}

.envs__page-title {
  margin: 0;
  font-size: var(--text-display);
  font-weight: 600;
}

.envs__count {
  color: var(--color-muted);
}

.envs__head-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-left: auto;
}

.envs__intro {
  margin: 0;
  color: var(--color-muted);
}

.envs__columns {
  display: grid;
  grid-template-columns: minmax(14rem, 20rem) minmax(0, 1fr);
  gap: 1.5rem;
  align-items: start;
}

@media (max-width: 52rem) {
  .envs__columns {
    grid-template-columns: minmax(0, 1fr);
  }
}

.envs__side {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem;
  border-radius: var(--radius-lg);
  background: var(--color-sidebar);
}

.envs__list {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.envs__entry {
  display: flex;
  align-items: center;
  gap: 0.125rem;
  border-radius: var(--radius-md);
}

.envs__entry--before {
  box-shadow: inset 0 2px 0 var(--color-accent);
}

.envs__entry--after {
  box-shadow: inset 0 -2px 0 var(--color-accent);
}

.envs__handle {
  display: inline-grid;
  flex: none;
  place-items: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: none;
  font: inherit;
  color: var(--color-muted);
  cursor: grab;
}

.envs__handle:focus-visible,
.envs__item:focus-visible,
.envs__color:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.envs__item {
  display: flex;
  flex: 1;
  gap: 0.5rem;
  align-items: center;
  min-width: 0;
  padding: 0.375rem 0.5rem;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.envs__item[aria-current="true"] {
  font-weight: 600;
  background-color: var(--color-accent-soft);
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
  flex: 1;
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
  font-weight: 400;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-muted);
}

.envs__item-meta {
  flex: none;
  font-size: var(--text-small);
  font-weight: 400;
  color: var(--color-muted);
}

.envs__new {
  align-self: flex-start;
  margin-top: 0.375rem;
}

.envs__side-note {
  margin: 0.75rem 0 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.envs__form {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  min-width: 0;
}

.envs__label {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.envs__name-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.envs__name-field {
  position: relative;
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
}

.envs__name-field .envs__swatch {
  position: absolute;
  left: 0.75rem;
}

.envs__name-field input {
  box-sizing: border-box;
  flex: 1;
  min-width: 0;
  padding-left: 1.75rem;
}

.envs__color {
  display: inline-flex;
  flex: none;
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

.envs__title {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: baseline;
  margin: 0.75rem 0 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.envs__title-hint {
  font-size: var(--text-small);
  font-weight: 400;
  color: var(--color-muted);
}

.envs__title-hint code {
  font-family: var(--font-mono);
  color: var(--color-link);
}

.envs__table {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  border-collapse: separate;
  border-spacing: 0;
  background: var(--color-surface);
}

.envs__table th {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--color-border);
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-align: left;
  text-transform: uppercase;
  color: var(--color-muted);
}

.envs__table td {
  padding: 0.375rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
}

.envs__table tr:last-child td {
  border-bottom: 0;
}

.envs__table td:last-child,
.envs__table th:last-child {
  width: 2rem;
}

.envs__table input {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  font-family: var(--font-mono);
}

.envs__local-name {
  padding: 0 0.5rem;
  font-family: var(--font-mono);
}

.envs__scope {
  min-height: var(--control-sm);
  font-size: var(--text-small);
}

.envs__scope--shared {
  background-color: var(--color-success-soft);
}

.envs__scope--local {
  background-color: var(--color-accent-soft);
}

.envs__add-row {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
}

.envs__add-row .envs__hint {
  flex: 1;
}

.envs__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.envs__hint--block {
  display: block;
}

.envs__usage {
  margin: 0;
}

.envs__check {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
}

.envs__error {
  margin: 0;
  color: var(--color-danger);
}

.envs__footer {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border);
}
</style>
