<script setup lang="ts">
/**
 * En-tête d'une section d'un projet dans la barre latérale (I1) : icône, nom, nombre
 * d'éléments, puis ses actions (slot). Aligné sur la colonne de tête des lignes du projet.
 */
import SectionIcon from "./SectionIcon.vue";
import type { SectionIconName } from "./sectionIcons";

defineProps<{ icon: SectionIconName; title: string; count?: number }>();
defineSlots<{ default?(): unknown }>();
</script>

<template>
  <div class="section-head">
    <span class="section-head__lead" aria-hidden="true">
      <SectionIcon :name="icon" />
    </span>
    <h3 class="section-head__title">{{ title }}</h3>
    <span v-if="count !== undefined && count > 0" class="section-head__count">{{ count }}</span>
    <span class="section-head__actions"><slot /></span>
  </div>
</template>

<style scoped>
.section-head {
  display: flex;
  align-items: center;
  gap: var(--tree-gap, 0.5rem);
  min-height: 1.875rem;
  margin-top: 0.25rem;
  padding: 0 var(--tree-pad, 0.375rem) 0
    calc(var(--tree-pad, 0.375rem) + var(--tree-indent, 1.25rem));
}

/* L'icône occupe la colonne de la pastille du projet ; le nom s'aligne sur celui du projet. */
.section-head__lead {
  display: grid;
  flex: none;
  place-items: center;
  width: calc(var(--tree-lead, 2.5rem) - var(--tree-indent, 1.25rem));
  color: var(--color-muted);
}

.section-head__title {
  margin: 0;
  overflow: hidden;
  font-size: var(--text-body);
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text);
}

.section-head__count {
  flex: none;
  font: 400 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.section-head__actions {
  display: flex;
  flex: none;
  margin-left: auto;
}
</style>
