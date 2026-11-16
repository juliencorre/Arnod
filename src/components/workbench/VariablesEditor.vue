<script setup lang="ts">
/**
 * Onglet « Variables » d'une requête (maquette 4.2-b bis, docs/plan-mvp-e-tests-etapes.md, EX1 à
 * EX5) : chaque ligne est une extraction, action OpenCollection `set-variable` après la réponse.
 * Elle range une valeur de la réponse (`body.id`, `headers.Location`, `status`) ou de la requête
 * envoyée (`request.body.ref`) dans une variable que les étapes suivantes du test lisent. Montre
 * la dernière valeur extraite et les requêtes qui lisent ces variables. Aucun script.
 */
import { computed, ref, watch } from "vue";

import { useWorkbenchStore } from "@/stores/workbench";
import type { ExtractedValue, Extraction, VariableUser } from "@/types/ipc";

const props = defineProps<{
  projectId: string;
  path: string;
  /** Valeurs extraites au dernier envoi, ou `null` avant tout envoi. */
  extracted: ExtractedValue[] | null;
}>();
const extractions = defineModel<Extraction[]>({ required: true });

const workbench = useWorkbenchStore();
const users = ref<VariableUser[]>([]);

function add(): void {
  extractions.value = [...extractions.value, { name: "", source: "body.", enabled: true }];
}

function remove(index: number): void {
  extractions.value = extractions.value.filter((_, i) => i !== index);
}

function nameOf(extraction: Extraction, position: number): string {
  return extraction.name.trim() === "" ? `la ligne ${String(position + 1)}` : extraction.name;
}

/** Dernière valeur extraite pour une ligne, ou le message de la source absente. */
function lastOf(extraction: Extraction): { text: string; missing: boolean } | null {
  const found = props.extracted?.find((e) => e.name === extraction.name.trim());
  if (!found) return null;
  return found.value === null
    ? { text: "absente", missing: true }
    : { text: found.value, missing: false };
}

/** Noms des variables, pour chercher les requêtes qui les lisent (EX5). */
const names = computed(() =>
  extractions.value.map((e) => e.name.trim()).filter((name) => name !== ""),
);

watch(
  () => [props.path, names.value.join("\u0000")] as const,
  async () => {
    users.value =
      names.value.length === 0
        ? []
        : await workbench.variableUsers(props.projectId, props.path, names.value).catch(() => []);
  },
  { immediate: true },
);
</script>

<template>
  <div class="vars stack">
    <div class="vars__table" role="group" aria-label="Variables extraites">
      <div v-if="extractions.length > 0" class="vars__head" aria-hidden="true">
        <span />
        <span>Variable</span>
        <span>Vient de</span>
        <span>Dernière valeur</span>
        <span>Portée</span>
        <span />
      </div>
      <div
        v-for="(extraction, index) in extractions"
        :key="index"
        class="vars__row"
        :class="{ 'vars__row--off': !extraction.enabled }"
      >
        <input
          v-model="extraction.enabled"
          type="checkbox"
          :aria-label="`${extraction.enabled ? 'Désactiver' : 'Activer'} ${nameOf(extraction, index)}`"
        />
        <input
          v-model="extraction.name"
          class="vars__input vars__name"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="nom.de.variable"
          :aria-label="`Nom de la variable, ligne ${String(index + 1)}`"
        />
        <input
          v-model="extraction.source"
          class="vars__input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="body.id"
          :aria-label="`Vient de, ligne ${String(index + 1)}`"
        />
        <span
          class="vars__value"
          :class="{ 'vars__value--missing': lastOf(extraction)?.missing }"
          :title="lastOf(extraction)?.text"
          >{{ lastOf(extraction)?.text ?? "—" }}</span
        >
        <span class="vars__scope">le test</span>
        <button
          type="button"
          class="vars__remove"
          :aria-label="`Supprimer ${nameOf(extraction, index)}`"
          @click="remove(index)"
        >
          ✕
        </button>
      </div>
    </div>
    <p class="vars__add">
      <button type="button" class="vars__button" @click="add">+ Variable</button>
      <span class="hint"
        >ou «&nbsp;Garder&nbsp;» une valeur tirée dans «&nbsp;Envoyé&nbsp;» ;
        <code>body.chemin</code>, <code>headers.Nom</code>, <code>status</code></span
      >
    </p>

    <template v-if="users.length > 0">
      <h3 class="section">Utilisée par · {{ users.length }}</h3>
      <ul class="vars__users">
        <li v-for="user in users" :key="user.path">
          <button
            type="button"
            class="vars__user"
            @click="workbench.openRequest(projectId, user.path)"
          >
            <span class="vars__method" :class="`vars__method--${user.method.toLowerCase()}`">{{
              user.method
            }}</span>
            <span class="vars__user-name">{{ user.name }}</span>
            <code class="vars__user-url">{{ user.url }}</code>
          </button>
        </li>
      </ul>
    </template>

    <p class="vars__note" role="note">
      La variable est remplie à chaque exécution de cette requête ; les étapes suivantes du test la
      reçoivent. Aucun script à écrire.
    </p>
  </div>
</template>

<style scoped>
.stack {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.vars__head,
.vars__row {
  display: grid;
  grid-template-columns: auto minmax(0, 1.1fr) minmax(0, 1fr) minmax(0, 1fr) 4.5rem auto;
  gap: 0.25rem 0.5rem;
  align-items: center;
}

.vars__head {
  padding: 0 0.25rem;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.vars__row {
  padding: 0.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.vars__table {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.vars__row--off :is(.vars__input, .vars__value) {
  color: var(--color-muted);
}

.vars__input {
  min-width: 0;
  box-sizing: border-box;
  min-height: var(--control-sm);
  padding: 0.1875rem 0.5rem;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font: var(--text-body) var(--font-mono);
  color: var(--color-text);
  background: none;
}

.vars__input:hover {
  border-color: var(--color-border);
}

.vars__name {
  font-weight: 600;
  color: var(--color-link);
}

.vars__value {
  overflow: hidden;
  font: var(--text-body) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-success);
}

.vars__value--missing {
  color: var(--color-danger);
}

.vars__scope {
  font-size: var(--text-body);
  color: var(--color-muted);
}

.vars__remove {
  border: 0;
  background: none;
  font-size: var(--text-body);
  line-height: 1;
  color: var(--color-muted);
  cursor: pointer;
}

.vars__add {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  margin: 0;
}

.vars__button {
  min-height: var(--control-sm);
  padding: 0 0.625rem;
  border: 1px dashed var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  cursor: pointer;
}

.hint {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.section {
  margin: 0.5rem 0 0;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.vars__users {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.vars__user {
  display: grid;
  grid-template-columns: 3.5rem minmax(0, 1fr) minmax(0, 1.4fr);
  gap: 0.75rem;
  align-items: center;
  width: 100%;
  min-height: var(--control-md);
  padding: 0 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  font: inherit;
  font-size: var(--text-body);
  text-align: left;
  color: var(--color-text);
  cursor: pointer;
}

.vars__method {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.vars__method--get {
  color: var(--color-success);
}

.vars__method--post,
.vars__method--put,
.vars__method--patch {
  color: var(--color-link);
}

.vars__method--delete {
  color: var(--color-danger);
}

.vars__user-name,
.vars__user-url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vars__user-url {
  font: var(--text-small) var(--font-mono);
  color: var(--color-muted);
}

.vars__note {
  margin: 0.75rem 0 0;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-lg);
  font-size: var(--text-body);
  color: var(--color-muted);
  background: var(--color-sidebar);
}

:is(.vars__input, .vars__remove, .vars__button, .vars__user):focus-visible,
.vars__row input[type="checkbox"]:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}
</style>
