<script setup lang="ts">
/**
 * Arbre des zones : une zone, ou deux nœuds de part et d'autre d'une
 * séparation (F4). Pendant le glisser, la proportion affichée est locale ;
 * elle est enregistrée par le cœur au relâchement.
 */
import { computed, ref } from "vue";

import SplitHandle from "@/components/zones/SplitHandle.vue";
import ZoneView from "@/components/zones/ZoneView.vue";
import { useWindowsStore } from "@/stores/windows";
import type { LayoutNode } from "@/types/ipc";

const props = defineProps<{ node: LayoutNode; projectId: string | null }>();

const windows = useWindowsStore();
const preview = ref<number | null>(null);
const ratio = computed(() =>
  props.node.kind === "split" ? (preview.value ?? props.node.ratio) : 1,
);

function commit(next: number): void {
  if (props.node.kind === "split") void windows.resize(props.node.id, next);
}
</script>

<template>
  <ZoneView v-if="node.kind === 'zone'" :zone="node" :project-id="projectId" />
  <div v-else class="split" :class="`split--${node.axis}`">
    <div class="split__pane" :style="{ flexGrow: ratio }">
      <ZoneLayout :node="node.first" :project-id="projectId" />
    </div>
    <SplitHandle
      :axis="node.axis"
      :ratio="ratio"
      :min-first="node.axis === 'row' ? 280 : 160"
      :min-second="node.axis === 'row' ? 280 : 160"
      :label="node.axis === 'row' ? 'Largeur des zones' : 'Hauteur des zones'"
      @preview="preview = $event"
      @commit="commit"
    />
    <div class="split__pane" :style="{ flexGrow: 1 - ratio }">
      <ZoneLayout :node="node.second" :project-id="projectId" />
    </div>
  </div>
</template>

<style scoped>
.split {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.split--row {
  flex-direction: row;
}

.split--column {
  flex-direction: column;
}

.split__pane {
  display: flex;
  flex-basis: 0;
  flex-shrink: 1;
  min-width: 0;
  min-height: 0;
}
</style>
