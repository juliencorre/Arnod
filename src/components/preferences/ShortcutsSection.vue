<script setup lang="ts">
/**
 * Préférences → Raccourcis (écran 1.6-f bis, version 1, `docs/plan-v1.md`) : chaque action et
 * sa combinaison, par usage. Un clic sur une combinaison attend la nouvelle (Échap annule) ; le
 * cœur la valide et signale l'action qui la tient déjà, avant de la remplacer (« Remplacer »,
 * « Annuler »). « Rétablir les raccourcis par défaut » revient aux combinaisons d'origine.
 * Ctrl+Tab et Ctrl+0…9 sont fixes.
 */
import { computed, nextTick, ref } from "vue";

import BaseButton from "@/components/ui/BaseButton.vue";
import SearchInput from "@/components/ui/SearchInput.vue";
import { comboOf, formatCombo } from "@/lib/shortcuts";
import { useInterfaceStore } from "@/stores/interface";
import { useToastStore } from "@/stores/toasts";
import type { ShortcutConflict, ShortcutGroup, ShortcutView } from "@/types/ipc";

const shortcuts = useInterfaceStore();
const toasts = useToastStore();

const GROUPS: { id: ShortcutGroup; label: string }[] = [
  { id: "request", label: "Requête et réponse" },
  { id: "window", label: "Fenêtre et onglets" },
  { id: "project", label: "Projet et workspace" },
];

const query = ref("");
/** Action dont la combinaison est attendue. */
const capturing = ref<string | null>(null);
/** Combinaison tapée, déjà prise : en attente de « Remplacer ». */
const conflict = ref<{ id: string; conflict: ShortcutConflict } | null>(null);
/** Refus du cœur (combinaison réservée, non reconnue), sous la ligne concernée. */
const refusal = ref<{ id: string; message: string } | null>(null);

/** Sans casse ni accents, pour la recherche. */
function plain(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

const all = computed<ShortcutView[]>(() => shortcuts.view?.shortcuts ?? []);

/** Actions qui répondent à la recherche, par libellé ou par touche. */
const visible = computed(() => {
  const terms = plain(query.value).split(/\s+/).filter(Boolean);
  return all.value.filter((shortcut) => {
    const text = plain(
      [shortcut.label, ...shortcut.keys, ...shortcut.keys.map(formatCombo)].join(" "),
    );
    return terms.every((term) => text.includes(term));
  });
});

const groups = computed(() =>
  GROUPS.map((group) => ({
    ...group,
    items: visible.value.filter((shortcut) => shortcut.group === group.id),
  })).filter((group) => group.items.length > 0),
);

const changed = computed(() => all.value.some((s) => s.keys.join(" ") !== s.defaults.join(" ")));

function buttonId(id: string): string {
  return `shortcut-${id}`;
}

async function focusButton(id: string): Promise<void> {
  await nextTick();
  document.getElementById(buttonId(id))?.focus();
}

function startCapture(id: string): void {
  capturing.value = id;
  conflict.value = null;
  refusal.value = null;
}

function stopCapture(): void {
  const id = capturing.value;
  capturing.value = null;
  if (id !== null) void focusButton(id);
}

async function assign(id: string, combo: string, replace: boolean): Promise<void> {
  try {
    const found = await shortcuts.setShortcut(id, combo, replace);
    conflict.value = found === null ? null : { id, conflict: found };
    if (found === null) toasts.show(`✓ ${formatCombo(combo)} enregistré.`);
  } catch (error) {
    refusal.value = { id, message: error instanceof Error ? error.message : String(error) };
  }
}

/** Touche pressée pendant l'attente : Échap annule, un modificateur seul attend la suite. */
function onCaptureKeydown(event: KeyboardEvent, id: string): void {
  if (capturing.value !== id) return;
  // La touche ne doit déclencher aucune action, ni fermer les préférences.
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    stopCapture();
    return;
  }
  if (event.key === "Tab" && !(event.ctrlKey || event.metaKey || event.altKey)) {
    // Tab seul garde son rôle : sortir du bouton.
    capturing.value = null;
    return;
  }
  const combo = comboOf(event);
  if (combo === null) return;
  capturing.value = null;
  void focusButton(id);
  void assign(id, combo, false);
}

async function replace(): Promise<void> {
  const pending = conflict.value;
  if (!pending) return;
  await assign(pending.id, pending.conflict.keys, true);
  void focusButton(pending.id);
}

function cancelConflict(): void {
  const pending = conflict.value;
  conflict.value = null;
  if (pending) void focusButton(pending.id);
}

function reset(): void {
  conflict.value = null;
  refusal.value = null;
  shortcuts
    .resetShortcuts()
    .then(() => {
      toasts.show("Raccourcis par défaut rétablis.");
    })
    .catch(toasts.showError);
}

/** « Ctrl+Maj+K », ou « Aucun » pour une action sans raccourci. */
function keysLabel(shortcut: ShortcutView): string {
  return shortcut.keys.length === 0 ? "Aucun" : shortcut.keys.map(formatCombo).join(" ou ");
}
</script>

<template>
  <section class="shortcuts" aria-labelledby="shortcuts-title">
    <div>
      <h3 id="shortcuts-title" class="shortcuts__title">Raccourcis</h3>
      <p class="shortcuts__hint">Propres à ce poste. Un clic sur une combinaison la change.</p>
    </div>

    <div class="shortcuts__tools">
      <SearchInput
        v-model="query"
        class="shortcuts__search"
        label="Rechercher une action ou une touche"
        placeholder="Rechercher une action ou une touche…"
      />
      <BaseButton variant="link" :disabled="!changed" @click="reset">
        Rétablir les raccourcis par défaut
      </BaseButton>
    </div>

    <p v-if="groups.length === 0" class="shortcuts__empty">Aucune action ne correspond.</p>
    <section
      v-for="group in groups"
      :key="group.id"
      class="shortcuts__group"
      :aria-labelledby="`shortcuts-group-${group.id}`"
    >
      <h4 :id="`shortcuts-group-${group.id}`" class="shortcuts__group-title">
        {{ group.label }}
      </h4>
      <ul class="shortcuts__list">
        <li v-for="shortcut in group.items" :key="shortcut.id" class="shortcuts__item">
          <div class="shortcuts__row">
            <span :id="`shortcut-label-${shortcut.id}`" class="shortcuts__label">
              {{ shortcut.label }}
            </span>
            <button
              v-if="shortcut.editable"
              :id="buttonId(shortcut.id)"
              type="button"
              class="shortcuts__keys"
              :class="{
                'shortcuts__keys--capturing': capturing === shortcut.id,
                'shortcuts__keys--none': shortcut.keys.length === 0,
              }"
              :aria-describedby="`shortcut-label-${shortcut.id}`"
              :aria-label="
                capturing === shortcut.id
                  ? `Appuyez sur la nouvelle combinaison de ${shortcut.label}, Échap pour annuler`
                  : `${shortcut.label} : ${keysLabel(shortcut)}. Changer`
              "
              @click="capturing === shortcut.id ? stopCapture() : startCapture(shortcut.id)"
              @keydown="onCaptureKeydown($event, shortcut.id)"
              @blur="capturing === shortcut.id && (capturing = null)"
            >
              <template v-if="capturing === shortcut.id">Appuyez sur une combinaison…</template>
              <template v-else-if="shortcut.keys.length === 0">Aucun</template>
              <template v-else>
                <kbd v-for="keys in shortcut.keys" :key="keys" class="shortcuts__kbd">{{
                  formatCombo(keys)
                }}</kbd>
              </template>
            </button>
            <span v-else class="shortcuts__fixed" :title="'Raccourci fixe'">
              <kbd v-for="keys in shortcut.keys" :key="keys" class="shortcuts__kbd">{{
                formatCombo(keys)
              }}</kbd>
              <span class="shortcuts__note">fixe</span>
            </span>
          </div>
          <p v-if="conflict?.id === shortcut.id" class="shortcuts__conflict" role="alert">
            <span>
              {{ formatCombo(conflict.conflict.keys) }} est déjà pris par «
              {{ conflict.conflict.label }} ».
            </span>
            <BaseButton @click="replace">Remplacer</BaseButton>
            <BaseButton variant="link" @click="cancelConflict">Annuler</BaseButton>
          </p>
          <p v-if="refusal?.id === shortcut.id" class="shortcuts__refusal" role="alert">
            {{ refusal.message }}
          </p>
        </li>
      </ul>
    </section>
  </section>
</template>

<style scoped>
.shortcuts {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.shortcuts__title {
  margin: 0;
  font-size: var(--text-title);
  font-weight: 600;
}

.shortcuts__hint {
  margin: 0.25rem 0 0;
  color: var(--color-muted);
}

.shortcuts__tools {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.shortcuts__search {
  flex: 1 1 10rem;
  min-width: 0;
  max-width: 18rem;
}

.shortcuts__empty {
  margin: 0;
  color: var(--color-muted);
}

.shortcuts__group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.shortcuts__group-title {
  margin: 0;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.shortcuts__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.shortcuts__item {
  border-bottom: 1px solid var(--color-border);
}

.shortcuts__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  min-height: var(--control-md);
  padding: 0.125rem 0;
}

.shortcuts__label {
  min-width: 0;
}

.shortcuts__keys {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  min-height: var(--control-sm);
  padding: 0 0.375rem;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font: inherit;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.shortcuts__keys:hover {
  border-color: var(--color-border-strong);
}

.shortcuts__keys:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.shortcuts__keys--capturing {
  border-color: var(--color-accent);
  color: var(--color-muted);
  background-color: var(--color-accent-soft);
}

.shortcuts__keys--none {
  color: var(--color-muted);
}

.shortcuts__fixed {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0 0.375rem;
}

.shortcuts__kbd {
  padding: 0.0625rem 0.375rem;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  font: var(--text-small) var(--font-mono);
  background-color: var(--color-surface);
}

.shortcuts__note {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.shortcuts__conflict {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  margin: 0 0 0.5rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  background-color: var(--color-accent-soft);
}

.shortcuts__refusal {
  margin: 0 0 0.5rem;
  font-size: var(--text-small);
  color: var(--color-danger);
}
</style>
