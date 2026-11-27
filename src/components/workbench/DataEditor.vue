<script setup lang="ts">
/**
 * Onglet « Données » d'une requête (4.7-a, 4.7-a bis) : un tableau, une colonne par variable, une
 * ligne par cas ; ou un fichier CSV du projet, en lecture seule, relu à chaque envoi. À droite, le
 * cas choisi : sa requête résolue et ses valeurs. Le tableau s'enregistre seul (`data_save`) ;
 * coller des cellules d'un tableur ajoute des lignes (JD2).
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import RowRemoveButton from "@/components/ui/RowRemoveButton.vue";
import SaveState from "@/components/ui/SaveState.vue";
import { useAutosave } from "@/composables/useAutosave";
import { useGuard } from "@/composables/useGuard";
import { useDatasetsStore } from "@/stores/datasets";
import { useWorkbenchStore } from "@/stores/workbench";
import type { DataTable, TreeNode } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const datasets = useDatasetsStore();
const guard = useGuard();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const set = computed(() => datasets.sets[props.editorKey] ?? null);
const editable = computed(() => set.value?.link.editable ?? true);
/** Tableau en cours d'édition ; `null` sans jeu de données. */
const table = ref<DataTable | null>(null);

/** Tableau vide : la référence quand la requête n'a pas encore de jeu de données. */
const EMPTY: DataTable = { columns: [], rows: [] };

const autosave = useAutosave(
  () => (editable.value ? (table.value ?? EMPTY) : null),
  async () => {
    const current = editor.value;
    if (!current || !table.value || table.value.columns.length === 0) return;
    const saved = await datasets.save(
      props.editorKey,
      current.projectId,
      current.path,
      table.value,
    );
    // Le cœur a lié le tableau à la requête (étiquette `data:`) : le brouillon le garde.
    if (current.draft) current.draft.data = saved.link;
  },
);

async function load(): Promise<void> {
  const current = editor.value;
  if (!current) return;
  const loaded = await datasets.load(props.editorKey, current.projectId, current.path);
  table.value = loaded
    ? { columns: [...loaded.columns], rows: loaded.rows.map((r) => [...r]) }
    : null;
  autosave.reset();
}

watch(
  () => [editor.value?.path, editor.value?.draft?.data?.file] as const,
  () => void load().catch(() => undefined),
  { immediate: true },
);

const caseCount = computed(
  () => table.value?.rows.filter((row) => row.some((cell) => cell.trim() !== "")).length ?? 0,
);
const current = computed(() =>
  Math.min(Math.max(datasets.selected[props.editorKey] ?? 1, 1), Math.max(caseCount.value, 1)),
);
const preview = computed(() => datasets.previews[props.editorKey] ?? null);
const outcome = computed(() => datasets.outcomes[props.editorKey] ?? null);

function caseStatus(number: number): "ok" | "ko" | null {
  const found = outcome.value?.cases.find((c) => c.number === number);
  if (!found) return null;
  return found.outcome?.passed === true ? "ok" : "ko";
}

async function refreshPreview(): Promise<void> {
  const current = editor.value;
  const environment = current ? workbench.activeEnvironment(current.projectId) : null;
  if (!current?.draft || environment === null || caseCount.value === 0) return;
  await datasets.preview(
    props.editorKey,
    current.projectId,
    current.path,
    current.draft,
    environment,
  );
}

watch(
  () => [current.value, set.value?.link.file, autosave.state.value] as const,
  ([, , state]) => {
    if (state === "saved") void refreshPreview().catch(() => undefined);
  },
  { immediate: true },
);

function selectCase(number: number): void {
  datasets.select(props.editorKey, number);
  const shown = outcome.value?.cases.find((c) => c.number === number);
  const current = editor.value;
  if (current && shown?.outcome) current.outcome = shown.outcome;
}

/** Commence un tableau : les variables de l'adresse deviennent les colonnes. */
function start(): void {
  const url = editor.value?.draft?.url ?? "";
  const names = [...url.matchAll(/\{\{\s*([^}$\s]+)\s*\}\}/g)]
    .map((m) => m[1] ?? "")
    .filter((name) => name !== "" && name !== "baseUrl");
  const columns = names.length > 0 ? [...new Set(names)] : ["valeur"];
  table.value = { columns, rows: [columns.map(() => "")] };
}

function addColumn(): void {
  if (!table.value) return;
  table.value.columns.push(`colonne${String(table.value.columns.length + 1)}`);
  for (const row of table.value.rows) row.push("");
}

function removeColumn(index: number): void {
  if (!table.value || table.value.columns.length <= 1) return;
  table.value.columns.splice(index, 1);
  for (const row of table.value.rows) row.splice(index, 1);
}

function addRow(): void {
  table.value?.rows.push(table.value.columns.map(() => ""));
}

function removeRow(index: number): void {
  table.value?.rows.splice(index, 1);
}

/** Coller des cellules d'un tableur (tabulations) ajoute des lignes (JD2). */
async function onPaste(event: ClipboardEvent): Promise<void> {
  const text = event.clipboardData?.getData("text/plain") ?? "";
  if (!table.value || !text.includes("\t")) return;
  event.preventDefault();
  const rows = await datasets.parse(text);
  const width = table.value.columns.length;
  for (const row of rows) {
    const cells = row.slice(0, width);
    while (cells.length < width) cells.push("");
    table.value.rows.push(cells);
  }
}

async function importFile(): Promise<void> {
  const current = editor.value;
  if (!current) return;
  if (await datasets.importFile(props.editorKey, current.projectId, current.path)) {
    const link = datasets.sets[props.editorKey]?.link;
    if (current.draft && link) current.draft.data = link;
    await load();
  }
}

async function removeSet(): Promise<void> {
  const current = editor.value;
  if (!current) return;
  await datasets.remove(props.editorKey, current.projectId, current.path);
  if (current.draft) delete current.draft.data;
  table.value = null;
  autosave.reset();
}

async function openFile(): Promise<void> {
  const current = editor.value;
  if (current) await datasets.open(current.projectId, current.path);
}

const isStep = computed(() => {
  const current = editor.value;
  if (!current) return false;
  const parent = current.path.split("/").slice(0, -1).join("/");
  const nodes = workbench.trees[current.projectId]?.nodes ?? [];
  const find = (list: TreeNode[]): boolean =>
    list.some((n) => n.kind === "folder" && (n.path === parent ? n.test : find(n.children)));
  return parent !== "" && find(nodes);
});
</script>

<template>
  <div class="data" @paste="guard(() => onPaste($event))">
    <p v-if="isStep" class="data__note">
      Cette requête est une étape d'un test : un jeu de données ne sert que pour une requête jouée
      seule. Pour répéter des étapes, ajoutez une boucle au test.
    </p>
    <template v-if="table">
      <div class="data__bar">
        <span class="data__label">Source</span>
        <span class="data__source" :class="{ 'data__source--on': editable }">Tableau</span>
        <span class="data__source" :class="{ 'data__source--on': !editable }">Fichier CSV</span>
        <span class="data__spacer" />
        <SaveState
          v-if="editable"
          :state="autosave.state.value"
          :error="autosave.error.value"
          @retry="autosave.flush()"
        />
        <BaseButton v-if="editable" @click="addRow">+ Cas</BaseButton>
        <BaseButton v-if="editable" variant="link" @click="addColumn">+ Colonne</BaseButton>
        <BaseButton variant="link" @click="guard(removeSet)">Retirer le jeu de données</BaseButton>
      </div>
      <p v-if="!editable && set" class="data__file">
        <span class="data__badge">CSV</span> <b>{{ set.link.file }}</b> · {{ caseCount }} lignes
        <BaseButton variant="link" @click="guard(openFile)">Ouvrir</BaseButton>
      </p>
      <p v-if="set?.error" class="data__error" role="alert">{{ set.error }}</p>
      <div class="data__grid">
        <table class="data__table" aria-label="Jeu de données">
          <thead>
            <tr>
              <th scope="col">#</th>
              <th v-for="(column, c) in table.columns" :key="c" scope="col">
                <span class="data__head">
                  <input
                    v-if="editable"
                    v-model="table.columns[c]"
                    class="data__input data__input--head"
                    :aria-label="`Nom de la colonne ${c + 1}`"
                  />
                  <span v-else>{{ column }}</span>
                  <RowRemoveButton
                    v-if="editable && table.columns.length > 1"
                    :label="`Retirer la colonne ${column}`"
                    @click="removeColumn(c)"
                  />
                </span>
              </th>
              <th v-if="editable" scope="col"><span class="visually-hidden">Retirer</span></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(row, r) in table.rows"
              :key="r"
              :class="{ 'data__row--current': current === r + 1 }"
            >
              <th scope="row">
                <button
                  type="button"
                  class="data__case"
                  :aria-label="`Choisir le cas ${r + 1}`"
                  :aria-pressed="current === r + 1"
                  @click="selectCase(r + 1)"
                >
                  <span v-if="caseStatus(r + 1)" :class="`data__mark--${caseStatus(r + 1)}`">{{
                    caseStatus(r + 1) === "ok" ? "✓" : "✕"
                  }}</span>
                  {{ r + 1 }}
                </button>
              </th>
              <td v-for="(_, c) in table.columns" :key="c">
                <input
                  v-if="editable"
                  v-model="row[c]"
                  class="data__input"
                  :aria-label="`Cas ${r + 1}, ${table.columns[c]}`"
                />
                <span v-else>{{ row[c] }}</span>
              </td>
              <td v-if="editable">
                <RowRemoveButton :label="`Retirer le cas ${r + 1}`" @click="removeRow(r)" />
              </td>
            </tr>
          </tbody>
        </table>
        <aside class="data__case-view" aria-label="Cas choisi">
          <p class="data__nav">
            <b>Cas {{ current }}</b> sur {{ caseCount }}
            <button
              type="button"
              class="data__arrow"
              aria-label="Cas précédent"
              :disabled="current <= 1"
              @click="selectCase(current - 1)"
            >
              ‹
            </button>
            <button
              type="button"
              class="data__arrow"
              aria-label="Cas suivant"
              :disabled="current >= caseCount"
              @click="selectCase(current + 1)"
            >
              ›
            </button>
          </p>
          <template v-if="preview">
            <p class="data__label">Requête du cas</p>
            <code class="data__url">{{ preview.method }} {{ preview.url }}</code>
            <p class="data__label">Valeurs du cas</p>
            <p v-for="[name, value] in preview.values" :key="name" class="data__value">
              <code>{{ name }}</code> {{ value }}
            </p>
          </template>
          <p class="data__muted">
            {{
              outcome
                ? `${String(outcome.cases.filter((c) => c.outcome?.passed).length)} cas réussis sur ${String(outcome.cases.length)}.`
                : `Pas encore envoyé. « Envoyer » joue les ${String(caseCount)} cas.`
            }}
          </p>
        </aside>
      </div>
      <p class="data__muted">
        Chaque colonne est une variable, à écrire <code v-text="'{{nom}}'" /> dans l'adresse, le
        corps ou une assertion. Coller des cellules depuis un tableur ajoute des lignes.
      </p>
    </template>
    <div v-else class="data__empty">
      <p>
        Jouez cette requête une fois par cas : une colonne par variable, une ligne par cas. Le test
        compte une fois ; il échoue dès qu'un cas échoue.
      </p>
      <p class="data__actions">
        <BaseButton @click="start">+ Tableau</BaseButton>
        <BaseButton @click="guard(importFile)">Fichier CSV…</BaseButton>
      </p>
      <p class="data__muted">
        Un fichier choisi est rangé dans le projet (dossier données), partagé avec l'équipe et relu
        à chaque envoi.
      </p>
    </div>
  </div>
</template>

<style scoped>
.data {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.data__bar,
.data__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.data__spacer {
  flex: 1;
}

.data__label {
  margin: 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.data__source {
  padding: 0 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  color: var(--color-muted);
}

.data__source--on {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-accent-soft);
}

.data__file {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
}

.data__badge {
  padding: 0 0.375rem;
  border-radius: var(--radius-sm);
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-family-data-text);
  background: var(--color-family-data-bg);
}

.data__error {
  margin: 0;
  color: var(--color-danger);
}

.data__grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(14rem, 18rem);
  gap: 0.75rem;
}

.data__table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-small);
}

.data__table th,
.data__table td {
  padding: 0.125rem 0.25rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
}

.data__head {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.data__row--current {
  background: var(--color-accent-soft);
}

.data__input {
  width: 100%;
  min-height: var(--control-sm);
  padding: 0 0.375rem;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  font: var(--text-small) var(--font-mono);
  color: var(--color-text);
  background-color: transparent;
}

.data__input:hover,
.data__input:focus-visible {
  border-color: var(--color-border-control);
  background-color: var(--color-bg);
}

.data__input--head {
  font-weight: 600;
}

.data__case {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  min-width: 2rem;
  min-height: 1.5rem;
  padding: 0 0.25rem;
  border: 0;
  border-radius: var(--radius-sm);
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.data__case:focus-visible,
.data__arrow:focus-visible,
.data__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.data__mark--ok {
  color: var(--color-success);
}

.data__mark--ko {
  color: var(--color-danger);
}

.data__case-view {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.data__nav {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  margin: 0;
}

.data__arrow {
  min-width: 1.5rem;
  min-height: 1.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-sm);
  font: inherit;
  background-color: var(--color-surface);
  cursor: pointer;
}

.data__url {
  font: var(--text-small) var(--font-mono);
  overflow-wrap: anywhere;
}

.data__value {
  margin: 0;
  font-size: var(--text-small);
}

.data__muted,
.data__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.data__empty {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.data__empty p {
  margin: 0;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}
</style>
