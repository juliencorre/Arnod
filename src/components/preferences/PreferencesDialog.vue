<script setup lang="ts">
/**
 * Préférences du poste (1.6-e), ouvertes par Ctrl+, ou par Fichier → Préférences… : une colonne
 * de sections, puis la section choisie : « Apparence » (1.6-f) et « Raccourcis » (1.6-f bis,
 * version 1, docs/plan-v1.md), « Réseau et certificats » (lot H, docs/plan-mvp-h-reseau-auth.md,
 * PR1) et « Mises à jour » (lot L allégé, 1.6-g).
 */
import { onBeforeUnmount, onMounted } from "vue";

import AppearanceSection from "@/components/preferences/AppearanceSection.vue";
import NetworkSection from "@/components/preferences/NetworkSection.vue";
import ShortcutsSection from "@/components/preferences/ShortcutsSection.vue";
import UpdatesSection from "@/components/preferences/UpdatesSection.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import { events } from "@/lib/ipc";
import { useInterfaceStore } from "@/stores/interface";
import { type PreferencesSection, useNetworkStore } from "@/stores/network";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";

const network = useNetworkStore();
const windows = useWindowsStore();
const toasts = useToastStore();
const shortcuts = useInterfaceStore();

const SECTIONS: { id: PreferencesSection; label: string }[] = [
  { id: "appearance", label: "Apparence" },
  { id: "network", label: "Réseau et certificats" },
  { id: "shortcuts", label: "Raccourcis" },
  { id: "updates", label: "Mises à jour" },
];

function open(): void {
  network.openPreferences().catch(toasts.showError);
}

/** Ctrl+, par défaut : la virgule tapée, quelle que soit la disposition du clavier (1.6-f bis). */
function onKeydown(event: KeyboardEvent): void {
  if (shortcuts.matches(event, "preferences")) {
    event.preventDefault();
    open();
  }
}

let unlisten: (() => void) | null = null;
onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  events
    .onPreferences((label) => {
      if (label === windows.label) open();
    })
    .then((stop) => {
      unlisten = stop;
    })
    .catch(() => undefined);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  unlisten?.();
});
</script>

<template>
  <BaseDialog
    :open="network.preferencesOpen"
    title="Préférences"
    wide
    closable
    @close="network.closePreferences()"
  >
    <div class="preferences">
      <nav class="preferences__nav" aria-label="Sections des préférences">
        <button
          v-for="item in SECTIONS"
          :key="item.id"
          type="button"
          class="preferences__section"
          :aria-current="network.section === item.id ? 'page' : undefined"
          @click="network.section = item.id"
        >
          {{ item.label }}
        </button>
        <p class="preferences__note">Enregistrées automatiquement. Propres à ce poste.</p>
      </nav>
      <div class="preferences__content">
        <AppearanceSection v-if="network.section === 'appearance'" />
        <NetworkSection v-else-if="network.section === 'network'" />
        <ShortcutsSection v-else-if="network.section === 'shortcuts'" />
        <UpdatesSection v-else-if="network.section === 'updates'" />
      </div>
    </div>
  </BaseDialog>
</template>

<style scoped>
.preferences {
  display: grid;
  grid-template-columns: 11rem minmax(0, 1fr);
  gap: 1.25rem;
  min-height: 24rem;
}

.preferences__nav {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding-right: 1rem;
  border-right: 1px solid var(--color-border);
}

.preferences__section {
  min-height: var(--control-md);
  padding: 0 0.625rem;
  border: 0;
  border-radius: var(--radius-md);
  font: inherit;
  text-align: left;
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.preferences__section[aria-current="page"] {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-sidebar);
}

.preferences__section:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.preferences__note {
  margin: auto 0 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.preferences__content {
  min-width: 0;
}
</style>
