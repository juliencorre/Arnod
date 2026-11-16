<script setup lang="ts">
/**
 * Aperçu d'un corps HTML (`docs/plan-mvp-a-reponse.md`, R4) : l'arbre réduit par le cœur, rendu
 * en éléments Vue sans attribut. Ni `v-html` ni cadre : rien ne s'exécute, rien ne se charge.
 * Une balise hors de la liste (qui ne devrait pas arriver du cœur) est rendue en `span`.
 */
import { h, type VNode } from "vue";

import type { HtmlNode } from "@/types/ipc";

const props = defineProps<{ nodes: HtmlNode[] }>();

/** Balises que le cœur peut renvoyer (`response::preview::ALLOWED`). */
const ALLOWED = new Set([
  "a",
  "abbr",
  "article",
  "b",
  "blockquote",
  "br",
  "code",
  "dd",
  "div",
  "dl",
  "dt",
  "em",
  "footer",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "header",
  "hr",
  "i",
  "li",
  "main",
  "nav",
  "ol",
  "p",
  "pre",
  "section",
  "small",
  "span",
  "strong",
  "table",
  "td",
  "th",
  "tr",
  "ul",
]);

function draw(node: HtmlNode): VNode | string {
  if (node.kind === "text") return node.text;
  return h(ALLOWED.has(node.tag) ? node.tag : "span", node.children.map(draw));
}

function Tree(): (VNode | string)[] {
  return props.nodes.map(draw);
}
</script>

<template>
  <div class="preview" role="tabpanel" aria-label="Aperçu du corps HTML" tabindex="0">
    <!-- Contenu de l'onglet « Corps » (ResponsePanel), focalisable pour défiler au clavier. -->
    <Tree />
  </div>
</template>

<style scoped>
.preview {
  min-height: 6rem;
  padding: 0.75rem 1rem;
  overflow: auto;
  overflow-wrap: anywhere;
  color: var(--color-text);
}

.preview:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.preview :deep(h1) {
  font-size: var(--text-title);
}

.preview :deep(h2),
.preview :deep(h3) {
  font-size: var(--text-body-large);
}

.preview :deep(h4),
.preview :deep(h5),
.preview :deep(h6) {
  font-size: var(--text-body);
}

.preview :deep(a) {
  text-decoration: underline;
  color: var(--color-link);
}

.preview :deep(pre),
.preview :deep(code) {
  font-family: var(--font-mono);
}

.preview :deep(table) {
  border-collapse: collapse;
}

.preview :deep(td),
.preview :deep(th) {
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--color-border);
}
</style>
