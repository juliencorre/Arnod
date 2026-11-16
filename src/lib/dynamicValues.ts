/**
 * Liste des valeurs dynamiques proposée à la frappe de `{{$` (écran 5.1-h) : repérage de la saisie
 * en cours, filtre d'affichage et insertion. Le cœur fournit les noms, leurs descriptions et un
 * exemple (`dynamic_values`) et tire les valeurs à l'envoi ; ici, on ne fait qu'aider la frappe.
 */
import type { DynamicValue } from "@/types/ipc";

/** Saisie d'une valeur dynamique en cours : `{{` à `start`, texte tapé après le `$`. */
export interface DynamicQuery {
  start: number;
  query: string;
}

/** Caractère possible dans un nom de valeur dynamique en cours de frappe (`$date:+3d`). */
function isNameChar(c: string): boolean {
  return /^[A-Za-z0-9:+-]$/.test(c);
}

/**
 * La saisie `{{$…` qui finit au curseur `caret`, ou `null` : rien qu'un nom en cours de frappe
 * entre `{{$` et le curseur (pas d'espace, pas de `}}`).
 */
export function dynamicQuery(text: string, caret: number): DynamicQuery | null {
  let i = caret;
  while (i > 0 && isNameChar(text.charAt(i - 1))) i--;
  if (i < 3 || text.slice(i - 3, i) !== "{{$") return null;
  return { start: i - 3, query: text.slice(i, caret) };
}

/** Sans casse ni accents, pour comparer. */
function fold(s: string): string {
  return s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

/**
 * Valeurs qui répondent à `query` (le texte tapé après `$`) : celles dont le nom commence par lui,
 * puis celles dont le nom ou la description le contient, dans l'ordre du cœur.
 */
export function filterDynamic(list: readonly DynamicValue[], query: string): DynamicValue[] {
  const q = fold(query);
  if (q === "") return [...list];
  const name = (v: DynamicValue): string => fold(v.name.slice(1));
  const first = list.filter((v) => name(v).startsWith(q));
  const then = list.filter(
    (v) => !name(v).startsWith(q) && (name(v).includes(q) || fold(v.description).includes(q)),
  );
  return [...first, ...then];
}

/** Texte et position du curseur après l'insertion de `{{name}}` à la place de la saisie. */
export function insertDynamic(
  text: string,
  at: DynamicQuery,
  caret: number,
  name: string,
): { text: string; caret: number } {
  let end = caret;
  // Le reste d'un nom déjà écrit après le curseur est remplacé, et un `}}` qui suit est gardé.
  while (end < text.length && isNameChar(text.charAt(end))) end++;
  const rest = text.startsWith("}}", end) ? text.slice(end + 2) : text.slice(end);
  const inserted = `{{${name}}}`;
  return {
    text: text.slice(0, at.start) + inserted + rest,
    caret: at.start + inserted.length,
  };
}

/** Propriétés recopiées du champ dans le miroir qui mesure la position du curseur. */
const MIRRORED = [
  "boxSizing",
  "width",
  "fontFamily",
  "fontSize",
  "fontWeight",
  "fontStyle",
  "letterSpacing",
  "lineHeight",
  "textIndent",
  "tabSize",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
] as const;

/**
 * Rectangle à l'écran de la ligne du caractère `index` d'un champ (`top`, `bottom`, `left`), mesuré
 * par un miroir invisible qui reprend le texte et la mise en forme du champ.
 */
export function caretBox(
  field: HTMLInputElement | HTMLTextAreaElement,
  index: number,
): { top: number; bottom: number; left: number; right: number } {
  const style = getComputedStyle(field);
  const mirror = document.createElement("div");
  for (const key of MIRRORED) mirror.style[key] = style[key];
  mirror.style.position = "absolute";
  mirror.style.visibility = "hidden";
  mirror.style.top = "0";
  mirror.style.left = "-9999px";
  mirror.style.overflow = "hidden";
  mirror.style.whiteSpace = field instanceof HTMLTextAreaElement ? "pre-wrap" : "pre";
  mirror.textContent = field.value.slice(0, index);
  const mark = document.createElement("span");
  mark.textContent = "​";
  mirror.appendChild(mark);
  document.body.appendChild(mirror);
  const box = field.getBoundingClientRect();
  const line = Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.4;
  const top = box.top + mark.offsetTop - field.scrollTop;
  const left = Math.min(box.left + mark.offsetLeft - field.scrollLeft, box.right);
  mirror.remove();
  return { top, bottom: top + line, left, right: left };
}
