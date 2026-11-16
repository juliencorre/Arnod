<script setup lang="ts">
/**
 * Secret saisi en clair dans l'authentification (lot K, écran 4.2-c) : la requête est enregistrée
 * dans le projet, donc partagée avec l'équipe. Tant que l'utilisateur n'a pas choisi, le cœur ne
 * l'écrit pas (`Editor.secretHold`). « Mettre dans une variable locale » range la valeur au coffre
 * du système et le champ devient `{{nom}}` ; « Laisser dans le test » l'écrit telle quelle.
 */
import { computed, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useInputStore } from "@/stores/input";
import { useToastStore } from "@/stores/toasts";

const props = defineProps<{ editorKey: string }>();

const inputs = useInputStore();
const toasts = useToastStore();
const busy = ref(false);

const secret = computed(() => inputs.pendingSecrets(props.editorKey)[0] ?? null);
/** Ce que le champ deviendra. */
const reference = computed(() => (secret.value ? `{{ ${secret.value.suggested} }}` : ""));

async function toLocal(): Promise<void> {
  const s = secret.value;
  if (!s || busy.value) return;
  busy.value = true;
  try {
    const env = await inputs.secretToLocal(props.editorKey, s);
    toasts.show(`Valeur rangée dans la variable locale secrète {{${s.suggested}}} de « ${env} ».`);
  } catch (caught) {
    toasts.showError(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section v-if="secret" class="secret" role="alert" aria-labelledby="secret-title">
    <h3 id="secret-title" class="secret__title">
      Ce {{ secret.label }} serait partagé avec l'équipe
    </h3>
    <p class="secret__text">
      La requête est enregistrée automatiquement dans le projet, partagé avec l'équipe. Le
      {{ secret.label }} n'y est pas écrit tant que vous n'avez pas choisi.
    </p>
    <div class="secret__actions">
      <BaseButton :disabled="busy" @click="toLocal">Mettre dans une variable locale</BaseButton>
      <BaseButton :disabled="busy" @click="inputs.keepSecrets(editorKey)">
        Laisser dans le test
      </BaseButton>
    </div>
    <p class="secret__note">
      Le champ deviendra <code>{{ reference }}</code
      >, valeur rangée dans le coffre du système, jamais envoyée à l'équipe.
    </p>
  </section>
</template>

<style scoped>
.secret {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-width: 40rem;
  padding: 0.875rem 1rem;
  border: 1px solid var(--color-accent);
  border-radius: var(--radius-lg);
  background: var(--color-accent-soft);
}

.secret__title {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.secret__text,
.secret__note {
  margin: 0;
  line-height: 1.5;
}

.secret__note {
  font-size: var(--text-small);
  color: var(--color-accent-text);
}

.secret__note code {
  font-family: var(--font-mono);
}

.secret__actions {
  display: flex;
  gap: 0.5rem;
}
</style>
