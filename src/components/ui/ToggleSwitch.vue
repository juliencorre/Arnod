<script setup lang="ts">
/**
 * Interrupteur (`role="switch"`), libellé dans le slot par défaut. Le libellé est associé
 * explicitement : `for` le relie au bouton (un clic sur le texte bascule) et
 * `aria-labelledby` donne au bouton son nom accessible.
 */
import { useId } from "vue";

const model = defineModel<boolean>({ required: true });
defineProps<{ disabled?: boolean }>();

const switchId = useId();
const labelId = useId();
</script>

<template>
  <label class="toggle" :class="{ 'toggle--disabled': disabled }" :for="switchId">
    <button
      :id="switchId"
      type="button"
      role="switch"
      class="toggle__switch"
      :aria-checked="model"
      :aria-labelledby="labelId"
      :disabled="disabled"
      @click="model = !model"
    >
      <span class="toggle__thumb" />
    </button>
    <span :id="labelId" class="toggle__label"><slot /></span>
  </label>
</template>

<style scoped>
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.625rem;
  cursor: pointer;
}

.toggle--disabled {
  cursor: not-allowed;
}

.toggle__switch {
  position: relative;
  flex: none;
  width: 2.25rem;
  height: 1.25rem;
  padding: 0;
  border: 1px solid var(--color-border-strong);
  border-radius: 999px;
  background: var(--color-chip);
  cursor: inherit;
  transition: background-color 120ms ease;
}

.toggle__switch[aria-checked="true"] {
  border-color: var(--color-accent);
  background: var(--color-accent);
}

.toggle__switch:disabled {
  opacity: 0.55;
}

/* Activé mais non modifiable (workspace déjà par défaut) : l'état reste lisible. */
.toggle__switch[aria-checked="true"]:disabled {
  opacity: 0.8;
}

.toggle__switch:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.toggle__thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 1rem;
  height: 1rem;
  border-radius: 50%;
  background: var(--color-surface);
  transition: transform 120ms ease;
}

.toggle__switch[aria-checked="true"] .toggle__thumb {
  transform: translateX(1rem);
}
</style>
