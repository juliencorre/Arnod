<script setup lang="ts">
/**
 * Sélecteur de workspace en haut de la barre latérale (2b, W4) : recherche,
 * liste des workspaces, « + Nouveau workspace » (W5), accès à la gestion des
 * workspaces (2d) et au référentiel (2e). Clavier : la recherche
 * reçoit le focus, flèches pour parcourir la liste, Échap pour fermer.
 */
import { computed, nextTick, onBeforeUnmount, ref, useId, useTemplateRef } from "vue";

import ColorDot from "@/components/ui/ColorDot.vue";
import SearchInput from "@/components/ui/SearchInput.vue";
import { countLabel } from "@/lib/format";
import { useCatalogStore } from "@/stores/catalog";
import { useDialogStore } from "@/stores/dialogs";
import { type ManageTab, useNavigationStore } from "@/stores/navigation";
import type { WorkspaceView } from "@/types/ipc";

const props = defineProps<{ workspace: WorkspaceView }>();

const catalog = useCatalogStore();
const dialogs = useDialogStore();
const navigation = useNavigationStore();
const popoverId = useId();
const root = useTemplateRef<HTMLElement>("root");
const trigger = useTemplateRef<HTMLButtonElement>("trigger");

const open = ref(false);
const query = ref("");

const filtered = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return catalog.workspaces.filter((w) => w.name.toLowerCase().includes(needle));
});

function onOutside(event: PointerEvent): void {
  if (!(event.target instanceof Node) || !root.value?.contains(event.target)) close(false);
}

async function show(): Promise<void> {
  open.value = true;
  query.value = "";
  document.addEventListener("pointerdown", onOutside, true);
  await nextTick();
  root.value?.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
}

function close(restoreFocus = true): void {
  if (!open.value) return;
  open.value = false;
  document.removeEventListener("pointerdown", onOutside, true);
  if (restoreFocus) trigger.value?.focus();
}

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onOutside, true);
});

function toggle(): void {
  if (open.value) close();
  else void show();
}

async function choose(workspaceId: string): Promise<void> {
  close();
  await navigation.showWorkspace(workspaceId);
}

function manage(tab: ManageTab): void {
  close(false);
  navigation.showManage(tab, { workspaceId: props.workspace.id });
}

function createWorkspace(): void {
  close(false);
  dialogs.openNewWorkspace({ open: true });
}

/** Déplace le focus entre la recherche et les boutons de la liste. */
function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    close();
    return;
  }
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const focusables = [
    ...(root.value?.querySelectorAll<HTMLElement>(
      '.switcher__popover input[type="search"], .switcher__popover button',
    ) ?? []),
  ];
  const index = focusables.indexOf(document.activeElement as HTMLElement);
  const next = event.key === "ArrowDown" ? index + 1 : index - 1;
  const target = focusables[(next + focusables.length) % focusables.length];
  if (target) {
    event.preventDefault();
    target.focus();
  }
}
</script>

<template>
  <div ref="root" class="switcher">
    <button
      ref="trigger"
      type="button"
      class="switcher__trigger"
      aria-haspopup="dialog"
      :aria-expanded="open"
      :aria-controls="open ? popoverId : undefined"
      @click="toggle"
    >
      <ColorDot :color="props.workspace.color" />
      <span class="switcher__text">
        <span class="switcher__name">
          {{ props.workspace.name }}
          <span v-if="props.workspace.isDefault" class="switcher__badge">par défaut</span>
        </span>
        <span class="switcher__meta">
          Workspace · {{ countLabel(props.workspace.projectIds.length, "projet", "projets") }}
        </span>
      </span>
      <span class="switcher__chevron" aria-hidden="true">{{ open ? "▴" : "▾" }}</span>
    </button>

    <div
      v-if="open"
      :id="popoverId"
      class="switcher__popover"
      role="dialog"
      aria-label="Changer de workspace"
      @keydown="onKeydown"
    >
      <SearchInput v-model="query" label="Rechercher un workspace" />
      <ul class="switcher__list" aria-label="Workspaces">
        <li v-for="item in filtered" :key="item.id">
          <button
            type="button"
            class="switcher__item"
            :class="{ 'switcher__item--current': item.id === props.workspace.id }"
            :aria-current="item.id === props.workspace.id ? 'true' : undefined"
            @click="choose(item.id)"
          >
            <ColorDot :color="item.color" size="sm" />
            <span class="switcher__item-name">{{ item.name }}</span>
            <span v-if="item.isDefault" class="switcher__badge">par défaut</span>
            <span class="switcher__count">
              <span class="visually-hidden">Projets : </span>{{ item.projectIds.length }}
            </span>
            <span class="switcher__check" aria-hidden="true">{{
              item.id === props.workspace.id ? "✓" : ""
            }}</span>
          </button>
        </li>
        <li v-if="filtered.length === 0" class="switcher__empty">Aucun workspace ne correspond.</li>
      </ul>
      <div class="switcher__actions">
        <button
          type="button"
          class="switcher__action switcher__action--create"
          @click="createWorkspace"
        >
          + Nouveau workspace
        </button>
        <button type="button" class="switcher__action" @click="manage('workspaces')">
          Gérer les workspaces…
        </button>
        <button type="button" class="switcher__action" @click="manage('registry')">
          <span class="switcher__action-label">Référentiel des projets</span>
          <span class="switcher__count">{{
            countLabel(catalog.projects.length, "projet", "projets")
          }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.switcher {
  position: relative;
}

.switcher__trigger {
  box-sizing: border-box;
  display: flex;
  align-items: flex-start;
  gap: 0.625rem;
  width: 100%;
  padding: 0.625rem;
  border: 1px solid transparent;
  border-radius: 10px;
  font: inherit;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.switcher__trigger:hover,
.switcher__trigger[aria-expanded="true"] {
  border-color: var(--color-border);
  background: var(--color-surface);
}

.switcher__trigger:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: 1px;
}

.switcher__trigger > .dot {
  margin-top: 0.25rem;
}

.switcher__text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.switcher__name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.375rem;
  font-weight: 600;
}

.switcher__meta {
  font-size: 0.75rem;
  color: var(--color-muted);
}

.switcher__chevron {
  color: var(--color-muted);
}

.switcher__badge {
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  font-size: 0.65625rem;
  font-weight: 400;
  color: var(--color-muted);
  background: var(--color-chip);
}

.switcher__popover {
  position: absolute;
  z-index: 30;
  top: calc(100% + 0.375rem);
  left: 0;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 20rem;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.switcher__list {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  max-height: 18rem;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.switcher__item,
.switcher__action {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  padding: 0.5rem 0.625rem;
  border: none;
  border-radius: 6px;
  font: inherit;
  font-size: 0.9375rem;
  text-align: left;
  color: var(--color-text);
  background: none;
  cursor: pointer;
}

.switcher__item:hover,
.switcher__item:focus-visible,
.switcher__action:hover,
.switcher__action:focus-visible {
  outline: none;
  background: var(--color-sidebar);
}

.switcher__item--current {
  background: var(--color-accent-soft);
}

.switcher__item-name {
  flex: 1;
  min-width: 0;
}

.switcher__action-label {
  flex: 1;
}

.switcher__count {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.switcher__check {
  width: 1rem;
  color: var(--color-link);
}

.switcher__empty {
  padding: 0.5rem 0.625rem;
  font-size: 0.875rem;
  color: var(--color-muted);
}

.switcher__actions {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-border);
}

.switcher__action--create {
  font-weight: 600;
  color: var(--color-link);
}
</style>
