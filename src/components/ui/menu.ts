/** Entrée d'un menu (`BaseMenu`). */
export interface MenuItem {
  /** Identifiant renvoyé par l'événement `select`. */
  id: string;
  label: string;
  /** Sous-titre (« Le projet reste dans « Tous les projets ». »). */
  hint?: string;
  /** Raccourci affiché à droite (« Ctrl+N »). */
  shortcut?: string;
  /** Action destructrice, affichée en rouge. */
  tone?: "danger";
  disabled?: boolean;
  /** Défini pour une entrée à cocher (`menuitemcheckbox`). */
  checked?: boolean;
  /** Avec `checked` : choix exclusif (`menuitemradio`), comme un environnement. */
  radio?: boolean;
  /** Couleur d'une pastille devant le libellé (`var(--color-…)`). */
  swatch?: string;
  /** Mention courte après le libellé (« protégé »). */
  tag?: string;
  /** Détail à droite, en police à chasse fixe (« localhost:8080 »). */
  detail?: string;
  /** Trait de séparation avant l'entrée. */
  separatorBefore?: boolean;
  /** Sous-menu (un seul niveau). */
  children?: MenuItem[];
}

/** Attributs et écouteurs à lier au déclencheur du menu (`v-bind="attrs"`). */
export interface MenuTriggerAttrs {
  "aria-haspopup": "menu";
  "aria-expanded": boolean;
  "aria-controls": string | undefined;
  onClick: () => void;
  onKeydown: (event: KeyboardEvent) => void;
}

/** Rectangle à l'écran, en pixels (comme `getBoundingClientRect`). */
export interface Box {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/** Dimensions d'un menu ou de la fenêtre, en pixels. */
export interface Size {
  width: number;
  height: number;
}

/** Coin haut gauche d'un menu en position fixe. */
export interface Point {
  top: number;
  left: number;
}

/** Marge gardée entre un menu et le bord de la fenêtre. */
export const VIEWPORT_MARGIN = 8;

/** Garde `[start, start + size]` dans `[margin, limit - margin]`, le début d'abord. */
function clamp(start: number, size: number, limit: number): number {
  return Math.max(VIEWPORT_MARGIN, Math.min(start, limit - size - VIEWPORT_MARGIN));
}

/**
 * Place un menu sous son déclencheur, aligné sur son bord de début ou de fin, sans sortir de la
 * fenêtre : au-dessus s'il manque de place dessous et qu'il y en a plus dessus. Le menu est en
 * position fixe, donc jamais rogné par une colonne qui défile (barre latérale, zone).
 */
export function placeBelow(
  anchor: Box,
  menu: Size,
  viewport: Size,
  align: "start" | "end",
  gap = 4,
): Point {
  const below = anchor.bottom + gap;
  const roomBelow = viewport.height - VIEWPORT_MARGIN - below;
  const roomAbove = anchor.top - gap - VIEWPORT_MARGIN;
  const top =
    menu.height <= roomBelow || roomBelow >= roomAbove ? below : anchor.top - gap - menu.height;
  const left = align === "end" ? anchor.right - menu.width : anchor.left;
  return {
    top: clamp(top, menu.height, viewport.height),
    left: clamp(left, menu.width, viewport.width),
  };
}

/**
 * Place un sous-menu à côté de son entrée, du côté préféré (`end` : à droite), de l'autre côté
 * s'il n'y tient pas ; sa première entrée s'aligne sur celle du parent, sans sortir de la fenêtre.
 */
export function placeBeside(
  anchor: Box,
  menu: Size,
  viewport: Size,
  prefer: "start" | "end",
  gap = 6,
): Point {
  const after = anchor.right + gap;
  const before = anchor.left - gap - menu.width;
  const fitsAfter = after + menu.width <= viewport.width - VIEWPORT_MARGIN;
  const fitsBefore = before >= VIEWPORT_MARGIN;
  const useAfter = prefer === "end" ? fitsAfter || !fitsBefore : fitsAfter && !fitsBefore;
  return {
    top: clamp(anchor.top - gap, menu.height, viewport.height),
    left: clamp(useAfter ? after : before, menu.width, viewport.width),
  };
}
