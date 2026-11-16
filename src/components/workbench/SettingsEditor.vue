<script setup lang="ts">
/**
 * Réglages d'un endpoint (EP7) : délai, redirections, encodage. « Hérité » reprend le réglage
 * du dossier, puis de la collection, puis le défaut d'Arnod (R3) ; sa valeur est affichée.
 */
import { computed } from "vue";

import type { RequestSettings, ResolvedSettings } from "@/types/ipc";

const props = defineProps<{ inherited: ResolvedSettings }>();
const settings = defineModel<RequestSettings>({ required: true });

type Key = keyof RequestSettings;

interface Row {
  key: Key;
  label: string;
  hint: string;
  options: { value: number | boolean | null; label: string }[];
}

function seconds(ms: number): string {
  return ms === 0 ? "aucun" : `${String(Math.round(ms / 100) / 10).replace(".", ",")} s`;
}

function yesNo(value: boolean): string {
  return value ? "oui" : "non";
}

/** Une valeur propre absente des choix proposés est ajoutée, pour rester visible. */
function withCurrent(
  options: Row["options"],
  current: number | boolean | null,
  label: (value: number) => string,
): Row["options"] {
  if (typeof current === "number" && !options.some((o) => o.value === current)) {
    return [...options, { value: current, label: label(current) }];
  }
  return options;
}

const rows = computed<Row[]>(() => {
  const s = settings.value;
  const i = props.inherited;
  return [
    {
      key: "timeoutMs",
      label: "Délai maximal",
      hint: "Au-delà, la requête échoue.",
      options: withCurrent(
        [
          { value: null, label: `Hérité · ${seconds(i.timeoutMs)}` },
          { value: 5000, label: "5 s" },
          { value: 10000, label: "10 s" },
          { value: 60000, label: "60 s" },
          { value: 0, label: "Aucun" },
        ],
        s.timeoutMs,
        seconds,
      ),
    },
    {
      key: "followRedirects",
      label: "Suivre les redirections",
      hint: "Codes 301, 302, 307, 308.",
      options: [
        { value: null, label: `Hérité · ${yesNo(i.followRedirects)}` },
        { value: true, label: "Oui" },
        { value: false, label: "Non" },
      ],
    },
    {
      key: "maxRedirects",
      label: "Redirections au plus",
      hint: "Évite les boucles.",
      options: withCurrent(
        [
          { value: null, label: `Hérité · ${String(i.maxRedirects)}` },
          { value: 1, label: "1" },
          { value: 3, label: "3" },
          { value: 10, label: "10" },
        ],
        s.maxRedirects,
        String,
      ),
    },
    {
      key: "encodeUrl",
      label: "Encoder l'adresse",
      hint: "Espaces et accents dans le chemin et les paramètres.",
      options: [
        { value: null, label: `Hérité · ${yesNo(i.encodeUrl)}` },
        { value: true, label: "Oui" },
        { value: false, label: "Non" },
      ],
    },
  ];
});

function choose(key: Key, value: number | boolean | null): void {
  const current = settings.value;
  if (key === "followRedirects" || key === "encodeUrl") {
    current[key] = typeof value === "boolean" ? value : null;
  } else {
    current[key] = typeof value === "number" ? value : null;
  }
}
</script>

<template>
  <div class="settings">
    <div v-for="row in rows" :key="row.key" class="setting">
      <div class="setting__text">
        <span :id="`setting-${row.key}`" class="setting__label">{{ row.label }}</span>
        <span class="setting__hint">{{ row.hint }}</span>
      </div>
      <div class="setting__choices" role="group" :aria-labelledby="`setting-${row.key}`">
        <button
          v-for="option in row.options"
          :key="String(option.value)"
          type="button"
          class="setting__choice"
          :aria-pressed="settings[row.key] === option.value"
          @click="choose(row.key, option.value)"
        >
          {{ option.label }}
        </button>
      </div>
    </div>
    <p class="hint">« Hérité » reprend le réglage du dossier, puis celui de la collection.</p>
  </div>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.setting {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem 1rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.setting__text {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.setting__label {
  font-size: var(--text-body);
  font-weight: 600;
}

.setting__hint,
.hint {
  margin: 0;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.setting__choices {
  display: flex;
  flex-wrap: wrap;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.setting__choice {
  padding: 0.3125rem 0.625rem;
  border: 0;
  border-left: 1px solid var(--color-border);
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-muted);
  background: var(--color-surface);
  cursor: pointer;
}

.setting__choice:first-child {
  border-left: 0;
}

.setting__choice[aria-pressed="true"] {
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-chip);
}

.setting__choice:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}
</style>
