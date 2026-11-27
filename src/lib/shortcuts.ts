/**
 * Raccourcis clavier (écran 1.6-f bis, `docs/plan-v1.md`) : lecture d'une touche pressée sous
 * forme canonique et affichage en français. Le cœur tient la liste des actions, valide une
 * combinaison et signale les conflits (`src-tauri/src/interface.rs`) ; ce module ne fait que
 * traduire le clavier.
 *
 * Forme canonique : `Ctrl+Alt+Shift+Touche`. Lettres et chiffres d'après la touche physique
 * (`event.code`, pour l'AZERTY : Ctrl+1 donne « & »), comme la barre oblique inverse ; les signes
 * d'après le caractère tapé (Ctrl+, sur toute disposition). Cmd vaut Ctrl.
 */

/**
 * Combinaisons par défaut, identiques à celles du cœur (`interface::ACTIONS`, vérifié par un
 * test Rust) : l'interface s'en sert tant qu'elle n'a pas lu les préférences du poste.
 */
export const DEFAULT_SHORTCUTS = {
  send: ["Ctrl+Enter"],
  copyCurl: ["Ctrl+Shift+C"],
  filterResponse: ["Ctrl+F"],
  newTest: ["Ctrl+N"],
  newFolder: ["Ctrl+Shift+N"],
  filterTree: ["Ctrl+F"],
  saveContract: ["Ctrl+S"],
  togglePlacement: ["Ctrl+J"],
  toggleSidebar: ["Ctrl+Backslash"],
  search: ["Ctrl+K"],
  nextTab: ["Ctrl+Tab", "Ctrl+Shift+Tab"],
  closeTab: ["Ctrl+W"],
  environments: ["Ctrl+Shift+E"],
  manageWorkspaces: ["Ctrl+Shift+W"],
  workspaces: ["Ctrl+1…9", "Ctrl+0"],
  preferences: ["Ctrl+,"],
};

/** Action qui a un raccourci. */
export type ShortcutId = keyof typeof DEFAULT_SHORTCUTS;

/** Touches lues d'après le caractère tapé (`event.key`). */
const NAMED: Record<string, string> = {
  Tab: "Tab",
  " ": "Space",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
};
const SIGNS = new Set([",", ".", ";", "/", "'", "[", "]", "-", "=", "`"]);

/** Touche principale d'un événement, sous forme canonique ; `null` pour un modificateur seul. */
function keyOf(event: KeyboardEvent): string | null {
  const code = event.code;
  const letter = /^Key([A-Z])$/.exec(code)?.[1];
  if (letter !== undefined) return letter;
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(code)?.[1];
  if (digit !== undefined) return digit;
  if (/^F(?:[1-9]|1[0-2])$/.test(code)) return code;
  if (code === "Backslash") return "Backslash";
  if (code === "Enter" || code === "NumpadEnter" || event.key === "Enter") return "Enter";
  const named = NAMED[event.key];
  if (named !== undefined) return named;
  if (SIGNS.has(event.key)) return event.key;
  // Lettre sans code physique (événement de test, clavier virtuel).
  if (/^[a-z0-9]$/i.test(event.key)) return event.key.toUpperCase();
  return null;
}

/** Combinaison canonique d'une touche pressée, `null` pour un modificateur seul ou Échap. */
export function comboOf(event: KeyboardEvent): string | null {
  const key = keyOf(event);
  if (key === null) return null;
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** La touche pressée est l'une des combinaisons `keys`. */
export function matchesKeys(event: KeyboardEvent, keys: readonly string[]): boolean {
  const combo = comboOf(event);
  return combo !== null && keys.includes(combo);
}

const LABELS: Record<string, string> = {
  Shift: "Maj",
  Enter: "Entrée",
  Backslash: "\\",
  Space: "Espace",
  Backspace: "Retour arrière",
  Delete: "Suppr",
  Insert: "Inser",
  Home: "Début",
  End: "Fin",
  PageUp: "Page préc.",
  PageDown: "Page suiv.",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

/** « Ctrl+Maj+K », « Ctrl+Entrée », « Ctrl+\ » : une combinaison telle qu'on la lit. */
export function formatCombo(combo: string): string {
  // « Ctrl++ » n'existe pas : le dernier segment est toujours la touche.
  return combo
    .split("+")
    .map((part) => LABELS[part] ?? part)
    .join("+");
}
