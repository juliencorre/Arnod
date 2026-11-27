<script setup lang="ts">
/**
 * Utilisation d'un template dans un dossier (4.8-d) : on ne règle que les valeurs des paramètres,
 * avec l'origine de chacune (saisi ici, propriété d'un dossier parent, valeur par défaut) ; le
 * contenu reste celui du template, en lecture seule. Une valeur vide reprend, dans l'ordre, la
 * propriété du dossier parent puis la valeur par défaut (R17).
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel } from "@/lib/format";
import { useTemplatesStore } from "@/stores/templates";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ParamValue, TreeNode } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const templates = useTemplatesStore();
const workbench = useWorkbenchStore();
const guard = useGuard();
const view = computed(() => templates.instanceView(props.projectId, props.path));
const error = ref<string | null>(null);
const crumbs = computed(() => props.path.split("/").slice(0, -1).join(" / "));

watch(
  () => [props.path, workbench.trees[props.projectId]] as const,
  async () => {
    try {
      await templates.instance(props.projectId, props.path);
      error.value = null;
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    }
  },
  { immediate: true },
);

/** Valeurs saisies, modifiées sur place. */
const values = ref<Record<string, string>>({});
watch(
  () => view.value?.values,
  (found) => {
    values.value = Object.fromEntries((found ?? []).map((v) => [v.name, v.value]));
  },
  { immediate: true },
);

function save(): void {
  guard(() => templates.setValues(props.projectId, props.path, { ...values.value }));
}

function origin(value: ParamValue): string {
  switch (value.origin.kind) {
    case "here":
      return "saisi ici";
    case "property":
      return `propriété de ${value.origin.folder}`;
    case "default":
      return "valeur par défaut";
    case "missing":
      return "obligatoire, sans valeur";
  }
}

/** Résultat d'une requête du contenu dans le dernier run (statuts de la barre latérale). */
function result(node: TreeNode): string {
  const status = workbench.statuses[props.projectId]?.[node.path];
  if (!status) return "—";
  return status.passed ? "✓" : "✕";
}

function flat(nodes: TreeNode[], depth = 0): { node: TreeNode; depth: number }[] {
  return nodes.flatMap((node) => [
    { node, depth },
    ...(node.kind === "folder" ? flat(node.children, depth + 1) : []),
  ]);
}
</script>

<template>
  <section v-if="view" class="instance" :aria-label="`Utilisation ${view.name}`">
    <nav class="instance__crumbs" aria-label="Fil d'Ariane">{{ crumbs }}</nav>
    <header class="instance__head">
      <span class="instance__badge" aria-hidden="true">◇</span>
      <h2 class="instance__title">{{ view.name }}</h2>
      <span class="instance__spacer" />
      <BaseButton
        v-if="view.template"
        variant="link"
        @click="workbench.openFolder(projectId, view.templatePath)"
        >Ouvrir le template</BaseButton
      >
      <BaseButton variant="primary" @click="guard(() => workbench.runFolder(projectId, path))"
        >▶ Exécuter</BaseButton
      >
    </header>
    <p class="instance__note">
      <template v-if="view.template">
        ◇ Utilise le template <b>{{ view.template.name }}</b> ·
        {{ countLabel(view.template.tests, "test", "tests") }}. Seules les valeurs des paramètres se
        règlent ici.
      </template>
      <template v-else>Le template {{ view.templatePath }} n'existe plus.</template>
    </p>
    <h3 class="instance__label">Valeurs des paramètres</h3>
    <table class="instance__table" aria-label="Valeurs des paramètres">
      <thead>
        <tr>
          <th scope="col">Paramètre</th>
          <th scope="col">Valeur</th>
          <th scope="col">Origine</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="value in view.values" :key="value.name">
          <td>
            <code>{{ value.name }}</code>
          </td>
          <td>
            <input
              v-model="values[value.name]"
              class="instance__input"
              :aria-label="`Valeur de ${value.name}`"
              :placeholder="value.effective"
              @change="save"
            />
          </td>
          <td :class="value.origin.kind === 'missing' ? 'instance__missing' : 'instance__muted'">
            {{ origin(value) }}
          </td>
        </tr>
      </tbody>
    </table>
    <p class="instance__muted">
      Une valeur vide reprend, dans l'ordre : propriété du dossier parent, puis valeur par défaut du
      template.
    </p>
    <h3 class="instance__label">Contenu du template (lecture seule)</h3>
    <ul class="instance__content">
      <li
        v-for="{ node, depth } in flat(view.content)"
        :key="node.path"
        :style="{ paddingLeft: `${depth}rem` }"
      >
        <b v-if="node.kind === 'request'" class="instance__method">{{ node.method }}</b>
        {{ node.name }}
        <span class="instance__muted">{{ result(node) }}</span>
      </li>
    </ul>
  </section>
  <p v-else-if="error" class="instance__missing" role="alert">{{ error }}</p>
  <p v-else class="instance__muted">Chargement…</p>
</template>

<style scoped>
.instance {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
  padding: 1rem 1.25rem;
}

.instance__crumbs,
.instance__muted,
.instance__note {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.instance__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.instance__badge {
  font-size: var(--text-title);
  color: var(--color-family-files-text);
}

.instance__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.instance__spacer {
  flex: 1;
}

.instance__label {
  margin: 0.5rem 0 0;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.instance__table {
  width: 100%;
  border-collapse: collapse;
}

.instance__table th,
.instance__table td {
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
}

.instance__input {
  width: 100%;
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: var(--text-small) var(--font-mono);
  color: var(--color-text);
  background-color: var(--color-bg);
}

.instance__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.instance__missing {
  color: var(--color-danger);
}

.instance__content {
  margin: 0;
  padding: 0;
  list-style: none;
}

.instance__method {
  margin-right: 0.5rem;
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}
</style>
