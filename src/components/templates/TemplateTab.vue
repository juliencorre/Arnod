<script setup lang="ts">
/**
 * Un template (4.8-c) : ses paramètres (valeur par défaut, obligatoire, description), son contenu
 * et toutes ses utilisations, avec leur dernière exécution. « ◇ Utiliser dans un dossier… » crée
 * une utilisation dans le référentiel (4.8-d). Le cœur range et écrit (R16, R17).
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import RowRemoveButton from "@/components/ui/RowRemoveButton.vue";
import { useGuard } from "@/composables/useGuard";
import { onTablistKeydown } from "@/lib/tablist";
import { countLabel } from "@/lib/format";
import { useTemplatesStore } from "@/stores/templates";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TemplateKind, TemplateParam, TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const templates = useTemplatesStore();
const workbench = useWorkbenchStore();
const guard = useGuard();
const view = computed(() => templates.view(props.projectId, props.path));
const error = ref<string | null>(null);
const tab = ref<"params" | "content" | "uses">("params");

watch(
  () => [props.path, workbench.trees[props.projectId]] as const,
  async () => {
    try {
      await templates.read(props.projectId, props.path);
      error.value = null;
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    }
  },
  { immediate: true },
);

const KINDS: Record<TemplateKind, string> = {
  test: "test",
  folder: "dossier",
  tree: "arborescence",
};

/** Paramètres modifiés sur place. */
const params = ref<TemplateParam[]>([]);
watch(
  () => view.value?.parameters,
  (found) => {
    params.value = (found ?? []).map((p) => ({ ...p }));
  },
  { immediate: true },
);

function save(): void {
  const clean = params.value.filter((p) => p.name.trim() !== "");
  guard(() => templates.saveParams(props.projectId, props.path, clean));
}

function addParam(): void {
  params.value.push({
    name: `param${String(params.value.length + 1)}`,
    default: "",
    description: "",
  });
  save();
}

function removeParam(name: string): void {
  params.value = params.value.filter((p) => p.name !== name);
  save();
}

/** Dernière exécution d'une utilisation : sa ligne dans le run montré. */
function lastRun(path: string): { text: string; tone: string } {
  const item = workbench.runs[props.projectId]?.items.find((i) => i.path === path);
  if (!item) return { text: "—", tone: "none" };
  const detail = item.detail === null ? "" : ` · ${item.detail}`;
  return item.status === "passed"
    ? { text: `✓ Réussi${detail}`, tone: "ok" }
    : item.status === "failed" || item.status === "error"
      ? { text: `✕ Échec${detail}`, tone: "ko" }
      : { text: "—", tone: "none" };
}

function openNode(node: TreeNode): void {
  if (node.kind === "request") void workbench.openRequest(props.projectId, node.path);
}

function flat(nodes: TreeNode[], depth = 0): { node: TreeNode; depth: number }[] {
  return nodes.flatMap((node) => [
    { node, depth },
    ...(node.kind === "folder" ? flat(node.children, depth + 1) : []),
  ]);
}

// --- « Utiliser dans un dossier… » (4.8-d) -------------------------------------------------

const using = ref(false);
const ids = { folder: useId(), name: useId() };
const folder = ref("");
const useName = ref("");
const useError = ref<string | null>(null);
const folders = computed(() => {
  const out: string[] = [""];
  const walk = (nodes: TreeNode[]): void => {
    for (const node of nodes) {
      if (node.kind !== "folder" || node.test || node.template !== null) continue;
      out.push(node.path);
      walk(node.children);
    }
  };
  walk(workbench.trees[props.projectId]?.tests ?? []);
  return out;
});

function startUse(): void {
  using.value = true;
  useName.value = view.value?.name ?? "";
  folder.value = "";
  useError.value = null;
}

async function submitUse(): Promise<void> {
  try {
    await templates.use(props.projectId, props.path, folder.value, useName.value.trim());
    using.value = false;
  } catch (caught) {
    useError.value = caught instanceof Error ? caught.message : String(caught);
  }
}
</script>

<template>
  <section v-if="view" class="template" :aria-label="`Template ${view.name}`">
    <nav class="template__crumbs" aria-label="Fil d'Ariane">
      {{ ["templates", ...(view.category === "" ? [] : view.category.split("/"))].join(" / ") }}
    </nav>
    <header class="template__head">
      <span class="template__badge" aria-hidden="true">◇</span>
      <h2 class="template__title">{{ view.name }}</h2>
      <span class="template__kind">{{ KINDS[view.kind] }}</span>
      <span class="template__muted">
        {{ countLabel(view.folders, "dossier", "dossiers") }} ·
        {{ countLabel(view.tests, "test", "tests") }} ·
        {{ countLabel(view.assertions, "assertion", "assertions") }}
      </span>
      <span class="template__spacer" />
      <BaseButton variant="primary" @click="startUse">◇ Utiliser dans un dossier…</BaseButton>
    </header>
    <div class="template__tabs" role="tablist" aria-label="Template" @keydown="onTablistKeydown">
      <button
        v-for="t in [
          { id: 'params' as const, label: `Paramètres ${params.length}` },
          { id: 'content' as const, label: 'Contenu' },
          { id: 'uses' as const, label: `Utilisations ${view.uses.length}` },
        ]"
        :key="t.id"
        type="button"
        role="tab"
        class="template__tab"
        :aria-selected="tab === t.id"
        :tabindex="tab === t.id ? 0 : -1"
        @click="tab = t.id"
      >
        {{ t.label }}
      </button>
    </div>

    <div v-if="tab === 'params'" role="tabpanel" class="template__panel">
      <table class="template__table" aria-label="Paramètres du template">
        <thead>
          <tr>
            <th scope="col">Paramètre</th>
            <th scope="col">Défaut</th>
            <th scope="col">Obligatoire</th>
            <th scope="col">Description</th>
            <th scope="col"><span class="visually-hidden">Retirer</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="param in params" :key="param.name">
            <td>
              <input
                v-model="param.name"
                class="template__input"
                :aria-label="`Nom du paramètre ${param.name}`"
                @change="save"
              />
            </td>
            <td>
              <input
                v-model="param.default"
                class="template__input"
                :aria-label="`Valeur par défaut de ${param.name}`"
                @change="save"
              />
            </td>
            <td class="template__muted">{{ param.default.trim() === "" ? "oui" : "non" }}</td>
            <td>
              <input
                v-model="param.description"
                class="template__input"
                :aria-label="`Description de ${param.name}`"
                @change="save"
              />
            </td>
            <td>
              <RowRemoveButton
                :label="`Retirer le paramètre ${param.name}`"
                @click="removeParam(param.name)"
              />
            </td>
          </tr>
        </tbody>
      </table>
      <BaseButton @click="addParam">+ Paramètre</BaseButton>
      <p class="template__muted">
        Sans valeur par défaut, un paramètre est obligatoire : chaque utilisation lui donne une
        valeur, ou la reprend d'une propriété de ses dossiers parents.
      </p>
    </div>

    <div v-else-if="tab === 'content'" role="tabpanel" class="template__panel">
      <ul class="template__content">
        <li
          v-for="{ node, depth } in flat(view.content)"
          :key="node.path"
          :style="{ paddingLeft: `${depth}rem` }"
        >
          <template v-if="node.kind === 'request'">
            <b class="template__method">{{ node.method }}</b>
            <button type="button" class="template__link" @click="openNode(node)">
              {{ node.name }}
            </button>
          </template>
          <span v-else class="template__folder">{{ node.name }}</span>
        </li>
      </ul>
      <p class="template__muted">
        Modifier le contenu du template change toutes ses utilisations à la prochaine exécution.
      </p>
    </div>

    <div v-else role="tabpanel" class="template__panel">
      <table class="template__table" aria-label="Utilisations du template">
        <thead>
          <tr>
            <th scope="col">Utilisé dans</th>
            <th scope="col">Valeurs</th>
            <th scope="col">Dernière exécution</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="use in view.uses" :key="use.path">
            <td>
              <button
                type="button"
                class="template__link"
                @click="workbench.openFolder(projectId, use.path)"
              >
                {{ use.path }}
              </button>
            </td>
            <td class="template__muted">
              {{
                Object.entries(use.values)
                  .map(([k, v]) => `${k}=${v}`)
                  .join(" · ") || "valeurs par défaut"
              }}
            </td>
            <td :class="`template__result--${lastRun(use.path).tone}`">
              {{ lastRun(use.path).text }}
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="view.uses.length === 0" class="template__muted">
        Pas encore utilisé : « ◇ Utiliser dans un dossier… » l'ajoute au référentiel de tests.
      </p>
    </div>

    <BaseDialog :open="using" title="◇ Utiliser le template" @close="using = false">
      <form class="template__form" novalidate @submit.prevent="submitUse">
        <label :for="ids.folder" class="template__label">Dans le dossier</label>
        <select :id="ids.folder" v-model="folder" class="template__input">
          <option v-for="f in folders" :key="f" :value="f">
            {{ f === "" ? "Référentiel de tests" : f }}
          </option>
        </select>
        <label :for="ids.name" class="template__label">Nom de l'utilisation</label>
        <input :id="ids.name" v-model="useName" class="template__input" autocomplete="off" />
        <p class="template__muted">
          On ne règle que les valeurs des paramètres ; le contenu reste celui du template.
        </p>
        <p v-if="useError" class="template__error" role="alert">{{ useError }}</p>
        <div class="template__actions">
          <BaseButton @click="using = false">Annuler</BaseButton>
          <BaseButton variant="primary" type="submit" :disabled="useName.trim() === ''"
            >Utiliser</BaseButton
          >
        </div>
      </form>
    </BaseDialog>
  </section>
  <p v-else-if="error" class="template__error" role="alert">{{ error }}</p>
  <p v-else class="template__muted">Chargement…</p>
</template>

<style scoped>
.template {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
  padding: 1rem 1.25rem;
}

.template__crumbs,
.template__muted {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.template__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.template__badge {
  font-size: var(--text-title);
  color: var(--color-family-files-text);
}

.template__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.template__kind {
  padding: 0 0.375rem;
  border-radius: var(--radius-sm);
  font: 600 var(--text-caption) var(--font-mono);
  color: var(--color-family-files-text);
  background: var(--color-family-files-bg);
}

.template__spacer {
  flex: 1;
}

.template__tabs {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--color-border);
}

.template__tab {
  min-height: var(--control-md);
  padding: 0 0.75rem;
  border: 0;
  border-bottom: 2px solid transparent;
  font: inherit;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.template__tab[aria-selected="true"] {
  border-bottom-color: var(--color-accent);
  font-weight: 600;
  color: var(--color-text);
}

.template__tab:focus-visible,
.template__link:focus-visible,
.template__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.template__panel {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
}

.template__table {
  width: 100%;
  border-collapse: collapse;
}

.template__table th,
.template__table td {
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
}

.template__table th {
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.template__input {
  width: 100%;
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-bg);
}

.template__content {
  margin: 0;
  padding: 0;
  list-style: none;
}

.template__method {
  margin-right: 0.5rem;
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.template__folder {
  font-weight: 600;
}

.template__link {
  padding: 0;
  border: 0;
  font: inherit;
  color: var(--color-link);
  background: none;
  cursor: pointer;
}

.template__result--ok {
  color: var(--color-success);
}

.template__result--ko,
.template__error {
  color: var(--color-danger);
}

.template__result--none {
  color: var(--color-muted);
}

.template__form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.template__label {
  font-weight: 600;
}

.template__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}
</style>
