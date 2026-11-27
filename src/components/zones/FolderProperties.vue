<script setup lang="ts">
/**
 * Propriétés d'un dossier (4.8-a) : définies ici, héritées des dossiers parents ou de
 * l'environnement, avec leur origine. Les tests du dossier et de ses sous-dossiers les lisent
 * avec `{{nom}}` ; le plus proche l'emporte (R15). Une propriété définie ici se modifie ou se
 * retire ; une héritée se redéfinit. Le cœur écrit `request.variables` du `folder.yml`.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { useFlowsStore } from "@/stores/flows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { FolderProperty } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const flows = useFlowsStore();
const workbench = useWorkbenchStore();
const guard = useGuard();
const list = computed(() => flows.properties[flows.folderKey(props.projectId, props.path)] ?? null);
const environment = computed(() => workbench.activeEnvironment(props.projectId));

watch(
  () => [props.projectId, props.path, environment.value] as const,
  () => void flows.loadProperties(props.projectId, props.path).catch(() => undefined),
  { immediate: true },
);

/** Propriétés définies ici, telles que l'utilisateur les modifie. */
const here = ref<{ name: string; value: string }[]>([]);
watch(
  list,
  (found) => {
    here.value = (found ?? [])
      .filter((p) => p.origin.kind === "here")
      .map((p) => ({ name: p.name, value: p.value }));
  },
  { immediate: true },
);

function origin(property: FolderProperty): string {
  switch (property.origin.kind) {
    case "here":
      return "défini ici";
    case "folder":
      return `hérité de ${property.origin.folder.split("/").at(-1) ?? property.origin.folder}`;
    case "project":
      return "variable du projet";
    case "environment":
      return `environnement ${property.origin.name}`;
  }
}

function save(): void {
  const clean = here.value.filter((p) => p.name.trim() !== "");
  guard(() => flows.saveProperties(props.projectId, props.path, clean));
}

function redefine(property: FolderProperty): void {
  here.value.push({ name: property.name, value: property.value });
  save();
}

function remove(name: string): void {
  here.value = here.value.filter((p) => p.name !== name);
  save();
}

/** Nouvelle propriété, nommée sur place. */
const adding = ref(false);
const newName = ref("");
const newValue = ref("");

function add(): void {
  const name = newName.value.trim();
  if (name === "") return;
  here.value.push({ name, value: newValue.value });
  newName.value = "";
  newValue.value = "";
  adding.value = false;
  save();
}

const shown = computed(() => {
  const found = list.value ?? [];
  const defined = new Set(here.value.map((p) => p.name));
  return found.filter((p) => p.origin.kind !== "here" && !defined.has(p.name));
});
</script>

<template>
  <div class="props">
    <table class="props__table" aria-label="Propriétés du dossier">
      <thead>
        <tr>
          <th scope="col">Nom</th>
          <th scope="col">Valeur</th>
          <th scope="col">Origine</th>
          <th scope="col"><span class="visually-hidden">Action</span></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="property in here" :key="`here-${property.name}`">
          <td>
            <code>{{ property.name }}</code>
          </td>
          <td>
            <input
              v-model="property.value"
              class="props__input"
              :aria-label="`Valeur de ${property.name}`"
              @change="save"
            />
          </td>
          <td class="props__origin">défini ici</td>
          <td>
            <BaseButton variant="link" @click="remove(property.name)">Retirer</BaseButton>
          </td>
        </tr>
        <tr v-for="property in shown" :key="`inherited-${property.name}`" class="props__inherited">
          <td>
            <code>{{ property.name }}</code>
          </td>
          <td>
            <code>{{ property.value }}</code>
          </td>
          <td class="props__origin">{{ origin(property) }}</td>
          <td>
            <BaseButton
              v-if="property.origin.kind !== 'environment'"
              variant="link"
              @click="redefine(property)"
              >Redéfinir</BaseButton
            >
          </td>
        </tr>
        <tr v-if="adding">
          <td>
            <input
              v-model="newName"
              class="props__input"
              aria-label="Nom de la propriété"
              @keydown.enter="add"
            />
          </td>
          <td>
            <input
              v-model="newValue"
              class="props__input"
              aria-label="Valeur de la propriété"
              @keydown.enter="add"
            />
          </td>
          <td class="props__origin">défini ici</td>
          <td><BaseButton variant="link" @click="add">Ajouter</BaseButton></td>
        </tr>
      </tbody>
    </table>
    <BaseButton v-if="!adding" @click="adding = true">+ Propriété</BaseButton>
    <p class="props__note">
      Les tests de ce dossier et de tous ses sous-dossiers utilisent ces valeurs avec
      <code v-text="'{{nom}}'" />. Un sous-dossier hérite des propriétés de ses parents et peut les
      redéfinir. Ordre de résolution : dossier le plus proche → dossiers parents → environnement.
    </p>
  </div>
</template>

<style scoped>
.props {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
}

.props__table {
  width: 100%;
  border-collapse: collapse;
}

.props__table th,
.props__table td {
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
}

.props__table th {
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.props__origin,
.props__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.props__note {
  margin: 0;
}

.props__input {
  width: 100%;
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: var(--text-small) var(--font-mono);
  color: var(--color-text);
  background-color: var(--color-bg);
}

.props__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}
</style>
