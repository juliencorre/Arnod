<script setup lang="ts">
/**
 * Corps de la réponse (`docs/plan-mvp-a-reponse.md`, écran 5.1-b bis) : format (détecté,
 * modifiable), mise en forme numérotée et colorée par le cœur, aperçu d'un HTML, copier,
 * enregistrer, effacer, et filtre JSONPath dont les valeurs trouvées sont surlignées.
 */
import { computed, nextTick, ref, watch } from "vue";

import ToggleSwitch from "@/components/ui/ToggleSwitch.vue";
import HtmlPreview from "@/components/workbench/HtmlPreview.vue";
import ResponseFilter from "@/components/workbench/ResponseFilter.vue";
import { useResponseStore } from "@/stores/response";
import { useWorkbenchStore } from "@/stores/workbench";
import type { BodyToken, ResponseFormat } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const store = useResponseStore();
const workbench = useWorkbenchStore();
const received = computed(() => workbench.editors[props.editorKey]?.outcome?.response ?? null);
const view = computed(() => store.views[props.editorKey] ?? null);
const rendered = computed(() => view.value?.rendered ?? null);
const format = computed(() => rendered.value?.format ?? "text");
const filter = ref<{ focus: () => void } | null>(null);
const code = ref<HTMLElement | null>(null);

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

/** Ctrl+F (Cmd+F) dans le corps : ouvre le filtre. */
function onKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "f") {
    event.preventDefault();
    filter.value?.focus();
  }
}
</script>

<template>
  <div class="body" @keydown="onKeydown">
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
    <div
      v-else
      ref="code"
      class="body__view code"
      role="region"
      aria-label="Corps de la réponse"
      tabindex="0"
    >
      <div v-for="(line, index) in rendered?.lines ?? []" :key="index" class="code__line">
        <span class="code__number" aria-hidden="true">{{ index + 1 }}</span>
        <span class="code__text"
          ><span
            v-for="(token, t) in line"
            :key="t"
            :class="['tok--' + token.kind, markOf(token)]"
            >{{ token.text }}</span
          ></span
        >
      </div>
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

.code {
  padding: 0.5rem 0;
  overflow: auto;
  font: var(--text-body) / 1.55 var(--font-mono);
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
