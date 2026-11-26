<script setup lang="ts">
/**
 * Vue d'un dossier du référentiel (4.1-a bis, AM37 ; décisions Q3 et Q4 du plan d'alignement) :
 * fil d'Ariane, « dossier · N tests et M sous-dossiers · T tests au total », « + Test »,
 * « + Sous-dossier », « ▶ Exécuter le dossier », puis le contenu dans l'ordre d'exécution :
 * #, Type, Nom, Contenu, Résultat. Un test ou un sous-dossier se nomme sur place (« Entrée pour
 * créer »). Les nombres sont des comptages d'affichage sur l'arborescence fournie par le cœur.
 *
 * AM38 (4.1-a ter) : ⋮⋮ réordonne les lignes (glisser, ou ↑ ↓ sur la poignée) ; déposer un
 * élément sur le milieu d'un sous-dossier l'y range. Le cœur écrit les rangs (`tree_reorder`).
 */
import { computed, nextTick, ref, useId, useTemplateRef } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel } from "@/lib/format";
import { countStatuses, requestPaths } from "@/lib/status";
import { findFolder, folderSummary, testCount, type FolderNode } from "@/lib/tree";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const workbench = useWorkbenchStore();
const guard = useGuard();
const titleId = useId();
const handleHintId = useId();

const folder = computed<FolderNode | null>(() =>
  findFolder(workbench.trees[props.projectId]?.tests ?? [], props.path),
);
const crumbs = computed(() => ["Référentiel de tests", ...props.path.split("/").slice(0, -1)]);
const name = computed(() => props.path.split("/").at(-1) ?? props.path);

/** « dossier · 2 tests, 1 requête libre et 1 sous-dossier · 5 tests au total » (sans les zéros). */
const summary = computed(() => {
  const current = folder.value;
  if (!current) return "";
  const counts = folderSummary(current);
  const parts = [countLabel(counts.tests, "test", "tests")];
  if (counts.requests > 0)
    parts.push(countLabel(counts.requests, "requête libre", "requêtes libres"));
  if (counts.folders > 0) parts.push(countLabel(counts.folders, "sous-dossier", "sous-dossiers"));
  const last = parts.pop() ?? "";
  const listed = parts.length > 0 ? `${parts.join(", ")} et ${last}` : last;
  return `dossier · ${listed} · ${countLabel(counts.total, "test", "tests")} au total`;
});

type Kind = "request" | "test" | "folder";
const kindOf = (node: TreeNode): Kind =>
  node.kind === "request" ? "request" : node.test ? "test" : "folder";

/** Colonne « Contenu » : étapes d'un test, tests d'un dossier ; rien pour une requête. */
function content(node: TreeNode): string {
  if (node.kind === "request") return "—";
  if (node.test) return countLabel(node.children.length, "étape", "étapes");
  return countLabel(testCount(node), "test", "tests");
}

/** Colonne « Résultat » : dernière exécution, « réussies/exécutées » pour un dossier. */
function result(node: TreeNode): { text: string; tone: "ok" | "ko" | "none" } {
  const counts = countStatuses(requestPaths(node), workbench.statuses[props.projectId]);
  if (counts.executed === 0) return { text: "—", tone: "none" };
  if (kindOf(node) === "folder") {
    return {
      text: `${String(counts.passed)}/${String(counts.executed)}`,
      tone: counts.passed === counts.executed ? "ok" : "ko",
    };
  }
  return counts.passed === counts.executed
    ? { text: "✓ Réussi", tone: "ok" }
    : { text: "✕ Échec", tone: "ko" };
}

function open(node: TreeNode): void {
  if (node.kind === "request") void workbench.openRequest(props.projectId, node.path);
  else if (node.test) void workbench.openTest(props.projectId, node.path);
  else void workbench.openFolder(props.projectId, node.path);
}

// --- Réordonner et ranger (AM38) --------------------------------------------------------

const ROW_DRAG = "application/x-arnod-ligne";
const table = useTemplateRef<HTMLElement>("table");
/** Ligne survolée pendant un glisser, et où irait l'élément. */
const dropping = ref<{ path: string; where: "before" | "after" | "into" } | null>(null);
let dragged: string | null = null;

/** Nouvel ordre : `path` retiré, puis remis à la place `index` (dans la liste sans lui). */
function moved(path: string, index: number): string[] {
  const order = (folder.value?.children ?? []).map((child) => child.path).filter((p) => p !== path);
  order.splice(Math.max(0, Math.min(index, order.length)), 0, path);
  return order;
}

async function focusHandle(path: string): Promise<void> {
  await nextTick();
  table.value
    ?.querySelector<HTMLElement>(`.folder__handle[data-path="${CSS.escape(path)}"]`)
    ?.focus();
}

/** ↑ ↓ sur la poignée : la ligne monte ou descend d'un rang (R12, équivalent du glisser). */
function onHandleKey(event: KeyboardEvent, node: TreeNode, index: number): void {
  const delta = event.key === "ArrowUp" ? -1 : event.key === "ArrowDown" ? 1 : 0;
  if (delta === 0) return;
  event.preventDefault();
  const count = folder.value?.children.length ?? 0;
  const target = index + delta;
  if (target < 0 || target >= count) return;
  guard(async () => {
    await workbench.reorder(props.projectId, props.path, moved(node.path, target));
    await focusHandle(node.path);
  });
}

function onDragStart(event: DragEvent, node: TreeNode): void {
  dragged = node.path;
  event.dataTransfer?.setData(ROW_DRAG, node.path);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}

/** Place du dépôt dans la ligne : avant, après, ou dans un sous-dossier (son milieu). */
function placeOf(event: DragEvent, node: TreeNode): "before" | "after" | "into" {
  const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const ratio = box.height > 0 ? (event.clientY - box.top) / box.height : 0.5;
  if (kindOf(node) === "folder" && ratio > 0.25 && ratio < 0.75) return "into";
  return ratio < 0.5 ? "before" : "after";
}

function onDragOver(event: DragEvent, node: TreeNode): void {
  if (dragged === null || dragged === node.path) return;
  event.preventDefault();
  dropping.value = { path: node.path, where: placeOf(event, node) };
}

function onDrop(event: DragEvent, node: TreeNode, index: number): void {
  const source = dragged ?? event.dataTransfer?.getData(ROW_DRAG) ?? null;
  const where = dropping.value?.where ?? "before";
  dropping.value = null;
  dragged = null;
  if (source === null || source === "" || source === node.path) return;
  event.preventDefault();
  if (where === "into") {
    guard(() => workbench.move(props.projectId, source, node.path));
    return;
  }
  const from = folder.value?.children.findIndex((child) => child.path === source) ?? -1;
  // Index dans la liste sans l'élément glissé.
  const at = (where === "before" ? index : index + 1) - (from >= 0 && from < index ? 1 : 0);
  guard(() => workbench.reorder(props.projectId, props.path, moved(source, at)));
}

function onDragEnd(): void {
  dragged = null;
  dropping.value = null;
}

// --- Nom saisi sur place (« Entrée pour créer ») -----------------------------------------

const creating = ref<"test" | "folder" | null>(null);
const newName = ref("");
const error = ref<string | null>(null);
const field = useTemplateRef<HTMLInputElement>("field");

async function startCreate(kind: "test" | "folder"): Promise<void> {
  creating.value = kind;
  newName.value = "";
  error.value = null;
  await nextTick();
  field.value?.focus();
}

async function submitCreate(): Promise<void> {
  const kind = creating.value;
  const wanted = newName.value.trim();
  if (kind === null) return;
  if (wanted === "") {
    error.value = kind === "test" ? "Donnez un nom au test." : "Donnez un nom au sous-dossier.";
    return;
  }
  try {
    if (kind === "test") await workbench.createTest(props.projectId, props.path, wanted);
    else await workbench.createFolder(props.projectId, props.path, wanted);
    creating.value = null;
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
}

function cancelCreate(): void {
  creating.value = null;
  error.value = null;
}
</script>

<template>
  <section v-if="folder" class="folder" :aria-labelledby="titleId">
    <header class="folder__head">
      <nav class="folder__crumbs" aria-label="Fil d'Ariane">
        <template v-for="(crumb, index) in crumbs" :key="index"
          ><span>{{ crumb }}</span
          ><span class="folder__sep" aria-hidden="true"> / </span></template
        ><span class="folder__crumb--current">{{ name }}</span>
      </nav>
      <div class="folder__title-row">
        <h2 :id="titleId" class="folder__title">{{ name }}</h2>
        <span class="folder__spacer" />
        <BaseButton @click="startCreate('test')">+ Test</BaseButton>
        <BaseButton @click="startCreate('folder')">+ Sous-dossier</BaseButton>
        <BaseButton @click="guard(() => workbench.runFolder(projectId, path))">
          ▶ Exécuter le dossier
        </BaseButton>
      </div>
      <p class="folder__summary">{{ summary }}</p>
    </header>

    <p :id="handleHintId" class="visually-hidden">
      Flèches haut et bas pour déplacer la ligne ; glisser pour réordonner, ou déposer sur un
      sous-dossier pour l'y ranger.
    </p>
    <table ref="table" class="folder__table">
      <thead>
        <tr>
          <th scope="col" class="folder__grip"><span class="visually-hidden">Ordre</span></th>
          <th scope="col" class="folder__num">#</th>
          <th scope="col">Type</th>
          <th scope="col">Nom</th>
          <th scope="col">Contenu</th>
          <th scope="col">Résultat</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(node, index) in folder.children"
          :key="node.path"
          class="folder__row"
          :class="dropping?.path === node.path && `folder__row--${dropping.where}`"
          @dragover="onDragOver($event, node)"
          @dragleave="dropping = null"
          @drop="onDrop($event, node, index)"
        >
          <td class="folder__grip">
            <button
              type="button"
              class="folder__handle"
              draggable="true"
              :data-path="node.path"
              :aria-label="`Déplacer ${node.name}`"
              :aria-describedby="handleHintId"
              @dragstart="onDragStart($event, node)"
              @dragend="onDragEnd"
              @keydown="onHandleKey($event, node, index)"
            >
              ⋮⋮
            </button>
          </td>
          <td class="folder__num">{{ index + 1 }}</td>
          <td>
            <span
              v-if="node.kind === 'request'"
              class="folder__method"
              :class="node.method && `folder__method--${node.method.toLowerCase()}`"
              >{{ node.method }}</span
            >
            <span v-else class="folder__kind">{{
              kindOf(node) === "test" ? "test" : "dossier"
            }}</span>
          </td>
          <td>
            <button type="button" class="folder__name" @click="open(node)">{{ node.name }}</button>
          </td>
          <td class="folder__muted">{{ content(node) }}</td>
          <td :class="`folder__result folder__result--${result(node).tone}`">
            {{ result(node).text }}
          </td>
        </tr>
        <tr v-if="creating" class="folder__row folder__row--new">
          <td class="folder__grip" />
          <td class="folder__num">{{ folder.children.length + 1 }}</td>
          <td>
            <span class="folder__kind">{{ creating === "test" ? "test" : "dossier" }}</span>
          </td>
          <td colspan="3">
            <form class="folder__create" novalidate @submit.prevent="submitCreate">
              <input
                ref="field"
                v-model="newName"
                type="text"
                autocomplete="off"
                :aria-label="
                  creating === 'test' ? 'Nom du nouveau test' : 'Nom du nouveau sous-dossier'
                "
                :placeholder="creating === 'test' ? 'Nom du test' : 'Nom du sous-dossier'"
                :aria-invalid="error !== null"
                @keydown.esc.prevent="cancelCreate"
              />
              <span class="folder__hint">Entrée pour créer · Échap pour annuler</span>
            </form>
            <p v-if="error" class="folder__error" role="alert">{{ error }}</p>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="folder.children.length === 0 && !creating" class="folder__empty">
      Dossier vide : « + Test » y crée un test, « + Sous-dossier » un dossier.
    </p>
    <p class="folder__note">
      L'ordre de la liste est l'ordre d'exécution. Glissez ⋮⋮ pour réordonner, ou déposez un élément
      sur un sous-dossier pour l'y ranger.
    </p>
  </section>
  <p v-else class="folder__empty">Ce dossier n'existe plus.</p>
</template>

<style scoped>
.folder {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
  font-size: var(--text-body);
}

.folder__head {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.folder__crumbs {
  color: var(--color-muted);
}

.folder__crumb--current {
  color: var(--color-text);
}

.folder__title-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.folder__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.folder__spacer {
  flex: 1;
}

.folder__summary,
.folder__note,
.folder__empty {
  margin: 0;
  color: var(--color-muted);
}

.folder__table {
  width: 100%;
  border-collapse: collapse;
}

.folder__table th {
  padding: 0.375rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
  font: 600 var(--text-caption) / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-align: left;
  text-transform: uppercase;
  color: var(--color-muted);
}

.folder__table td {
  padding: 0.375rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
}

.folder__num {
  width: 2rem;
  color: var(--color-muted);
}

.folder__grip {
  width: 1.5rem;
}

.folder__handle {
  display: inline-grid;
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

.folder__handle:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

/* Où irait la ligne glissée : un trait avant ou après, un cadre pour « ranger dans ». */
.folder__row--before td {
  box-shadow: inset 0 2px 0 var(--color-accent);
}

.folder__row--after td {
  box-shadow: inset 0 -2px 0 var(--color-accent);
}

.folder__row--into td {
  background-color: var(--color-accent-soft);
}

.folder__method {
  font: 700 var(--text-caption) var(--font-mono);
}

.folder__method--get {
  color: var(--color-success);
}

.folder__method--post,
.folder__method--put,
.folder__method--patch {
  color: var(--color-link);
}

.folder__method--delete {
  color: var(--color-danger);
}

.folder__kind {
  font: var(--text-small) var(--font-mono);
  color: var(--color-muted);
}

.folder__name {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  text-align: left;
  color: var(--color-text);
  cursor: pointer;
}

.folder__name:hover {
  text-decoration: underline;
}

.folder__name:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.folder__muted {
  color: var(--color-muted);
}

.folder__result--ok {
  color: var(--color-success);
}

.folder__result--ko {
  color: var(--color-danger);
}

.folder__result--none {
  color: var(--color-muted);
}

.folder__create {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
}

.folder__create input {
  flex: 1;
  min-width: 12rem;
}

.folder__hint {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.folder__error {
  margin: 0.25rem 0 0;
  color: var(--color-danger);
}
</style>
