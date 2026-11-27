<script setup lang="ts">
/**
 * « Nouvel environnement » (3.3-b, AM34) : nom, adresse de base (la variable `baseUrl`),
 * variables reprises d'un autre environnement (secrets non copiés), place dans la chaîne et
 * protection. Le cœur crée l'environnement et le range (`environment_create`).
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { countLabel } from "@/lib/format";
import { useWorkbenchStore } from "@/stores/workbench";

const props = defineProps<{ open: boolean; projectId: string }>();
const emit = defineEmits<{ close: []; created: [name: string] }>();

const workbench = useWorkbenchStore();
const ids = { name: useId(), url: useId(), copy: useId(), after: useId(), error: useId() };

const list = computed(() => workbench.environments[props.projectId]?.environments ?? []);

const name = ref("");
const baseUrl = ref("");
const copyFrom = ref<string | null>(null);
/** Environnement après lequel il prend place ; `null` : en tête de la chaîne. */
const after = ref<string | null>(null);
const protectedEnv = ref(false);
const error = ref<string | null>(null);
const busy = ref(false);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    name.value = "";
    baseUrl.value = "";
    copyFrom.value = null;
    after.value = list.value.at(-1)?.name ?? null;
    protectedEnv.value = false;
    error.value = null;
  },
  { immediate: true },
);

/** « 4 variables, secrets non copiés » : ce que reprend l'environnement choisi. */
const copyHint = computed(() => {
  const source = list.value.find((env) => env.name === copyFrom.value);
  if (!source) return "";
  const count = source.variables.length + source.locals.length;
  return `${countLabel(count, "variable", "variables")}, secrets non copiés`;
});

/** « , avant Pré-production » : l'environnement qui suivra le nouveau. */
const beforeHint = computed(() => {
  const index = after.value === null ? -1 : list.value.findIndex((e) => e.name === after.value);
  const next = list.value[index + 1];
  return next ? `avant ${next.name}` : "en fin de chaîne";
});

async function submit(): Promise<void> {
  if (busy.value) return;
  if (name.value.trim() === "") {
    error.value = "Donnez un nom à l'environnement.";
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    await workbench.createEnvironment(props.projectId, {
      name: name.value.trim(),
      baseUrl: baseUrl.value.trim(),
      copyFrom: copyFrom.value,
      after: after.value,
      protected: protectedEnv.value,
    });
    emit("created", name.value.trim());
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog :open="open" title="Nouvel environnement" @close="emit('close')">
    <form id="new-environment" class="new-env" novalidate @submit.prevent="submit">
      <div class="new-env__field">
        <label :for="ids.name">Nom</label>
        <input
          :id="ids.name"
          v-model="name"
          type="text"
          autocomplete="off"
          placeholder="par exemple Recette client"
          :aria-invalid="error !== null"
          :aria-describedby="error ? ids.error : undefined"
        />
        <p class="new-env__hint">Le nom que tout le monde utilise chez vous.</p>
      </div>
      <div class="new-env__field">
        <label :for="ids.url">URL de base <span class="new-env__optional">facultatif</span></label>
        <input
          :id="ids.url"
          v-model="baseUrl"
          type="url"
          autocomplete="off"
          spellcheck="false"
          placeholder="https://"
        />
        <p class="new-env__hint">Devient la variable <code>baseUrl</code> de cet environnement.</p>
      </div>
      <div class="new-env__field">
        <label :for="ids.copy"
          >Reprendre les variables de <span class="new-env__optional">facultatif</span></label
        >
        <select :id="ids.copy" v-model="copyFrom">
          <option :value="null">Aucun</option>
          <option v-for="env in list" :key="env.name" :value="env.name">{{ env.name }}</option>
        </select>
        <p v-if="copyHint" class="new-env__hint">{{ copyHint }}</p>
      </div>
      <div class="new-env__field">
        <label :for="ids.after">Position dans la chaîne</label>
        <select :id="ids.after" v-model="after">
          <option :value="null">En tête</option>
          <option v-for="env in list" :key="env.name" :value="env.name">
            Après {{ env.name }}
          </option>
        </select>
        <p class="new-env__hint">{{ beforeHint }}</p>
      </div>
      <label class="new-env__check">
        <input v-model="protectedEnv" type="checkbox" />
        Protégé : confirmer avant toute écriture
      </label>
      <p v-if="error" :id="ids.error" class="new-env__error" role="alert">{{ error }}</p>
    </form>
    <template #footer>
      <BaseButton @click="emit('close')">Annuler</BaseButton>
      <BaseButton variant="primary" type="submit" form="new-environment" :disabled="busy">
        Créer
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.new-env {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
}

.new-env__field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.new-env__field label {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.new-env__optional {
  margin-left: 0.25rem;
  font-style: italic;
}

.new-env__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.new-env__check {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  font-size: var(--text-body);
}

.new-env__error {
  margin: 0;
  font-size: var(--text-body);
  color: var(--color-danger);
}
</style>
