<script setup lang="ts">
/**
 * « Plus » (1.2-d bis, R21) : les sections disponibles et masquées du projet, chacune avec sa
 * raison d'être et « Afficher » ; une section s'affiche seule à sa première utilisation. Dans
 * cette version, « Plus » ne propose que les Templates (Q7) ; les autres viennent avec la leur.
 */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useGuard } from "@/composables/useGuard";
import { MORE_SECTIONS, useTemplatesStore } from "@/stores/templates";

const props = defineProps<{ projectId: string; name: string }>();

const templates = useTemplatesStore();
const guard = useGuard();
const open = ref(false);
const hidden = computed(() =>
  MORE_SECTIONS.filter((s) => !templates.sectionShown(props.projectId, s.id)),
);
</script>

<template>
  <div v-if="hidden.length > 0" class="more">
    <button
      type="button"
      class="more__toggle"
      :aria-expanded="open"
      :aria-label="`Plus de sections pour ${name} : ${hidden.length}`"
      @click="open = !open"
    >
      <span class="more__chevron" aria-hidden="true">{{ open ? "▾" : "▸" }}</span>
      <span class="more__title">Plus</span>
      <span class="more__count">{{ hidden.length }}</span>
    </button>
    <ul v-if="open" class="more__list">
      <li v-for="section in hidden" :key="section.id" class="more__item">
        <span class="more__name">{{ section.title }}</span>
        <span class="more__hint">{{ section.hint }}</span>
        <BaseButton
          variant="link"
          :aria-label="`Afficher ${section.title}`"
          @click="guard(() => templates.showSection(projectId, section.id, true))"
          >Afficher</BaseButton
        >
      </li>
      <li class="more__note">Une section s'affiche seule à sa première utilisation.</li>
    </ul>
  </div>
</template>

<style scoped>
.more__toggle {
  display: flex;
  align-items: center;
  gap: var(--tree-gap);
  width: 100%;
  min-height: 1.875rem;
  padding: 0 var(--tree-pad) 0 var(--tree-pad);
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  text-align: left;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.more__toggle:hover {
  background: var(--color-accent-soft);
}

.more__toggle:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.more__chevron {
  width: var(--tree-indent);
  text-align: center;
}

.more__title {
  font-weight: 600;
}

.more__count {
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  background: var(--color-chip);
}

.more__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.more__item {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.25rem 0.5rem;
  padding: 0.25rem var(--tree-pad) 0.25rem calc(var(--tree-pad) + var(--tree-indent) * 2);
}

.more__name {
  font-weight: 600;
}

.more__hint,
.more__note {
  flex-basis: 100%;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.more__note {
  padding: 0.25rem var(--tree-pad) 0.25rem calc(var(--tree-pad) + var(--tree-indent) * 2);
}
</style>
