<script setup lang="ts">
/**
 * Menu déroulant accessible, avec un niveau de sous-menu (« Ajouter à un autre
 * workspace ▸ »). Le déclencheur est fourni par le parent via le slot `trigger`,
 * qui reçoit les attributs à lier (`v-bind="attrs"`).
 *
 * Clavier : flèches, Début, Fin, Entrée ou Espace, flèche droite et gauche pour
 * le sous-menu, Échap pour fermer ; le focus revient au déclencheur. `open()` et
 * `close()` sont exposés pour un menu contextuel.
 *
 * Le menu et son sous-menu sont en position fixe, placés depuis le déclencheur et
 * gardés dans la fenêtre : une colonne qui défile (barre latérale, zone) ne les
 * rogne pas. Ils suivent le déclencheur quand la page défile ou change de taille.
 */
import { computed, nextTick, onBeforeUnmount, ref, useId, useTemplateRef, watch } from "vue";

import {
  placeBelow,
  placeBeside,
  type MenuItem,
  type MenuTriggerAttrs,
  type Point,
  type Size,
} from "./menu";

const props = withDefaults(
  defineProps<{
    items: MenuItem[];
    label: string;
    heading?: string;
    /** Bord du déclencheur sur lequel s'aligne le menu. */
    align?: "start" | "end";
  }>(),
  { heading: undefined, align: "start" },
);
/** `closed` : le menu s'est fermé, avec ou sans choix. */
const emit = defineEmits<{ select: [id: string]; closed: [] }>();
defineSlots<{ trigger(props: { open: boolean; attrs: MenuTriggerAttrs }): unknown }>();

const root = useTemplateRef<HTMLElement>("root");
const popover = useTemplateRef<HTMLElement>("popover");
const menuId = useId();
const isOpen = ref(false);
/** Entrée active du menu principal (-1 : aucune). */
const active = ref(-1);
/** Entrée dont le sous-menu est ouvert. */
const subOpen = ref<number | null>(null);
/** Entrée active du sous-menu (-1 : focus dans le menu principal). */
const subActive = ref(-1);
let returnFocus: HTMLElement | null = null;
/** Position fixe du menu et du sous-menu ouverts (`null` : pas encore mesurée). */
const menuAt = ref<Point | null>(null);
const subAt = ref<Point | null>(null);

const subItems = computed<MenuItem[]>(() =>
  subOpen.value === null ? [] : (props.items[subOpen.value]?.children ?? []),
);

function hasChildren(item: MenuItem | undefined): boolean {
  return (item?.children?.length ?? 0) > 0;
}

/** Entrée activable suivante (`delta` = 1) ou précédente (-1), en boucle. */
function step(list: MenuItem[], from: number, delta: 1 | -1): number {
  const enabled = list.flatMap((item, index) => (item.disabled === true ? [] : [index]));
  if (enabled.length === 0) return -1;
  const position = enabled.indexOf(from);
  if (position === -1) return (delta > 0 ? enabled[0] : enabled.at(-1)) ?? -1;
  return enabled[(position + delta + enabled.length) % enabled.length] ?? -1;
}

async function focusActive(): Promise<void> {
  await nextTick();
  const selector =
    subOpen.value !== null && subActive.value >= 0
      ? `[data-level="1"][data-index="${subActive.value}"]`
      : `[data-level="0"][data-index="${active.value}"]`;
  root.value?.querySelector<HTMLElement>(selector)?.focus();
}

function viewport(): Size {
  const { clientWidth, clientHeight } = document.documentElement;
  return { width: clientWidth, height: clientHeight };
}

function sizeOf(element: HTMLElement): Size {
  return { width: element.offsetWidth, height: element.offsetHeight };
}

/** Place le menu sous le déclencheur et le sous-menu à côté de son entrée, dans la fenêtre. */
function place(): void {
  const anchor = root.value;
  const menu = popover.value;
  if (!anchor || !menu) return;
  menuAt.value = placeBelow(anchor.getBoundingClientRect(), sizeOf(menu), viewport(), props.align);
  const sub = menu.querySelector<HTMLElement>(".menu__list--sub");
  const item = sub?.parentElement;
  subAt.value =
    sub && item
      ? placeBeside(
          item.getBoundingClientRect(),
          sizeOf(sub),
          viewport(),
          props.align === "end" ? "start" : "end",
        )
      : null;
}

/** Coordonnées CSS d'un menu placé ; sans elles, il garde sa place naturelle pour être mesuré. */
function at(point: Point | null): Record<string, string> | undefined {
  return point ? { top: `${String(point.top)}px`, left: `${String(point.left)}px` } : undefined;
}

// Après le rendu : le menu (ou le sous-menu) vient d'apparaître, ou ses entrées ont changé.
watch([isOpen, subOpen, () => props.items], place, { flush: "post" });

/** Clic ailleurs : fermeture ; défilement d'une colonne (capture) ou fenêtre redimensionnée : le menu suit. */
function listen(on: boolean): void {
  if (on) {
    document.addEventListener("pointerdown", onOutside, true);
    document.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
  } else {
    document.removeEventListener("pointerdown", onOutside, true);
    document.removeEventListener("scroll", place, true);
    window.removeEventListener("resize", place);
  }
}

function onOutside(event: PointerEvent): void {
  if (!(event.target instanceof Node) || !root.value?.contains(event.target)) close(false);
}

function open(last = false): void {
  if (isOpen.value) return;
  returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  isOpen.value = true;
  subOpen.value = null;
  subActive.value = -1;
  active.value = step(props.items, -1, last ? -1 : 1);
  void focusActive();
  listen(true);
}

/** Ferme le menu ; `restore` rend le focus au déclencheur. */
function close(restore = true): void {
  if (!isOpen.value) return;
  isOpen.value = false;
  active.value = -1;
  subOpen.value = null;
  subActive.value = -1;
  menuAt.value = null;
  subAt.value = null;
  listen(false);
  if (restore) returnFocus?.focus();
  returnFocus = null;
  emit("closed");
}

onBeforeUnmount(() => {
  listen(false);
});

function toggle(): void {
  if (isOpen.value) close();
  else open();
}

function onTriggerKeydown(event: KeyboardEvent): void {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  event.preventDefault();
  open(event.key === "ArrowUp");
}

const triggerAttrs = computed<MenuTriggerAttrs>(() => ({
  "aria-haspopup": "menu",
  "aria-expanded": isOpen.value,
  "aria-controls": isOpen.value ? menuId : undefined,
  onClick: toggle,
  onKeydown: onTriggerKeydown,
}));

function openSub(index: number): void {
  active.value = index;
  subOpen.value = index;
  subActive.value = step(props.items[index]?.children ?? [], -1, 1);
  void focusActive();
}

function closeSub(): void {
  subOpen.value = null;
  subActive.value = -1;
  void focusActive();
}

function activate(index: number): void {
  const item = props.items[index];
  if (!item || item.disabled === true) return;
  if (hasChildren(item)) {
    openSub(index);
    return;
  }
  emit("select", item.id);
  close();
}

function activateSub(index: number): void {
  const item = subItems.value[index];
  if (!item || item.disabled === true) return;
  emit("select", item.id);
  close();
}

function hover(index: number): void {
  if (props.items[index]?.disabled === true) return;
  active.value = index;
  if (hasChildren(props.items[index])) {
    subOpen.value = index;
    subActive.value = -1;
  } else {
    subOpen.value = null;
  }
  void focusActive();
}

function hoverSub(index: number): void {
  if (subItems.value[index]?.disabled === true) return;
  subActive.value = index;
  void focusActive();
}

function onMenuKeydown(event: KeyboardEvent): void {
  const inSub = subOpen.value !== null && subActive.value >= 0;
  const list = inSub ? subItems.value : props.items;
  const current = inSub ? subActive.value : active.value;
  const move = (index: number): void => {
    if (inSub) {
      subActive.value = index;
    } else {
      active.value = index;
      subOpen.value = null;
    }
    void focusActive();
  };

  switch (event.key) {
    case "ArrowDown":
      move(step(list, current, 1));
      break;
    case "ArrowUp":
      move(step(list, current, -1));
      break;
    case "Home":
      move(step(list, -1, 1));
      break;
    case "End":
      move(step(list, -1, -1));
      break;
    case "Enter":
    case " ":
      if (inSub) activateSub(current);
      else activate(current);
      break;
    case "ArrowRight":
      if (inSub || !hasChildren(props.items[current])) return;
      openSub(current);
      break;
    case "ArrowLeft":
      if (!inSub) return;
      closeSub();
      break;
    case "Escape":
      if (inSub) closeSub();
      else close();
      break;
    case "Tab":
      // Le focus suit son cours naturel ; le menu se ferme simplement.
      close(false);
      return;
    default:
      return;
  }
  event.preventDefault();
  event.stopPropagation();
}

/**
 * Ouverture par le parent, sans passer par le déclencheur : menu contextuel
 * (clic droit, Maj+F10). Le focus revient à l'élément actif à l'ouverture.
 */
defineExpose({ open, close });

function role(item: MenuItem): "menuitem" | "menuitemcheckbox" | "menuitemradio" {
  if (item.checked === undefined) return "menuitem";
  return item.radio === true ? "menuitemradio" : "menuitemcheckbox";
}
</script>

<template>
  <div ref="root" class="menu">
    <slot name="trigger" :open="isOpen" :attrs="triggerAttrs" />
    <div
      v-if="isOpen"
      ref="popover"
      class="menu__popover"
      :style="at(menuAt)"
      @keydown="onMenuKeydown"
    >
      <p v-if="heading" class="menu__heading">{{ heading }}</p>
      <ul :id="menuId" role="menu" :aria-label="label" class="menu__list">
        <template v-for="(item, index) in items" :key="item.id">
          <li v-if="item.separatorBefore" role="separator" class="menu__separator" />
          <li
            :role="role(item)"
            tabindex="-1"
            class="menu__item"
            :class="{
              'menu__item--danger': item.tone === 'danger',
              'menu__item--active': index === active,
            }"
            :data-level="0"
            :data-index="index"
            :aria-disabled="item.disabled === true ? 'true' : undefined"
            :aria-checked="item.checked === undefined ? undefined : item.checked"
            :aria-haspopup="hasChildren(item) ? 'menu' : undefined"
            :aria-expanded="hasChildren(item) ? subOpen === index : undefined"
            @click="activate(index)"
            @mouseenter="hover(index)"
          >
            <span v-if="item.checked !== undefined" class="menu__check" aria-hidden="true">
              {{ item.checked ? "✓" : "" }}
            </span>
            <span
              v-if="item.swatch"
              class="menu__swatch"
              :style="{ background: item.swatch }"
              aria-hidden="true"
            />
            <span class="menu__text">
              <span class="menu__label"
                >{{ item.label }}<span v-if="item.tag" class="menu__tag">{{ item.tag }}</span></span
              >
              <span v-if="item.hint" class="menu__hint">{{ item.hint }}</span>
            </span>
            <span v-if="item.detail" class="menu__detail">{{ item.detail }}</span>
            <kbd v-if="item.shortcut" class="menu__shortcut">{{ item.shortcut }}</kbd>
            <span v-if="hasChildren(item)" class="menu__arrow" aria-hidden="true">▸</span>
            <ul
              v-if="subOpen === index"
              role="menu"
              :aria-label="item.label"
              class="menu__list menu__list--sub"
              :style="at(subAt)"
            >
              <li
                v-for="(child, childIndex) in item.children"
                :key="child.id"
                :role="role(child)"
                tabindex="-1"
                class="menu__item"
                :class="{
                  'menu__item--danger': child.tone === 'danger',
                  'menu__item--active': childIndex === subActive,
                }"
                :data-level="1"
                :data-index="childIndex"
                :aria-disabled="child.disabled === true ? 'true' : undefined"
                :aria-checked="child.checked === undefined ? undefined : child.checked"
                @click.stop="activateSub(childIndex)"
                @mouseenter="hoverSub(childIndex)"
              >
                <span v-if="child.checked !== undefined" class="menu__check" aria-hidden="true">
                  {{ child.checked ? "✓" : "" }}
                </span>
                <span class="menu__text">
                  <span class="menu__label">{{ child.label }}</span>
                  <span v-if="child.hint" class="menu__hint">{{ child.hint }}</span>
                </span>
              </li>
            </ul>
          </li>
        </template>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.menu {
  display: inline-block;
}

/* Position fixe : `top` et `left` viennent de place(). */
.menu__popover {
  position: fixed;
  z-index: 20;
  width: max-content;
  min-width: 15rem;
  max-width: min(26rem, 90vw);
  padding: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.menu__heading {
  margin: 0.25rem 0.625rem 0.375rem;
  font: 600 0.6875rem / 1.4 var(--font-mono);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-muted);
}

.menu__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.menu__list--sub {
  position: fixed;
  z-index: 21;
  width: max-content;
  min-width: 13rem;
  max-width: min(26rem, 90vw);
  padding: 0.375rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-surface);
  box-shadow: var(--shadow-popover);
}

.menu__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem 0.625rem;
  border-radius: 6px;
  color: var(--color-text);
  cursor: pointer;
  outline: none;
}

.menu__item--active {
  background: var(--color-accent-soft);
}

.menu__item:focus-visible {
  outline: 2px solid var(--color-focus);
  outline-offset: -2px;
}

.menu__item[aria-disabled="true"] {
  cursor: default;
  color: var(--color-subtle);
  background: none;
}

.menu__item--danger {
  color: var(--color-danger);
}

.menu__check {
  width: 1rem;
  text-align: center;
}

.menu__text {
  display: flex;
  flex: 1;
  flex-direction: column;
}

.menu__hint {
  font-size: 0.8125rem;
  color: var(--color-muted);
}

.menu__shortcut,
.menu__detail {
  font: 0.75rem var(--font-mono);
  color: var(--color-muted);
}

.menu__detail {
  max-width: 12rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.menu__swatch {
  flex: none;
  width: 0.625rem;
  height: 0.625rem;
  border-radius: 2px;
}

/* L'espace avant la mention fait partie du nom accessible (« Recette protégé »). */
.menu__tag::before {
  content: " ";
}

.menu__tag {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-link);
}

.menu__arrow {
  color: var(--color-muted);
}

.menu__separator {
  height: 1px;
  margin: 0.375rem 0.5rem;
  background: var(--color-border);
}
</style>
