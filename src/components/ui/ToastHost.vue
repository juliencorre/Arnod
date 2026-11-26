<script setup lang="ts">
/**
 * Affiche la notification courante en bas de la fenêtre. La zone `role="status"`
 * reste en place pour que les lecteurs d'écran annoncent chaque message.
 */
import { useToastStore } from "@/stores/toasts";

const toasts = useToastStore();
</script>

<template>
  <div class="toast-host" role="status" aria-live="polite">
    <div
      v-if="toasts.current"
      :key="toasts.current.id"
      class="toast"
      :class="`toast--${toasts.current.tone}`"
      @mouseenter="toasts.pause()"
      @mouseleave="toasts.resume()"
      @focusin="toasts.pause()"
      @focusout="toasts.resume()"
    >
      <span class="toast__message">{{ toasts.current.message }}</span>
      <button
        v-if="toasts.current.action"
        type="button"
        class="toast__action"
        @click="toasts.runAction()"
      >
        {{ toasts.current.action.label }}
      </button>
      <!-- Toute notification se ferme ; une erreur ne se ferme que là (ER7). -->
      <!-- Le ✕ est dessiné par la feuille de style : le texte de la notification reste le message. -->
      <button
        type="button"
        class="toast__close"
        aria-label="Fermer la notification"
        @click="toasts.dismiss()"
      />
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  z-index: 50;
  right: 1rem;
  bottom: 1.5rem;
  left: 1rem;
  display: flex;
  justify-content: center;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  max-width: 48rem;
  padding: 0.625rem 0.875rem;
  border-radius: var(--radius-lg);
  color: var(--color-toast-text);
  background: var(--color-toast-bg);
  box-shadow: var(--shadow-popover);
  pointer-events: auto;
}

.toast--error {
  border-left: 4px solid var(--color-danger);
}

.toast__action {
  flex: none;
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: var(--radius-md);
  font: inherit;
  font-weight: 600;
  color: var(--color-toast-action);
  background: none;
  cursor: pointer;
}

.toast__action:hover {
  text-decoration: underline;
}

.toast__action:focus-visible,
.toast__close:focus-visible {
  outline: 2px solid var(--color-toast-action);
}

/* × sans bord, cible de 24 × 24 px (AC4). */
.toast__close {
  display: grid;
  flex: none;
  place-items: center;
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0;
  border: none;
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-toast-text);
  background: none;
  cursor: pointer;
  opacity: 0.8;
}

.toast__close::before {
  content: "✕";
}

.toast__close:hover {
  opacity: 1;
}
</style>
