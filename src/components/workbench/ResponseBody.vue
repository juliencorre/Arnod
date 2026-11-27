<script setup lang="ts">
/**
 * Corps de la réponse (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis) : format (détecté,
 * modifiable), mise en forme numérotée et colorée par le cœur, aperçu d'un HTML, copier,
 * enregistrer, effacer, et filtre JSONPath dont les valeurs trouvées sont surlignées.
 *
 * AM32 (4.2-b bis) : un clic sur une valeur propose « Extraire dans une variable », comme le
 * filtre ; une valeur lue par une extraction de la requête est marquée « → nom ».
 */
import { computed, nextTick, ref, watch } from "vue";

import ToggleSwitch from "@/components/ui/ToggleSwitch.vue";
import HtmlPreview from "@/components/workbench/HtmlPreview.vue";
import ResponseFilter from "@/components/workbench/ResponseFilter.vue";
import { useGuard } from "@/composables/useGuard";
import { bodyPathOf } from "@/lib/sent";
import { useInterfaceStore } from "@/stores/interface";
import { useResponseStore } from "@/stores/response";
import { useWorkbenchStore } from "@/stores/workbench";
import type { BodyToken, ResponseFormat } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const store = useResponseStore();
const shortcuts = useInterfaceStore();
const workbench = useWorkbenchStore();
const received = computed(() => workbench.editors[props.editorKey]?.outcome?.response ?? null);
const view = computed(() => store.views[props.editorKey] ?? null);
const rendered = computed(() => view.value?.rendered ?? null);
const format = computed(() => rendered.value?.format ?? "text");
const filter = ref<{ focus: () => void } | null>(null);
const code = ref<HTMLElement | null>(null);
const root = ref<HTMLElement | null>(null);

const FORMATS: { id: ResponseFormat; label: string }[] = [
  { id: "json", label: "JSON" },
  { id: "xml", label: "XML" },
  { id: "html", label: "HTML" },
  { id: "text", label: "Texte brut" },
];
const detectedLabel = computed(
  () => FORMATS.find((f) => f.id === rendered.value?.detected)?.label ?? "",
);

// Nouvelle réponse (ou autre requête) : mise en forme, et filtre relancé sur le nouveau corps.
watch(
  [() => props.editorKey, received],
  ([key]) => {
    store.view(key);
    void store.render(key);
    if (store.views[key]?.expression) void store.runFilter(key);
  },
  { immediate: true },
);

const preview = computed({
  get: () => view.value?.preview ?? false,
  set: (on: boolean) => void store.setPreview(props.editorKey, on),
});

function onFormat(event: Event): void {
  void store.setFormat(
    props.editorKey,
    (event.target as HTMLSelectElement).value as ResponseFormat,
  );
}

// --- Surlignage des correspondances (affichage seulement) ------------------------------------

const matched = computed(
  () => new Set(view.value?.result?.matches.map((m) => m.path) ?? ([] as string[])),
);
const currentPath = computed(() => store.currentMatch(props.editorKey)?.path ?? null);

/** Le chemin et ses ancêtres (`$.a[0].b` → `$`, `$.a`, `$.a[0]`, `$.a[0].b`), hors guillemets. */
function* lineage(path: string): Generator<string> {
  let quoted = false;
  for (let i = 1; i < path.length; i++) {
    const c = path[i];
    if (c === "'" && path[i - 1] !== "\\") quoted = !quoted;
    if (!quoted && (c === "." || c === "[")) yield path.slice(0, i);
  }
  yield path;
}

function markOf(token: BodyToken): "" | "tok--match" | "tok--match tok--current" {
  if (token.path === undefined || matched.value.size === 0) return "";
  let found = false;
  for (const path of lineage(token.path)) {
    if (path === currentPath.value) return "tok--match tok--current";
    if (matched.value.has(path)) found = true;
  }
  return found ? "tok--match" : "";
}

watch(currentPath, async (path) => {
  if (path === null) return;
  await nextTick();
  code.value?.querySelector(".tok--current")?.scrollIntoView({ block: "nearest" });
});

// --- Extraire depuis une valeur (AM32) ----------------------------------------------

const guard = useGuard();
const VALUE_KINDS = new Set(["string", "number", "literal"]);

/** Valeur qui peut être extraite : une valeur JSON (pas une clé ni une ponctuation). */
const extractable = (token: BodyToken): boolean =>
  token.path !== undefined && VALUE_KINDS.has(token.kind);

/** Nom de la variable qui lit chaque valeur du corps, par chemin canonique. */
const extractedAt = computed(() => {
  const names = new Map<string, string>();
  for (const extraction of workbench.editors[props.editorKey]?.draft?.extractions ?? []) {
    const path = bodyPathOf(extraction.source);
    if (path !== null && extraction.enabled) names.set(path, extraction.name);
  }
  return names;
});

/** Variable qui lit la valeur `token`, s'il y en a une. */
function extractedName(token: BodyToken): string | null {
  return extractable(token) && token.path !== undefined
    ? (extractedAt.value.get(token.path) ?? null)
    : null;
}

/** Valeur cliquée et place de la proposition, dans le repère du corps. */
const picked = ref<{ path: string; left: number; top: number } | null>(null);
const pickAction = ref<HTMLButtonElement | null>(null);

async function onCodeClick(event: MouseEvent): Promise<void> {
  const target =
    event.target instanceof Element ? event.target.closest<HTMLElement>("[data-path]") : null;
  const frame = root.value?.getBoundingClientRect();
  if (!target || !frame) {
    picked.value = null;
    return;
  }
  const box = target.getBoundingClientRect();
  picked.value = {
    path: target.dataset.path ?? "",
    left: box.left - frame.left,
    top: box.bottom - frame.top,
  };
  await nextTick();
  pickAction.value?.focus();
}

function extractPicked(): void {
  const path = picked.value?.path;
  picked.value = null;
  if (path) guard(() => store.extractAt(props.editorKey, path));
  code.value?.focus();
}

function closePick(): void {
  picked.value = null;
  code.value?.focus();
}

/** La proposition se ferme quand le focus la quitte. */
function onPickFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget;
  const menu = event.currentTarget;
  if (menu instanceof HTMLElement && next instanceof Node && menu.contains(next)) return;
  picked.value = null;
}

/** Ctrl+F par défaut (Cmd+F) dans le corps : ouvre le filtre (1.6-f bis). */
function onKeydown(event: KeyboardEvent): void {
  if (shortcuts.matches(event, "filterResponse")) {
    event.preventDefault();
    filter.value?.focus();
  }
}
</script>

<template>
  <div ref="root" class="body" @keydown="onKeydown">
    <div class="body__bar">
      <select
        class="body__format"
        aria-label="Format du corps"
        :title="detectedLabel ? `Détecté : ${detectedLabel}` : undefined"
        :value="format"
        :disabled="!received"
        @change="onFormat"
      >
        <option v-for="f in FORMATS" :key="f.id" :value="f.id">{{ f.label }}</option>
      </select>
      <ToggleSwitch
        v-model="preview"
        :disabled="format !== 'html'"
        :title="format === 'html' ? undefined : 'L\'aperçu rend un corps HTML'"
      >
        Aperçu
      </ToggleSwitch>
      <span class="body__spacer" />
      <button
        type="button"
        class="body__tool"
        aria-label="Copier le corps"
        title="Copier le corps"
        :disabled="!received"
        @click="store.copy(editorKey)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M5.5 5.5h7v8h-7z" />
          <path d="M3.5 10.5v-8h7" />
        </svg>
      </button>
      <button
        type="button"
        class="body__tool"
        aria-label="Enregistrer le corps dans un fichier…"
        title="Enregistrer le corps dans un fichier…"
        :disabled="!received"
        @click="store.save(editorKey)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M8 2.5v8M4.8 7.5 8 10.7l3.2-3.2" />
          <path d="M2.8 11.5v2h10.4v-2" />
        </svg>
      </button>
      <button
        type="button"
        class="body__tool"
        aria-label="Effacer la réponse"
        title="Effacer la réponse"
        :disabled="!received"
        @click="store.clear(editorKey)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M3 4.5h10M6.3 4.5V2.8h3.4v1.7" />
          <path d="M4.3 4.5l.6 8.7h6.2l.6-8.7" />
        </svg>
      </button>
    </div>
    <p v-if="received?.truncated" class="body__notice">
      Le corps dépasse 5 Mo : seul le début est affiché, copié ou enregistré.
    </p>
    <p v-if="rendered?.notice" class="body__notice">{{ rendered.notice }}</p>
    <p v-if="rendered && rendered.lines.length === 0" class="body__empty">Réponse sans corps.</p>
    <HtmlPreview v-else-if="preview && view?.nodes" :nodes="view.nodes" class="body__view" />
    <!-- Contenu de l'onglet « Corps » (ResponsePanel), focalisable pour défiler au clavier. -->
    <div
      v-else
      ref="code"
      class="body__view code"
      role="tabpanel"
      aria-label="Corps de la réponse"
      tabindex="0"
      @click="onCodeClick"
      @scroll="picked = null"
    >
      <div v-for="(line, index) in rendered?.lines ?? []" :key="index" class="code__line">
        <span class="code__number" aria-hidden="true">{{ index + 1 }}</span>
        <span class="code__text"
          ><template v-for="(token, t) in line" :key="t"
            ><span
              :class="[
                'tok--' + token.kind,
                markOf(token),
                { 'tok--pickable': extractable(token) },
              ]"
              :data-path="extractable(token) ? token.path : undefined"
              >{{ token.text }}</span
            ><span v-if="extractedName(token)" class="tok__extracted"
              >→ {{ extractedName(token) }}</span
            ></template
          ></span
        >
      </div>
    </div>
    <div
      v-if="picked"
      class="body__pick"
      role="menu"
      aria-label="Valeur du corps"
      :style="{ left: `${picked.left}px`, top: `${picked.top}px` }"
      @keydown.esc.stop="closePick"
      @focusout="onPickFocusOut"
    >
      <button
        ref="pickAction"
        type="button"
        role="menuitem"
        class="body__pick-item"
        @click.stop="extractPicked"
      >
        Extraire dans une variable
      </button>
    </div>
    <ResponseFilter
      v-if="format === 'json' || view?.expression"
      ref="filter"
      :editor-key="editorKey"
    />
  </div>
</template>

<style scoped>
.body {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.body__bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--color-border);
}

.body__format {
  /* Largeur du format choisi, là où le moteur le permet (Chromium). */
  field-sizing: content;
  min-height: var(--control-sm);
  font-family: var(--font-mono);
  font-weight: 600;
}

.body__spacer {
  flex: 1;
}

.body__tool {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--control-sm);
  height: var(--control-sm);
  padding: 0;
  border: 0;
  border-radius: var(--radius-md);
  color: var(--color-muted);
  background: none;
  cursor: pointer;
}

.body__tool svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.body__tool:hover:not(:disabled) {
  color: var(--color-text);
  background-color: var(--color-chip);
}

.body__tool:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.body__tool:focus-visible,
.code:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.body__notice {
  margin: 0;
  padding: 0.375rem 0.75rem;
  border-bottom: 1px solid var(--color-border);
  font-size: var(--text-small);
  color: var(--color-muted);
}

.body__view {
  max-height: 28rem;
}

/* Une valeur se clique pour être extraite (AM32). */
.tok--pickable {
  cursor: pointer;
}

.tok--pickable:hover {
  text-decoration: underline dotted;
}

.tok__extracted {
  margin-left: 0.5rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.body__pick {
  position: absolute;
  z-index: 3;
  padding: 0.25rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.body__pick-item {
  display: block;
  padding: 0.375rem 0.625rem;
  border: 0;
  border-radius: var(--radius-md);
  background: none;
  font: inherit;
  color: var(--color-text);
  cursor: pointer;
}

.body__pick-item:hover {
  background-color: var(--color-accent-soft);
}

.body__pick-item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.code {
  padding: 0.5rem 0;
  overflow: auto;
  font: var(--editor-font-size) / 1.55 var(--editor-font);
  color: var(--color-text);
}

.body__empty {
  margin: 0;
  padding: 0.75rem;
  color: var(--color-muted);
}

.code__line {
  display: flex;
  min-width: max-content;
}

.code__number {
  flex: none;
  width: 3ch;
  padding: 0 0.75rem 0 0.5rem;
  text-align: right;
  color: var(--color-subtle);
  user-select: none;
}

.code__text {
  padding-right: 0.75rem;
  white-space: pre;
}

.tok--string,
.tok--attrValue {
  color: var(--color-code-string);
}

.tok--number {
  color: var(--color-code-number);
}

.tok--literal {
  color: var(--color-code-keyword);
}

.tok--tag {
  color: var(--color-code-tag);
}

.tok--attr {
  color: var(--color-code-attr);
}

.tok--punct,
.tok--comment {
  color: var(--color-muted);
}

.tok--comment {
  font-style: italic;
}

.tok--match {
  border-radius: var(--radius-sm);
  background-color: var(--color-match);
}

.tok--current {
  outline: 1px solid var(--color-accent);
}
</style>
