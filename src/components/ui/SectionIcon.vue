<script setup lang="ts">
/** Icône d'une section de la barre latérale (I1, I2) : couleur du texte, décorative. */
import { computed } from "vue";

import { SECTION_ICONS, type SectionIconName } from "./sectionIcons";

const props = defineProps<{ name: SectionIconName }>();
const paths = computed(() => SECTION_ICONS[props.name]);
</script>

<template>
  <svg
    class="section-icon"
    :data-icon="name"
    width="16"
    height="16"
    viewBox="0 0 16 16"
    aria-hidden="true"
  >
    <path v-for="(d, index) in paths" :key="index" :d="d" />
  </svg>
</template>

<style scoped>
.section-icon {
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
