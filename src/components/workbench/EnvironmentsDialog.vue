<script setup lang="ts">
/**
 * Gestion des environnements du projet : variables partagées (versionnées),
 * variables locales (hors du dépôt) dont les secrets vont dans le coffre du système.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { EnvironmentDoc, EnvironmentView } from "@/types/ipc";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const nameId = useId();
const colorId = useId();

const COLORS = [
  ["gray", "Gris"],
  ["blue", "Bleu"],
  ["green", "Vert"],
  ["orange", "Orange"],
  ["red", "Rouge"],
  ["purple", "Violet"],
] as const;

const editing = ref<string | null>(null);
const draft = ref<EnvironmentDoc | null>(null);
const newLocal = ref("");
const localValues = ref<Record<string, string>>({});
const error = ref<string | null>(null);

const list = computed(() => workbench.environments?.environments ?? []);
const current = computed<EnvironmentView | null>(
  () => list.value.find((e) => e.name === editing.value) ?? null,
);

function load(env: EnvironmentView | undefined): void {
  error.value = null;
  localValues.value = {};
  if (!env) {
    editing.value = null;
    draft.value = null;
    return;
  }
  editing.value = env.name;
  draft.value = {
    version: env.version,
    name: env.name,
    color: env.color,
    protected: env.protected ?? false,
    variables: env.variables.map((v) => ({ ...v })),
    locals: [...env.locals],
  };
}

watch(
  () => props.open,
  (open) => {
    if (open)
      load(list.value.find((e) => e.name === workbench.environments?.active) ?? list.value[0]);
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

async function save(): Promise<void> {
  const doc = draft.value;
  if (!doc) return;
  await guard(async () => {
    const previous = editing.value;
    await workbench.saveEnvironment(doc, previous);
    editing.value = doc.name;
    toasts.show(`Environnement « ${doc.name} » enregistré.`);
  });
}

async function create(): Promise<void> {
  await guard(async () => {
    let name = "Nouvel environnement";
    for (let n = 2; list.value.some((e) => e.name === name); n++)
      name = `Nouvel environnement ${n}`;
    await workbench.saveEnvironment(
      { version: 1, name, color: "gray", protected: false, variables: [], locals: [] },
      null,
    );
    load(workbench.environments?.environments.find((e) => e.name === name));
  });
}

async function remove(): Promise<void> {
  const name = editing.value;
  if (name === null) return;
  await guard(async () => {
    await workbench.deleteEnvironment(name);
    load(list.value[0]);
    toasts.show(`Environnement « ${name} » supprimé.`);
  });
}

async function move(delta: -1 | 1): Promise<void> {
  const names = list.value.map((e) => e.name);
  const index = names.findIndex((n) => n === editing.value);
  const target = index + delta;
  const a = names[index];
  const b = names[target];
  if (a === undefined || b === undefined) return;
  names[index] = b;
  names[target] = a;
  await guard(() => workbench.reorderEnvironments(names));
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
    await workbench.setLocal(env, name, localValues.value[name] ?? "", secret);
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
    description="Les variables partagées sont enregistrées dans le projet. Les variables locales restent sur ce poste."
    @close="emit('close')"
  >
    <div class="envs">
      <ul class="envs__list" aria-label="Environnements du projet">
        <li v-for="env in list" :key="env.name">
          <button
            type="button"
            class="envs__item"
            :aria-current="env.name === editing"
            @click="load(env)"
          >
            {{ env.name }}
          </button>
        </li>
        <li><BaseButton variant="link" @click="create">+ Nouvel environnement</BaseButton></li>
      </ul>

      <form v-if="draft" class="envs__form" novalidate @submit.prevent="save">
        <label :for="nameId" class="envs__label">Nom</label>
        <input :id="nameId" v-model="draft.name" type="text" autocomplete="off" />
        <label :for="colorId" class="envs__label">Couleur</label>
        <select :id="colorId" v-model="draft.color">
          <option v-for="[id, label] in COLORS" :key="id" :value="id">{{ label }}</option>
        </select>
        <label class="envs__check">
          <input v-model="draft.protected" type="checkbox" />
          Environnement protégé (confirmation avant d'envoyer une requête qui modifie des données)
        </label>

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
          <button
            type="button"
            :aria-label="`Supprimer la variable ${index + 1}`"
            @click="removeVariable(index)"
          >
            ×
          </button>
        </div>
        <button type="button" class="envs__add" @click="addVariable">+ Ajouter une variable</button>

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
          <BaseButton @click="saveLocal(name, true)">Enregistrer comme secret</BaseButton>
          <BaseButton @click="saveLocal(name, false)">Enregistrer</BaseButton>
        </div>
        <div class="envs__var">
          <input
            v-model="newLocal"
            type="text"
            placeholder="Nom de la variable locale"
            aria-label="Nouvelle variable locale"
            @keydown.enter.prevent="addLocal"
          />
          <BaseButton @click="addLocal">Déclarer</BaseButton>
        </div>

        <p v-if="error" class="envs__error" role="alert">{{ error }}</p>
        <div class="envs__actions">
          <BaseButton variant="danger" @click="remove">Supprimer</BaseButton>
          <BaseButton aria-label="Monter l'environnement" @click="move(-1)">↑</BaseButton>
          <BaseButton aria-label="Descendre l'environnement" @click="move(1)">↓</BaseButton>
          <span class="envs__spacer" />
          <BaseButton @click="emit('close')">Fermer</BaseButton>
          <BaseButton variant="primary" type="submit">Enregistrer</BaseButton>
        </div>
      </form>
    </div>
  </BaseDialog>
</template>

<style scoped>
.envs {
  display: grid;
  grid-template-columns: 12rem minmax(24rem, 1fr);
  gap: 1rem;
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
  width: 100%;
  padding: 0.375rem 0.5rem;
  border: 0;
  border-radius: 6px;
  background: none;
  font: inherit;
  color: var(--color-text);
  text-align: left;
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
}

.envs__form input[type="text"],
.envs__form input[type="password"],
.envs__form select {
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: 6px;
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.envs__form :focus-visible,
.envs__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.envs__label {
  font-weight: 600;
}

.envs__check {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.envs__title {
  margin: 0.5rem 0 0;
  font-size: 0.9375rem;
}

.envs__hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.envs__var {
  display: grid;
  grid-template-columns: 1fr 1.5fr auto;
  gap: 0.375rem;
}

.envs__local {
  display: grid;
  grid-template-columns: 8rem 1fr auto auto;
  align-items: center;
  gap: 0.375rem;
}

.envs__add {
  align-self: flex-start;
  border: 0;
  background: none;
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.envs__error {
  margin: 0;
  color: var(--color-danger);
}

.envs__actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.envs__spacer {
  flex: 1;
}
</style>
