<script setup lang="ts">
/**
 * Historique d'un élément partagé, en clair (`docs/plan-v2.md`, lot F, 7.1-d, CF5, R14) : qui,
 * quand, ce qui a changé, le message de l'envoi ; jamais un commit ni un chemin. « Voir ce qui a
 * changé » montre les deux versions côte à côte.
 */
import { onMounted, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { formatAgo } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { initial } from "@/lib/sync";
import { useSyncStore } from "@/stores/sync";
import type { JournalEntry } from "@/types/ipc";

const props = defineProps<{ projectId: string; path: string }>();

const sync = useSyncStore();
const entries = ref<JournalEntry[] | null>(null);
const error = ref<string | null>(null);
const opened = ref<number | null>(null);

async function load(): Promise<void> {
  error.value = null;
  try {
    entries.value = await ipc.syncJournal(props.projectId, props.path);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
  }
}

onMounted(load);
watch(() => [props.projectId, props.path, sync.views[props.projectId]?.checkedAt], load);
</script>

<template>
  <div class="team">
    <p v-if="sync.receivedBy(projectId, path)" class="team__received">
      <span class="team__avatar" aria-hidden="true">{{
        initial(sync.receivedBy(projectId, path) ?? "")
      }}</span>
      {{ sync.receivedBy(projectId, path) }} a modifié cette requête — modification récupérée.
    </p>
    <p v-if="error" class="team__error" role="alert">{{ error }}</p>
    <p v-else-if="entries && entries.length === 0" class="team__empty">
      Pas encore d'historique : la requête n'a pas été envoyée à l'équipe.
    </p>
    <ol v-else-if="entries" class="team__list">
      <li v-for="(entry, index) in entries" :key="`${entry.at}-${index}`" class="team__entry">
        <span class="team__avatar" aria-hidden="true">{{ initial(entry.author) }}</span>
        <div class="team__body">
          <p class="team__line">
            <strong>{{ entry.author }}</strong> · {{ entry.detail }}
            <span v-if="entry.message" class="team__message">« {{ entry.message }} »</span>
          </p>
          <p class="team__when">{{ formatAgo(entry.at) }}</p>
          <BaseButton
            v-if="entry.before !== null || entry.after !== null"
            variant="link"
            :aria-expanded="opened === index"
            @click="opened = opened === index ? null : index"
            >Voir ce qui a changé</BaseButton
          >
          <div v-if="opened === index" class="team__diff">
            <div>
              <p class="team__when">Avant</p>
              <pre>{{ entry.before ?? "(n'existait pas)" }}</pre>
            </div>
            <div>
              <p class="team__when">Après</p>
              <pre>{{ entry.after ?? "(supprimée)" }}</pre>
            </div>
          </div>
        </div>
      </li>
    </ol>
  </div>
</template>

<style scoped>
.team__received {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  margin: 0 0 0.75rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-accent-soft);
}

.team__error {
  color: var(--color-danger);
}

.team__empty,
.team__when {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.team__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.team__entry {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.team__avatar {
  display: inline-grid;
  flex: none;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  background: var(--color-accent-soft);
}

.team__body {
  flex: 1;
  min-width: 0;
}

.team__line {
  margin: 0;
}

.team__message {
  color: var(--color-muted);
}

.team__diff {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.team__diff pre {
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
