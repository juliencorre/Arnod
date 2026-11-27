<script setup lang="ts">
/**
 * « ◇ Créer un template de ce dossier » (4.8-b, R18) : nom, « Ranger dans », paramètres détectés
 * par le cœur (variables lues par les tests et données par les propriétés, cochées ; celles de
 * l'environnement, non cochées), puis remplacer le dossier par une utilisation ou le garder.
 */
import { computed, ref, useId, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useTemplatesStore } from "@/stores/templates";
import type { TemplateCandidate } from "@/types/ipc";

const props = defineProps<{ open: boolean; projectId: string; folder: string; summary: string }>();
const emit = defineEmits<{ close: [] }>();

const templates = useTemplatesStore();
const ids = { name: useId(), place: useId(), error: useId() };
const name = ref("");
const category = ref("");
const categories = ref<string[]>([]);
const candidates = ref<(TemplateCandidate & { picked: boolean })[]>([]);
const replace = ref(true);
const error = ref<string | null>(null);
const busy = ref(false);

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    name.value = props.folder.split("/").at(-1) ?? props.folder;
    error.value = null;
    try {
      const [found, places] = await Promise.all([
        templates.candidates(props.projectId, props.folder),
        templates.categories(props.projectId),
      ]);
      candidates.value = found.map((c) => ({ ...c, picked: c.checked }));
      categories.value = places;
      category.value = places[0] ?? "";
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    }
  },
  { immediate: true },
);

const picked = computed(() => candidates.value.filter((c) => c.picked));

async function submit(): Promise<void> {
  if (name.value.trim() === "" || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    await templates.create(
      props.projectId,
      props.folder,
      name.value.trim(),
      category.value,
      picked.value.map((c) => ({ name: c.name, default: c.value, description: "" })),
      replace.value,
    );
    emit("close");
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog
    :open="open"
    title="◇ Créer un template"
    :description="`À partir de ${folder} : ${summary}.`"
    @close="emit('close')"
  >
    <form class="tpl" novalidate @submit.prevent="submit">
      <label :for="ids.name" class="tpl__label">Nom du template</label>
      <input :id="ids.name" v-model="name" class="tpl__input" autocomplete="off" />
      <label :for="ids.place" class="tpl__label">Ranger dans</label>
      <input
        :id="ids.place"
        v-model="category"
        class="tpl__input"
        list="template-categories"
        placeholder="templates /"
      />
      <datalist id="template-categories">
        <option v-for="c in categories" :key="c" :value="c" />
      </datalist>
      <fieldset class="tpl__set">
        <legend class="tpl__label">Paramètres détectés</legend>
        <p class="tpl__hint">Variables utilisées par les tests, à exposer comme paramètres.</p>
        <label v-for="c in candidates" :key="c.name" class="tpl__candidate">
          <input v-model="c.picked" type="checkbox" />
          <code>{{ c.name }}</code> <span>{{ c.value }}</span>
          <span class="tpl__hint">{{ c.origin }}</span>
        </label>
        <p v-if="candidates.length === 0" class="tpl__hint">
          Aucune variable lue par ces tests n'a de valeur dans le dossier ou l'environnement.
        </p>
        <p class="tpl__hint">
          Les valeurs actuelles deviennent les valeurs par défaut ; une variable non cochée reste
          fournie par l'environnement.
        </p>
      </fieldset>
      <fieldset class="tpl__set">
        <label class="tpl__choice">
          <input v-model="replace" type="radio" :value="true" />
          <span
            ><b>Remplacer le dossier par une utilisation du template</b><br /><span
              class="tpl__hint"
              >Le dossier devient une utilisation : même résultat, mais modifiable à un seul
              endroit.</span
            ></span
          >
        </label>
        <label class="tpl__choice">
          <input v-model="replace" type="radio" :value="false" />
          <span
            ><b>Garder le dossier tel quel</b><br /><span class="tpl__hint"
              >Le template est une copie indépendante.</span
            ></span
          >
        </label>
      </fieldset>
      <p v-if="error" :id="ids.error" class="tpl__error" role="alert">{{ error }}</p>
      <div class="tpl__actions">
        <BaseButton @click="emit('close')">Annuler</BaseButton>
        <BaseButton variant="primary" type="submit" :disabled="busy || name.trim() === ''">
          Créer le template
        </BaseButton>
      </div>
    </form>
  </BaseDialog>
</template>

<style scoped>
.tpl {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.tpl__label {
  font-weight: 600;
}

.tpl__input {
  min-height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-control);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background-color: var(--color-bg);
}

.tpl__input:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.tpl__set {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin: 0;
  padding: 0.5rem 0 0;
  border: 0;
}

.tpl__candidate,
.tpl__choice {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
}

.tpl__hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.tpl__error {
  margin: 0;
  color: var(--color-danger);
}

.tpl__actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
