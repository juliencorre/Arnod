<script setup lang="ts">
/**
 * Conflit d'une récupération (`docs/plan-v2.md`, lot F, 7.1-c, CF1 à CF3) : pour chaque élément
 * changé des deux côtés, « Vous » ou le collègue, avec la date ; « Garder les deux » pour une
 * ligne de liste (une assertion) ; ce qu'un seul côté a changé est déjà fusionné (CF2). Rien
 * n'est écrit avant « Valider la résolution » ; abandonner laisse le projet tel quel (R13).
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel, formatAgo } from "@/lib/format";
import { initial, kindLabel } from "@/lib/sync";
import { useSyncStore } from "@/stores/sync";
import type { SyncChoice } from "@/types/ipc";

const sync = useSyncStore();
const guard = useGuard();

const files = computed(() => sync.conflicts?.conflicts ?? []);
/** Choix : fichier → élément → version. */
const choices = ref<Record<string, Record<string, SyncChoice>>>({});
const yaml = ref<string | null>(null);

watch(
  () => sync.conflicts,
  () => {
    choices.value = {};
    yaml.value = null;
  },
);

const total = computed(() => files.value.reduce((n, f) => n + f.elements.length, 0));
const chosen = computed(() =>
  files.value.reduce(
    (n, f) => n + f.elements.filter((e) => choices.value[f.item.key]?.[e.key]).length,
    0,
  ),
);

function pick(file: string, key: string, choice: SyncChoice): void {
  choices.value = { ...choices.value, [file]: { ...choices.value[file], [key]: choice } };
}

function chosenFor(file: string, key: string): SyncChoice | undefined {
  return choices.value[file]?.[key];
}

function shown(value: string | null): string {
  return value ?? "absent";
}
</script>

<template>
  <BaseDialog
    :open="sync.conflicts !== null"
    :title="`${countLabel(files.length, 'conflit', 'conflits')} à résoudre`"
    description="Un collègue et vous avez modifié le même élément. Le reste a été récupéré."
    wide
    closable
    @close="sync.conflicts = null"
  >
    <section v-for="file in files" :key="file.item.key" class="conflict">
      <h3 class="conflict__title">
        <span class="conflict__kind">{{ kindLabel(file.item.kind) }}</span>
        {{ file.item.label }}
        <span v-if="file.item.place" class="conflict__muted">{{ file.item.place }}</span>
      </h3>
      <fieldset v-for="element in file.elements" :key="element.key" class="conflict__element">
        <legend class="conflict__legend">{{ element.label }}</legend>
        <label class="conflict__side">
          <input
            type="radio"
            :name="`${file.item.key}:${element.key}`"
            :checked="chosenFor(file.item.key, element.key) === 'ours'"
            @change="pick(file.item.key, element.key, 'ours')"
          />
          <span class="conflict__avatar" aria-hidden="true">V</span>
          <span class="conflict__who">Vous</span>
          <code class="conflict__value">{{ shown(element.ours) }}</code>
        </label>
        <label class="conflict__side">
          <input
            type="radio"
            :name="`${file.item.key}:${element.key}`"
            :checked="chosenFor(file.item.key, element.key) === 'theirs'"
            @change="pick(file.item.key, element.key, 'theirs')"
          />
          <span class="conflict__avatar" aria-hidden="true">{{ initial(file.author) }}</span>
          <span class="conflict__who"
            >{{ file.author }}
            <span class="conflict__muted">{{ file.at ? formatAgo(file.at) : "" }}</span></span
          >
          <code class="conflict__value">{{ shown(element.theirs) }}</code>
        </label>
        <label v-if="element.both" class="conflict__side conflict__side--both">
          <input
            type="radio"
            :name="`${file.item.key}:${element.key}`"
            :checked="chosenFor(file.item.key, element.key) === 'both'"
            @change="pick(file.item.key, element.key, 'both')"
          />
          <span>Ou combiner : garder les deux</span>
        </label>
      </fieldset>
      <ul v-if="file.merged.length > 0" class="conflict__merged">
        <li v-for="merged in file.merged" :key="merged.label">
          <span class="conflict__ok" aria-hidden="true">✓</span> {{ merged.label }} : fusionné
          automatiquement · seul{{ merged.theirs ? ` ${file.author} l'a` : "s vous l'avez" }}
          modifié
        </li>
      </ul>
      <BaseButton
        variant="link"
        :aria-expanded="yaml === file.item.key"
        @click="yaml = yaml === file.item.key ? null : file.item.key"
        >Voir le YAML</BaseButton
      >
      <div v-if="yaml === file.item.key" class="conflict__yaml">
        <div>
          <p class="conflict__muted">Vous</p>
          <pre>{{ file.ours ?? "(supprimé)" }}</pre>
        </div>
        <div>
          <p class="conflict__muted">{{ file.author }}</p>
          <pre>{{ file.theirs ?? "(supprimé)" }}</pre>
        </div>
      </div>
    </section>

    <template #footer>
      <BaseButton :disabled="sync.busy" @click="guard(() => sync.abandon())"
        >Abandonner la récupération</BaseButton
      >
      <BaseButton
        variant="primary"
        :disabled="sync.busy || chosen < total"
        @click="guard(() => sync.resolve(choices))"
        >Valider la résolution</BaseButton
      >
    </template>
  </BaseDialog>
</template>

<style scoped>
.conflict {
  margin-bottom: 1.25rem;
}

.conflict__title {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  margin: 0 0 0.5rem;
  font-size: var(--text-body);
  font-weight: 600;
}

.conflict__kind {
  padding: 0 0.375rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  color: var(--color-muted);
  background: var(--color-chip);
}

.conflict__muted {
  margin: 0;
  font-size: var(--text-small);
  font-weight: 400;
  color: var(--color-muted);
}

.conflict__element {
  margin: 0 0 0.75rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.conflict__legend {
  padding: 0 0.25rem;
  font-weight: 600;
}

.conflict__side {
  display: grid;
  grid-template-columns: auto auto 9rem 1fr;
  gap: 0.5rem;
  align-items: center;
  min-height: var(--control-md);
  cursor: pointer;
}

.conflict__side--both {
  display: flex;
}

.conflict__avatar {
  display: inline-grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  background: var(--color-accent-soft);
}

.conflict__value {
  font-family: var(--editor-font);
  font-size: var(--text-small);
  overflow-wrap: anywhere;
}

.conflict__merged {
  margin: 0 0 0.5rem;
  padding: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
  list-style: none;
}

.conflict__ok {
  color: var(--color-success);
}

.conflict__yaml {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.conflict__yaml pre {
  max-height: 16rem;
  margin: 0;
  padding: 0.5rem;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-family: var(--editor-font);
  font-size: var(--text-small);
}
</style>
