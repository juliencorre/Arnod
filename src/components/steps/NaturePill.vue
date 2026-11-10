<script setup lang="ts">
/**
 * Pastille d'une nature d'étape, colorée par sa famille (catalogue des étapes).
 * Les couleurs viennent des jetons `--color-family-*` de `src/styles.css`.
 */
import type { StepFamily } from "@/types/ipc";

withDefaults(defineProps<{ name: string; family: StepFamily; size?: "sm" | "md" }>(), {
  size: "sm",
});
</script>

<template>
  <span class="pill" :class="[`pill--${family}`, `pill--${size}`]">{{ name }}</span>
</template>

<style scoped>
.pill {
  display: inline-flex;
  align-items: center;
  box-sizing: border-box;
  border: 1px solid var(--pill-border);
  border-radius: 999px;
  font-family: var(--font-mono);
  font-weight: 600;
  white-space: nowrap;
  color: var(--pill-text);
  background: var(--pill-bg);
}

.pill--sm {
  height: 1.25rem;
  padding: 0 0.5rem;
  font-size: 0.6875rem;
}

.pill--md {
  height: 1.5rem;
  padding: 0 0.5625rem;
  font-size: 0.75rem;
}

.pill--calls {
  --pill-bg: var(--color-family-calls-bg);
  --pill-border: var(--color-family-calls-border);
  --pill-text: var(--color-family-calls-text);
}

.pill--data {
  --pill-bg: var(--color-family-data-bg);
  --pill-border: var(--color-family-data-border);
  --pill-text: var(--color-family-data-text);
}

.pill--messages {
  --pill-bg: var(--color-family-messages-bg);
  --pill-border: var(--color-family-messages-border);
  --pill-text: var(--color-family-messages-text);
}

.pill--jobs {
  --pill-bg: var(--color-family-jobs-bg);
  --pill-border: var(--color-family-jobs-border);
  --pill-text: var(--color-family-jobs-text);
}

.pill--interface {
  --pill-bg: var(--color-family-interface-bg);
  --pill-border: var(--color-family-interface-border);
  --pill-text: var(--color-family-interface-text);
}

.pill--code {
  --pill-bg: var(--color-family-code-bg);
  --pill-border: var(--color-family-code-border);
  --pill-text: var(--color-family-code-text);
}

.pill--files {
  --pill-bg: var(--color-family-files-bg);
  --pill-border: var(--color-family-files-border);
  --pill-text: var(--color-family-files-text);
}
</style>
