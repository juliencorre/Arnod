<script setup lang="ts">
/**
 * Synchroniser (`docs/plan-v2.md`, lot E, 7.1-b, SY5, SY6) : ce qui arrive des collègues
 * (élément, ce qui a changé, auteur, quand), mes modifications par élément, à cocher, un
 * message, puis « Récupérer puis envoyer » ; « Envoyer seulement » quand rien n'arrive. Tout
 * se décrit en clair (R14) : jamais un fichier ni un commit.
 */
import { computed, ref, watch } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { useGuard } from "@/composables/useGuard";
import { countLabel, formatAgo } from "@/lib/format";
import { forgeBadge, initial, kindLabel } from "@/lib/sync";
import { useSyncStore } from "@/stores/sync";

const sync = useSyncStore();
const guard = useGuard();

const view = computed(() => (sync.panel ? (sync.views[sync.panel] ?? null) : null));
const chosen = ref<string[]>([]);
const message = ref("");

// À chaque ouverture : toutes mes modifications cochées, message vide.
watch(
  () => sync.panel,
  () => {
    chosen.value = view.value?.outgoing.map((item) => item.key) ?? [];
    message.value = "";
  },
);
// Une modification qui disparaît (envoyée, annulée) n'est plus cochée.
watch(
  () => view.value?.outgoing.map((item) => item.key) ?? [],
  (keys) => {
    chosen.value = chosen.value.filter((key) => keys.includes(key));
  },
);

const checked = computed(() =>
  view.value?.checkedAt ? `vérifié ${formatAgo(view.value.checkedAt)}` : "jamais vérifié",
);
const incoming = computed(() => view.value?.incoming ?? []);
const outgoing = computed(() => view.value?.outgoing ?? []);

function close(): void {
  sync.panel = null;
}

function run(pull: boolean, push: boolean): Promise<void> {
  const id = sync.panel;
  if (!id) return Promise.resolve();
  return sync.pullPush(id, push ? chosen.value : [], message.value, pull, push).then(() => {
    if (push && sync.conflicts === null) close();
  });
}
</script>

<template>
  <BaseDialog :open="view !== null" title="Synchronisation" wide closable @close="close">
    <template v-if="view?.remote">
      <p class="panel__checked">{{ checked }}</p>
      <p v-if="view.error" class="panel__error" role="alert">
        Dernière vérification impossible : {{ view.error }}
      </p>
      <div class="panel__repo">
        <span class="panel__forge" aria-hidden="true">{{ forgeBadge(view.remote.forge) }}</span>
        <span class="panel__repo-name">{{ view.remote.name.replace("/", " / ") }}</span>
        <span class="panel__muted">{{ view.remote.host }} · branche {{ view.remote.branch }}</span>
        <span class="panel__spacer" />
        <span v-if="view.members.length > 0" class="panel__members">
          <span
            v-for="member in view.members.slice(0, 4)"
            :key="member"
            class="panel__avatar"
            :title="member"
            aria-hidden="true"
            >{{ initial(member) }}</span
          >
          <span class="panel__muted">{{
            countLabel(view.members.length, "membre", "membres")
          }}</span>
        </span>
      </div>

      <section class="panel__section" aria-labelledby="sync-incoming">
        <div class="panel__head">
          <h3 id="sync-incoming" class="panel__title">À récupérer · {{ incoming.length }}</h3>
          <BaseButton
            :disabled="sync.busy || incoming.length === 0"
            @click="guard(() => run(true, false))"
            >Récupérer</BaseButton
          >
        </div>
        <p v-if="incoming.length === 0" class="panel__muted">Rien de nouveau dans l'équipe.</p>
        <ul v-else class="panel__list">
          <li v-for="item in incoming" :key="item.key" class="panel__item">
            <span class="panel__kind">{{ kindLabel(item.kind) }}</span>
            <span class="panel__label">{{ item.label }}</span>
            <span class="panel__detail">{{ item.detail }}</span>
            <span class="panel__spacer" />
            <span v-if="item.author" class="panel__muted">
              {{ item.author }}<template v-if="item.at"> · {{ formatAgo(item.at) }}</template>
            </span>
          </li>
        </ul>
      </section>

      <section class="panel__section" aria-labelledby="sync-outgoing">
        <div class="panel__head">
          <h3 id="sync-outgoing" class="panel__title">Mes modifications · {{ outgoing.length }}</h3>
          <span class="panel__muted">{{
            countLabel(chosen.length, "sélectionnée", "sélectionnées")
          }}</span>
        </div>
        <p v-if="outgoing.length === 0" class="panel__muted">Aucune modification à envoyer.</p>
        <ul v-else class="panel__list">
          <li v-for="item in outgoing" :key="item.key" class="panel__item">
            <label class="panel__check">
              <input v-model="chosen" type="checkbox" :value="item.key" />
              <span class="panel__kind">{{ kindLabel(item.kind) }}</span>
              <span class="panel__label">{{ item.label }}</span>
              <span class="panel__detail">{{ item.detail }}</span>
            </label>
          </li>
        </ul>
        <label class="panel__message">
          <span class="visually-hidden">Message de l'envoi</span>
          <input
            v-model="message"
            class="panel__input"
            type="text"
            placeholder="Message : ce que ces modifications apportent"
            :disabled="outgoing.length === 0"
          />
        </label>
      </section>
    </template>

    <template #footer>
      <p class="panel__note">
        Les secrets (variables locales) ne quittent jamais votre poste. Arnod récupère toujours
        avant d'envoyer.
      </p>
      <div class="panel__actions">
        <BaseButton
          v-if="incoming.length === 0"
          :disabled="sync.busy || chosen.length === 0"
          @click="guard(() => run(false, true))"
          >Envoyer seulement</BaseButton
        >
        <BaseButton
          variant="primary"
          :disabled="sync.busy || (chosen.length === 0 && incoming.length === 0)"
          @click="guard(() => run(true, chosen.length > 0))"
          >{{ chosen.length > 0 ? "Récupérer puis envoyer" : "Récupérer" }}</BaseButton
        >
      </div>
    </template>
  </BaseDialog>
</template>

<style scoped>
.panel__checked,
.panel__muted {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.panel__error {
  margin: 0.5rem 0 0;
  font-size: var(--text-small);
  color: var(--color-danger);
}

.panel__repo {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  margin: 0.75rem 0 1rem;
}

.panel__forge,
.panel__kind {
  padding: 0 0.375rem;
  border-radius: var(--radius-md);
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-muted);
  background: var(--color-chip);
}

.panel__repo-name,
.panel__label {
  font-weight: 600;
}

.panel__spacer {
  flex: 1;
}

.panel__members {
  display: inline-flex;
  gap: 0.25rem;
  align-items: center;
}

.panel__avatar {
  display: inline-grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  font-weight: 600;
  color: var(--color-accent-text);
  background: var(--color-accent-soft);
}

.panel__section {
  margin-bottom: 1rem;
}

.panel__head {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.25rem;
}

.panel__title {
  margin: 0;
  font-size: var(--text-body);
  font-weight: 600;
}

.panel__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.panel__item,
.panel__check {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  min-height: var(--control-sm);
}

.panel__check {
  flex: 1;
  cursor: pointer;
}

.panel__detail {
  color: var(--color-muted);
}

.panel__message {
  display: block;
  margin-top: 0.5rem;
}

.panel__input {
  box-sizing: border-box;
  width: 100%;
  height: var(--control-md);
  padding: 0 0.5rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background: var(--color-surface);
}

.panel__note {
  flex: 1;
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.panel__actions {
  display: flex;
  gap: 0.5rem;
}
</style>
