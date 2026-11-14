<script setup lang="ts">
/**
 * « Copier ou déplacer » une requête (lot J, écran 4.1-g) : destinations cherchées par leur nom
 * (opérations des contrats, dossiers et tests du référentiel, autres projets), place dans un test,
 * et « Ce qui change » calculé par le cœur pour la destination choisie. Le dialogue est unique
 * par fenêtre (store `transfer`), ouvert par « Copier ou déplacer… » du menu d'une requête.
 *
 * Motif « combobox » : le champ de recherche garde le focus, ↑ ↓ choisissent la destination
 * (`aria-activedescendant`) ; Entrée sur un autre projet l'ouvre.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import { useTransferStore } from "@/stores/transfer";
import type { TransferChange, TransferMode, TransferTarget, TransferTargets } from "@/types/ipc";

const transfer = useTransferStore();
const catalog = useCatalogStore();
const ids = { search: useId(), list: useId(), changes: useId(), position: useId() };

const source = computed(() => transfer.dialog);
const open = computed(() => source.value !== null);
/** Projet de destination (le projet de la requête, ou un autre choisi dans la liste). */
const target = ref("");
const targets = ref<TransferTargets | null>(null);
const query = ref("");
const selected = ref<string | null>(null);
/** Place choisie dans un test, par test (`steps.length` : à la fin). */
const positions = ref<Record<string, number>>({});
const changes = ref<TransferChange[]>([]);
const error = ref<string | null>(null);
const busy = ref(false);

interface Row {
  id: string;
  label: string;
  note?: string;
  current?: boolean;
  destination?: TransferTarget;
  /** Autre projet : l'ouvre au lieu de le choisir. */
  project?: string;
}

interface Group {
  title: string;
  rows: Row[];
}

const sameProject = computed(() => target.value === source.value?.projectId);
const targetName = computed(() => catalog.projectById(target.value)?.name ?? "");

const subtitle = computed(() => {
  const t = targets.value;
  const method = t?.method ?? "";
  if (!sameProject.value || !t) return method;
  const op = t.operations.find((o) => o.current);
  if (op) return `${method} · sous ${op.operation}, ${op.contractName}`;
  const folder = t.folders.find((f) => f.current);
  if (folder) return `${method} · ${folder.path === "" ? "à la racine" : `dans ${folder.path}`}`;
  const test = t.tests.find((x) => x.current !== null);
  if (test) return `${method} · étape ${String((test.current ?? 0) + 1)} de ${test.name}`;
  return method;
});

function fold(text: string): string {
  return text.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

const groups = computed<Group[]>(() => {
  const t = targets.value;
  if (!t) return [];
  const q = fold(query.value.trim());
  const keep = (row: Row): boolean => q === "" || fold(row.label).includes(q);
  const out: Group[] = [
    {
      title: "Contrats · endpoint",
      rows: t.operations.map((o) => ({
        id: `op:${o.contract}#${o.operation}`,
        label: `${o.contractName} › ${o.operation}`,
        current: o.current,
        destination: { kind: "operation", contract: o.contract, operation: o.operation },
      })),
    },
    {
      title: "Requêtes libres",
      rows: t.folders.map((f) => ({
        id: `folder:${f.path}`,
        label: f.path === "" ? `Racine de ${targetName.value}` : f.path.replaceAll("/", " / "),
        current: f.current,
        destination: { kind: "folder", folder: f.path },
      })),
    },
    {
      title: "Tests · étape",
      rows: t.tests.map((x) => ({
        id: `test:${x.path}`,
        label: x.path.replaceAll("/", " / "),
        destination: { kind: "step", test: x.path, position: positionOf(x.path) },
      })),
    },
  ];
  if (sameProject.value) {
    out.push({
      title: "Autres projets",
      rows: catalog.projects
        .filter((p) => p.id !== target.value && !catalog.isInbox(p.id))
        .map((p) => ({ id: `project:${p.id}`, label: `${p.name} ›`, project: p.id })),
    });
  }
  return out.map((g) => ({ ...g, rows: g.rows.filter(keep) })).filter((g) => g.rows.length > 0);
});

const rows = computed(() => groups.value.flatMap((g) => g.rows));
const choice = computed(() => rows.value.find((r) => r.id === selected.value) ?? null);
const chosenTest = computed(() => {
  const c = choice.value;
  if (c?.destination?.kind !== "step") return null;
  const path = c.destination.test;
  return targets.value?.tests.find((x) => x.path === path) ?? null;
});

function positionOf(test: string): number {
  const saved = positions.value[test];
  if (saved !== undefined) return saved;
  return targets.value?.tests.find((x) => x.path === test)?.steps.length ?? 0;
}

/** Libellé de la place dans le test (« après l'étape 1 »). */
function placeLabel(position: number): string {
  return position === 0 ? "en tête" : `après l'étape ${String(position)}`;
}

/** Déplacer vers l'endroit où la requête est déjà : rien ne changerait. */
const stays = computed(() => {
  const c = choice.value;
  if (!c?.destination || !sameProject.value) return false;
  if (c.current === true) return true;
  const test = chosenTest.value;
  if (test?.current == null) return false;
  const p = positionOf(test.path);
  return p === test.current || p === test.current + 1;
});

async function loadTargets(): Promise<void> {
  const s = source.value;
  if (!s || target.value === "") return;
  error.value = null;
  try {
    targets.value = await ipc.transferTargets(s.projectId, s.path, target.value);
  } catch (caught) {
    targets.value = null;
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
  // La première destination qui n'est pas la place actuelle.
  selected.value = rows.value.find((r) => r.destination && r.current !== true)?.id ?? null;
}

watch(
  source,
  (s) => {
    if (!s) return;
    target.value = s.projectId;
    query.value = "";
    positions.value = {};
    changes.value = [];
    void loadTargets();
  },
  { immediate: true },
);

watch(query, () => {
  if (!rows.value.some((r) => r.id === selected.value)) selected.value = rows.value[0]?.id ?? null;
});

/** Mode montré dans « Ce qui change » : déplacer, sauf si la requête resterait en place. */
const previewMode = computed<TransferMode>(() => (stays.value ? "copy" : "move"));

let previewSeq = 0;
watch(
  [choice, previewMode, () => (chosenTest.value ? positionOf(chosenTest.value.path) : -1)],
  async () => {
    const s = source.value;
    const destination = choice.value?.destination;
    const seq = ++previewSeq;
    if (!s || !destination) {
      changes.value = [];
      return;
    }
    try {
      const next = await ipc.transferPreview(
        s.projectId,
        s.path,
        target.value,
        destination,
        previewMode.value,
      );
      if (seq === previewSeq) changes.value = next;
    } catch (caught) {
      if (seq === previewSeq) {
        changes.value = [];
        error.value = caught instanceof Error ? caught.message : String(caught);
      }
    }
  },
);

function enterProject(id: string): void {
  target.value = id;
  query.value = "";
  void loadTargets();
}

function backToSource(): void {
  const s = source.value;
  if (s) enterProject(s.projectId);
}

function choose(row: Row): void {
  if (row.project !== undefined) enterProject(row.project);
  else selected.value = row.id;
}

function onKeydown(event: KeyboardEvent): void {
  const list = rows.value;
  if (list.length === 0) return;
  const index = list.findIndex((r) => r.id === selected.value);
  if (event.key === "ArrowDown") {
    selected.value = list[(index + 1) % list.length]?.id ?? null;
  } else if (event.key === "ArrowUp") {
    selected.value = list[(index - 1 + list.length) % list.length]?.id ?? null;
  } else if (event.key === "Enter") {
    const row = list[index];
    if (row?.project !== undefined) enterProject(row.project);
  } else return;
  event.preventDefault();
  document.getElementById(optionId(selected.value ?? ""))?.scrollIntoView({ block: "nearest" });
}

function optionId(id: string): string {
  return `${ids.list}-${String(rows.value.findIndex((r) => r.id === id))}`;
}

function setPosition(event: Event): void {
  const test = chosenTest.value;
  if (!test) return;
  positions.value[test.path] = Number((event.target as HTMLSelectElement).value);
}

async function submit(mode: TransferMode): Promise<void> {
  const s = source.value;
  const destination = choice.value?.destination;
  if (!s || !destination || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    await transfer.apply(s, target.value, destination, mode);
    transfer.closeDialog();
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog
    :open="open"
    wide
    :title="`Copier ou déplacer « ${source?.name ?? ''} »`"
    :description="subtitle"
    @close="transfer.closeDialog()"
  >
    <div class="copy-move">
      <div class="copy-move__pick">
        <button v-if="!sameProject" type="button" class="copy-move__back" @click="backToSource">
          ‹ {{ targetName }}
        </button>
        <input
          :id="ids.search"
          v-model="query"
          type="search"
          class="copy-move__search"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded="true"
          :aria-controls="ids.list"
          :aria-activedescendant="selected ? optionId(selected) : undefined"
          aria-label="Chercher une destination"
          placeholder="⌕ Chercher une destination"
          autocomplete="off"
          @keydown="onKeydown"
        />
        <div :id="ids.list" role="listbox" class="copy-move__list" aria-label="Destinations">
          <div v-for="group in groups" :key="group.title" role="group" :aria-label="group.title">
            <p class="copy-move__group" aria-hidden="true">{{ group.title }}</p>
            <div
              v-for="row in group.rows"
              :id="optionId(row.id)"
              :key="row.id"
              role="option"
              class="copy-move__option"
              :class="{ 'copy-move__option--active': row.id === selected }"
              :aria-selected="row.id === selected"
              @mousedown.prevent
              @click="choose(row)"
            >
              <span class="copy-move__label">{{ row.label }}</span>
              <span v-if="row.current" class="copy-move__note">actuel</span>
              <span
                v-else-if="row.destination?.kind === 'step' && row.id === selected"
                class="copy-move__note copy-move__note--accent"
              >
                {{ placeLabel(row.destination.position) }}
              </span>
            </div>
          </div>
          <p v-if="groups.length === 0" class="copy-move__none">
            Aucune destination ne correspond.
          </p>
        </div>
        <div v-if="chosenTest" class="copy-move__place">
          <label :for="ids.position">Place</label>
          <select :id="ids.position" :value="positionOf(chosenTest.path)" @change="setPosition">
            <option v-for="p in chosenTest.steps.length + 1" :key="p" :value="p - 1">
              {{ placeLabel(p - 1) }}{{ p > 1 ? ` (${chosenTest.steps[p - 2]})` : "" }}
            </option>
          </select>
        </div>
      </div>
      <section :id="ids.changes" class="copy-move__changes" aria-label="Ce qui change">
        <p class="copy-move__heading">Ce qui change</p>
        <p v-if="stays" class="copy-move__stays">
          Elle y est déjà : seule une copie est possible ici.
        </p>
        <dl class="copy-move__items">
          <div
            v-for="change in changes"
            :key="change.title"
            class="copy-move__item"
            :class="{ 'copy-move__item--warning': change.warning }"
          >
            <dt>{{ change.title }}</dt>
            <dd>{{ change.detail }}</dd>
          </div>
        </dl>
      </section>
    </div>
    <p v-if="error" class="copy-move__error" role="alert">{{ error }}</p>
    <template #footer>
      <p class="copy-move__hint">
        Même chose dans l'arborescence : Ctrl+C, Ctrl+X, Ctrl+V ; Ctrl+Z annule.
      </p>
      <BaseButton @click="transfer.closeDialog()">Annuler</BaseButton>
      <BaseButton :disabled="busy || !choice?.destination" @click="submit('copy')">
        Copier
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="busy || !choice?.destination || stays"
        @click="submit('move')"
      >
        Déplacer
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.copy-move {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
  min-height: 22rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.copy-move__pick {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
  padding: 0.75rem;
  border-right: 1px solid var(--color-border);
}

.copy-move__back {
  align-self: flex-start;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-accent-text);
  cursor: pointer;
}

.copy-move__back:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.copy-move__search {
  min-height: var(--control-md);
  padding: 0 0.625rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  font: inherit;
  color: var(--color-text);
}

.copy-move__search:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -1px;
}

.copy-move__list {
  flex: 1;
  max-height: 20rem;
  overflow-y: auto;
}

.copy-move__group {
  margin: 0.5rem 0 0.25rem;
  font: 600 var(--text-caption) / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.copy-move__option {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border-radius: var(--radius-md);
  cursor: pointer;
}

.copy-move__option:hover {
  background: var(--color-chip);
}

.copy-move__option--active,
.copy-move__option--active:hover {
  background: var(--color-accent-soft);
  box-shadow: inset 2px 0 0 var(--color-accent);
}

.copy-move__label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-move__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.copy-move__note--accent {
  color: var(--color-accent-text);
}

.copy-move__none {
  margin: 0.5rem 0;
  color: var(--color-muted);
}

.copy-move__place {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--text-small);
}

.copy-move__place select {
  flex: 1;
  min-height: var(--control-sm);
}

.copy-move__changes {
  min-width: 0;
  padding: 0.75rem 1rem;
  background: var(--color-bg);
}

.copy-move__heading {
  margin: 0 0 0.5rem;
  font: 600 var(--text-caption) / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.copy-move__stays {
  margin: 0 0 0.5rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.copy-move__items {
  margin: 0;
}

.copy-move__item {
  padding: 0.5rem 0;
  border-bottom: 1px solid var(--color-border);
}

.copy-move__item dt {
  font-weight: 600;
}

.copy-move__item dd {
  margin: 0.125rem 0 0;
  font-size: var(--text-small);
  line-height: 1.5;
  color: var(--color-muted);
}

.copy-move__item--warning dt {
  color: var(--color-danger);
}

.copy-move__error {
  margin: 0.5rem 0 0;
  color: var(--color-danger);
}

.copy-move__hint {
  flex: 1;
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
