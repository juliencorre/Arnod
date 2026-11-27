<script setup lang="ts">
/**
 * Préférences → Apparence (écran 1.6-f, version 1, `docs/plan-v1.md`) : thème du système ou
 * forcé, zoom de l'interface, police et taille des éditeurs. Le cœur ne propose que des valeurs
 * qu'il accepte, enregistre et applique à toutes les fenêtres ; tout s'enregistre seul.
 */
import { computed, onBeforeUnmount, onMounted, ref, useId } from "vue";

import { useInterfaceStore } from "@/stores/interface";
import { useToastStore } from "@/stores/toasts";
import type { Appearance, InterfaceTheme } from "@/types/ipc";

const shortcuts = useInterfaceStore();
const toasts = useToastStore();
const ids = { zoom: useId(), font: useId(), size: useId(), zoomHint: useId() };

const appearance = computed(() => shortcuts.appearance);
const zooms = computed(() => shortcuts.view?.zooms ?? [100]);
const fonts = computed(() => shortcuts.view?.fonts ?? []);
const sizes = computed(() => shortcuts.view?.sizes ?? [13]);

/** Thème du système en ce moment, pour « Comme le système ». */
const systemDark = ref(false);
let media: MediaQueryList | null = null;
function follow(): void {
  systemDark.value = media?.matches ?? false;
}
onMounted(() => {
  media =
    typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-color-scheme: dark)")
      : null;
  follow();
  media?.addEventListener("change", follow);
});
onBeforeUnmount(() => {
  media?.removeEventListener("change", follow);
});

const THEMES = computed<{ id: InterfaceTheme; label: string; detail: string }[]>(() => [
  {
    id: "system",
    label: "Comme le système",
    detail: systemDark.value ? "sombre en ce moment" : "clair en ce moment",
  },
  { id: "light", label: "Clair", detail: "" },
  { id: "dark", label: "Sombre", detail: "" },
]);

function save(change: Partial<Appearance>): void {
  shortcuts.saveAppearance(change).catch(toasts.showError);
}

function onZoom(event: Event): void {
  save({ zoom: Number((event.target as HTMLSelectElement).value) });
}

function onFont(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  save({ editorFont: value === "" ? null : value });
}

function onSize(event: Event): void {
  save({ editorFontSize: Number((event.target as HTMLSelectElement).value) });
}
</script>

<template>
  <section class="appearance" aria-labelledby="appearance-title">
    <div>
      <h3 id="appearance-title" class="appearance__title">Apparence</h3>
      <p class="appearance__hint">Réglages de ce poste.</p>
    </div>

    <fieldset class="appearance__group">
      <legend class="appearance__legend">Thème</legend>
      <div class="appearance__themes">
        <label
          v-for="theme in THEMES"
          :key="theme.id"
          class="appearance__theme"
          :class="{ 'appearance__theme--on': appearance.theme === theme.id }"
        >
          <input
            type="radio"
            name="appearance-theme"
            class="appearance__radio"
            :checked="appearance.theme === theme.id"
            @change="save({ theme: theme.id })"
          />
          <span class="appearance__label">{{ theme.label }}</span>
          <span v-if="theme.detail" class="appearance__detail">{{ theme.detail }}</span>
        </label>
      </div>
    </fieldset>

    <div class="appearance__group">
      <label :for="ids.zoom" class="appearance__legend">Zoom de l'interface</label>
      <div class="appearance__row">
        <select
          :id="ids.zoom"
          class="appearance__select"
          :value="appearance.zoom"
          :aria-describedby="ids.zoomHint"
          @change="onZoom"
        >
          <option v-for="zoom in zooms" :key="zoom" :value="zoom">{{ zoom }} %</option>
        </select>
        <span :id="ids.zoomHint" class="appearance__detail">
          Ctrl + et Ctrl − ; Ctrl 0 revient à 100 %
        </span>
      </div>
    </div>

    <div class="appearance__group">
      <span class="appearance__legend">Police des éditeurs</span>
      <div class="appearance__row">
        <label :for="ids.font" class="visually-hidden">Police des éditeurs</label>
        <select
          :id="ids.font"
          class="appearance__select appearance__select--wide"
          :value="appearance.editorFont ?? ''"
          @change="onFont"
        >
          <option value="">Police du système</option>
          <option v-for="font in fonts" :key="font" :value="font">{{ font }}</option>
        </select>
        <label :for="ids.size" class="visually-hidden">Taille du texte des éditeurs</label>
        <select
          :id="ids.size"
          class="appearance__select"
          :value="appearance.editorFontSize"
          @change="onSize"
        >
          <option v-for="size in sizes" :key="size" :value="size">{{ size }} px</option>
        </select>
      </div>
      <p class="appearance__sample">{ "id": "ord_8F2kQ", "status": "pending" }</p>
      <p class="appearance__detail">
        Corps des requêtes et des réponses, contrats, envoyé. Une police absente du poste laisse la
        place à la police à chasse fixe du système.
      </p>
    </div>
  </section>
</template>

<style scoped>
.appearance {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.appearance__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.appearance__hint {
  margin: 0.25rem 0 0;
  color: var(--color-muted);
}

.appearance__group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0;
  padding: 0;
  border: 0;
}

.appearance__legend {
  padding: 0;
  font-weight: 600;
}

.appearance__themes {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.appearance__theme {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--control-md);
  padding: 0 0.75rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  cursor: pointer;
}

.appearance__theme--on {
  border-color: var(--color-accent);
  background-color: var(--color-accent-soft);
}

.appearance__theme:has(:focus-visible) {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.appearance__radio {
  margin: 0;
}

.appearance__label {
  font-weight: 600;
}

.appearance__detail {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.appearance__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.appearance__select {
  min-width: 6rem;
}

.appearance__select--wide {
  min-width: 12rem;
}

.appearance__sample {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: var(--editor-font-size) / 1.5 var(--editor-font);
  background-color: var(--color-surface);
}
</style>
