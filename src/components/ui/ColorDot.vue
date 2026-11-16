<script setup lang="ts">
/** Pastille de couleur d'un workspace (jeton `--color-ws-<couleur>`). Décorative. */
import type { WorkspaceColor } from "@/types/ipc";

withDefaults(defineProps<{ color: WorkspaceColor; size?: "sm" | "md" }>(), { size: "md" });
</script>

<template>
  <span
    class="dot"
    :class="`dot--${size}`"
    :style="{ background: `var(--color-ws-${color})` }"
    aria-hidden="true"
  />
</template>

<style scoped>
.dot {
  display: inline-block;
  flex: none;
  border-radius: var(--radius-sm);
}

.dot--sm {
  width: 0.625rem;
  height: 0.625rem;
}

.dot--md {
  width: 0.875rem;
  height: 0.875rem;
  border-radius: var(--radius-sm);
}
</style>
