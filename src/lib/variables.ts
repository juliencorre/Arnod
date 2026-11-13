/**
 * Variables inconnues soulignées pendant la saisie (docs/plan-mvp-b-saisie-curl.md, 5.1-b ter) :
 * découpage d'affichage d'un texte et directive `v-variables`. Le cœur décide quelles
 * variables sont connues (`request_input`) ; ici, on ne fait que surligner.
 */
import { computed, inject, type ComputedRef, type Directive, type InjectionKey } from "vue";

/** Variables connues de la requête éditée ; `null` : pas encore lues, rien n'est souligné. */
export const KNOWN_VARIABLES: InjectionKey<ComputedRef<ReadonlySet<string> | null>> =
  Symbol("variables connues");

/** Variables connues fournies par l'éditeur de la requête, s'il y en a un. */
export function useKnownVariables(): ComputedRef<ReadonlySet<string> | null> {
  return inject(
    KNOWN_VARIABLES,
    computed(() => null),
  );
}

/** Morceau d'un texte : texte simple, ou `{{nom}}` inconnu. */
export interface Segment {
  text: string;
  unknown: boolean;
}

/**
 * Découpe `text` autour de ses `{{nom}}` (comme le cœur : `{{` puis `}}`, nom sans espaces
 * autour). Une valeur dynamique `{{$…}}` n'est jamais inconnue (R1).
 */
export function variableSegments(text: string, known: ReadonlySet<string>): Segment[] {
  const segments: Segment[] = [];
  let rest = text;
  for (;;) {
    const start = rest.indexOf("{{");
    const end = start < 0 ? -1 : rest.indexOf("}}", start + 2);
    if (start < 0 || end < 0) break;
    const name = rest.slice(start + 2, end).trim();
    const unknown = name !== "" && !name.startsWith("$") && !known.has(name);
    if (start > 0) segments.push({ text: rest.slice(0, start), unknown: false });
    segments.push({ text: rest.slice(start, end + 2), unknown });
    rest = rest.slice(end + 2);
  }
  if (rest !== "") segments.push({ text: rest, unknown: false });
  return segments;
}

type Field = HTMLInputElement | HTMLTextAreaElement;

interface MarkState {
  overlay: HTMLElement;
  known: ReadonlySet<string> | null;
  observer: ResizeObserver | null;
  sync: () => void;
}

const states = new WeakMap<Field, MarkState>();

/** Propriétés recopiées du champ, pour que le calque tombe lettre pour lettre sur le texte. */
const COPIED = [
  "fontFamily",
  "fontSize",
  "fontWeight",
  "fontStyle",
  "letterSpacing",
  "textIndent",
  "textAlign",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
  "boxSizing",
] as const;

function place(field: Field, overlay: HTMLElement): void {
  const style = getComputedStyle(field);
  for (const key of COPIED) overlay.style[key] = style[key];
  if (field instanceof HTMLTextAreaElement) {
    overlay.style.lineHeight = style.lineHeight;
    overlay.style.whiteSpace = "pre-wrap";
  } else {
    // Un champ d'une ligne centre son texte : le calque fait de même.
    const inner =
      field.clientHeight -
      Number.parseFloat(style.paddingTop) -
      Number.parseFloat(style.paddingBottom);
    overlay.style.lineHeight = inner > 0 ? `${String(inner)}px` : style.lineHeight;
    overlay.style.whiteSpace = "pre";
  }
  overlay.style.left = `${String(field.offsetLeft)}px`;
  overlay.style.top = `${String(field.offsetTop)}px`;
  overlay.style.width = `${String(field.offsetWidth)}px`;
  overlay.style.height = `${String(field.offsetHeight)}px`;
  overlay.scrollLeft = field.scrollLeft;
  overlay.scrollTop = field.scrollTop;
}

function render(field: Field, state: MarkState): void {
  const segments = state.known ? variableSegments(field.value, state.known) : [];
  const unknown = segments.filter((s) => s.unknown).map((s) => s.text);
  state.overlay.hidden = unknown.length === 0;
  if (unknown.length === 0) {
    state.overlay.replaceChildren();
    field.removeAttribute("data-unknown-variables");
    return;
  }
  state.overlay.replaceChildren(
    ...segments.map((s) => {
      if (!s.unknown) return document.createTextNode(s.text);
      const mark = document.createElement("mark");
      mark.textContent = s.text;
      return mark;
    }),
  );
  // Pour les tests et les lecteurs d'écran : le nom des variables inconnues du champ.
  field.dataset.unknownVariables = unknown.join(" ");
  place(field, state.overlay);
}

/**
 * `v-variables="known"` sur un champ ou une zone de texte : souligne en rouge chaque `{{nom}}`
 * absent de `known`. Un calque transparent, sans clic, posé sur le champ, porte le soulignement.
 */
export const vVariables: Directive<Field, ReadonlySet<string> | null> = {
  mounted(field, binding) {
    const overlay = document.createElement("div");
    overlay.className = "variable-marks";
    overlay.setAttribute("aria-hidden", "true");
    overlay.hidden = true;
    field.insertAdjacentElement("afterend", overlay);
    const parent = field.parentElement;
    if (parent && getComputedStyle(parent).position === "static") {
      parent.style.position = "relative";
    }
    const state: MarkState = {
      overlay,
      known: binding.value ?? null,
      observer: null,
      sync: () => {
        render(field, state);
      },
    };
    field.addEventListener("input", state.sync);
    field.addEventListener("scroll", state.sync);
    if (typeof ResizeObserver !== "undefined") {
      state.observer = new ResizeObserver(state.sync);
      state.observer.observe(field);
    }
    states.set(field, state);
    render(field, state);
  },
  updated(field, binding) {
    const state = states.get(field);
    if (!state) return;
    state.known = binding.value ?? null;
    render(field, state);
  },
  beforeUnmount(field) {
    const state = states.get(field);
    if (!state) return;
    field.removeEventListener("input", state.sync);
    field.removeEventListener("scroll", state.sync);
    state.observer?.disconnect();
    state.overlay.remove();
    states.delete(field);
  },
};
