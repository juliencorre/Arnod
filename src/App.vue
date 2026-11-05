<script setup lang="ts">
import { computed, onMounted } from "vue";

import { formatTime } from "@/lib/format";
import { useHelloStore } from "@/stores/hello";

const store = useHelloStore();

const nativeTime = computed(() => {
  const timestampMs = store.response?.timestampMs;
  if (timestampMs === undefined) return null;
  return {
    iso: new Date(timestampMs).toISOString(),
    label: formatTime(timestampMs),
  };
});

// F1/F2 : le texte affiché au démarrage provient de la commande Rust `hello`.
onMounted(() => store.sayHello());
</script>

<template>
  <main class="hello">
    <h1 class="hello__message" data-testid="message" aria-live="polite">
      {{ store.response?.message ?? "…" }}
    </h1>

    <p class="hello__time" data-testid="native-time">
      Heure du cœur natif :
      <time v-if="nativeTime" :datetime="nativeTime.iso">{{ nativeTime.label }}</time>
      <span v-else>—</span>
    </p>

    <button type="button" class="hello__button" :disabled="store.loading" @click="store.sayHello()">
      Dire bonjour
    </button>

    <p v-if="store.error" class="hello__error" role="alert">{{ store.error }}</p>
  </main>
</template>

<style scoped>
.hello {
  box-sizing: border-box;
  min-height: 100vh;
  padding: 2rem 1.5rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  text-align: center;
}

.hello__message {
  margin: 0;
  font-size: clamp(2rem, 6vw, 3.25rem);
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.1;
}

.hello__time {
  margin: 0;
  color: var(--color-muted);
  font-variant-numeric: tabular-nums;
}

.hello__time time,
.hello__time span {
  color: var(--color-text);
}

.hello__button {
  margin-top: 0.5rem;
  padding: 0.6rem 1.25rem;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  font-weight: 600;
  color: var(--color-accent-text);
  background: var(--color-accent);
  cursor: pointer;
  transition:
    background-color 120ms ease,
    transform 120ms ease;
}

.hello__button:hover:not(:disabled) {
  background: var(--color-accent-hover);
}

.hello__button:active:not(:disabled) {
  transform: translateY(1px);
}

.hello__button:disabled {
  cursor: progress;
  opacity: 0.6;
}

.hello__button:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.hello__error {
  margin: 0;
  max-width: 40rem;
  color: var(--color-danger);
}
</style>
