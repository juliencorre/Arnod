<script setup lang="ts">
/**
 * Section « Templates » d'un projet dans la barre latérale (4.8, R21) : les templates rangés par
 * dossier de `templates/`, chacun avec sa nature (test, dossier, arborescence) ; un clic ouvre le
 * template (4.8-c). Elle s'affiche dès que le projet a un template, ou depuis « Plus ».
 */
import { computed, ref } from "vue";

import SectionHeading from "@/components/ui/SectionHeading.vue";
import { useTemplatesStore } from "@/stores/templates";
import { useWorkbenchStore } from "@/stores/workbench";
import type { TemplateKind, TemplateSummary } from "@/types/ipc";

const props = defineProps<{ projectId: string }>();

const templates = useTemplatesStore();
const workbench = useWorkbenchStore();
const expanded = ref(true);
const list = computed(() => templates.list(props.projectId));

const KINDS: Record<TemplateKind, string> = {
  test: "test",
  folder: "dossier",
  tree: "arborescence",
};

/** Templates par dossier de rangement, dans l'ordre de `templates/`. */
const groups = computed(() => {
  const out: { category: string; items: TemplateSummary[] }[] = [];
  for (const item of list.value) {
    let group = out.find((g) => g.category === item.category);
    if (!group) {
      group = { category: item.category, items: [] };
      out.push(group);
    }
    group.items.push(item);
  }
  return out;
});
</script>

<template>
  <section class="templates" :aria-label="`Templates de ${projectId}`">
    <SectionHeading
      icon="templates"
      title="Templates"
      :count="list.length"
      :expanded="expanded"
      @toggle="expanded = !expanded"
    />
    <ul v-if="expanded" class="templates__list">
      <template v-for="group in groups" :key="group.category">
        <li v-if="group.category !== ''" class="templates__category">{{ group.category }}</li>
        <li v-for="item in group.items" :key="item.path">
          <button
            type="button"
            class="templates__row"
            :class="{ 'templates__row--nested': group.category !== '' }"
            @click="workbench.openFolder(projectId, item.path, true)"
            @dblclick="workbench.openFolder(projectId, item.path)"
          >
            <span class="templates__mark" aria-hidden="true">◇</span>
            <span class="templates__name">{{ item.name }}</span>
            <span class="templates__kind">{{ KINDS[item.kind] }}</span>
          </button>
        </li>
      </template>
      <li v-if="list.length === 0" class="templates__empty">
        Créez un template depuis la vue d'un dossier : « ◇ Créer un template de ce dossier ».
      </li>
    </ul>
  </section>
</template>

<style scoped>
.templates__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.templates__category {
  padding: 0.125rem var(--tree-pad) 0.125rem
    calc(var(--tree-pad) + var(--tree-indent) + var(--tree-lead) + var(--tree-gap));
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-muted);
}

.templates__row {
  display: flex;
  align-items: center;
  gap: var(--tree-gap);
  width: 100%;
  min-height: 1.75rem;
  padding: 0 var(--tree-pad) 0 calc(var(--tree-pad) + var(--tree-indent));
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.templates__row--nested {
  padding-left: calc(var(--tree-pad) + var(--tree-indent) * 2);
}

.templates__row:hover {
  background: var(--color-accent-soft);
}

.templates__row:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.templates__mark {
  display: grid;
  flex: none;
  place-items: center;
  width: var(--tree-lead);
  color: var(--color-family-files-text);
}

.templates__name {
  overflow: hidden;
  flex: 1;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.templates__kind,
.templates__empty {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.templates__empty {
  padding: 0.25rem var(--tree-pad) 0.25rem
    calc(var(--tree-pad) + var(--tree-indent) + var(--tree-lead) + var(--tree-gap));
}
</style>
