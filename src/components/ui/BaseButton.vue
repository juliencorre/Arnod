<script setup lang="ts">
/** Bouton de l'application : principal (accent), secondaire, danger ou lien. */
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "danger" | "link";
    type?: "button" | "submit";
    disabled?: boolean;
  }>(),
  { variant: "secondary", type: "button", disabled: false },
);
</script>

<template>
  <button :type="type" class="button" :class="`button--${variant}`" :disabled="disabled">
    <slot />
  </button>
</template>

<style scoped>
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.375rem;
  padding: 0.5rem 1rem;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  font-weight: 600;
  line-height: 1.25;
  cursor: pointer;
  transition:
    background-color 120ms ease,
    border-color 120ms ease;
}

.button:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.button--primary {
  color: var(--color-accent-text);
  background: var(--color-accent);
}

.button--primary:hover:not(:disabled) {
  background: var(--color-accent-hover);
}

.button--secondary {
  color: var(--color-text);
  background: var(--color-surface);
  border-color: var(--color-border);
}

.button--secondary:hover:not(:disabled) {
  border-color: var(--color-border-strong);
}

.button--danger {
  color: var(--color-danger);
  background: var(--color-surface);
  border-color: var(--color-border);
}

.button--danger:hover:not(:disabled) {
  border-color: var(--color-danger);
}

.button--link {
  padding: 0.25rem 0;
  color: var(--color-link);
  background: none;
}

.button--link:hover:not(:disabled) {
  text-decoration: underline;
}
</style>
