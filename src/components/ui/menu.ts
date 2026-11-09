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
