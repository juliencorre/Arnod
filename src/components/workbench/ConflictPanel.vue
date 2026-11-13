<script setup lang="ts">
/**
 * Brouillon en conflit avec son fichier (lot I, écran 4.1-e) : la requête a changé sur le disque
 * pendant qu'elle était modifiée ici. Chaque élément qui diffère (calculé par le cœur) se règle
 * par « disque » ou « brouillon » ; « Appliquer et enregistrer » écrit le fichier une seule fois.
 * Rien n'est enregistré avant. Montre aussi le cas d'un fichier supprimé hors d'Arnod alors que
 * l'onglet avait un brouillon : « Restaurer » ou « Fermer l'onglet ».
 */
import { computed, useId } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useToastStore } from "@/stores/toasts";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ConflictItem, ConflictSide } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();
const emit = defineEmits<{ close: [] }>();

const workbench = useWorkbenchStore();
const toasts = useToastStore();
const ids = { title: useId() };
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const conflict = computed(() => editor.value?.conflict ?? null);
const different = computed(() => conflict.value?.items.filter((i) => !i.identical) ?? []);
const identical = computed(() => conflict.value?.items.filter((i) => i.identical) ?? []);
const ready = computed(() => workbench.conflictReady(props.editorKey));

/** Heure affichée (« 15:04 »). */
function time(value: Date | number | null | undefined): string {
  if (value === null || value === undefined) return "";
  return new Date(value).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
}

const diskTime = computed(() => time(conflict.value?.modifiedAt));
const draftTime = computed(() => time(editor.value?.draftSince));

function plural(n: number, one: string, many: string): string {
  return `${String(n)} ${n === 1 ? one : many}`;
}

function chosen(item: ConflictItem): ConflictSide | null {
  return editor.value?.choices[item.key] ?? null;
}

function choose(item: ConflictItem, side: ConflictSide): void {
  workbench.chooseConflict(props.editorKey, item.key, side);
}

async function apply(): Promise<void> {
  try {
    await workbench.resolveConflict(props.editorKey);
    if (!editor.value?.conflict) toasts.show("Conflit réglé : la requête est enregistrée.");
  } catch (caught) {
    toasts.showError(caught);
  }
}

async function restore(): Promise<void> {
  try {
    await workbench.restoreRemoved(props.editorKey);
  } catch (caught) {
    toasts.showError(caught);
  }
}
</script>

<template>
  <section v-if="editor" class="conflict" role="region" :aria-labelledby="ids.title">
    <div class="conflict__alert" role="alert">
      <span class="conflict__mark" aria-hidden="true">!</span>
      <div>
        <h2 :id="ids.title" class="conflict__title">
          {{
            conflict
              ? "Cette requête a changé sur le disque pendant que vous la modifiiez"
              : "Cette requête a été supprimée hors d'Arnod"
          }}
        </h2>
        <p v-if="conflict" class="conflict__text">
          Le fichier <code>{{ editor.path }}</code> a été modifié hors d'Arnod
          <template v-if="diskTime">à {{ diskTime }}</template> (un git pull, par exemple). Votre
          brouillon<template v-if="draftTime">, commencé à {{ draftTime }},</template> n'est pas
          encore enregistré : rien n'est écrasé tant que vous n'avez pas choisi.
        </p>
        <p v-else class="conflict__text">
          Le fichier <code>{{ editor.path }}</code> n'existe plus (un git pull, par exemple). Votre
          brouillon est gardé : restaurez-le pour réécrire le fichier, ou fermez l'onglet.
        </p>
      </div>
    </div>

    <template v-if="conflict">
      <p class="conflict__count">
        <strong>{{ plural(different.length, "différence", "différences") }}</strong>
        <span v-if="identical.length > 0">
          {{ plural(identical.length, "élément identique", "éléments identiques") }}
        </span>
      </p>
      <table class="conflict__table">
        <thead>
          <tr>
            <th scope="col">Élément</th>
            <th scope="col">
              Sur le disque<template v-if="diskTime"> · {{ diskTime }}</template>
            </th>
            <th scope="col">
              Votre brouillon<template v-if="draftTime"> · {{ draftTime }}</template>
            </th>
            <th scope="col">Garder</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in different" :key="item.key" class="conflict__row">
            <th scope="row" class="conflict__label">{{ item.label }}</th>
            <td class="conflict__value">
              <template v-if="item.disk !== null">
                <code>{{ item.disk }}</code>
                <span v-if="item.draft === null" class="conflict__added">ajouté</span>
              </template>
              <span v-else class="conflict__absent">retiré</span>
            </td>
            <td class="conflict__value">
              <code v-if="item.draft !== null">{{ item.draft }}</code>
              <span v-else class="conflict__absent">absent</span>
            </td>
            <td>
              <div
                class="conflict__choice"
                role="radiogroup"
                :aria-label="`Version gardée : ${item.label}`"
              >
                <label class="conflict__option">
                  <input
                    type="radio"
                    :name="`${editorKey}-${item.key}`"
                    value="disk"
                    :checked="chosen(item) === 'disk'"
                    @change="choose(item, 'disk')"
                  />
                  disque
                </label>
                <label class="conflict__option">
                  <input
                    type="radio"
                    :name="`${editorKey}-${item.key}`"
                    value="draft"
                    :checked="chosen(item) === 'draft'"
                    @change="choose(item, 'draft')"
                  />
                  brouillon
                </label>
              </div>
            </td>
          </tr>
          <tr v-for="item in identical" :key="item.key" class="conflict__row conflict__row--same">
            <th scope="row" class="conflict__label">{{ item.label }}</th>
            <td class="conflict__value">
              <code v-if="item.disk !== null">{{ item.disk }}</code>
              <span v-else class="conflict__absent">absent</span>
            </td>
            <td class="conflict__value">
              <code v-if="item.draft !== null">{{ item.draft }}</code>
              <span v-else class="conflict__absent">absent</span>
            </td>
            <td class="conflict__same">identique</td>
          </tr>
        </tbody>
      </table>
      <div class="conflict__actions">
        <BaseButton variant="primary" :disabled="!ready" @click="apply">
          Appliquer et enregistrer
        </BaseButton>
        <BaseButton @click="workbench.chooseConflict(editorKey, null, 'disk')">
          Tout prendre du disque
        </BaseButton>
        <BaseButton @click="workbench.chooseConflict(editorKey, null, 'draft')">
          Tout garder de mon brouillon
        </BaseButton>
      </div>
      <p class="conflict__note">
        <template v-if="!ready">Choisissez une version pour chaque différence. </template>Le choix
        se fait par élément ; « Appliquer » écrit le fichier une seule fois. Les autres requêtes
        ouvertes ne sont pas touchées.
      </p>
    </template>
    <div v-else class="conflict__actions">
      <BaseButton variant="primary" @click="restore">Restaurer</BaseButton>
      <BaseButton @click="emit('close')">Fermer l'onglet</BaseButton>
    </div>
  </section>
</template>

<style scoped>
.conflict {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.875rem;
  max-width: 54rem;
  min-width: 0;
}

.conflict__alert {
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-lg);
  background: var(--color-danger-soft);
}

.conflict__mark {
  display: grid;
  flex: none;
  place-items: center;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: var(--radius-pill);
  font-weight: 600;
  color: var(--color-surface);
  background: var(--color-danger);
}

.conflict__title {
  margin: 0 0 0.25rem;
  font-size: var(--text-body-large);
  font-weight: 600;
}

.conflict__text {
  margin: 0;
  line-height: 1.5;
  color: var(--color-muted);
}

.conflict__text code,
.conflict__value code {
  font-family: var(--font-mono);
  font-size: var(--text-small);
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.conflict__count {
  display: flex;
  gap: 0.625rem;
  align-items: baseline;
  margin: 0;
  color: var(--color-muted);
}

.conflict__count strong {
  font-size: var(--text-body-large);
  color: var(--color-text);
}

.conflict__table {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  border-spacing: 0;
  overflow: hidden;
  background-color: var(--color-surface);
}

.conflict__table thead th {
  padding: 0.5rem 0.875rem;
  border-bottom: 1px solid var(--color-border);
  font: 600 var(--text-caption) var(--font-mono);
  text-align: left;
  text-transform: uppercase;
  color: var(--color-muted);
  background-color: var(--color-bg);
}

.conflict__table td,
.conflict__label {
  padding: 0.5rem 0.875rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
  vertical-align: middle;
}

.conflict__row:last-child td,
.conflict__row:last-child .conflict__label {
  border-bottom: none;
}

.conflict__label {
  font-weight: 400;
}

.conflict__value {
  max-width: 14rem;
}

.conflict__absent {
  font-style: italic;
  color: var(--color-muted);
}

.conflict__added {
  display: block;
  font-size: var(--text-small);
  color: var(--color-success);
}

.conflict__row--same {
  color: var(--color-muted);
}

.conflict__same {
  color: var(--color-muted);
}

.conflict__choice {
  display: flex;
  gap: 1.25rem;
  white-space: nowrap;
}

.conflict__option {
  display: inline-flex;
  gap: 0.375rem;
  align-items: center;
  cursor: pointer;
}

.conflict__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.625rem;
}

.conflict__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
