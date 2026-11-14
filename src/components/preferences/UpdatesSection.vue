<script setup lang="ts">
/**
 * Préférences → Mises à jour (lot L allégé, écran 1.6-g) : version installée, version plus
 * récente trouvée et ses points, vérification au démarrage puis chaque jour, « Vérifier
 * maintenant », canal stable ou préversions. Arnod ne télécharge ni n'installe rien : « Voir la
 * version » ouvre la page de la release ; l'installeur se lance à la main, sans droits
 * administrateur. Tout s'enregistre seul.
 */
import { computed, onMounted } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import { useToastStore } from "@/stores/toasts";
import { useUpdatesStore } from "@/stores/updates";
import type { UpdateChannel } from "@/types/ipc";

const updates = useUpdatesStore();
const toasts = useToastStore();

const view = computed(() => updates.view);
const latest = computed(() => view.value?.latest ?? null);

onMounted(() => {
  updates.load().catch(toasts.showError);
});

/** « aujourd'hui 09:12 », « hier 18:40 » ou « 12 novembre 2026, 09:12 ». */
function when(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return "jamais";
  const date = new Date(ms);
  const time = date.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  const today = new Date();
  const yesterday = new Date(today.getTime() - 86_400_000);
  if (date.toDateString() === today.toDateString()) return `aujourd'hui ${time}`;
  if (date.toDateString() === yesterday.toDateString()) return `hier ${time}`;
  return `${date.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" })}, ${time}`;
}

/** « 14 novembre 2026 ». */
function day(iso: string): string {
  const date = new Date(`${iso}T12:00:00`);
  return Number.isNaN(date.getTime())
    ? iso
    : date.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

function setAutomatic(event: Event): void {
  const v = view.value;
  if (!v) return;
  updates.save((event.target as HTMLInputElement).checked, v.prefs.channel).catch(toasts.showError);
}

function setChannel(channel: UpdateChannel): void {
  const v = view.value;
  if (!v || v.prefs.channel === channel) return;
  updates.save(v.prefs.automatic, channel).catch(toasts.showError);
}

function openRelease(): void {
  const url = latest.value?.url;
  if (url) updates.open(url).catch(toasts.showError);
}
</script>

<template>
  <section class="updates" aria-labelledby="updates-title">
    <div>
      <h3 id="updates-title" class="updates__title">Mises à jour</h3>
      <p class="updates__hint">
        Arnod {{ view?.current ?? "" }}, installée sur ce poste. Réglages de ce poste.
      </p>
    </div>

    <div v-if="latest" class="updates__card" role="status">
      <p class="updates__card-title">
        <span class="updates__dot" aria-hidden="true">●</span>
        Arnod {{ latest.version }} est disponible{{ latest.prerelease ? " (préversion)" : "" }}
      </p>
      <p class="updates__card-text">
        Publiée{{ latest.published ? ` le ${day(latest.published)}` : "" }}. Téléchargez
        l'installeur depuis sa page : il s'installe dans votre profil, sans droits administrateur,
        et Arnod rouvre vos projets, onglets et brouillons.
      </p>
      <ul v-if="latest.highlights.length > 0" class="updates__list">
        <li v-for="item in latest.highlights" :key="item">{{ item }}</li>
      </ul>
      <div class="updates__actions">
        <BaseButton variant="primary" @click="openRelease">Voir la version</BaseButton>
      </div>
    </div>
    <p v-else-if="view?.checked" class="updates__uptodate" role="status">
      ✓ Arnod est à jour sur le canal
      {{ view.prefs.channel === "stable" ? "stable" : "des préversions" }}.
    </p>

    <fieldset class="updates__group">
      <legend class="updates__legend">Vérification</legend>
      <label class="updates__choice">
        <input type="checkbox" :checked="view?.prefs.automatic ?? true" @change="setAutomatic" />
        <span>
          <span class="updates__label"
            >Chercher une nouvelle version au démarrage, puis chaque jour</span
          >
          <span class="updates__detail">
            Arnod lit seulement la liste des versions publiées ; rien ne se télécharge ni ne
            s'installe sans vous.
          </span>
        </span>
      </label>
      <p class="updates__line">
        Dernière vérification : {{ when(view?.prefs.lastCheck) }} ·
        <button
          type="button"
          class="updates__link"
          :disabled="updates.checking"
          @click="updates.check()"
        >
          {{ updates.checking ? "Vérification…" : "Vérifier maintenant" }}
        </button>
      </p>
      <p v-if="updates.error" class="updates__error" role="alert">{{ updates.error }}</p>
    </fieldset>

    <fieldset class="updates__group">
      <legend class="updates__legend">Canal</legend>
      <label class="updates__choice">
        <input
          type="radio"
          name="updates-channel"
          :checked="(view?.prefs.channel ?? 'stable') === 'stable'"
          @change="setChannel('stable')"
        />
        <span>
          <span class="updates__label">Stable</span>
          <span class="updates__detail">Recommandé.</span>
        </span>
      </label>
      <label class="updates__choice">
        <input
          type="radio"
          name="updates-channel"
          :checked="view?.prefs.channel === 'prerelease'"
          @change="setChannel('prerelease')"
        />
        <span>
          <span class="updates__label">Préversions</span>
          <span class="updates__detail">
            Les nouveautés une à deux semaines plus tôt, moins éprouvées.
          </span>
        </span>
      </label>
    </fieldset>

    <p class="updates__note">
      La vérification passe par le proxy et les autorités de « Réseau et certificats ».
    </p>
  </section>
</template>

<style scoped>
.updates {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.updates__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.updates__hint {
  margin: 0.25rem 0 0;
  color: var(--color-muted);
}

.updates__card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-lg);
}

.updates__card-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
  font-weight: 600;
}

.updates__dot {
  color: var(--color-accent);
}

.updates__card-text {
  margin: 0;
  line-height: 1.5;
}

.updates__list {
  margin: 0;
  padding-left: 1.25rem;
  line-height: 1.6;
}

.updates__actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.25rem;
}

.updates__uptodate {
  margin: 0;
  color: var(--color-success);
}

.updates__group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0;
  padding: 0;
  border: 0;
}

.updates__legend {
  margin-bottom: 0.25rem;
  padding: 0;
  font-weight: 600;
}

.updates__choice {
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  cursor: pointer;
}

.updates__choice input {
  margin-top: 0.15rem;
}

.updates__label,
.updates__detail {
  display: block;
}

.updates__detail {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.updates__line {
  margin: 0 0 0 1.6rem;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.updates__link {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-accent-text);
  cursor: pointer;
}

.updates__link:disabled {
  cursor: default;
  opacity: 0.6;
}

.updates__link:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}

.updates__error {
  margin: 0 0 0 1.6rem;
  font-size: var(--text-small);
  color: var(--color-danger);
}

.updates__note {
  margin: 0;
  font-size: var(--text-small);
  line-height: 1.5;
  color: var(--color-muted);
}
</style>
