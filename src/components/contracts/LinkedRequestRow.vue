<script setup lang="ts">
/**
 * Requête liée à un contrat, sous son opération ou dans « Hors contrat »
 * (docs/plan-requetes-dans-contrats.md, R3, R4, RC5). Un clic l'ouvre en onglet, dans la fenêtre
 * qui suit la sélection (Z5) ; son menu reprend les actions de l'arborescence. La ligne suit la
 * grille de la barre latérale : sa colonne de tête montre « ↳ » sous une opération, sa méthode
 * hors contrat.
 */
import { computed } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem } from "@/components/ui/menu";
import StatusMark from "@/components/workbench/StatusMark.vue";
import { startRequestDrag } from "@/lib/requestDrag";
import { useToastStore } from "@/stores/toasts";
import { useTransferStore } from "@/stores/transfer";
import { useWindowsStore } from "@/stores/windows";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { LinkedView } from "@/types/ipc";

const props = defineProps<{
  request: LinkedView;
  projectId: string;
  depth: number;
  /** Méthode en tête (hors contrat) ; sinon « ↳ », la méthode étant celle de l'opération. */
  showMethod?: boolean;
}>();
const emit = defineEmits<{ action: [id: string, request: LinkedView] }>();

const workbench = useWorkbenchStore();
const windows = useWindowsStore();

const ITEMS: MenuItem[] = [
  { id: "rename", label: "Renommer" },
  { id: "duplicate", label: "Dupliquer" },
  // Lot J (4.1-g).
  { id: "copy-move", label: "Copier ou déplacer…" },
  { id: "delete", label: "Mettre à la corbeille", tone: "danger", separatorBefore: true },
];

/** Requête affichée dans l'onglet actif de cette fenêtre. */
const selected = computed(() => {
  const content = windows.activeTab?.content;
  return (
    content?.kind === "request" &&
    content.projectId === props.projectId &&
    content.path === props.request.path
  );
});
const dirty = computed(() => workbench.isDirty(requestKey(props.projectId, props.request.path)));
// Lot J : Ctrl+C, Ctrl+X et Ctrl+Z sur la ligne, glisser (Ctrl : copier).
const transfer = useTransferStore();
const toasts = useToastStore();
const source = computed(() => ({
  projectId: props.projectId,
  path: props.request.path,
  name: props.request.name,
}));

function onKeydown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
  if (event.code === "KeyC" || event.code === "KeyX") {
    transfer.copy(source.value, event.code === "KeyC" ? "copy" : "move");
  } else if (event.code === "KeyZ" && transfer.last) {
    void transfer.undo().catch(toasts.showError);
  } else return;
  event.preventDefault();
  event.stopPropagation();
}

/** Dernière exécution (`docs/plan-fin-premier-usage.md`, ST1). */
const status = computed(() => workbench.statuses[props.projectId]?.[props.request.path] ?? null);
</script>

<template>
  <li class="linked">
    <div
      class="linked__row"
      :class="{ 'linked__row--selected': selected }"
      :style="{ '--depth': depth }"
    >
      <button
        type="button"
        class="linked__main"
        :aria-current="selected ? 'page' : undefined"
        draggable="true"
        @dragstart="startRequestDrag($event, source)"
        @keydown="onKeydown"
        @click="workbench.openRequest(projectId, request.path, true)"
        @dblclick="workbench.openRequest(projectId, request.path)"
      >
        <span
          v-if="showMethod"
          class="linked__lead linked__method"
          :class="`linked__method--${request.method.toLowerCase()}`"
        >
          {{ request.method }}
        </span>
        <span v-else class="linked__lead linked__link" aria-hidden="true">↳</span>
        <span class="linked__name">{{ request.name }}</span>
        <span class="linked__end">
          <span v-if="dirty" class="linked__dirty" title="Modifiée, non enregistrée">●</span>
          <StatusMark v-if="status" class="linked__status" :status="status" />
        </span>
      </button>
      <BaseMenu
        :items="ITEMS"
        :label="`Actions pour ${request.name}`"
        align="end"
        @select="(id) => emit('action', id, request)"
      >
        <template #trigger="{ attrs }">
          <button
            type="button"
            class="linked__more"
            v-bind="attrs"
            :aria-label="`Actions pour ${request.name}`"
          >
            ⋯
          </button>
        </template>
      </BaseMenu>
    </div>
  </li>
</template>

<style scoped>
.linked {
  list-style: none;
}

.linked__row {
  display: flex;
  align-items: center;
  padding-left: calc(var(--tree-pad) + var(--tree-indent) * var(--depth));
  border-radius: var(--radius-md);
}

.linked__row:hover,
.linked__row--selected {
  background: var(--color-accent-soft);
}

.linked__main {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--tree-gap);
  min-width: 0;
  min-height: 1.75rem;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  font-size: var(--text-body);
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
}

.linked__main:focus-visible,
.linked__more:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
  border-radius: var(--radius-md);
}

.linked__lead {
  box-sizing: border-box;
  flex: none;
  width: var(--tree-lead);
}

.linked__link {
  padding-right: 0.125rem;
  text-align: right;
  color: var(--color-muted);
}

.linked__method {
  font: 700 var(--text-caption) var(--font-mono);
  color: var(--color-muted);
}

.linked__method--get {
  color: var(--color-success);
}

.linked__method--post,
.linked__method--put,
.linked__method--patch {
  color: var(--color-link);
}

.linked__method--delete {
  color: var(--color-danger);
}

.linked__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.linked__end {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 0.25rem;
  margin-left: auto;
  padding-right: 0.25rem;
}

.linked__dirty {
  font-size: 0.625rem;
  color: var(--color-accent);
}

.linked__more {
  border: 0;
  background: none;
  padding: 0.125rem 0.5rem;
  color: var(--color-muted);
  cursor: pointer;
}
</style>
