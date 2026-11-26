<script setup lang="ts">
/**
 * Onglet « Envoyé » de la réponse (maquette 5.1-h, docs/plan-mvp-e-tests-etapes.md, VD2, VD3) :
 * la requête telle qu'elle est partie (secrets masqués par le cœur), les valeurs dynamiques tirées
 * à cet envoi, les variables lues et les valeurs extraites. « Garder » range une valeur tirée dans
 * une variable du test : une extraction de la valeur envoyée, ajoutée à la requête.
 *
 * AM29 : « URL résolue », « En-têtes », le corps envoyé avec ses valeurs tirées surlignées, puis
 * les variables utilisées et leur source (nom de l'environnement, « locale », « extraite »).
 */
import { computed } from "vue";

import { tokenLine } from "@/lib/auth";
import { drawnParts, sourceLabel } from "@/lib/sent";
import { useWorkbenchStore } from "@/stores/workbench";
import type { DrawnValue } from "@/types/ipc";

const props = defineProps<{ editorKey: string }>();

const workbench = useWorkbenchStore();
const editor = computed(() => workbench.editors[props.editorKey] ?? null);
const sent = computed(() => editor.value?.outcome?.sent ?? null);

/** Corps envoyé, réindenté s'il est en JSON. */
const body = computed(() => {
  const text = sent.value?.body ?? null;
  if (text === null) return null;
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
});

/** Corps découpé autour des valeurs tirées, pour les surligner. */
const bodyParts = computed(() =>
  body.value === null ? [] : drawnParts(body.value, sent.value?.drawn ?? []),
);

/** Variable où la valeur tirée est déjà gardée, d'après les extractions de la requête. */
function keptAs(drawn: DrawnValue): string | null {
  if (drawn.source === null) return null;
  return editor.value?.draft?.extractions.find((e) => e.source === drawn.source)?.name ?? null;
}

function keep(drawn: DrawnValue): void {
  workbench.keepDrawn(props.editorKey, drawn);
}
</script>

<template>
  <div v-if="sent" class="sent" role="tabpanel" aria-label="Requête envoyée" tabindex="0">
    <h3 class="sent__title">URL résolue</h3>
    <p class="sent__line">
      <span class="sent__method">{{ sent.method }}</span>
      <code class="sent__url">{{ sent.url }}</code>
    </p>

    <h3 class="sent__title">En-têtes · {{ sent.headers.length }}</h3>
    <dl class="sent__headers">
      <template v-for="([name, value], index) in sent.headers" :key="index">
        <dt>{{ name }}</dt>
        <dd>{{ value }}</dd>
      </template>
    </dl>

    <!-- AM35 (3.4-b) : le jeton OAuth 2 ajouté à l'envoi, jamais sa valeur. -->
    <template v-if="sent.oauth2">
      <h3 class="sent__title">Jeton OAuth 2</h3>
      <p class="sent__token">
        {{ tokenLine(sent.oauth2.status) }} · de <code>{{ sent.oauth2.tokenUrl }}</code>
      </p>
    </template>

    <template v-if="body !== null">
      <h3 class="sent__title">Corps envoyé</h3>
      <pre class="sent__body"><template v-for="(part, index) in bodyParts" :key="index"><mark
            v-if="part.drawn"
            class="sent__drawn"
            :title="`Tirée à cet envoi : {{${part.drawn}}}`"
          >{{ part.text }}</mark><template v-else>{{ part.text }}</template></template></pre>
    </template>

    <template v-if="sent.drawn.length > 0">
      <h3 class="sent__title">Valeurs tirées à cet envoi · {{ sent.drawn.length }}</h3>
      <ul class="sent__table" aria-label="Valeurs tirées">
        <li v-for="(drawn, index) in sent.drawn" :key="index" class="sent__row">
          <code class="sent__name">{{ drawn.name }}</code>
          <code class="sent__value" :title="drawn.value">{{ drawn.value }}</code>
          <span v-if="keptAs(drawn)" class="sent__kept">gardée : {{ keptAs(drawn) }}</span>
          <button
            v-else-if="drawn.source !== null"
            type="button"
            class="sent__keep"
            :aria-label="`Garder ${drawn.name} dans une variable`"
            :title="`Extraire ${drawn.source} après chaque envoi`"
            @click="keep(drawn)"
          >
            Garder
          </button>
          <span v-else class="sent__none" title="La valeur ne forme pas seule une valeur envoyée"
            >—</span
          >
        </li>
      </ul>
      <p class="sent__note">
        Chaque occurrence est tirée à l'envoi, une valeur par occurrence. «&nbsp;Garder&nbsp;» la
        range dans une variable du test : les étapes suivantes la réutilisent, sans script.
      </p>
    </template>

    <template v-if="sent.used.length > 0">
      <h3 class="sent__title">Variables utilisées · {{ sent.used.length }}</h3>
      <ul class="sent__table" aria-label="Variables utilisées">
        <li v-for="used in sent.used" :key="used.name" class="sent__row">
          <code class="sent__name">{{ used.name }}</code>
          <code class="sent__value" :title="used.value">{{ used.value }}</code>
          <span class="sent__origin" :class="`sent__origin--${used.source}`">{{
            sourceLabel(used.source, sent.environment)
          }}</span>
        </li>
      </ul>
    </template>

    <template v-if="sent.extracted.length > 0">
      <h3 class="sent__title">Valeurs extraites · {{ sent.extracted.length }}</h3>
      <ul class="sent__table" aria-label="Valeurs extraites">
        <li v-for="value in sent.extracted" :key="value.name" class="sent__row">
          <code class="sent__name">{{ value.name }}</code>
          <code
            class="sent__value"
            :class="{ 'sent__value--missing': value.value === null }"
            :title="value.message ?? value.value ?? ''"
            >{{ value.value ?? "absente" }}</code
          >
          <span class="sent__origin">{{ value.source }}</span>
        </li>
      </ul>
    </template>

    <p class="sent__note">
      Exactement ce qui est parti sur le réseau, variables remplacées. Les secrets restent masqués ;
      «&nbsp;Copier en cURL&nbsp;» les remplace par <code>$TOKEN</code>.
    </p>
  </div>
  <p v-else class="sent__empty">Envoyez la requête pour voir ce qui part.</p>
</template>

<style scoped>
.sent {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
  font-size: var(--text-body);
}

.sent__line {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  margin: 0;
}

.sent__method {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-link);
}

.sent__url {
  font: var(--text-body) var(--font-mono);
  overflow-wrap: anywhere;
}

.sent__title {
  margin: 0.5rem 0 0;
  font-size: var(--text-caption);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.sent__body {
  max-height: 16rem;
  margin: 0;
  padding: 0.75rem;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font: var(--text-body) var(--font-mono);
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--color-sidebar);
}

.sent__table {
  margin: 0;
  padding: 0;
  list-style: none;
}

.sent__row {
  display: grid;
  grid-template-columns: 8.5rem minmax(0, 1fr) auto;
  gap: 0.75rem;
  align-items: center;
  min-height: var(--control-sm);
  border-bottom: 1px solid var(--color-border);
}

.sent__name {
  overflow: hidden;
  font: 600 var(--text-body) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sent__value {
  overflow: hidden;
  font: var(--text-body) var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-link);
}

.sent__value--missing {
  color: var(--color-danger);
}

.sent__kept {
  padding: 0 0.5rem;
  border-radius: var(--radius-pill);
  font-size: var(--text-small);
  color: var(--color-link);
  background: var(--color-accent-soft);
}

.sent__keep {
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-weight: 600;
  color: var(--color-link);
  cursor: pointer;
}

.sent__keep:focus-visible,
.sent:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.sent__none,
.sent__origin,
.sent__note,
.sent__empty {
  font-size: var(--text-small);
  color: var(--color-muted);
}

.sent__note {
  margin: 0;
}

.sent__headers {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.25rem 1rem;
  margin: 0;
  font: var(--text-body) var(--font-mono);
}

.sent__headers dt {
  font-weight: 600;
}

.sent__headers dd {
  margin: 0;
  overflow-wrap: anywhere;
}

/* Variable locale : sa valeur reste sur le poste (5.1-h). */
.sent__origin--local {
  padding: 0 0.3125rem;
  border-radius: var(--radius-md);
  color: var(--color-link);
  background: var(--color-accent-soft);
}

/* Valeur tirée à cet envoi, dans le corps envoyé (AM29). */
.sent__drawn {
  padding: 0 0.125rem;
  border-radius: var(--radius-sm);
  color: inherit;
  background: var(--color-accent-soft);
}

.sent__token {
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--color-muted);
}
</style>
