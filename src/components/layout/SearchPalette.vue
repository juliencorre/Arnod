<script setup lang="ts">
/**
 * Recherche Ctrl+K (écrans 1.5-a et 1.5-b, `docs/plan-v2.md`, lot A, RH1 à RH6) : une palette
 * au-dessus de la zone. Le cœur cherche par nom ou par adresse et groupe les résultats ; la
 * palette les montre, surligne la saisie et ouvre le résultat choisi : Entrée dans la zone
 * active, Ctrl+Entrée dans une autre zone (créée à droite s'il n'y en a qu'une), Échap ferme et
 * rend le focus. Motif combobox : le champ garde le focus, ↑ ↓ choisissent.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, useId, useTemplateRef, watch } from "vue";

import { useProjectActions } from "@/composables/useProjectActions";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useInterfaceStore } from "@/stores/interface";
import { useSearchStore } from "@/stores/search";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";
import type { ContentRef, SearchHit, SearchKind } from "@/types/ipc";

const search = useSearchStore();
const shortcuts = useInterfaceStore();
const catalog = useCatalogStore();
const windows = useWindowsStore();
const workbench = useWorkbenchStore();
const toasts = useToastStore();
const projectActions = useProjectActions();
const input = useTemplateRef<HTMLInputElement>("input");
const listId = useId();
const optionId = (index: number): string => `${listId}-${String(index)}`;
/** Élément qui avait le focus à l'ouverture, qui le retrouve à la fermeture. */
let returnFocus: HTMLElement | null = null;

/** « Workspace E-commerce · 2 projets », ou « Tous les projets · 5 projets ». */
const scopeLabel = computed(() => {
  const id = search.scope;
  const workspace = id === null ? null : catalog.workspaceById(id);
  const count = workspace ? workspace.projectIds.length : (catalog.snapshot?.projects.length ?? 0);
  const name = workspace ? `Workspace ${workspace.name}` : "Tous les projets";
  return `${name} · ${countLabel(count, "projet", "projets")}`;
});

const KIND_LABELS: Partial<Record<SearchKind, string>> = {
  test: "TEST",
  folder: "DOSSIER",
  contract: "CONTRAT",
  environment: "ENV.",
  project: "PROJET",
};

/** Libellé coupé en passages surlignés ou non (positions en caractères). */
function parts(hit: SearchHit): { text: string; mark: boolean }[] {
  const chars = Array.from(hit.label);
  const out: { text: string; mark: boolean }[] = [];
  let at = 0;
  for (const [start, end] of hit.highlight) {
    if (start < at) continue;
    if (start > at) out.push({ text: chars.slice(at, start).join(""), mark: false });
    out.push({ text: chars.slice(start, end).join(""), mark: true });
    at = end;
  }
  if (at < chars.length) out.push({ text: chars.slice(at).join(""), mark: false });
  return out;
}

/** Rang d'un résultat parmi tous ceux montrés (pour ↑ ↓ et `aria-activedescendant`). */
function indexOf(group: number, hit: number): number {
  const groups = search.results?.groups ?? [];
  return groups.slice(0, group).reduce((n, g) => n + g.hits.length, 0) + hit;
}

/** Contenu qu'ouvre un résultat ; `null` pour un projet ou un environnement. */
function contentOf(hit: SearchHit): ContentRef | null {
  const projectId = hit.projectId;
  switch (hit.kind) {
    case "request":
      return hit.path === null ? null : { kind: "request", projectId, path: hit.path };
    case "test":
      return hit.path === null ? null : { kind: "test", projectId, path: hit.path };
    case "folder":
      return hit.path === null ? null : { kind: "folder", projectId, path: hit.path };
    case "operation":
      if (hit.path !== null) return { kind: "request", projectId, path: hit.path };
      return hit.contract === null ? null : { kind: "contract", projectId, path: hit.contract };
    case "contract":
      return hit.path === null ? null : { kind: "contract", projectId, path: hit.path };
    default:
      return null;
  }
}

/** Ctrl+Entrée : une autre zone de la fenêtre, ou une nouvelle à droite de la zone active. */
async function openElsewhere(content: ContentRef): Promise<void> {
  const active = windows.activeZone;
  const other = windows.zones.find((zone) => zone.id !== active?.id);
  if (other) {
    await windows.open(content, other.id);
    return;
  }
  await windows.open(content, active?.id ?? null);
  const tab = windows.tabFor(content);
  if (active && tab) await windows.split(active.id, tab.id, "right");
}

async function openHit(hit: SearchHit, elsewhere: boolean): Promise<void> {
  close(false);
  const content = contentOf(hit);
  try {
    if (content !== null) {
      if (elsewhere) await openElsewhere(content);
      else await windows.open(content);
    } else if (hit.kind === "environment") {
      await workbench.openEnvironments(hit.projectId, hit.label);
    } else if (hit.kind === "project") {
      await projectActions.openProject(hit.projectId, search.scope);
    }
  } catch (caught) {
    toasts.showError(caught);
  }
}

function show(): void {
  if (!search.open) {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }
  search.show();
  void nextTick(() => {
    input.value?.focus();
    input.value?.select();
  });
}

function close(restore = true): void {
  search.close();
  if (restore) returnFocus?.focus();
  returnFocus = null;
}

function onInputKeydown(event: KeyboardEvent): void {
  switch (event.key) {
    case "ArrowDown":
    case "ArrowUp":
      event.preventDefault();
      search.move(event.key === "ArrowDown" ? 1 : -1);
      break;
    case "Enter": {
      event.preventDefault();
      const hit = search.current;
      if (hit) void openHit(hit, event.ctrlKey || event.metaKey);
      break;
    }
    case "Escape":
      event.preventDefault();
      event.stopPropagation();
      close();
      break;
    case "Tab":
      // Le champ garde le focus tant que la palette est ouverte.
      event.preventDefault();
      break;
  }
}

/** Ctrl+K par défaut, d'après les raccourcis du poste (1.6-f bis). */
function onGlobalKeydown(event: KeyboardEvent): void {
  if (event.defaultPrevented || !shortcuts.matches(event, "search")) return;
  event.preventDefault();
  show();
}

// Le résultat choisi reste visible pendant ↑ ↓.
watch(
  () => search.selected,
  (index) => {
    void nextTick(() => {
      document.getElementById(optionId(index))?.scrollIntoView({ block: "nearest" });
    });
  },
);

onMounted(() => {
  window.addEventListener("keydown", onGlobalKeydown);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});
</script>

<template>
  <div v-if="search.open" class="palette-backdrop" @pointerdown.self="close()">
    <div class="palette" role="dialog" aria-modal="true" aria-label="Rechercher">
      <div class="palette__field">
        <input
          ref="input"
          class="palette__input"
          type="text"
          role="combobox"
          spellcheck="false"
          autocomplete="off"
          aria-label="Rechercher un test, un endpoint, une adresse…"
          placeholder="Rechercher un test, un endpoint, une adresse…"
          aria-autocomplete="list"
          :aria-expanded="search.hits.length > 0"
          :aria-controls="listId"
          :aria-activedescendant="search.current ? optionId(search.selected) : undefined"
          :value="search.text"
          @input="search.type(($event.target as HTMLInputElement).value)"
          @keydown="onInputKeydown"
        />
        <span class="palette__scope">{{ scopeLabel }}</span>
      </div>

      <div :id="listId" class="palette__results" role="listbox" aria-label="Résultats">
        <div
          v-for="(group, g) in search.results?.groups ?? []"
          :key="group.title"
          role="group"
          :aria-labelledby="`${listId}-g${String(g)}`"
          class="palette__group"
        >
          <p :id="`${listId}-g${String(g)}`" class="palette__group-title">
            {{ group.title }}
            <span v-if="group.total > group.hits.length" class="palette__more">
              · {{ group.hits.length }} sur {{ group.total }}
            </span>
          </p>
          <div
            v-for="(hit, h) in group.hits"
            :id="optionId(indexOf(g, h))"
            :key="`${hit.kind}:${hit.projectId}:${hit.path ?? hit.label}:${hit.detail}`"
            role="option"
            class="palette__hit"
            :aria-selected="search.selected === indexOf(g, h)"
            @pointerdown.prevent
            @click="openHit(hit, $event.ctrlKey || $event.metaKey)"
            @pointermove="search.selected = indexOf(g, h)"
          >
            <span
              v-if="hit.method"
              class="palette__method"
              :class="`palette__method--${hit.method.toLowerCase()}`"
              >{{ hit.method }}</span
            >
            <span v-else class="palette__kind">{{ KIND_LABELS[hit.kind] ?? "" }}</span>
            <span class="palette__label"
              ><template v-for="(part, p) in parts(hit)" :key="p"
                ><mark v-if="part.mark" class="palette__mark">{{ part.text }}</mark
                ><template v-else>{{ part.text }}</template></template
              ></span
            >
            <span class="palette__detail">{{ hit.detail }}</span>
          </div>
        </div>
        <p v-if="search.error" class="palette__empty" role="alert">{{ search.error }}</p>
        <p
          v-else-if="search.results && search.results.groups.length === 0 && search.text.trim()"
          class="palette__empty"
        >
          Aucun résultat pour « {{ search.text.trim() }} ».
        </p>
        <p v-else-if="!search.text.trim()" class="palette__empty">
          Un nom, une adresse (POST /orders), une URL complète ou une #étiquette.
        </p>
      </div>

      <p class="palette__keys">
        ↑↓ choisir · Entrée ouvrir · Ctrl+Entrée dans une autre zone · Échap fermer
      </p>
    </div>
  </div>
</template>

<style scoped>
.palette-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-top: 12vh;
  background-color: var(--backdrop);
}

.palette {
  display: flex;
  flex-direction: column;
  width: min(40rem, calc(100vw - 2rem));
  max-height: 70vh;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-lg);
  background-color: var(--color-surface);
  box-shadow: var(--shadow-dialog);
  overflow: hidden;
}

.palette__field {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.palette__input {
  flex: 1;
  min-width: 0;
  min-height: var(--control-lg);
  border: 0;
  font: inherit;
  font-size: var(--text-body-large);
  color: var(--color-text);
  background-color: transparent;
}

.palette__input:focus-visible {
  outline: none;
}

.palette__scope {
  flex: none;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.palette__results {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.25rem 0;
}

.palette__group-title {
  margin: 0;
  padding: 0.5rem 0.75rem 0.25rem;
  font: 600 var(--text-caption) var(--font-mono);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.palette__more {
  text-transform: none;
  letter-spacing: 0;
}

.palette__hit {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--control-md);
  padding: 0 0.75rem;
  cursor: pointer;
}

.palette__hit[aria-selected="true"] {
  background-color: var(--color-accent-soft);
}

.palette__method,
.palette__kind {
  flex: none;
  min-width: 3.5rem;
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.palette__method--get {
  color: var(--color-success);
}

.palette__method--post,
.palette__method--put,
.palette__method--patch {
  color: var(--color-link);
}

.palette__method--delete {
  color: var(--color-danger);
}

.palette__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text);
}

.palette__mark {
  font-weight: 600;
  color: inherit;
  background-color: var(--color-match);
}

.palette__detail {
  flex: none;
  max-width: 45%;
  margin-left: auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--text-small);
  color: var(--color-muted);
}

.palette__empty {
  margin: 0;
  padding: 0.75rem;
  color: var(--color-muted);
}

.palette__keys {
  margin: 0;
  padding: 0.375rem 0.75rem;
  border-top: 1px solid var(--color-border);
  font-size: var(--text-small);
  color: var(--color-muted);
}
</style>
