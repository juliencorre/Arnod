/**
 * Propriétés des composants d'Arnod, pour documentation : les composants sont écrits en Vue
 * (src/components/), les aperçus de ce design system sont des rendus statiques.
 */

/** Bouton (BaseButton.vue). Libellé dans le contenu. */
export interface ButtonProps {
  /** Par défaut `secondary`. Un seul `primary` par vue. */
  variant?: "primary" | "secondary" | "danger" | "link";
  /** Par défaut `button`. */
  type?: "button" | "submit";
  disabled?: boolean;
}

/** Interrupteur (ToggleSwitch.vue). Libellé dans le contenu, état par `v-model`. */
export interface ToggleProps {
  modelValue: boolean;
  disabled?: boolean;
}

/** Champ de recherche (SearchInput.vue). Texte par `v-model` ; Échap le vide. */
export interface SearchInputProps {
  modelValue: string;
  /** Nom accessible, repris comme texte indicatif. */
  label: string;
  placeholder?: string;
}

/** Entrée d'un menu (menu.ts). */
export interface MenuItem {
  id: string;
  label: string;
  /** Seconde ligne, en `color-muted`. */
  hint?: string;
  /** Mention après le libellé, en `color-link` (« protégé »). */
  tag?: string;
  /** Valeur en police à chasse fixe, à droite. */
  detail?: string;
  shortcut?: string;
  /** Couleur d'un nuancier (valeur CSS, en général `var(--color-ws-…)`). */
  swatch?: string;
  /** Choix : ✓ quand vrai. */
  checked?: boolean;
  tone?: "danger";
  disabled?: boolean;
  separatorBefore?: boolean;
  children?: MenuItem[];
}

/** Menu (BaseMenu.vue). Déclencheur dans l'emplacement `trigger`. */
export interface MenuProps {
  items: MenuItem[];
  /** Nom accessible du menu. */
  label: string;
  /** En-tête en capitales, au-dessus des entrées. */
  heading?: string;
}

/** Fenêtre modale (BaseDialog.vue). Contenu par défaut, boutons dans `footer`, événement `close`. */
export interface DialogProps {
  open: boolean;
  title: string;
  description?: string;
  /** 46 rem au lieu de 36 rem. */
  wide?: boolean;
}

/** Notification (store `toasts`, affichée par ToastHost.vue). */
export interface Toast {
  message: string;
  tone: "info" | "error";
  action?: { label: string };
}

/** En-tête d'une section de la barre latérale (SectionHeading.vue). Actions dans le contenu. */
export interface SectionHeadingProps {
  icon:
    | "requests"
    | "contracts"
    | "dashboard"
    | "products"
    | "requirements"
    | "mocks"
    | "templates"
    | "tests"
    | "suites"
    | "campaigns"
    | "anomalies";
  title: string;
  /** Masqué à zéro. */
  count?: number;
}

/** Pastille de workspace (ColorDot.vue). */
export interface ColorDotProps {
  color: "gray" | "orange" | "blue" | "green" | "purple" | "red" | "yellow" | "teal";
  /** Par défaut `md`. */
  size?: "sm" | "md";
}

/** Marque de la dernière exécution (StatusMark.vue). Rien si jamais exécutée. */
export interface StatusMarkProps {
  /** `at` : fin de l'exécution, en millisecondes depuis l'époque Unix. */
  status?: { passed: boolean; at: number };
}

/** Pastille de nature d'étape (NaturePill.vue). */
export interface NaturePillProps {
  family: "calls" | "data" | "messages" | "jobs" | "interface" | "code" | "files";
  /** Nom de la nature (« REST »). */
  name: string;
  /** Par défaut `sm`. */
  size?: "sm" | "md";
}
